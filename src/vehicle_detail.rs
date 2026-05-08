// Vehicle detail upgrades: additive geometry that all variants share —
// translucent windshield + side windows, 2 side mirrors, 2 mud flaps behind
// rear wheels, 2 tail lights (red emissive), 2 headlight chrome reflectors,
// 2 door handles. Distinct from variants.rs which adds variant-specific bling
// (TJ grille, Bronco letters).
//
// Every detail mesh carries the `VehicleDetail` marker component so future
// systems can identify, hide, or swap them without touching DefaultSkin /
// VariantSkin children.
//
// Public API:
//   VehicleDetailPlugin
//   VehicleDetail  (component marker)

use bevy::prelude::*;
use std::f32::consts::PI;
use crate::vehicle::VehicleRoot;

// ── Plugin ────────────────────────────────────────────────────────────────────

pub struct VehicleDetailPlugin;

impl Plugin for VehicleDetailPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, attach_details_once);
    }
}

// ── Component marker ─────────────────────────────────────────────────────────

/// Marker placed on every additive detail mesh child (windshield, mirrors,
/// mud flaps, tail lights, headlight reflectors, door handles). Allows future
/// systems to query or despawn detail geometry independently of DefaultSkin /
/// VariantSkin children.
#[derive(Component)]
pub struct VehicleDetail;

// ── One-shot attach system ────────────────────────────────────────────────────

/// Runs every Update frame but executes its body exactly once (guarded by a
/// `Local<bool>`). Waits until `VehicleRoot` is inserted (Startup → Update
/// ordering guarantee), then spawns all additive detail children and attaches
/// them to the chassis entity.
fn attach_details_once(
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

    // ── Helper closures ──────────────────────────────────────────────────────
    // These return Entity ids that are subsequently added as chassis children.

    // ── 1. Windshield ────────────────────────────────────────────────────────
    // Thin angled cuboid: 1.6 × 0.7 × 0.05, translucent blue-tinted glass.
    // Positioned above the cabin front, tilted back −0.4 rad so it reads as
    // a proper raked windshield rather than a flat upright pane.
    let windshield_mesh = meshes.add(Cuboid::new(1.6, 0.7, 0.05));
    let windshield_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(0.4, 0.6, 0.8, 0.4),
        alpha_mode: AlphaMode::Blend,
        perceptual_roughness: 0.05,
        ..default()
    });
    let windshield = commands.spawn((
        VehicleDetail,
        Mesh3d(windshield_mesh),
        MeshMaterial3d(windshield_mat),
        Transform::from_translation(Vec3::new(0.0, 0.6, -1.0))
            .with_rotation(Quat::from_rotation_x(-0.4)),
    )).id();

    // ── 2. Side mirrors (LH + RH) ────────────────────────────────────────────
    // Each mirror = housing cuboid (dark gray) + mirror-face inset (chrome).
    // Mounted at the door / A-pillar junction: (±1.10, 0.45, −1.4).
    let mirror_housing_mesh = meshes.add(Cuboid::new(0.15, 0.18, 0.10));
    let mirror_face_mesh    = meshes.add(Cuboid::new(0.13, 0.16, 0.02));

    let mirror_housing_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.15, 0.15, 0.18),
        perceptual_roughness: 0.7,
        ..default()
    });
    let mirror_face_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.85, 0.85, 0.92),
        perceptual_roughness: 0.1,
        metallic: 0.9,
        ..default()
    });

    let mut mirror_ids: Vec<Entity> = Vec::with_capacity(2);
    for side in [-1.0_f32, 1.0_f32] {
        // Housing
        let housing = commands.spawn((
            VehicleDetail,
            Mesh3d(mirror_housing_mesh.clone()),
            MeshMaterial3d(mirror_housing_mat.clone()),
            Transform::from_translation(Vec3::new(side * 1.10, 0.45, -1.4)),
        )).id();
        // Mirror face — slight forward offset so it sits flush on the front
        // face of the housing (housing half-depth = 0.05, face half-depth = 0.01).
        let face = commands.spawn((
            VehicleDetail,
            Mesh3d(mirror_face_mesh.clone()),
            MeshMaterial3d(mirror_face_mat.clone()),
            // offset: 0.05 (housing front face) - 0.01 (face half) = 0.04 from
            // housing centre, pointing −Z (outward front).
            Transform::from_translation(Vec3::new(0.0, 0.0, -0.04)),
        )).id();
        commands.entity(housing).add_child(face);
        mirror_ids.push(housing);
    }

    // ── 3. Mud flaps (rear LH + RH) ─────────────────────────────────────────
    // Thin rubber flap hanging just behind each rear wheel.
    // 0.30 × 0.40 × 0.04, matte black.
    let mudflap_mesh = meshes.add(Cuboid::new(0.30, 0.40, 0.04));
    let mudflap_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.06, 0.06, 0.06),
        perceptual_roughness: 0.95,
        ..default()
    });

    let mut mudflap_ids: Vec<Entity> = Vec::with_capacity(2);
    for side in [-1.0_f32, 1.0_f32] {
        let flap = commands.spawn((
            VehicleDetail,
            Mesh3d(mudflap_mesh.clone()),
            MeshMaterial3d(mudflap_mat.clone()),
            Transform::from_translation(Vec3::new(side * 1.10, -0.10, 1.85)),
        )).id();
        mudflap_ids.push(flap);
    }

    // ── 4. Tail lights (rear LH + RH) ────────────────────────────────────────
    // Small cuboid on the rear face, red emissive so they glow at night.
    // 0.20 × 0.10 × 0.03, positioned just below centre of the rear face.
    let taillight_mesh = meshes.add(Cuboid::new(0.20, 0.10, 0.03));
    let taillight_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.85, 0.10, 0.10),
        emissive: LinearRgba::rgb(0.8, 0.1, 0.1),
        perceptual_roughness: 0.2,
        ..default()
    });

    let mut taillight_ids: Vec<Entity> = Vec::with_capacity(2);
    for side in [-1.0_f32, 1.0_f32] {
        let tl = commands.spawn((
            VehicleDetail,
            Mesh3d(taillight_mesh.clone()),
            MeshMaterial3d(taillight_mat.clone()),
            Transform::from_translation(Vec3::new(side * 0.70, 0.10, 2.01)),
        )).id();
        taillight_ids.push(tl);
    }

    // ── 5. Headlight chrome reflectors (front LH + RH) ───────────────────────
    // Small cylinder r=0.12, h=0.08, chrome metallic.  The cylinder's default
    // axis is Y; rotate −PI/2 around X so the circular face points forward
    // (−Z), making the reflector dish face the direction of travel.
    let reflector_mesh = meshes.add(Cylinder::new(0.12, 0.08));
    let reflector_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.90, 0.90, 0.95),
        metallic: 0.9,
        perceptual_roughness: 0.05,
        ..default()
    });

    let mut reflector_ids: Vec<Entity> = Vec::with_capacity(2);
    for side in [-1.0_f32, 1.0_f32] {
        let ref_e = commands.spawn((
            VehicleDetail,
            Mesh3d(reflector_mesh.clone()),
            MeshMaterial3d(reflector_mat.clone()),
            Transform::from_translation(Vec3::new(side * 0.70, 0.10, -2.01))
                .with_rotation(Quat::from_rotation_x(-PI / 2.0)),
        )).id();
        reflector_ids.push(ref_e);
    }

    // ── 6. Door handles (LH + RH) ────────────────────────────────────────────
    // Small chrome bar standing slightly proud of the chassis side.
    // 0.20 × 0.04 × 0.04, chrome metallic.
    let handle_mesh = meshes.add(Cuboid::new(0.20, 0.04, 0.04));
    let handle_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.85, 0.85, 0.92),
        metallic: 0.85,
        perceptual_roughness: 0.1,
        ..default()
    });

    let mut handle_ids: Vec<Entity> = Vec::with_capacity(2);
    for side in [-1.0_f32, 1.0_f32] {
        let handle = commands.spawn((
            VehicleDetail,
            Mesh3d(handle_mesh.clone()),
            MeshMaterial3d(handle_mat.clone()),
            Transform::from_translation(Vec3::new(side * 1.005, 0.10, 0.4)),
        )).id();
        handle_ids.push(handle);
    }

    // ── Attach everything to the chassis ────────────────────────────────────
    // add_children is the only reliable parent API in Bevy 0.18 (set_parent_in_place
    // does not correctly attach children to an existing entity hierarchy).
    commands.entity(chassis).add_child(windshield);
    for &id in &mirror_ids   { commands.entity(chassis).add_child(id); }
    for &id in &mudflap_ids  { commands.entity(chassis).add_child(id); }
    for &id in &taillight_ids { commands.entity(chassis).add_child(id); }
    for &id in &reflector_ids { commands.entity(chassis).add_child(id); }
    for &id in &handle_ids   { commands.entity(chassis).add_child(id); }
}
