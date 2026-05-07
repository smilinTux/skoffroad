// Wheel-well detail: visible fender / wheel-arch geometry above each wheel.
// Blocks the visual gap between the chassis bottom and the wheel top so the
// vehicle looks less like a flying box.
//
// Per wheel position four pieces of geometry are spawned as chassis children:
//   1. Outer fender flare  – dark cuboid just outside each chassis side
//   2. Inner liner          – matte-black cuboid just above the wheel top
//   3. Two chrome bolts     – small cubes on the front and rear edges of the
//                             outer flare for offroad-truck flavour
//
// All entities carry the `WheelWell` marker component so future systems can
// query or despawn them independently of DefaultSkin / VehicleDetail children.
//
// Public API:
//   WheelWellPlugin
//   WheelWell  (component marker)

use bevy::prelude::*;
use crate::vehicle::VehicleRoot;

// ── Plugin ────────────────────────────────────────────────────────────────────

pub struct WheelWellPlugin;

impl Plugin for WheelWellPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, attach_wells_once);
    }
}

// ── Component marker ─────────────────────────────────────────────────────────

/// Marker placed on every wheel-well entity (outer fender flare, inner liner,
/// chrome bolts). Allows future systems to query or despawn wheel-well geometry
/// independently of DefaultSkin / VehicleDetail children.
#[derive(Component)]
pub struct WheelWell;

// ── Wheel anchor positions (chassis-local) ────────────────────────────────────

// Mirrors WHEEL_OFFSETS from vehicle.rs:
//   FL (-1.1, -0.35, -1.4)   FR (1.1, -0.35, -1.4)
//   RL (-1.1, -0.35,  1.4)   RR (1.1, -0.35,  1.4)
const WHEEL_OFFSETS: [Vec3; 4] = [
    Vec3::new(-1.1, -0.35, -1.4),
    Vec3::new( 1.1, -0.35, -1.4),
    Vec3::new(-1.1, -0.35,  1.4),
    Vec3::new( 1.1, -0.35,  1.4),
];

// ── Geometry constants ────────────────────────────────────────────────────────

// Outer fender flare dimensions (W × H × D in chassis-local axes)
const FLARE_W: f32 = 0.4;
const FLARE_H: f32 = 0.20;
const FLARE_D: f32 = 0.9;

// Inner liner dimensions
const LINER_W: f32 = 0.5;
const LINER_H: f32 = 0.05;
const LINER_D: f32 = 0.9;

// Chrome bolt side length (uniform cube)
const BOLT_SIDE: f32 = 0.05;

// ── One-shot attach system ────────────────────────────────────────────────────

/// Runs every Update frame but executes its body exactly once (guarded by a
/// `Local<bool>`). Waits until `VehicleRoot` is available, then spawns all
/// wheel-well geometry as children of the chassis entity.
fn attach_wells_once(
    mut done: Local<bool>,
    vehicle: Option<Res<VehicleRoot>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if *done {
        return;
    }
    let Some(vehicle) = vehicle else { return };
    *done = true;

    let chassis = vehicle.chassis;

    // ── Shared meshes (cloned per instance) ──────────────────────────────────

    let flare_mesh = meshes.add(Cuboid::new(FLARE_W, FLARE_H, FLARE_D));
    let liner_mesh = meshes.add(Cuboid::new(LINER_W, LINER_H, LINER_D));
    let bolt_mesh  = meshes.add(Cuboid::new(BOLT_SIDE, BOLT_SIDE, BOLT_SIDE));

    // ── Shared materials ──────────────────────────────────────────────────────

    // Outer fender flare: very dark blue-black (off-road truck body cladding)
    let flare_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.05, 0.05, 0.08),
        perceptual_roughness: 0.85,
        ..default()
    });

    // Inner liner: matte black (unpainted plastic / rubber)
    let liner_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.04, 0.04, 0.04),
        perceptual_roughness: 0.95,
        ..default()
    });

    // Chrome bolt heads
    let bolt_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.85, 0.85, 0.92),
        metallic: 0.8,
        perceptual_roughness: 0.15,
        ..default()
    });

    // ── Spawn per-wheel geometry ──────────────────────────────────────────────

    for offset in WHEEL_OFFSETS {
        let wheel_x = offset.x;
        let wheel_z = offset.z;

        // sign_x: +1 for right side, -1 for left side
        let sign_x: f32 = if wheel_x > 0.0 { 1.0 } else { -1.0 };

        // ── 1. Outer fender flare ─────────────────────────────────────────────
        // Sits just outside the chassis side (chassis half-width = 1.0 m).
        // Y = 0.10 in chassis space keeps it roughly centred on the wheel arch.
        let flare = commands.spawn((
            WheelWell,
            Mesh3d(flare_mesh.clone()),
            MeshMaterial3d(flare_mat.clone()),
            Transform::from_translation(Vec3::new(
                sign_x * 1.05,
                0.10,
                wheel_z,
            )),
        )).id();

        // ── 2. Inner liner ────────────────────────────────────────────────────
        // Sits just above the wheel top (Y = 0.05 in chassis space).
        // Pulled slightly inward (×0.92) so it doesn't poke through the flare.
        let liner = commands.spawn((
            WheelWell,
            Mesh3d(liner_mesh.clone()),
            MeshMaterial3d(liner_mat.clone()),
            Transform::from_translation(Vec3::new(
                wheel_x * 0.92,
                0.05,
                wheel_z,
            )),
        )).id();

        // ── 3. Chrome bolts (front and rear edges of the flare) ───────────────
        // Placed at ±(FLARE_D/2 - BOLT_SIDE) along Z from the flare centre,
        // protruding slightly outward from the flare face (+X for right side).
        let bolt_z_half = FLARE_D * 0.5 - BOLT_SIDE;
        let bolt_x_offset = sign_x * (FLARE_W * 0.5 + BOLT_SIDE * 0.5);

        let bolt_front = commands.spawn((
            WheelWell,
            Mesh3d(bolt_mesh.clone()),
            MeshMaterial3d(bolt_mat.clone()),
            Transform::from_translation(Vec3::new(
                sign_x * 1.05 + bolt_x_offset,
                0.10,
                wheel_z - bolt_z_half,
            )),
        )).id();

        let bolt_rear = commands.spawn((
            WheelWell,
            Mesh3d(bolt_mesh.clone()),
            MeshMaterial3d(bolt_mat.clone()),
            Transform::from_translation(Vec3::new(
                sign_x * 1.05 + bolt_x_offset,
                0.10,
                wheel_z + bolt_z_half,
            )),
        )).id();

        // Attach all four pieces to the chassis entity
        commands.entity(chassis).add_child(flare);
        commands.entity(chassis).add_child(liner);
        commands.entity(chassis).add_child(bolt_front);
        commands.entity(chassis).add_child(bolt_rear);
    }
}
