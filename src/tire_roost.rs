// Tire roost / debris chunks — Effect 1 of Sprint 67.
//
// When a wheel has high slip (slip_factor > 0.4, computed in vehicle.rs
// update_wheel_visuals) spawn 2-4 small cuboid "debris chunks" per wheel
// per spawn-frame that fly out the back of the truck in a low arc.
//
// Chunks are kinematic (no avian collider): gravity applied manually.
// Despawn after 2 s. Cap at 80 active chunks.
//
// Public API:
//   TireRoostPlugin
//   RoostState (resource)

use bevy::prelude::*;
use avian3d::prelude::LinearVelocity;

use crate::vehicle::{Chassis, DriveInput, VehicleRoot};

// ---- Plugin -----------------------------------------------------------------

pub struct TireRoostPlugin;

impl Plugin for TireRoostPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RoostState>()
           .add_systems(Update, (
               ensure_roost_material,
               spawn_roost_chunks,
               update_roost_chunks,
           ));
    }
}

// ---- Resources / Components -------------------------------------------------

/// Per-chunk runtime state tracked inside the resource.
pub struct Chunk {
    pub entity:   Entity,
    pub velocity: Vec3,
    pub lifetime: f32,   // seconds remaining
}

#[derive(Resource, Default)]
pub struct RoostState {
    pub active:     Vec<Chunk>,
    pub mat_handle: Option<Handle<StandardMaterial>>,
    pub mesh_handle: Option<Handle<Mesh>>,
}

/// Marker on each debris chunk entity.
#[derive(Component)]
struct RoostChunk;

// ---- Constants --------------------------------------------------------------

// Sprint 90: raised threshold 0.4 → 0.5 — only genuine wheel-spin produces roost.
const SLIP_THRESHOLD:    f32 = 0.5;
const MAX_ACTIVE_CHUNKS: usize = 60;
// Sprint 90: shorter lifetime 1.5 s — chunks hit the ground faster (realistic arc).
const CHUNK_LIFETIME:    f32 = 1.5;
const GRAVITY:           f32 = 9.81;

// Rear wheel offsets in chassis local space (matches vehicle.rs; index 2=RL, 3=RR).
const WHEEL_OFFSETS: [Vec3; 4] = [
    Vec3::new(-1.1, -0.35, -1.4),
    Vec3::new( 1.1, -0.35, -1.4),
    Vec3::new(-1.1, -0.35,  1.4),
    Vec3::new( 1.1, -0.35,  1.4),
];

// ---- Ensure material + mesh exist (one-shot) --------------------------------

fn ensure_roost_material(
    mut roost: ResMut<RoostState>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    vehicle: Option<Res<VehicleRoot>>,
) {
    if roost.mat_handle.is_some() {
        return;
    }
    if vehicle.is_none() {
        return;
    }
    // Sprint 90: slightly lighter/more varied dirt tone — mix of sandy dirt.
    // Slightly smaller chunks (0.04 m) for a grittier, more realistic roost.
    roost.mat_handle = Some(materials.add(StandardMaterial {
        base_color: Color::srgb(0.42, 0.32, 0.18),
        perceptual_roughness: 0.95,
        ..default()
    }));
    roost.mesh_handle = Some(meshes.add(Cuboid::new(0.04, 0.04, 0.04)));
}

// ---- Spawn system -----------------------------------------------------------

fn spawn_roost_chunks(
    mut commands: Commands,
    mut roost: ResMut<RoostState>,
    vehicle: Option<Res<VehicleRoot>>,
    chassis_q: Query<(&Transform, &LinearVelocity), With<Chassis>>,
    input: Res<DriveInput>,
    time: Res<Time>,
) {
    let Some(vehicle) = vehicle else { return };
    let Ok((c_transform, lin_vel)) = chassis_q.get(vehicle.chassis) else { return };
    let (Some(mat_handle), Some(mesh_handle)) =
        (roost.mat_handle.clone(), roost.mesh_handle.clone())
    else { return };

    let speed_mps = Vec3::new(lin_vel.x, lin_vel.y, lin_vel.z).length();
    // slip_factor mirrors the formula in vehicle.rs update_wheel_visuals.
    let slip_factor = (1.0 - (speed_mps / 8.0).clamp(0.0, 1.0)) * input.drive.abs();

    if slip_factor <= SLIP_THRESHOLD {
        return;
    }

    // Rate-limit: spawn only on every ~8th frame (was 6th) — slightly less frequent.
    let elapsed = time.elapsed_secs();
    let spawn_frame = (elapsed * 10.0) as u32;
    if spawn_frame % 8 != 0 {
        return;
    }

    if roost.active.len() >= MAX_ACTIVE_CHUNKS {
        return;
    }

    let chassis_fwd  = c_transform.forward().normalize();
    let chassis_pos  = c_transform.translation;
    let chassis_rot  = c_transform.rotation;

    // Simple deterministic pseudo-random from elapsed time.
    let seed = (elapsed * 1000.0) as u32;
    let rng = |n: u32| -> f32 {
        let v = seed.wrapping_add(n).wrapping_mul(2_654_435_761);
        ((v >> 16) as f32 / 65535.0) * 2.0 - 1.0   // [-1, 1]
    };

    // Slip factor drives chunk count: low slip → 1 chunk, full slip → 3.
    let count = 1 + ((slip_factor - SLIP_THRESHOLD) / (1.0 - SLIP_THRESHOLD) * 2.0).clamp(0.0, 2.0) as usize;

    for wi in 2..4usize {
        let wheel_local = WHEEL_OFFSETS[wi];
        let wheel_world = chassis_pos + chassis_rot * wheel_local;

        for ci in 0..count {
            if roost.active.len() >= MAX_ACTIVE_CHUNKS {
                break;
            }
            let i = (wi as u32) * 10 + ci as u32;

            // Sprint 90: velocity arc — chunks launch more backward (4-7 m/s)
            // with a lower upward component (1.5-2.5) for a flatter, realistic arc.
            let back_speed = 4.0 + rng(i).abs() * 3.0;   // 4–7 m/s backward
            let spread_x   = rng(i + 1) * 1.2;            // ±1.2 m/s lateral
            let up_speed   = 1.5 + rng(i + 3).abs() * 1.0; // 1.5-2.5 m/s up
            let vel = chassis_fwd * (-back_speed)
                + chassis_rot * Vec3::new(spread_x, up_speed, rng(i + 2) * 0.8);

            let entity = commands.spawn((
                RoostChunk,
                Mesh3d(mesh_handle.clone()),
                MeshMaterial3d(mat_handle.clone()),
                Transform::from_translation(wheel_world),
            )).id();

            roost.active.push(Chunk { entity, velocity: vel, lifetime: CHUNK_LIFETIME });
        }
    }
}

// ---- Update: kinematic motion + despawn ------------------------------------

fn update_roost_chunks(
    mut commands: Commands,
    mut roost: ResMut<RoostState>,
    mut transforms: Query<&mut Transform, With<RoostChunk>>,
    time: Res<Time>,
) {
    let dt = time.delta_secs();

    roost.active.retain_mut(|chunk| {
        chunk.lifetime -= dt;
        if chunk.lifetime <= 0.0 {
            commands.entity(chunk.entity).despawn();
            return false;
        }

        // Apply gravity kinematically.
        chunk.velocity.y -= GRAVITY * dt;

        if let Ok(mut tf) = transforms.get_mut(chunk.entity) {
            tf.translation += chunk.velocity * dt;
        }

        true
    });
}
