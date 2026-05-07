// Interior 3D: cockpit details visible from FirstPerson camera mode (V key
// cycle) — a steering wheel that rotates with driver input, a dashboard panel
// with twin gauge arcs, and 2 bucket seats. All elements are spawned as
// chassis-local children so they move/rotate with the vehicle.
//
// Sprint 34
//
// Public API:
//   Interior3dPlugin

use bevy::prelude::*;
use std::f32::consts::TAU;

use crate::vehicle::{DriveInput, VehicleRoot};

// ── Plugin ─────────────────────────────────────────────────────────────────────

pub struct Interior3dPlugin;

impl Plugin for Interior3dPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (attach_interior_once, rotate_steering_wheel));
    }
}

// ── Component markers ─────────────────────────────────────────────────────────

/// Invisible transform parent for the steering wheel ring + hub. Rotated each
/// frame by `rotate_steering_wheel` to reflect `DriveInput::steer`.
#[derive(Component)]
pub struct SteeringWheelMount;

// ── One-shot attach system ────────────────────────────────────────────────────

/// Runs every Update frame but fires exactly once (guarded by `Local<bool>`).
/// Waits for `VehicleRoot` to be inserted (guaranteed by Startup → Update
/// ordering), then builds the full cockpit and attaches it to the chassis.
fn attach_interior_once(
    mut done: Local<bool>,
    vehicle: Option<Res<VehicleRoot>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if *done { return; }
    let Some(vehicle) = vehicle else { return };
    *done = true;

    let chassis = vehicle.chassis;

    // ── Shared materials ──────────────────────────────────────────────────────

    // Dark interior plastic — dashboard, wheel ring, seat body.
    let dark_plastic = materials.add(StandardMaterial {
        base_color: Color::srgb(0.10, 0.10, 0.12),
        perceptual_roughness: 0.85,
        ..default()
    });

    // Steering wheel / trim colour — slightly lighter gray.
    let wheel_dark = materials.add(StandardMaterial {
        base_color: Color::srgb(0.15, 0.15, 0.18),
        perceptual_roughness: 0.80,
        ..default()
    });

    // Chrome hub accent.
    let chrome = materials.add(StandardMaterial {
        base_color: Color::srgb(0.78, 0.78, 0.82),
        metallic: 0.90,
        perceptual_roughness: 0.10,
        ..default()
    });

    // Seat upholstery — dark brownish-gray.
    let seat_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.18, 0.16, 0.14),
        perceptual_roughness: 0.90,
        ..default()
    });

    // Gauge face — bright unlit white (reads as self-lit instrument).
    let gauge_face = materials.add(StandardMaterial {
        base_color: Color::srgb(0.95, 0.95, 0.95),
        unlit: true,
        ..default()
    });

    // Gauge needle — small dark rod.
    let needle_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.12, 0.12, 0.14),
        perceptual_roughness: 0.80,
        ..default()
    });

    // ── 1. Dashboard panel ────────────────────────────────────────────────────
    // Cuboid 1.6 × 0.05 × 0.3, dark plastic.
    // Chassis local: (0, 0.4, −1.4) — just behind the windshield line, in
    // front of both driver and passenger.
    let dash_mesh = meshes.add(Cuboid::new(1.6, 0.05, 0.3));
    let dashboard = commands.spawn((
        Mesh3d(dash_mesh),
        MeshMaterial3d(dark_plastic.clone()),
        Transform::from_translation(Vec3::new(0.0, 0.4, -1.4)),
    )).id();

    // ── 2. Gauge arcs — tachometer (left) and speedometer (right) ────────────
    // Each gauge = thin cylinder r=0.10 h=0.03, unlit white, sitting on top
    // of the dashboard face at (±0.4, 0.42, −1.42) in chassis-local space.
    // A small dark needle cuboid (0.02 × 0.001 × 0.08) protrudes from centre.
    let gauge_cyl_mesh = meshes.add(Cylinder::new(0.10, 0.03));
    let needle_mesh    = meshes.add(Cuboid::new(0.02, 0.001, 0.08));

    let mut gauge_ids: Vec<Entity> = Vec::with_capacity(2);
    for side in [-0.4_f32, 0.4_f32] {
        let gauge = commands.spawn((
            Mesh3d(gauge_cyl_mesh.clone()),
            MeshMaterial3d(gauge_face.clone()),
            Transform::from_translation(Vec3::new(side, 0.42, -1.42)),
        )).id();

        // Needle: offset slightly along −Z so it projects from the gauge face.
        let needle = commands.spawn((
            Mesh3d(needle_mesh.clone()),
            MeshMaterial3d(needle_mat.clone()),
            // Place needle above the cylinder face (y + 0.02), pointing in −Z.
            Transform::from_translation(Vec3::new(0.0, 0.02, -0.02)),
        )).id();

        commands.entity(gauge).add_child(needle);
        gauge_ids.push(gauge);
    }

    // ── 3. Steering wheel mount (invisible parent, rotated per-frame) ─────────
    // Positioned at driver's side: chassis local (−0.4, 0.35, −1.0).
    // Slightly above dashboard, left of centre — driver position.
    let mount = commands.spawn((
        SteeringWheelMount,
        Transform::from_translation(Vec3::new(-0.4, 0.35, -1.0)),
        Visibility::default(),
    )).id();

    // ── 3a. Steering wheel spokes — ring of 5 cuboids around r=0.18 ──────────
    // Each spoke: 0.04 × 0.04 × 0.10.  Arrange in a circle; each translated
    // radially and rotated to point toward wheel centre.  The wheel lies in
    // the XY plane of the mount (mount Z = forward), so spokes orbit around
    // the mount's Z-axis.
    let spoke_mesh = meshes.add(Cuboid::new(0.04, 0.04, 0.10));
    const SPOKE_COUNT: usize = 5;
    const RING_RADIUS: f32 = 0.18;

    for i in 0..SPOKE_COUNT {
        let angle = (i as f32 / SPOKE_COUNT as f32) * TAU;
        let (sin_a, cos_a) = angle.sin_cos();

        // Position each spoke on the ring in the XY plane.
        let tx = sin_a * RING_RADIUS;
        let ty = cos_a * RING_RADIUS;

        // Rotate each spoke so its long axis points toward the ring centre.
        // Spoke long axis is Z; we rotate around Z by −angle to aim inward.
        // Actually: spoke sits at angle, it should point radially. The spoke
        // mesh is 0.04×0.04×0.10 — Z is the longest dimension.  We want Z to
        // aim toward the mount origin (i.e. radially inward).
        // Direction inward = (−sin_a, −cos_a, 0).  We need Z to align with
        // that direction.  Rotation from Vec3::Z to inward direction:
        let rot = Quat::from_rotation_z(-angle);

        let spoke = commands.spawn((
            Mesh3d(spoke_mesh.clone()),
            MeshMaterial3d(wheel_dark.clone()),
            Transform::from_translation(Vec3::new(tx, ty, 0.0))
                .with_rotation(rot),
        )).id();
        commands.entity(mount).add_child(spoke);
    }

    // ── 3b. Central hub ───────────────────────────────────────────────────────
    // Cuboid 0.10 × 0.04 × 0.10, chrome accent, at mount origin (0,0,0).
    let hub_mesh = meshes.add(Cuboid::new(0.10, 0.04, 0.10));
    let hub = commands.spawn((
        Mesh3d(hub_mesh),
        MeshMaterial3d(chrome.clone()),
        Transform::IDENTITY,
    )).id();
    commands.entity(mount).add_child(hub);

    // ── 4. Bucket seats ───────────────────────────────────────────────────────
    // Two seats: driver at (−0.4, 0.0, −0.5) and passenger at (0.4, 0.0, −0.5).
    // Each seat = seat-bottom + backrest spawned as children of the chassis.
    let bottom_mesh   = meshes.add(Cuboid::new(0.45, 0.10, 0.45));
    let backrest_mesh = meshes.add(Cuboid::new(0.45, 0.70, 0.10));

    for x_pos in [-0.4_f32, 0.4_f32] {
        // Seat bottom: horizontal cushion sitting just above chassis mid-plane.
        let bottom = commands.spawn((
            Mesh3d(bottom_mesh.clone()),
            MeshMaterial3d(seat_mat.clone()),
            Transform::from_translation(Vec3::new(x_pos, -0.05, -0.5)),
        )).id();

        // Backrest: tall slab angled slightly backward (−8°) so it reads as a
        // reclined bucket seat rather than an upright bench.
        // Placed at y=0.30 (rising above the seat bottom), z=−0.70 (rear edge).
        let backrest = commands.spawn((
            Mesh3d(backrest_mesh.clone()),
            MeshMaterial3d(seat_mat.clone()),
            Transform::from_translation(Vec3::new(x_pos, 0.30, -0.70))
                .with_rotation(Quat::from_rotation_x(-8_f32.to_radians())),
        )).id();

        commands.entity(chassis).add_child(bottom);
        commands.entity(chassis).add_child(backrest);
    }

    // ── Attach top-level cockpit pieces to chassis ────────────────────────────
    commands.entity(chassis).add_child(dashboard);
    for &g in &gauge_ids {
        commands.entity(chassis).add_child(g);
    }
    commands.entity(chassis).add_child(mount);
}

// ── Per-frame: rotate steering wheel with DriveInput ─────────────────────────

/// Reads `DriveInput::steer` (−1 … +1) and applies a smooth local Y-rotation
/// to the `SteeringWheelMount` so the wheel turns ~1 radian (≈57°) per unit
/// of steer input. A lerp factor of 0.15 per frame provides snappy but
/// non-instant response.
fn rotate_steering_wheel(
    input:      Res<DriveInput>,
    mut mounts: Query<&mut Transform, With<SteeringWheelMount>>,
) {
    let target_rot = Quat::from_rotation_z(input.steer * 1.0);

    for mut tf in mounts.iter_mut() {
        tf.rotation = tf.rotation.slerp(target_rot, 0.15);
    }
}
