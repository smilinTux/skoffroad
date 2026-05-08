// Ground ruts: persistent darker tire-track decals left in mud zones,
// distinct from decals.rs (which fades over 30s). Ruts last 90s, form a
// continuous strip behind each wheel rather than discrete dots.
//
// Spawn gate: wheel must be within 6 m XZ of any MudZone (from mud.rs).
// Spawn cadence: one quad every 0.4 m of wheel travel (not time-based).
// Quad size: 0.6 × 0.02 × 0.5 m, oriented to direction of motion.
// Color: dark mud brown srgba(0.20, 0.15, 0.08, 0.85), AlphaMode::Blend.
// Lifetime: 90 s; cap 1 500 entities (oldest-first cull when over).
//
// Public API:
//   GroundRutsPlugin

use bevy::prelude::*;
use std::collections::HashMap;
use crate::terrain::terrain_height_at;
use crate::vehicle::{Chassis, Wheel, VehicleRoot};
use crate::mud::MudZone;
use avian3d::prelude::LinearVelocity;

// ---- Constants --------------------------------------------------------------

/// Minimum XZ wheel travel (metres) between rut spawns.
const RUT_STRIDE_M: f32 = 0.4;
/// Height above terrain surface to place the rut quad.
const RUT_LIFT: f32 = 0.02;
/// Rut quad dimensions: 0.6 × 0.02 × 0.5 m.
const RUT_W: f32 = 0.6;
const RUT_H: f32 = 0.02;
const RUT_D: f32 = 0.5;
/// Seconds until a rut is fully transparent and despawned.
const FADE_DURATION_S: f32 = 90.0;
/// Initial alpha of the mud-brown color.
const BASE_ALPHA: f32 = 0.85;
/// Maximum XZ radius (metres) from a MudZone centre for eligibility.
const MUD_PROXIMITY_M: f32 = 6.0;
/// Maximum live rut entities.
const MAX_RUTS: usize = 1_500;

// ---- Resource ---------------------------------------------------------------

/// Tracks the last world-space position where a rut was spawned for each
/// wheel entity. Keyed by wheel Entity so it handles any number of wheels.
#[derive(Resource, Default)]
pub struct RutSpawnState {
    pub last_pos: HashMap<Entity, Vec3>,
}

// ---- Component --------------------------------------------------------------

#[derive(Component)]
pub struct Rut {
    /// Seconds since this rut was spawned.
    pub age_s: f32,
}

// ---- Plugin -----------------------------------------------------------------

pub struct GroundRutsPlugin;

impl Plugin for GroundRutsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RutSpawnState>()
           .add_systems(Update, (
               spawn_ruts.run_if(resource_exists::<VehicleRoot>),
               fade_ruts,
               cull_ruts,
           ));
    }
}

// ---- Systems ----------------------------------------------------------------

/// Spawns rut quads at grounded wheels that are near a MudZone and have
/// moved >= RUT_STRIDE_M since the last rut was placed.
fn spawn_ruts(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut state: ResMut<RutSpawnState>,
    vehicle: Res<VehicleRoot>,
    chassis_q: Query<(&Transform, &LinearVelocity), With<Chassis>>,
    wheel_q: Query<(Entity, &Transform, &Wheel)>,
    mud_q: Query<(&Transform, &MudZone)>,
) {
    let Ok((c_transform, lin_vel)) = chassis_q.get(vehicle.chassis) else { return };

    let chassis_pos = c_transform.translation;
    let chassis_rot = c_transform.rotation;

    // Velocity vector used to orient the rut quad along direction of travel.
    let vel_v = Vec3::new(lin_vel.x, lin_vel.y, lin_vel.z);
    let speed = vel_v.length();

    // Collect mud-zone centres once; reused per wheel.
    let mud_zones: Vec<(Vec2, f32)> = mud_q
        .iter()
        .map(|(t, mz)| (Vec2::new(t.translation.x, t.translation.z), mz.radius))
        .collect();

    // Shared mesh for all quads spawned this invocation.
    let mesh = meshes.add(Cuboid::new(RUT_W, RUT_H, RUT_D));

    for (wheel_entity, wheel_local, wheel) in wheel_q.iter() {
        if !wheel.is_grounded {
            continue;
        }

        // Compute world-space wheel position (wheel transform is child-local
        // to the chassis, mirroring the pattern in decals.rs).
        let wheel_world = chassis_pos + chassis_rot * wheel_local.translation;
        let wheel_xz = Vec2::new(wheel_world.x, wheel_world.z);

        // Gate: wheel must be within MUD_PROXIMITY_M of at least one MudZone.
        let in_mud = mud_zones.iter().any(|(centre, _radius)| {
            wheel_xz.distance(*centre) <= MUD_PROXIMITY_M
        });
        if !in_mud {
            // Still record position so we measure distance from the moment
            // the wheel re-enters mud, not from an arbitrary earlier point.
            state.last_pos.insert(wheel_entity, wheel_world);
            continue;
        }

        // Stride gate: only spawn once the wheel has moved >= RUT_STRIDE_M.
        let last = state.last_pos.get(&wheel_entity).copied().unwrap_or(wheel_world);
        let moved_xz = Vec2::new(wheel_world.x - last.x, wheel_world.z - last.z).length();
        if moved_xz < RUT_STRIDE_M {
            continue;
        }

        // Update last-spawn position for this wheel.
        state.last_pos.insert(wheel_entity, wheel_world);

        let terrain_y = terrain_height_at(wheel_world.x, wheel_world.z);
        let rut_pos = Vec3::new(wheel_world.x, terrain_y + RUT_LIFT, wheel_world.z);

        // Orient the rut along direction of motion (or chassis forward if
        // nearly stationary to avoid degenerate quads).
        let forward_dir = if speed > 0.1 {
            vel_v.normalize()
        } else {
            (chassis_rot * Vec3::NEG_Z).normalize()
        };

        // Rotation that aligns the quad's local +Z axis to forward_dir.
        // We project onto the XZ plane and compute a Y-axis rotation.
        let yaw = f32::atan2(forward_dir.x, forward_dir.z);
        let rut_rot = Quat::from_rotation_y(yaw);

        // Per-rut material so alpha can be mutated independently.
        let material = materials.add(StandardMaterial {
            base_color: Color::srgba(0.20, 0.15, 0.08, BASE_ALPHA),
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            ..default()
        });

        commands.spawn((
            Rut { age_s: 0.0 },
            Mesh3d(mesh.clone()),
            MeshMaterial3d(material),
            Transform::from_translation(rut_pos).with_rotation(rut_rot),
        ));
    }
}

/// Ages each rut and updates its material alpha every frame.
fn fade_ruts(
    mut rut_q: Query<(&mut Rut, &MeshMaterial3d<StandardMaterial>)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    time: Res<Time>,
) {
    let dt = time.delta_secs();

    for (mut rut, mat_handle) in rut_q.iter_mut() {
        rut.age_s += dt;
        let alpha = (1.0 - rut.age_s / FADE_DURATION_S).max(0.0) * BASE_ALPHA;

        if let Some(mat) = materials.get_mut(mat_handle.id()) {
            let c = mat.base_color.to_srgba();
            mat.base_color = Color::srgba(c.red, c.green, c.blue, alpha);
        }
    }
}

/// Despawns fully-faded ruts (age >= FADE_DURATION_S) and enforces MAX_RUTS
/// cap; oldest ruts are culled first when over the cap.
fn cull_ruts(
    mut commands: Commands,
    rut_q: Query<(Entity, &Rut)>,
) {
    // Collect all ruts sorted descending by age (oldest first).
    let mut ruts: Vec<(Entity, f32)> = rut_q
        .iter()
        .map(|(e, r)| (e, r.age_s))
        .collect();

    ruts.sort_unstable_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let mut despawned = 0usize;

    for (entity, age) in &ruts {
        // Always cull fully-faded ruts.
        if *age >= FADE_DURATION_S {
            commands.entity(*entity).despawn();
            despawned += 1;
            continue;
        }

        // Enforce cap: remove the oldest surviving ruts when over limit.
        let live = ruts.len().saturating_sub(despawned);
        if live > MAX_RUTS {
            commands.entity(*entity).despawn();
            despawned += 1;
        }
    }
}
