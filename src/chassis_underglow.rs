// Chassis underglow: a subtle dark "shadow plane" beneath the chassis that
// follows it on the ground, giving a visual depth cue so the vehicle reads
// as sitting on the terrain rather than floating. Plus four small soft
// dark patches under each wheel.
//
// Real shadows are disabled (sky.rs sets shadows_enabled: false on the
// directional light for performance), so these fake blobs are the primary
// depth cue keeping the chassis visually grounded.
//
// Public API:
//   ChassisUnderglowPlugin

use bevy::prelude::*;

use crate::terrain::terrain_height_at;
use crate::vehicle::{Chassis, VehicleRoot};

// ---- Constants --------------------------------------------------------------

/// Chassis shadow: slightly larger than the 2.0×4.0 chassis cuboid.
/// Plane3d::new takes half_size, so the full visible footprint is 4.8×8.8.
const CHASSIS_SHADOW_HALF: Vec2 = Vec2::new(2.4, 4.4);
const CHASSIS_SHADOW_ALPHA: f32 = 0.30;

/// Wheel blob half-size; full footprint 1.4×1.4.
const WHEEL_SHADOW_HALF: Vec2 = Vec2::new(0.7, 0.7);
const WHEEL_SHADOW_ALPHA: f32 = 0.40;

/// Shadow lift above terrain (m) to avoid z-fighting.
const SHADOW_LIFT: f32 = 0.02;

/// When chassis altitude above terrain exceeds this, start fading shadows.
const FADE_ALTITUDE_START: f32 = 1.5;
/// At this altitude + FADE_ALTITUDE_START (i.e. 6.5 m) shadows are invisible.
const FADE_ALTITUDE_RANGE: f32 = 5.0;

/// FL, FR, RL, RR wheel anchor offsets in chassis local space (mirror vehicle.rs).
const WHEEL_OFFSETS: [Vec3; 4] = [
    Vec3::new(-1.1, -0.35, -1.4),
    Vec3::new( 1.1, -0.35, -1.4),
    Vec3::new(-1.1, -0.35,  1.4),
    Vec3::new( 1.1, -0.35,  1.4),
];

// ---- Components -------------------------------------------------------------

/// Discriminates chassis-body shadow vs per-wheel shadow blobs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShadowKind {
    Chassis,
    Wheel,
}

/// Marker component on each of the 5 fake-shadow plane entities.
#[derive(Component)]
pub struct ChassisShadow {
    pub kind: ShadowKind,
    /// For `ShadowKind::Wheel`: index into WHEEL_OFFSETS (0-3).
    /// For `ShadowKind::Chassis`: unused (0).
    pub idx: usize,
}

// ---- Plugin -----------------------------------------------------------------

pub struct ChassisUnderglowPlugin;

impl Plugin for ChassisUnderglowPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_shadows)
           .add_systems(Update, update_shadows.run_if(resource_exists::<VehicleRoot>));
    }
}

// ---- Startup: spawn the 5 shadow plane entities ----------------------------

fn spawn_shadows(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Chassis blob — one large flat plane.
    let chassis_mesh = meshes.add(Plane3d::new(Vec3::Y, CHASSIS_SHADOW_HALF));
    let chassis_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(0.0, 0.0, 0.0, CHASSIS_SHADOW_ALPHA),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        double_sided: true,
        cull_mode: None,
        ..default()
    });

    commands.spawn((
        ChassisShadow { kind: ShadowKind::Chassis, idx: 0 },
        Mesh3d(chassis_mesh),
        MeshMaterial3d(chassis_mat),
        Transform::default(),
        Visibility::default(),
    ));

    // Four wheel blobs.
    let wheel_mesh = meshes.add(Plane3d::new(Vec3::Y, WHEEL_SHADOW_HALF));

    for i in 0..4 {
        let wheel_mat = materials.add(StandardMaterial {
            base_color: Color::srgba(0.0, 0.0, 0.0, WHEEL_SHADOW_ALPHA),
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            double_sided: true,
            cull_mode: None,
            ..default()
        });

        commands.spawn((
            ChassisShadow { kind: ShadowKind::Wheel, idx: i },
            Mesh3d(wheel_mesh.clone()),
            MeshMaterial3d(wheel_mat),
            Transform::default(),
            Visibility::default(),
        ));
    }
}

// ---- Update: reposition shadows each frame ---------------------------------

fn update_shadows(
    vehicle: Res<VehicleRoot>,
    chassis_q: Query<&Transform, With<Chassis>>,
    mut shadow_q: Query<
        (&ChassisShadow, &mut Transform, &MeshMaterial3d<StandardMaterial>),
        Without<Chassis>,
    >,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let Ok(chassis_tf) = chassis_q.get(vehicle.chassis) else { return };

    let chassis_pos = chassis_tf.translation;
    let chassis_rot = chassis_tf.rotation;

    // Yaw-only rotation: project chassis forward onto XZ plane, compute yaw.
    let chassis_fwd = chassis_rot * Vec3::NEG_Z;
    let yaw = chassis_fwd.x.atan2(-chassis_fwd.z);
    let yaw_only_rot = Quat::from_rotation_y(yaw);

    // Terrain height directly under chassis centre.
    let ground_y = terrain_height_at(chassis_pos.x, chassis_pos.z);

    // Altitude-based alpha fade: full opacity up to FADE_ALTITUDE_START,
    // linearly fading to zero at FADE_ALTITUDE_START + FADE_ALTITUDE_RANGE.
    let altitude = (chassis_pos.y - ground_y).max(0.0);
    let fade = if altitude > FADE_ALTITUDE_START {
        (1.0 - (altitude - FADE_ALTITUDE_START) / FADE_ALTITUDE_RANGE).clamp(0.0, 1.0)
    } else {
        1.0
    };

    for (shadow, mut tf, mat_handle) in shadow_q.iter_mut() {
        match shadow.kind {
            ShadowKind::Chassis => {
                let snap_y = terrain_height_at(chassis_pos.x, chassis_pos.z) + SHADOW_LIFT;
                tf.translation = Vec3::new(chassis_pos.x, snap_y, chassis_pos.z);
                tf.rotation    = yaw_only_rot;

                // Update alpha.
                if let Some(mat) = materials.get_mut(mat_handle) {
                    mat.base_color = Color::srgba(0.0, 0.0, 0.0, CHASSIS_SHADOW_ALPHA * fade);
                }
            }
            ShadowKind::Wheel => {
                let local_offset = WHEEL_OFFSETS[shadow.idx];
                let world_pos    = chassis_pos + chassis_rot * local_offset;
                let snap_y       = terrain_height_at(world_pos.x, world_pos.z) + SHADOW_LIFT;

                tf.translation = Vec3::new(world_pos.x, snap_y, world_pos.z);
                // Wheel blobs stay axis-aligned (no rotation needed — they're circles).
                tf.rotation    = Quat::IDENTITY;

                if let Some(mat) = materials.get_mut(mat_handle) {
                    mat.base_color = Color::srgba(0.0, 0.0, 0.0, WHEEL_SHADOW_ALPHA * fade);
                }
            }
        }
    }
}
