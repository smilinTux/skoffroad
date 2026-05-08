// Truck bed cargo: a toolbox + 2 jerry cans + a coiled rope + a partial tarp
// spawned in the rear bed of the chassis.  Adds offroad-rig flavor visible
// from the chase cam and FreeOrbit camera.
//
// All cargo entities are children of the chassis rigid-body so they follow
// the vehicle automatically (no additional physics required).
//
// Public API:
//   TruckBedCargoPlugin
//   TruckBedCargo       (marker component placed on every cargo entity)

use bevy::prelude::*;
use crate::vehicle::VehicleRoot;

// ── Plugin ────────────────────────────────────────────────────────────────────

pub struct TruckBedCargoPlugin;

impl Plugin for TruckBedCargoPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, attach_cargo_once);
    }
}

// ── Component ─────────────────────────────────────────────────────────────────

/// Marker component placed on every entity that forms part of the truck-bed
/// cargo load (toolbox, jerry cans, rope coil, tarp).
#[derive(Component)]
pub struct TruckBedCargo;

// ── Constants ─────────────────────────────────────────────────────────────────

// Chassis geometry (mirrors vehicle.rs):
//   full size 2.0 × 0.6 × 4.0   → half-extents 1.0 × 0.3 × 2.0
//   chassis top face at local Y = +0.30
//
// "Rear bed" zone: chassis-local Z >= +1.0.
// Cargo sits above the top face, so base Y for flat pieces is 0.30 + half-height.

// ── Toolbox ──────────────────────────────────────────────────────────────────
// A sturdy metal crate strapped across the bed.
const TOOLBOX_SIZE:  (f32, f32, f32) = (0.60, 0.25, 0.40);
const TOOLBOX_COLOR: Color           = Color::srgb(0.20, 0.20, 0.22);
const TOOLBOX_POS:   Vec3            = Vec3::new(0.0, 0.45, 1.5);

// ── Jerry cans ───────────────────────────────────────────────────────────────
// Army-olive fuel cans, one per side.
const CAN_BODY_SIZE:  (f32, f32, f32) = (0.20, 0.35, 0.30);
const CAN_COLOR:      Color           = Color::srgb(0.22, 0.30, 0.18);
const CAN_POS_LH:     Vec3            = Vec3::new(-0.65, 0.50, 1.4);
const CAN_POS_RH:     Vec3            = Vec3::new( 0.65, 0.50, 1.4);

// Carry handle on top of each can.
const HANDLE_SIZE:    (f32, f32, f32) = (0.18, 0.05, 0.04);
const HANDLE_COLOR:   Color           = Color::srgb(0.60, 0.60, 0.65);

// Filler spout on the short side of each can.
const SPOUT_RADIUS:   f32             = 0.04;
const SPOUT_HEIGHT:   f32             = 0.06;

// ── Rope coil ─────────────────────────────────────────────────────────────────
// Flat cylinder (torus substitute) — tan hemp colour.
const ROPE_RADIUS:    f32             = 0.20;
const ROPE_HEIGHT:    f32             = 0.10;
const ROPE_COLOR:     Color           = Color::srgb(0.65, 0.55, 0.40);
const ROPE_POS:       Vec3            = Vec3::new(0.0, 0.42, 1.85);

// ── Tarp / partial cover ──────────────────────────────────────────────────────
// Thin dark-green sheet under the toolbox — suggests a utility tarp draped
// over the bed floor.
const TARP_SIZE:      (f32, f32, f32) = (1.40, 0.04, 0.60);
const TARP_COLOR:     Color           = Color::srgb(0.15, 0.20, 0.12);
const TARP_POS:       Vec3            = Vec3::new(0.0, 0.32, 1.5);

// ── System ────────────────────────────────────────────────────────────────────

/// Spawns all cargo geometry as children of the chassis entity.
/// Runs every frame but executes only once (guarded by `Local<bool>`).
fn attach_cargo_once(
    mut done:      Local<bool>,
    vehicle:       Option<Res<VehicleRoot>>,
    mut commands:  Commands,
    mut meshes:    ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if *done { return; }
    let Some(vehicle) = vehicle else { return };
    *done = true;

    let chassis = vehicle.chassis;

    // ── Material helpers ──────────────────────────────────────────────────────

    let toolbox_mat = materials.add(StandardMaterial {
        base_color:           TOOLBOX_COLOR,
        perceptual_roughness: 0.60,
        metallic:             0.40,
        ..default()
    });

    let can_mat = materials.add(StandardMaterial {
        base_color:           CAN_COLOR,
        perceptual_roughness: 0.80,
        ..default()
    });

    let handle_mat = materials.add(StandardMaterial {
        base_color:           HANDLE_COLOR,
        perceptual_roughness: 0.30,
        metallic:             0.70,
        ..default()
    });

    let rope_mat = materials.add(StandardMaterial {
        base_color:           ROPE_COLOR,
        perceptual_roughness: 0.95,
        ..default()
    });

    let tarp_mat = materials.add(StandardMaterial {
        base_color:           TARP_COLOR,
        perceptual_roughness: 0.90,
        ..default()
    });

    // ── Tarp (spawn first so it visually sits below the toolbox) ──────────────

    let tarp_mesh = meshes.add(Cuboid::new(TARP_SIZE.0, TARP_SIZE.1, TARP_SIZE.2));
    let tarp = commands.spawn((
        TruckBedCargo,
        Mesh3d(tarp_mesh),
        MeshMaterial3d(tarp_mat),
        Transform::from_translation(TARP_POS),
    )).id();
    commands.entity(chassis).add_child(tarp);

    // ── Toolbox ───────────────────────────────────────────────────────────────

    let toolbox_mesh = meshes.add(Cuboid::new(
        TOOLBOX_SIZE.0, TOOLBOX_SIZE.1, TOOLBOX_SIZE.2,
    ));
    let toolbox = commands.spawn((
        TruckBedCargo,
        Mesh3d(toolbox_mesh),
        MeshMaterial3d(toolbox_mat),
        Transform::from_translation(TOOLBOX_POS),
    )).id();
    commands.entity(chassis).add_child(toolbox);

    // ── Jerry cans ────────────────────────────────────────────────────────────

    let can_mesh    = meshes.add(Cuboid::new(
        CAN_BODY_SIZE.0, CAN_BODY_SIZE.1, CAN_BODY_SIZE.2,
    ));
    let handle_mesh = meshes.add(Cuboid::new(
        HANDLE_SIZE.0, HANDLE_SIZE.1, HANDLE_SIZE.2,
    ));
    let spout_mesh  = meshes.add(Cylinder::new(SPOUT_RADIUS, SPOUT_HEIGHT));

    // Handle Y is half the can body height above the can origin so it sits on top.
    let handle_local_y = CAN_BODY_SIZE.1 * 0.5 + HANDLE_SIZE.1 * 0.5;
    // Spout projects from the +Z face (forward side) of the can body at mid-height.
    let spout_local_z  = CAN_BODY_SIZE.2 * 0.5 + SPOUT_HEIGHT * 0.5;

    for &can_pos in &[CAN_POS_LH, CAN_POS_RH] {
        // Can body.
        let can_body = commands.spawn((
            TruckBedCargo,
            Mesh3d(can_mesh.clone()),
            MeshMaterial3d(can_mat.clone()),
            Transform::from_translation(can_pos),
        )).id();
        commands.entity(chassis).add_child(can_body);

        // Chrome carry handle (child of the can body so it moves with it).
        let handle = commands.spawn((
            TruckBedCargo,
            Mesh3d(handle_mesh.clone()),
            MeshMaterial3d(handle_mat.clone()),
            Transform::from_translation(Vec3::new(0.0, handle_local_y, 0.0)),
        )).id();
        commands.entity(can_body).add_child(handle);

        // Small filler spout on the +Z face; rotate the cylinder 90° around X
        // so it points along +Z (the cylinder's long axis is initially Y).
        let spout = commands.spawn((
            TruckBedCargo,
            Mesh3d(spout_mesh.clone()),
            MeshMaterial3d(can_mat.clone()),
            Transform::from_translation(Vec3::new(0.0, 0.0, spout_local_z))
                .with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
        )).id();
        commands.entity(can_body).add_child(spout);
    }

    // ── Coiled rope ───────────────────────────────────────────────────────────

    let rope_mesh = meshes.add(Cylinder::new(ROPE_RADIUS, ROPE_HEIGHT));
    let rope = commands.spawn((
        TruckBedCargo,
        Mesh3d(rope_mesh),
        MeshMaterial3d(rope_mat),
        Transform::from_translation(ROPE_POS),
    )).id();
    commands.entity(chassis).add_child(rope);
}
