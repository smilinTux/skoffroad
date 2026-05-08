// Splash particles: dark-blue water/mud spray when a wheel enters a puddle
// while moving > 3 m/s. Reads MudPuddle component from mud_puddles.rs (placed
// by MudPuddlesPlugin at runtime). Spawns 8 sphere particles per hit in an
// upper-hemisphere fan, blended with chassis velocity so splashes spray in the
// direction of motion.
//
// Public API:
//   SplashParticlesPlugin
//   MudPuddle { center: Vec3, radius: f32 }   ← placed by mud_puddles.rs agent;
//   defined here so the crate compiles while mud_puddles.rs is still a stub.
//
// Sprint 32 — particles agent.

use bevy::prelude::*;
use avian3d::prelude::LinearVelocity;
use std::collections::VecDeque;

use crate::vehicle::{Chassis, Wheel, VehicleRoot};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Minimum chassis speed (m/s) required to trigger a splash.
const MIN_SPEED: f32 = 3.0;
/// How often (seconds) we may re-fire splashes for the *same* puddle.
/// Limits to ~5 spawns/sec (0.2 s cooldown between spawns for a given puddle).
const SPAWN_INTERVAL: f32 = 0.2;
/// Number of splash sphere particles spawned per puddle hit.
const PARTICLES_PER_HIT: usize = 8;
/// Maximum number of live splash entities before oldest-first culling.
const MAX_SPLASHES: usize = 100;
/// Radius of each splash sphere (metres).
const SPHERE_RADIUS: f32 = 0.10;
/// Starting alpha for the water/mud colour.
const INITIAL_ALPHA: f32 = 0.85;
/// Full lifetime of a splash particle (seconds).
const LIFETIME: f32 = 0.8;
/// Gravity constant applied to particle y-velocity.
const GRAVITY: f32 = 9.81;
/// Minimum outward speed (m/s) for a splash particle.
const VEL_MIN: f32 = 2.0;
/// Maximum outward speed (m/s) for a splash particle.
const VEL_MAX: f32 = 5.0;
/// Fraction of chassis velocity blended into each particle's initial velocity
/// so splashes drift in the direction of travel.
const CHASSIS_VEL_BLEND: f32 = 0.35;

// MudPuddle is defined in crate::mud_puddles. Re-export so existing
// references inside this module compile unchanged.
pub use crate::mud_puddles::MudPuddle;

// ---------------------------------------------------------------------------
// Splash component
// ---------------------------------------------------------------------------

/// Per-particle state for a water/mud splash sphere.
#[derive(Component)]
pub struct Splash {
    /// Seconds since spawn.
    pub age_s: f32,
    /// Current velocity (m/s). Y is reduced by gravity each tick.
    pub vel: Vec3,
}

// ---------------------------------------------------------------------------
// Resources
// ---------------------------------------------------------------------------

/// Ordered queue of live splash entities; front = oldest.
#[derive(Resource, Default)]
struct SplashQueue(VecDeque<Entity>);

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct SplashParticlesPlugin;

impl Plugin for SplashParticlesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SplashQueue>()
           .add_systems(Update, (detect_puddle_hits, tick_splashes));
    }
}

// ---------------------------------------------------------------------------
// Helpers: pseudo-random (no external crate)
// ---------------------------------------------------------------------------

/// LCG step; returns a value in [0, 1).
#[inline]
fn lcg_next(seed: &mut u32) -> f32 {
    *seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    *seed as f32 / u32::MAX as f32
}

/// Chassis-local wheel anchor position, mirroring WHEEL_OFFSETS in vehicle.rs.
/// FL=0, FR=1, RL=2, RR=3.
#[inline]
fn wheel_local_offset(index: usize) -> Vec3 {
    match index {
        0 => Vec3::new(-1.1, -0.35, -1.4),
        1 => Vec3::new( 1.1, -0.35, -1.4),
        2 => Vec3::new(-1.1, -0.35,  1.4),
        _ => Vec3::new( 1.1, -0.35,  1.4),
    }
}

// ---------------------------------------------------------------------------
// System: detect_puddle_hits
// ---------------------------------------------------------------------------
//
// Gated to ~5 spawns/sec via a Local<f32> elapsed timer. For each grounded
// wheel, checks XZ distance to every MudPuddle. On a hit (and if the per-puddle
// cooldown has expired) spawns PARTICLES_PER_HIT splash spheres.
//
// Per-puddle cooldown: tracked via a Local<Vec<(Entity, f32)>> mapping puddle
// entity → seconds-since-last-fire.
// ---------------------------------------------------------------------------

fn detect_puddle_hits(
    mut commands:   Commands,
    mut meshes:     ResMut<Assets<Mesh>>,
    mut materials:  ResMut<Assets<StandardMaterial>>,
    vehicle:        Option<Res<VehicleRoot>>,
    chassis_q:      Query<(&Transform, &LinearVelocity), With<Chassis>>,
    wheel_q:        Query<&Wheel, (Without<Chassis>, Without<Camera3d>)>,
    puddle_q:       Query<(Entity, &MudPuddle)>,
    mut queue:      ResMut<SplashQueue>,
    time:           Res<Time>,
    // Seconds since the global spawn gate last opened.
    mut gate_timer: Local<f32>,
    // Per-puddle cooldown: (puddle_entity, seconds_since_last_spawn).
    mut puddle_cooldowns: Local<Vec<(Entity, f32)>>,
) {
    let dt = time.delta_secs();

    // Advance per-puddle cooldown timers.
    for (_, cooldown) in puddle_cooldowns.iter_mut() {
        *cooldown += dt;
    }

    // Global gate: open at most every SPAWN_INTERVAL seconds.
    *gate_timer += dt;
    if *gate_timer < SPAWN_INTERVAL {
        return;
    }
    *gate_timer -= SPAWN_INTERVAL;

    // Need vehicle resource and chassis query.
    let Some(vehicle) = vehicle else { return };
    let Ok((chassis_tf, lin_vel)) = chassis_q.get(vehicle.chassis) else { return };

    let chassis_vel: Vec3     = lin_vel.0;
    let chassis_speed: f32    = chassis_vel.length();
    if chassis_speed < MIN_SPEED {
        return;
    }

    let chassis_pos  = chassis_tf.translation;
    let chassis_rot  = chassis_tf.rotation;

    // Build mesh and base material once per tick (shared across all new particles).
    let mesh_handle = meshes.add(Sphere::new(SPHERE_RADIUS).mesh().ico(1).unwrap());

    // Seed from elapsed time; incremented per particle for variety.
    let mut seed = (time.elapsed_secs() * 1_000_000.0) as u32;

    // Collect grounded wheel world positions.
    let grounded_positions: Vec<Vec3> = wheel_q
        .iter()
        .filter(|w| w.is_grounded)
        .map(|w| chassis_pos + chassis_rot * wheel_local_offset(w.index))
        .collect();

    if grounded_positions.is_empty() {
        return;
    }

    // For each puddle, check if any grounded wheel is inside its radius (XZ only).
    for (puddle_entity, puddle) in puddle_q.iter() {
        // Find the wheel hit position for this puddle, if any.
        let hit_pos_opt = grounded_positions.iter().find(|&&wp| {
            let dx = wp.x - puddle.center.x;
            let dz = wp.z - puddle.center.z;
            (dx * dx + dz * dz).sqrt() <= puddle.radius
        });
        let Some(&hit_pos) = hit_pos_opt else { continue };

        // Check per-puddle cooldown.
        let cooldown_entry = puddle_cooldowns
            .iter_mut()
            .find(|(e, _)| *e == puddle_entity);

        if let Some((_, elapsed)) = cooldown_entry {
            if *elapsed < SPAWN_INTERVAL {
                continue;
            }
            // Reset cooldown.
            *elapsed = 0.0;
        } else {
            // First time seeing this puddle; register it.
            puddle_cooldowns.push((puddle_entity, 0.0));
        }

        // Spawn PARTICLES_PER_HIT splash spheres.
        for _ in 0..PARTICLES_PER_HIT {
            // Random direction in the upper hemisphere (y >= 0).
            let theta = lcg_next(&mut seed) * std::f32::consts::TAU; // azimuth 0..2π
            let phi   = lcg_next(&mut seed) * std::f32::consts::FRAC_PI_2; // elevation 0..π/2
            let sin_phi = phi.sin();
            let dir = Vec3::new(
                theta.cos() * sin_phi,
                phi.cos().abs(), // always upward
                theta.sin() * sin_phi,
            )
            .normalize_or(Vec3::Y);

            // Random magnitude in [VEL_MIN, VEL_MAX].
            let speed = VEL_MIN + lcg_next(&mut seed) * (VEL_MAX - VEL_MIN);
            // Add a fraction of chassis velocity so spray tracks motion direction.
            let vel = dir * speed + chassis_vel * CHASSIS_VEL_BLEND;

            // Per-particle material so alpha can be mutated independently.
            let mat = materials.add(StandardMaterial {
                base_color: Color::srgba(0.30, 0.40, 0.55, INITIAL_ALPHA),
                alpha_mode: AlphaMode::Blend,
                ..default()
            });

            let entity = commands.spawn((
                Splash { age_s: 0.0, vel },
                Mesh3d(mesh_handle.clone()),
                MeshMaterial3d(mat),
                Transform::from_translation(hit_pos + Vec3::Y * 0.05),
            )).id();

            // Cap: cull oldest when over the limit.
            if queue.0.len() >= MAX_SPLASHES {
                if let Some(oldest) = queue.0.pop_front() {
                    commands.entity(oldest).despawn();
                }
            }
            queue.0.push_back(entity);
        }
    }
}

// ---------------------------------------------------------------------------
// System: tick_splashes
// ---------------------------------------------------------------------------
//
// Each frame: integrate position, apply gravity, decay alpha, despawn at
// age >= LIFETIME.
// ---------------------------------------------------------------------------

fn tick_splashes(
    mut commands:  Commands,
    mut queue:     ResMut<SplashQueue>,
    mut splashes:  Query<(Entity, &mut Transform, &mut Splash, &MeshMaterial3d<StandardMaterial>)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    time:          Res<Time>,
) {
    let dt = time.delta_secs();
    let mut expired: Vec<Entity> = Vec::new();

    for (entity, mut transform, mut splash, mat_handle) in splashes.iter_mut() {
        splash.age_s += dt;

        if splash.age_s >= LIFETIME {
            expired.push(entity);
            commands.entity(entity).despawn();
            continue;
        }

        // Integrate position.
        transform.translation += splash.vel * dt;

        // Apply gravity to y-velocity.
        splash.vel.y -= GRAVITY * dt;

        // Alpha: linearly decay INITIAL_ALPHA → 0 over LIFETIME seconds.
        let t     = (splash.age_s / LIFETIME).clamp(0.0, 1.0);
        let alpha = INITIAL_ALPHA * (1.0 - t);

        if let Some(mat) = materials.get_mut(&mat_handle.0) {
            mat.base_color = Color::srgba(0.30, 0.40, 0.55, alpha);
        }
    }

    // Remove expired entities from the queue so the cap accounting stays accurate.
    if !expired.is_empty() {
        queue.0.retain(|e| !expired.contains(e));
    }
}
