// Skid-mark decals: dark cuboid stripes spawned at each wheel contact point
// when lateral slip or hard braking exceeds a threshold.
//
// SLIP THRESHOLD: 1.5 m/s lateral velocity (or any braking with speed > 2 m/s).
// SPAWN SPACING : 0.05 m of travel between stripes per wheel.
// MAX STRIPES   : 200 (oldest despawned first via VecDeque).
// TOGGLE        : K key enables/disables new spawns.
// CLEAR         : Shift+K despawns all live stripes.

use bevy::prelude::*;
use avian3d::prelude::LinearVelocity;
use std::collections::{HashMap, VecDeque};
use crate::vehicle::{Chassis, Wheel, VehicleRoot};

// ---- Public resources -------------------------------------------------------

#[derive(Resource, Default)]
pub struct SkidQueue {
    pub entities: VecDeque<Entity>,
}

#[derive(Resource)]
pub struct SkidEnabled(pub bool);
impl Default for SkidEnabled {
    fn default() -> Self { Self(true) }
}

// ---- Constants --------------------------------------------------------------

/// Lateral velocity magnitude (m/s) at which skid marks begin.
const SLIP_THRESHOLD: f32 = 1.5;
/// Longitudinal speed (m/s) above which hard braking also leaves marks.
const BRAKE_SPEED_THRESHOLD: f32 = 2.0;
/// Minimum XZ travel (m) between consecutive stripe spawns per wheel.
const SPAWN_SPACING: f32 = 0.05;
/// Stripe lifts this far above the wheel contact point to avoid z-fighting.
const LIFT: f32 = 0.05;
/// Maximum number of live stripe entities before oldest is despawned.
const MAX_STRIPES: usize = 200;

// ---- Plugin -----------------------------------------------------------------

pub struct SkidmarksPlugin;
impl Plugin for SkidmarksPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SkidQueue>()
           .init_resource::<SkidEnabled>()
           .add_systems(Update, (
               spawn_skids.run_if(resource_exists::<VehicleRoot>),
               toggle_skid_enabled,
               clear_skids,
           ));
    }
}

// ---- Spawn system -----------------------------------------------------------

fn spawn_skids(
    mut commands:     Commands,
    mut meshes:       ResMut<Assets<Mesh>>,
    mut materials:    ResMut<Assets<StandardMaterial>>,
    mut queue:        ResMut<SkidQueue>,
    enabled:          Res<SkidEnabled>,
    vehicle:          Res<VehicleRoot>,
    chassis_q:        Query<(&Transform, &LinearVelocity), With<Chassis>>,
    wheel_q:          Query<(Entity, &Transform, &Wheel)>,
    mut last_spawn:   Local<HashMap<Entity, Vec3>>,
) {
    if !enabled.0 { return; }

    let Ok((c_transform, lin_vel)) = chassis_q.get(vehicle.chassis) else { return };

    let chassis_pos   = c_transform.translation;
    let chassis_rot   = c_transform.rotation;
    let chassis_right = (chassis_rot * Vec3::X).normalize();  // +X is right/lateral
    let lin_vel_v: Vec3 = lin_vel.0;

    // Lateral velocity (scalar) measured once at chassis centre.
    let v_lat = lin_vel_v.dot(chassis_right).abs();
    // Forward speed for braking check.
    let v_fwd = lin_vel_v.length();

    // Pre-build shared mesh/material handles only when we know we'll spawn.
    // We use an Option so we create them lazily inside the loop only on first use.
    let mut mesh_handle: Option<Handle<Mesh>> = None;
    let mut mat_handle:  Option<Handle<StandardMaterial>> = None;

    for (wheel_entity, wheel_local_transform, wheel) in &wheel_q {
        if !wheel.is_grounded { continue; }

        // Decide whether this wheel is slipping.
        let is_braking_skid = wheel.index < 2  // front wheels lock hardest
            && v_fwd > BRAKE_SPEED_THRESHOLD;
        let is_lateral_skid = v_lat > SLIP_THRESHOLD;

        if !is_lateral_skid && !is_braking_skid { continue; }

        // World position of the wheel anchor (wheel_local_transform is local to chassis).
        let world_pos = chassis_pos + chassis_rot * wheel_local_transform.translation;

        // Check spawn spacing: only emit one stripe per SPAWN_SPACING metres of XZ travel.
        let xz = Vec3::new(world_pos.x, 0.0, world_pos.z);
        if let Some(last) = last_spawn.get(&wheel_entity) {
            if xz.distance(*last) < SPAWN_SPACING {
                continue;
            }
        }
        last_spawn.insert(wheel_entity, xz);

        // Lazy mesh/material creation (shared across all stripes this frame).
        if mesh_handle.is_none() {
            mesh_handle = Some(meshes.add(Cuboid::new(0.1, 0.02, 0.5)));
        }
        if mat_handle.is_none() {
            mat_handle = Some(materials.add(StandardMaterial {
                base_color: Color::srgba(0.05, 0.05, 0.05, 0.9),
                alpha_mode: AlphaMode::Blend,
                ..default()
            }));
        }

        // Rotate stripe to lie along chassis travel direction (Y rotation only).
        let (yaw, _, _) = chassis_rot.to_euler(EulerRot::YXZ);
        let stripe_rot  = Quat::from_rotation_y(yaw);

        let stripe_pos = Vec3::new(world_pos.x, world_pos.y + LIFT, world_pos.z);

        let entity = commands.spawn((
            Mesh3d(mesh_handle.clone().unwrap()),
            MeshMaterial3d(mat_handle.clone().unwrap()),
            Transform::from_translation(stripe_pos).with_rotation(stripe_rot),
        )).id();

        // Enforce MAX_STRIPES cap: remove oldest first.
        if queue.entities.len() >= MAX_STRIPES {
            if let Some(oldest) = queue.entities.pop_front() {
                commands.entity(oldest).despawn();
            }
        }
        queue.entities.push_back(entity);
    }
}

// ---- Toggle system ----------------------------------------------------------

fn toggle_skid_enabled(
    keys:    Res<ButtonInput<KeyCode>>,
    mut enabled: ResMut<SkidEnabled>,
) {
    // K alone toggles; Shift+K is handled by clear_skids, so ignore it here.
    if keys.just_pressed(KeyCode::KeyK)
        && !keys.pressed(KeyCode::ShiftLeft)
        && !keys.pressed(KeyCode::ShiftRight)
    {
        enabled.0 = !enabled.0;
    }
}

// ---- Clear system -----------------------------------------------------------

fn clear_skids(
    keys:         Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    mut queue:    ResMut<SkidQueue>,
) {
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if shift && keys.just_pressed(KeyCode::KeyK) {
        for entity in queue.entities.drain(..) {
            commands.entity(entity).despawn();
        }
    }
}
