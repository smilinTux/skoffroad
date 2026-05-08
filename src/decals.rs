// Persistent tire-track decals: spawns small flat dark quads under each
// wheel that touches the ground, fading over ~30s. Independent of skidmarks
// (which are short transient lines for slip/braking events).
//
// Public API:
//   DecalsPlugin

use bevy::prelude::*;
use crate::terrain::terrain_height_at;
use crate::vehicle::{Chassis, Wheel, VehicleRoot};
use avian3d::prelude::LinearVelocity;

// ---- Constants --------------------------------------------------------------

/// Seconds between decal spawns (≈6-7 per second when moving).
const SPAWN_INTERVAL_S: f32 = 0.15;
/// Minimum chassis speed (m/s) to spawn decals.
const MIN_SPEED_MPS: f32 = 2.0;
/// Height above terrain surface to place each decal.
const DECAL_LIFT: f32 = 0.02;
/// Decal dimensions: flat 0.5 × 0.02 × 0.5 m cuboid.
const DECAL_W: f32 = 0.5;
const DECAL_H: f32 = 0.02;
const DECAL_D: f32 = 0.5;
/// Seconds until a decal is fully transparent and despawned.
const FADE_DURATION_S: f32 = 30.0;
/// Maximum live decal entities.
const MAX_DECALS: usize = 600;

// ---- Resource ---------------------------------------------------------------

#[derive(Resource)]
pub struct DecalState {
    /// Accumulator for the spawn interval timer (seconds).
    pub spawn_timer: f32,
    /// Current count of live decal entities.
    pub count: u32,
}

impl Default for DecalState {
    fn default() -> Self {
        Self {
            spawn_timer: 0.0,
            count: 0,
        }
    }
}

// ---- Component --------------------------------------------------------------

#[derive(Component)]
pub struct TireDecal {
    /// Seconds since this decal was spawned.
    pub age_s: f32,
}

// ---- Plugin -----------------------------------------------------------------

pub struct DecalsPlugin;

impl Plugin for DecalsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DecalState>()
           .add_systems(Update, (
               spawn_decals.run_if(resource_exists::<VehicleRoot>),
               fade_decals,
               cull_decals,
           ));
    }
}

// ---- Systems ----------------------------------------------------------------

/// Spawns decals at grounded wheels every SPAWN_INTERVAL_S when chassis speed > MIN_SPEED_MPS.
fn spawn_decals(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut state: ResMut<DecalState>,
    vehicle: Res<VehicleRoot>,
    chassis_q: Query<(&Transform, &LinearVelocity), With<Chassis>>,
    wheel_q: Query<(&Transform, &Wheel)>,
    time: Res<Time>,
) {
    let dt = time.delta_secs();
    state.spawn_timer += dt;

    if state.spawn_timer < SPAWN_INTERVAL_S {
        return;
    }
    state.spawn_timer = 0.0;

    let Ok((c_transform, lin_vel)) = chassis_q.get(vehicle.chassis) else { return };

    let speed = Vec3::new(lin_vel.x, lin_vel.y, lin_vel.z).length();
    if speed < MIN_SPEED_MPS {
        return;
    }

    let chassis_pos = c_transform.translation;
    let chassis_rot = c_transform.rotation;

    // Shared mesh — all decals this tick use the same mesh handle.
    let mesh = meshes.add(Cuboid::new(DECAL_W, DECAL_H, DECAL_D));

    for (wheel_local, wheel) in &wheel_q {
        if !wheel.is_grounded {
            continue;
        }

        // Compute world-space wheel position (wheel transform is child-local to chassis).
        let wheel_world = chassis_pos + chassis_rot * wheel_local.translation;
        let terrain_y = terrain_height_at(wheel_world.x, wheel_world.z);
        let decal_pos = Vec3::new(wheel_world.x, terrain_y + DECAL_LIFT, wheel_world.z);

        // Give each decal its own material so alpha can be mutated independently.
        let material = materials.add(StandardMaterial {
            base_color: Color::srgba(0.10, 0.07, 0.04, 0.85),
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            ..default()
        });

        commands.spawn((
            TireDecal { age_s: 0.0 },
            Mesh3d(mesh.clone()),
            MeshMaterial3d(material),
            Transform::from_translation(decal_pos),
        ));

        state.count = state.count.saturating_add(1);
    }
}

/// Ages each decal and updates its material alpha every frame.
fn fade_decals(
    mut decal_q: Query<(&mut TireDecal, &MeshMaterial3d<StandardMaterial>)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    time: Res<Time>,
) {
    let dt = time.delta_secs();

    for (mut decal, mat_handle) in decal_q.iter_mut() {
        decal.age_s += dt;
        let alpha = (1.0 - decal.age_s / FADE_DURATION_S).max(0.0) * 0.85;

        if let Some(mat) = materials.get_mut(mat_handle.id()) {
            let old_color = mat.base_color.to_srgba();
            mat.base_color = Color::srgba(old_color.red, old_color.green, old_color.blue, alpha);
        }
    }
}

/// Despawns fully-faded decals (age >= FADE_DURATION_S) and enforces MAX_DECALS cap.
fn cull_decals(
    mut commands: Commands,
    mut state: ResMut<DecalState>,
    decal_q: Query<(Entity, &TireDecal)>,
) {
    // Collect and sort oldest-first for cap enforcement.
    let mut decals: Vec<(Entity, f32)> = decal_q
        .iter()
        .map(|(e, d)| (e, d.age_s))
        .collect();

    // Sort descending by age so oldest are first.
    decals.sort_unstable_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let mut despawned = 0u32;

    for (entity, age) in &decals {
        // Always cull fully-faded decals.
        if *age >= FADE_DURATION_S {
            commands.entity(*entity).despawn();
            despawned += 1;
            continue;
        }

        // If still over cap, remove the oldest survivors.
        let live = (decals.len() as u32).saturating_sub(despawned);
        if live > MAX_DECALS as u32 {
            commands.entity(*entity).despawn();
            despawned += 1;
        }
    }

    state.count = state.count.saturating_sub(despawned);
}
