// Tire smoke: white-gray puff particles when a grounded wheel has high
// lateral slip (sideways wheel velocity > 4 m/s). Distinct from
// exhaust_smoke (back-of-chassis dark gray on heavy throttle).
//
// Puffs are white-gray spheres (radius 0.18) that fade out and grow over
// 1.0 s. Spawned at up to ~10 Hz per grounded wheel with |lateral slip| > 4 m/s.
// Capped at 80 live puffs.
//
// Public API:
//   TireSmokePlugin

use bevy::prelude::*;
use avian3d::prelude::{AngularVelocity, LinearVelocity};
use std::collections::VecDeque;

use crate::vehicle::{Chassis, Wheel, VehicleRoot};

// ---- Constants ---------------------------------------------------------------

/// Lateral velocity threshold (m/s) above which tire smoke spawns.
const SLIP_THRESHOLD: f32 = 4.0;
/// Cadence: one spawn check per 0.1 s (≈ 10 spawns/sec max).
const SPAWN_INTERVAL: f32 = 0.1;
/// Maximum live puff entities.
const MAX_PUFFS: usize = 80;
/// Visual radius of each puff sphere.
const PUFF_RADIUS: f32 = 0.18;
/// Starting alpha.
const INITIAL_ALPHA: f32 = 0.7;
/// Full lifetime of a puff in seconds.
const LIFETIME: f32 = 1.0;
/// Scale growth per second (multiplicative).
const SCALE_GROWTH: f32 = 0.7;

// ---- Components / Resources --------------------------------------------------

/// Per-puff state.
#[derive(Component)]
pub struct TireSmokePuff {
    pub age_s: f32,
    pub vel:   Vec3,
}

/// Ordered queue of live puff entities; front = oldest.
#[derive(Resource, Default)]
struct PuffQueue(VecDeque<Entity>);

// ---- Plugin ------------------------------------------------------------------

pub struct TireSmokePlugin;

impl Plugin for TireSmokePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PuffQueue>()
           .add_systems(Update, (spawn_slip_puffs, tick_puffs));
    }
}

// ---- Helpers -----------------------------------------------------------------

/// Cheap deterministic pseudo-random value in [-1, 1] from a scalar seed.
/// Uses sin() * large-prime fractional trick — avoids the `rand` crate.
fn pseudo_rand(seed: f32) -> f32 {
    let v = (seed.sin() * 43_758.545_3_f32).fract();
    v * 2.0 - 1.0
}

// ---- spawn_slip_puffs (Update) -----------------------------------------------

fn spawn_slip_puffs(
    mut commands:    Commands,
    mut meshes:      ResMut<Assets<Mesh>>,
    mut materials:   ResMut<Assets<StandardMaterial>>,
    vehicle:         Option<Res<VehicleRoot>>,
    chassis_q:       Query<(&Transform, &LinearVelocity, &AngularVelocity), With<Chassis>>,
    // Without<Chassis> and Without<Camera3d> avoid query conflicts with vehicle.rs wheel query
    wheel_q:         Query<&Wheel, (Without<Chassis>, Without<Camera3d>)>,
    mut timer:       Local<f32>,
    mut queue:       ResMut<PuffQueue>,
    time:            Res<Time>,
) {
    let Some(vehicle) = vehicle else { return };

    let dt = time.delta_secs();
    *timer += dt;
    if *timer < SPAWN_INTERVAL {
        return;
    }
    *timer -= SPAWN_INTERVAL;

    let Ok((chassis_tf, lin_vel, ang_vel)) = chassis_q.get(vehicle.chassis) else { return };

    let chassis_world_pos = chassis_tf.translation;
    let chassis_rot       = chassis_tf.rotation;
    let chassis_right     = (chassis_rot * Vec3::X).normalize();
    let lin_vel_v: Vec3   = lin_vel.0;
    let ang_vel_v: Vec3   = ang_vel.0;

    // Shared mesh for all puffs spawned this tick.
    let mesh_handle = meshes.add(Sphere::new(PUFF_RADIUS).mesh().ico(1).unwrap());

    let seed_base = time.elapsed_secs() * 1_000.0;
    let mut seed_offset = 0.0_f32;

    for wheel in &wheel_q {
        if !wheel.is_grounded {
            continue;
        }

        // Wheel world position: chassis_world + chassis_rot * wheel_local_pos.
        // We use the WHEEL_OFFSETS baked into the Wheel index. The Wheel component
        // stores the index; look up the canonical local offset from vehicle constants.
        // Rather than importing a private constant, we recompute from wheel.index
        // using the same offsets as vehicle.rs.
        let local_pos = wheel_local_offset(wheel.index);
        let wheel_world_pos = chassis_world_pos + chassis_rot * local_pos;

        // Wheel world velocity = chassis_lin_vel + chassis_ang_vel × r
        let r = wheel_world_pos - chassis_world_pos;
        let wheel_vel = lin_vel_v + ang_vel_v.cross(r);

        // Lateral component: project onto chassis right (+X) axis.
        let lateral = wheel_vel.dot(chassis_right);

        if lateral.abs() <= SLIP_THRESHOLD {
            continue;
        }

        // Unique seed per wheel per tick.
        seed_offset += 10.0;
        let s = seed_base + seed_offset + wheel.index as f32 * 100.0;

        // Random drift velocity for puff.
        let vel = Vec3::new(
            pseudo_rand(s)       * 0.3,
            0.5,
            pseudo_rand(s + 1.0) * 0.3,
        );

        // Each puff gets its own material so alpha can be mutated independently.
        let mat = materials.add(StandardMaterial {
            base_color: Color::srgba(0.85, 0.85, 0.88, INITIAL_ALPHA),
            alpha_mode: AlphaMode::Blend,
            ..default()
        });

        let entity = commands.spawn((
            TireSmokePuff { age_s: 0.0, vel },
            Mesh3d(mesh_handle.clone()),
            MeshMaterial3d(mat),
            Transform::from_translation(wheel_world_pos),
        )).id();

        // Cap: despawn oldest when over the limit.
        if queue.0.len() >= MAX_PUFFS {
            if let Some(oldest) = queue.0.pop_front() {
                commands.entity(oldest).despawn();
            }
        }
        queue.0.push_back(entity);
    }
}

// ---- tick_puffs (Update) -----------------------------------------------------

fn tick_puffs(
    mut commands:  Commands,
    mut queue:     ResMut<PuffQueue>,
    mut puffs:     Query<(Entity, &mut Transform, &mut TireSmokePuff, &MeshMaterial3d<StandardMaterial>)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    time:          Res<Time>,
) {
    let dt = time.delta_secs();
    let mut expired: Vec<Entity> = Vec::new();

    for (entity, mut transform, mut puff, mat_handle) in puffs.iter_mut() {
        puff.age_s += dt;

        if puff.age_s >= LIFETIME {
            expired.push(entity);
            commands.entity(entity).despawn();
            continue;
        }

        // Integrate position.
        transform.translation += puff.vel * dt;

        // Grow scale multiplicatively.
        transform.scale *= 1.0 + dt * SCALE_GROWTH;

        // Decay alpha: INITIAL_ALPHA → 0 linearly over LIFETIME seconds.
        let t = (puff.age_s / LIFETIME).clamp(0.0, 1.0);
        let alpha = INITIAL_ALPHA * (1.0 - t);

        if let Some(mat) = materials.get_mut(&mat_handle.0) {
            mat.base_color = Color::srgba(0.85, 0.85, 0.88, alpha);
        }
    }

    // Remove expired entities from the queue.
    if !expired.is_empty() {
        queue.0.retain(|e| !expired.contains(e));
    }
}

// ---- Local offset lookup -----------------------------------------------------

/// Returns the chassis-local wheel anchor position for a given wheel index.
/// Mirrors the WHEEL_OFFSETS constant in vehicle.rs (FL=0, FR=1, RL=2, RR=3).
#[inline]
fn wheel_local_offset(index: usize) -> Vec3 {
    match index {
        0 => Vec3::new(-1.1, -0.35, -1.4), // FL
        1 => Vec3::new( 1.1, -0.35, -1.4), // FR
        2 => Vec3::new(-1.1, -0.35,  1.4), // RL
        _ => Vec3::new( 1.1, -0.35,  1.4), // RR (index 3 or any unexpected)
    }
}
