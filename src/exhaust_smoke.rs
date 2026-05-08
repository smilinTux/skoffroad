// Exhaust smoke: emits dark gray puffs from the exhaust pipe location
// when the player is at heavy throttle (drive > 0.7). Independent of
// existing exhaust.rs (which handles flame backfire effects).
//
// Each puff is a small gray sphere (radius 0.15) that fades and rises
// over ~1.5 s, spawned at the chassis rear (local 0, -0.1, +1.9).
//
// Public API:
//   ExhaustSmokePlugin

use bevy::prelude::*;
use std::collections::VecDeque;

use crate::vehicle::{Chassis, DriveInput, VehicleRoot};

// ---- Constants ---------------------------------------------------------------

const HEAVY_THROTTLE: f32   = 0.7;
const SPAWN_INTERVAL: f32   = 0.05;  // seconds between puffs
const MAX_PUFFS: u32        = 80;
const LIFETIME: f32         = 1.5;   // seconds
const PUFF_RADIUS: f32      = 0.15;
const INITIAL_ALPHA: f32    = 0.6;

// Exhaust pipe in chassis local space (rear centre).
const EXHAUST_LOCAL: Vec3 = Vec3::new(0.0, -0.1, 1.9);

// ---- Components / Resources --------------------------------------------------

/// Per-puff state.
#[derive(Component)]
pub struct Smoke {
    pub age_s: f32,
    pub vel:   Vec3,
}

/// Ordered queue of live puff entities; front = oldest.
#[derive(Resource, Default)]
struct SmokeQueue(VecDeque<Entity>);

// ---- Plugin ------------------------------------------------------------------

pub struct ExhaustSmokePlugin;

impl Plugin for ExhaustSmokePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SmokeQueue>()
           .add_systems(Update, (spawn_puffs, tick_puffs));
    }
}

// ---- Helpers -----------------------------------------------------------------

/// Cheap deterministic pseudo-random value in [-1, 1] from a scalar seed.
/// Uses sin() * large-prime fractional trick — avoids the `rand` crate.
fn pseudo_rand(seed: f32) -> f32 {
    let v = (seed.sin() * 43_758.545_3_f32).fract();
    v * 2.0 - 1.0
}

// ---- spawn_puffs (Update) ----------------------------------------------------

fn spawn_puffs(
    mut commands:     Commands,
    mut meshes:       ResMut<Assets<Mesh>>,
    mut materials:    ResMut<Assets<StandardMaterial>>,
    vehicle:          Option<Res<VehicleRoot>>,
    chassis_q:        Query<&Transform, With<Chassis>>,
    input:            Res<DriveInput>,
    mut accumulator:  Local<f32>,
    mut count:        Local<u32>,
    mut queue:        ResMut<SmokeQueue>,
    time:             Res<Time>,
) {
    // Must have a chassis and be at heavy throttle.
    let Some(vehicle) = vehicle else { return };
    if input.drive < HEAVY_THROTTLE { return; }
    let Ok(chassis_tf) = chassis_q.get(vehicle.chassis) else { return };

    let dt = time.delta_secs();
    *accumulator += dt;

    // Spawn mesh once per interval tick.
    let mesh_handle = meshes.add(Sphere::new(PUFF_RADIUS).mesh().ico(1).unwrap());

    while *accumulator >= SPAWN_INTERVAL {
        *accumulator -= SPAWN_INTERVAL;

        // World-space spawn position: chassis origin + rotated local offset.
        let base_pos = chassis_tf.translation
            + chassis_tf.rotation * EXHAUST_LOCAL;

        // Small random offset ±0.1 on each axis, seeded from time + count.
        let seed_base = time.elapsed_secs() * 1_000.0 + *count as f32;
        let offset = Vec3::new(
            pseudo_rand(seed_base)         * 0.1,
            pseudo_rand(seed_base + 1.0)   * 0.1,
            pseudo_rand(seed_base + 2.0)   * 0.1,
        );
        let spawn_pos = base_pos + offset;

        // Initial velocity: upward + small random XZ drift.
        let random_x = pseudo_rand(seed_base + 3.0) * 0.5;
        let random_z = pseudo_rand(seed_base + 4.0) * 0.5;
        let vel = Vec3::new(random_x, 1.5, random_z);

        // Each puff gets its own material so alpha can be mutated independently.
        let mat = materials.add(StandardMaterial {
            base_color: Color::srgba(0.28, 0.28, 0.30, INITIAL_ALPHA),
            alpha_mode: AlphaMode::Blend,
            ..default()
        });

        let entity = commands.spawn((
            Smoke { age_s: 0.0, vel },
            Mesh3d(mesh_handle.clone()),
            MeshMaterial3d(mat),
            Transform::from_translation(spawn_pos),
        )).id();

        queue.0.push_back(entity);
        *count = count.wrapping_add(1);

        // Cap: despawn oldest when over the limit.
        while queue.0.len() as u32 > MAX_PUFFS {
            if let Some(oldest) = queue.0.pop_front() {
                commands.entity(oldest).despawn();
            }
        }
    }
}

// ---- tick_puffs (Update) -----------------------------------------------------

fn tick_puffs(
    mut commands:  Commands,
    mut queue:     ResMut<SmokeQueue>,
    mut puffs:     Query<(Entity, &mut Transform, &mut Smoke, &MeshMaterial3d<StandardMaterial>)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    time:          Res<Time>,
) {
    let dt = time.delta_secs();
    let mut expired: Vec<Entity> = Vec::new();

    for (entity, mut transform, mut smoke, mat_handle) in puffs.iter_mut() {
        smoke.age_s += dt;

        if smoke.age_s >= LIFETIME {
            expired.push(entity);
            commands.entity(entity).despawn();
            continue;
        }

        // Integrate position.
        transform.translation += smoke.vel * dt;

        // Grow scale slightly over lifetime.
        transform.scale *= 1.0 + dt * 0.5;

        // Decay alpha: 0.6 → 0 linearly over LIFETIME seconds.
        let t = (smoke.age_s / LIFETIME).clamp(0.0, 1.0);
        let alpha = INITIAL_ALPHA * (1.0 - t);

        if let Some(mat) = materials.get_mut(&mat_handle.0) {
            mat.base_color = Color::srgba(0.28, 0.28, 0.30, alpha);
        }
    }

    // Remove expired entities from the queue.
    if !expired.is_empty() {
        queue.0.retain(|e| !expired.contains(e));
    }
}
