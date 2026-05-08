// Engine bay: procedural V8 engine block visible through the grille of
// every variant. Composed of cuboid block + 8 cylinder lifters + air
// intake + alternator + valve covers. Visible from front-facing camera
// modes and through the windshield (translucent windshield in
// vehicle_detail).
//
// Chassis local-space reference:
//   Chassis half-extents: 1.0 x 0.4 x 2.0  (full: 2.0 x 0.8 x 4.0)
//   Front face at Z = -2.0.
//   Engine block sits at Z = -1.4 (0.6 m behind front face).
//
// Public API:
//   EngineBayPlugin
//   EngineBayPart  (marker component)

use bevy::prelude::*;
use std::f32::consts::PI;
use crate::vehicle::VehicleRoot;

// ── Plugin ────────────────────────────────────────────────────────────────────

pub struct EngineBayPlugin;

impl Plugin for EngineBayPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, attach_engine_once);
    }
}

// ── Component marker ─────────────────────────────────────────────────────────

/// Marker placed on every engine-bay mesh child. Allows future systems to
/// query, hide, or swap engine geometry independently of DefaultSkin /
/// VariantSkin / VehicleDetail children.
#[derive(Component)]
pub struct EngineBayPart;

// ── One-shot attach system ────────────────────────────────────────────────────

/// Runs every Update frame but executes its body exactly once (guarded by a
/// `Local<bool>`). Waits until `VehicleRoot` is inserted, then spawns all
/// engine-bay children and attaches them to the chassis entity.
fn attach_engine_once(
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

    // Engine block body — dark steel
    let block_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.20, 0.20, 0.22),
        metallic: 0.6,
        perceptual_roughness: 0.5,
        ..default()
    });

    // Cylinder lifters — near-black cast iron
    let lifter_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.10, 0.10, 0.12),
        metallic: 0.7,
        perceptual_roughness: 0.45,
        ..default()
    });

    // Valve covers — polished chrome
    let chrome_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.85, 0.85, 0.92),
        metallic: 0.9,
        perceptual_roughness: 0.1,
        ..default()
    });

    // Air intake — matte black
    let intake_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.08, 0.08, 0.08),
        metallic: 0.3,
        perceptual_roughness: 0.8,
        ..default()
    });

    // Belt — flat black rubber
    let belt_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.04, 0.04, 0.04),
        metallic: 0.0,
        perceptual_roughness: 0.95,
        ..default()
    });

    // ── 1. Engine block ───────────────────────────────────────────────────────
    // Cuboid 1.0 × 0.5 × 0.7 centred at chassis-local (0, 0.10, -1.4).
    // That puts it right behind the front grille (front face at Z = -2.0)
    // and between the front wheel wells.
    let block_mesh = meshes.add(Cuboid::new(1.0, 0.5, 0.7));
    let engine_block = commands.spawn((
        EngineBayPart,
        Mesh3d(block_mesh),
        MeshMaterial3d(block_mat),
        Transform::from_translation(Vec3::new(0.0, 0.10, -1.4)),
    )).id();

    // ── 2. V8 cylinder lifters (8 total, two banks of 4) ─────────────────────
    // Each cylinder: r=0.08, h=0.20.
    // V-configuration: left bank tilted -30° around Z, right bank +30°.
    // Positioned along X at -0.30, -0.10, 0.10, 0.30 per bank.
    // Bank origin: chassis-local Y = 0.35 (top of engine block),
    //              Z = -1.4 (same as block centre).
    let lifter_mesh = meshes.add(Cylinder::new(0.08, 0.20));
    let tilt_left  = Quat::from_rotation_z(-PI / 6.0);   // -30°
    let tilt_right = Quat::from_rotation_z( PI / 6.0);   //  +30°

    // left bank: negative-X side of V
    let left_bank_offset = Vec3::new(-0.10, 0.35, -1.4);
    // right bank: positive-X side of V
    let right_bank_offset = Vec3::new(0.10, 0.35, -1.4);

    let mut lifter_ids: Vec<Entity> = Vec::with_capacity(8);
    for i in 0..4_i32 {
        let x = -0.30 + i as f32 * 0.20;

        // Left bank — leaning left
        let left = commands.spawn((
            EngineBayPart,
            Mesh3d(lifter_mesh.clone()),
            MeshMaterial3d(lifter_mat.clone()),
            Transform::from_translation(left_bank_offset + Vec3::new(x, 0.0, 0.0))
                .with_rotation(tilt_left),
        )).id();

        // Right bank — leaning right
        let right = commands.spawn((
            EngineBayPart,
            Mesh3d(lifter_mesh.clone()),
            MeshMaterial3d(lifter_mat.clone()),
            Transform::from_translation(right_bank_offset + Vec3::new(x, 0.0, 0.0))
                .with_rotation(tilt_right),
        )).id();

        lifter_ids.push(left);
        lifter_ids.push(right);
    }

    // ── 3. Valve covers (one per bank) ────────────────────────────────────────
    // Cuboid 0.85 × 0.06 × 0.14 each, sitting on top of the cylinder banks,
    // slightly tilted to match bank angle.
    // Positioned at the same height / Z as the lifter tops.
    let valve_cover_mesh = meshes.add(Cuboid::new(0.85, 0.06, 0.14));

    // Left valve cover: centred at x = -0.10, y = 0.46, z = -1.4
    let valve_left = commands.spawn((
        EngineBayPart,
        Mesh3d(valve_cover_mesh.clone()),
        MeshMaterial3d(chrome_mat.clone()),
        Transform::from_translation(Vec3::new(-0.10, 0.46, -1.4))
            .with_rotation(tilt_left),
    )).id();

    // Right valve cover: centred at x = 0.10, y = 0.46, z = -1.4
    let valve_right = commands.spawn((
        EngineBayPart,
        Mesh3d(valve_cover_mesh),
        MeshMaterial3d(chrome_mat.clone()),
        Transform::from_translation(Vec3::new(0.10, 0.46, -1.4))
            .with_rotation(tilt_right),
    )).id();

    // ── 4. Air intake ─────────────────────────────────────────────────────────
    // Dark cuboid 0.4 × 0.25 × 0.3 on top-centre between the two valve covers.
    // Chrome filter cap: cylinder r=0.18, h=0.06 sitting on top.
    let intake_body_mesh = meshes.add(Cuboid::new(0.4, 0.25, 0.3));
    let intake_body = commands.spawn((
        EngineBayPart,
        Mesh3d(intake_body_mesh),
        MeshMaterial3d(intake_mat),
        Transform::from_translation(Vec3::new(0.0, 0.60, -1.4)),
    )).id();

    let filter_mesh = meshes.add(Cylinder::new(0.18, 0.06));
    let filter_cap = commands.spawn((
        EngineBayPart,
        Mesh3d(filter_mesh),
        MeshMaterial3d(chrome_mat.clone()),
        Transform::from_translation(Vec3::new(0.0, 0.76, -1.4)),
    )).id();

    // ── 5. Alternator ─────────────────────────────────────────────────────────
    // Small cylinder r=0.12, h=0.18 on the left side of the engine block.
    // Cylinder default axis is Y; rotate 90° around Z so it lies along X
    // (its circular face points outward from the side of the block).
    let alternator_mesh = meshes.add(Cylinder::new(0.12, 0.18));
    let alternator = commands.spawn((
        EngineBayPart,
        Mesh3d(alternator_mesh),
        MeshMaterial3d(chrome_mat.clone()),
        Transform::from_translation(Vec3::new(-0.45, 0.10, -1.4))
            .with_rotation(Quat::from_rotation_z(PI / 2.0)),
    )).id();

    // ── 6. Belt ───────────────────────────────────────────────────────────────
    // Thin cuboid 0.04 × 0.01 × 0.36 bridging alternator to engine centre pulley.
    // Stretched along X between x = -0.45 (alternator) and x = -0.09 (block).
    // Width (Y) 0.01 keeps it flush and barely visible as a thin strap.
    let belt_mesh = meshes.add(Cuboid::new(0.36, 0.01, 0.04));
    let belt = commands.spawn((
        EngineBayPart,
        Mesh3d(belt_mesh),
        MeshMaterial3d(belt_mat),
        Transform::from_translation(Vec3::new(-0.27, 0.10, -1.4)),
    )).id();

    // ── Attach all parts to the chassis ──────────────────────────────────────
    // add_child is the reliable parent API in Bevy 0.18.
    commands.entity(chassis).add_child(engine_block);
    for &id in &lifter_ids {
        commands.entity(chassis).add_child(id);
    }
    commands.entity(chassis).add_child(valve_left);
    commands.entity(chassis).add_child(valve_right);
    commands.entity(chassis).add_child(intake_body);
    commands.entity(chassis).add_child(filter_cap);
    commands.entity(chassis).add_child(alternator);
    commands.entity(chassis).add_child(belt);
}
