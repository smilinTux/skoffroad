// Trailers: G key spawns a small towable cargo trailer behind the chassis,
// connected via an Avian3D DistanceJoint so it physically follows.
// Pressing G again despawns the trailer and its joint.
//
// Public API:
//   TrailersPlugin
//   TrailerState (resource)

use bevy::prelude::*;
use avian3d::prelude::*;
use crate::vehicle::{Chassis, VehicleRoot};

// ---- Plugin ----------------------------------------------------------------

pub struct TrailersPlugin;

impl Plugin for TrailersPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TrailerState>()
           .init_resource::<TrailerEntity>()
           .add_systems(Update, toggle_with_g);
    }
}

// ---- Public API ------------------------------------------------------------

/// Tracks whether a trailer is currently attached.
#[derive(Resource, Default, Clone)]
pub struct TrailerState {
    pub attached: bool,
}

// ---- Components ------------------------------------------------------------

/// Marker component placed on the spawned trailer entity.
#[derive(Component)]
pub struct Trailer;

// ---- Private resources -----------------------------------------------------

/// Tracks the two entities that must be despawned on detach:
/// the trailer body and the joint entity.
#[derive(Resource, Default)]
struct TrailerEntity {
    trailer: Option<Entity>,
    joint:   Option<Entity>,
}

// ---- Constants -------------------------------------------------------------

/// Half-extents of the cargo trailer box (width, height, depth).
const TRAILER_HALF: Vec3 = Vec3::new(0.8, 0.2, 1.0);

/// Local-space offset from the chassis origin to the tow hitch at the rear.
/// The chassis faces –Z (Bevy forward), so +Z is behind.
const HITCH_LOCAL: Vec3 = Vec3::new(0.0, -0.3, 2.2);

/// Local-space offset from the trailer origin to its coupling point.
const COUPLER_LOCAL: Vec3 = Vec3::new(0.0, 0.0, -1.1);

/// Distance joint limits between hitch and coupler (metres).
const JOINT_MIN: f32 = 0.0;
const JOINT_MAX: f32 = 0.5;

// ---- System ----------------------------------------------------------------

fn toggle_with_g(
    keyboard:     Res<ButtonInput<KeyCode>>,
    mut state:    ResMut<TrailerState>,
    mut tracker:  ResMut<TrailerEntity>,
    vehicle_root: Option<Res<VehicleRoot>>,
    chassis_q:    Query<&GlobalTransform, With<Chassis>>,
    mut commands: Commands,
    mut meshes:   ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if !keyboard.just_pressed(KeyCode::KeyG) {
        return;
    }

    if state.attached {
        // --- Detach: despawn trailer and joint ---
        if let Some(e) = tracker.joint.take() {
            commands.entity(e).despawn();
        }
        if let Some(e) = tracker.trailer.take() {
            commands.entity(e).despawn();
        }
        state.attached = false;
        return;
    }

    // --- Attach: need the chassis entity and its world transform ---
    let chassis_entity = match vehicle_root.as_ref() {
        Some(vr) => vr.chassis,
        None => {
            warn!("TrailersPlugin: VehicleRoot not found, cannot attach trailer.");
            return;
        }
    };

    let chassis_gt = match chassis_q.get(chassis_entity) {
        Ok(gt) => gt,
        Err(_) => {
            warn!("TrailersPlugin: Chassis GlobalTransform not found.");
            return;
        }
    };

    // World position and orientation of the chassis.
    let chassis_pos = chassis_gt.translation();
    let chassis_rot = chassis_gt.to_scale_rotation_translation().1;

    // Spawn the trailer roughly 3 units behind the chassis.
    // chassis forward() points –Z in local, so behind = +Z in local = chassis_rot * +Z.
    let behind = chassis_rot * Vec3::Z;
    let trailer_pos = chassis_pos + behind * 3.0 - Vec3::Y * 0.3;

    let trailer_mesh = meshes.add(Cuboid::new(
        TRAILER_HALF.x * 2.0,
        TRAILER_HALF.y * 2.0,
        TRAILER_HALF.z * 2.0,
    ));
    let trailer_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.50, 0.35, 0.20),
        perceptual_roughness: 0.8,
        ..default()
    });

    let trailer_id = commands.spawn((
        Trailer,
        Mesh3d(trailer_mesh),
        MeshMaterial3d(trailer_mat),
        Transform::from_translation(trailer_pos)
            .with_rotation(chassis_rot),
        RigidBody::Dynamic,
        Collider::cuboid(TRAILER_HALF.x, TRAILER_HALF.y, TRAILER_HALF.z),
        Mass(300.0),
        LinearDamping(2.0),
        AngularDamping(8.0),
    )).id();

    // Spawn the distance joint as its own entity (Avian3D pattern).
    // Anchor1 is at the chassis hitch (local), anchor2 at the trailer coupler (local).
    let joint_id = commands.spawn(
        DistanceJoint::new(chassis_entity, trailer_id)
            .with_local_anchor1(HITCH_LOCAL)
            .with_local_anchor2(COUPLER_LOCAL)
            .with_limits(JOINT_MIN, JOINT_MAX)
            .with_compliance(0.0001),
    ).id();

    tracker.trailer = Some(trailer_id);
    tracker.joint   = Some(joint_id);
    state.attached  = true;
}
