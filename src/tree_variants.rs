// Tree variants: 4 visually distinct procedural tree types (pine, oak, palm, dead).
// Spawns ~24 trees at LCG-deterministic positions (seed=200), 6 of each variant.
// Additive placement — does NOT refactor scatter.rs.
//
// Public API:
//   TreeVariantsPlugin
//   TreeVariant  (marker component)

use bevy::prelude::*;
use avian3d::prelude::*;

use crate::terrain::terrain_height_at;

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct TreeVariantsPlugin;

impl Plugin for TreeVariantsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_tree_variants);
    }
}

// ---------------------------------------------------------------------------
// Marker component
// ---------------------------------------------------------------------------

/// Marks the root entity of every tree-variant tree.
#[derive(Component)]
pub struct TreeVariant;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Minimum distance from the origin before placing a tree.
const ORIGIN_CLEAR: f32 = 30.0;

/// World half-extent (±100 m square).
const WORLD_HALF: f32 = 100.0;

/// Number of each variant to spawn.
const PER_VARIANT: u32 = 6;

// ---------------------------------------------------------------------------
// LCG
// ---------------------------------------------------------------------------

struct Lcg(u64);

impl Lcg {
    fn new(seed: u32) -> Self {
        Self(seed as u64)
    }

    /// Next float in [0, 1).
    fn next_f32(&mut self) -> f32 {
        self.0 = self.0
            .wrapping_mul(1_664_525)
            .wrapping_add(1_013_904_223)
            & 0xFFFF_FFFF;
        (self.0 as f32) / (u32::MAX as f32)
    }
}

// ---------------------------------------------------------------------------
// Startup system
// ---------------------------------------------------------------------------

fn spawn_tree_variants(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // --- shared single-cylinder collider (one bounding cylinder for every tree) ---
    // radius 0.3, half-height 2.5 → full height 5.0
    const COLLIDER_RADIUS: f32 = 0.3;
    const COLLIDER_HALF_H: f32 = 2.5;

    // --- Pine materials & meshes ---
    let pine_trunk_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.30, 0.20, 0.12),
        perceptual_roughness: 0.9,
        ..default()
    });
    let pine_foliage_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.10, 0.35, 0.18),
        perceptual_roughness: 0.85,
        ..default()
    });
    // Trunk: cylinder r=0.3, h=5
    let pine_trunk_mesh = meshes.add(Cylinder::new(0.3, 5.0));
    // Foliage: 4 stacked cones (largest bottom → smallest top)
    let pine_cone0 = meshes.add(Cone { radius: 2.5, height: 2.5 });
    let pine_cone1 = meshes.add(Cone { radius: 2.0, height: 2.5 });
    let pine_cone2 = meshes.add(Cone { radius: 1.5, height: 2.5 });
    let pine_cone3 = meshes.add(Cone { radius: 1.0, height: 2.0 });

    // --- Oak materials & meshes ---
    let oak_trunk_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.35, 0.25, 0.15),
        perceptual_roughness: 0.9,
        ..default()
    });
    let oak_crown_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.20, 0.55, 0.25),
        perceptual_roughness: 0.85,
        ..default()
    });
    // Trunk: cylinder r=0.5, h=3
    let oak_trunk_mesh = meshes.add(Cylinder::new(0.5, 3.0));
    // Crown: sphere r=2.5, ico(2)
    let oak_crown_mesh = meshes.add(Sphere::new(2.5).mesh().ico(2).unwrap());

    // --- Palm materials & meshes ---
    let palm_trunk_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.55, 0.40, 0.25),
        perceptual_roughness: 0.88,
        ..default()
    });
    let palm_frond_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.30, 0.65, 0.25),
        perceptual_roughness: 0.80,
        ..default()
    });
    // Trunk: cylinder r=0.25, h=6
    let palm_trunk_mesh = meshes.add(Cylinder::new(0.25, 6.0));
    // Frond: thin cuboid 0.2 × 1.5 × 0.05 (6 per tree, radiating out)
    let palm_frond_mesh = meshes.add(Cuboid::new(0.2, 1.5, 0.05));

    // --- Dead tree materials & meshes ---
    let dead_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.40, 0.32, 0.25),
        perceptual_roughness: 0.92,
        ..default()
    });
    // Trunk: cylinder r=0.4, h=4
    let dead_trunk_mesh = meshes.add(Cylinder::new(0.4, 4.0));
    // Branches: thin cylinder r=0.06, h=1.5 (5 per tree)
    let dead_branch_mesh = meshes.add(Cylinder::new(0.06, 1.5));

    // ---------------------------------------------------------------------------
    // LCG placement — seed 200
    // ---------------------------------------------------------------------------

    let mut lcg = Lcg::new(200);

    // We draw candidates and accept them if they are outside the origin clear
    // radius; cap attempts to avoid an infinite loop.
    let variants: [u32; 4] = [PER_VARIANT; 4]; // pine, oak, palm, dead
    let mut counts = [0u32; 4];
    let total_needed: u32 = variants.iter().sum();
    let mut placed = 0u32;
    let mut attempts = 0u32;

    while placed < total_needed && attempts < 10_000 {
        attempts += 1;

        let x = (lcg.next_f32() * 2.0 - 1.0) * WORLD_HALF;
        let z = (lcg.next_f32() * 2.0 - 1.0) * WORLD_HALF;
        let rot_y = lcg.next_f32() * std::f32::consts::TAU;

        if x * x + z * z < ORIGIN_CLEAR * ORIGIN_CLEAR {
            continue;
        }

        // Pick the next variant that still needs trees (round-robin by modulo)
        let variant_idx = (placed % 4) as usize;
        if counts[variant_idx] >= variants[variant_idx] {
            // Find any variant that still needs trees
            let maybe = (0..4usize).find(|&i| counts[i] < variants[i]);
            if maybe.is_none() {
                break;
            }
        }
        // Re-pick: find the first variant that still needs trees in round-robin order
        let variant_idx = {
            let mut idx = (placed % 4) as usize;
            let mut found = false;
            for _ in 0..4 {
                if counts[idx] < variants[idx] {
                    found = true;
                    break;
                }
                idx = (idx + 1) % 4;
            }
            if !found { break; }
            idx
        };

        let ground_y = terrain_height_at(x, z);

        match variant_idx {
            0 => spawn_pine(
                &mut commands,
                &pine_trunk_mesh,
                &pine_cone0,
                &pine_cone1,
                &pine_cone2,
                &pine_cone3,
                &pine_trunk_mat,
                &pine_foliage_mat,
                Vec3::new(x, ground_y, z),
                rot_y,
                COLLIDER_RADIUS,
                COLLIDER_HALF_H,
            ),
            1 => spawn_oak(
                &mut commands,
                &oak_trunk_mesh,
                &oak_crown_mesh,
                &oak_trunk_mat,
                &oak_crown_mat,
                Vec3::new(x, ground_y, z),
                rot_y,
                COLLIDER_RADIUS,
                COLLIDER_HALF_H,
            ),
            2 => spawn_palm(
                &mut commands,
                &palm_trunk_mesh,
                &palm_frond_mesh,
                &palm_trunk_mat,
                &palm_frond_mat,
                Vec3::new(x, ground_y, z),
                rot_y,
                COLLIDER_RADIUS,
                COLLIDER_HALF_H,
            ),
            3 => spawn_dead(
                &mut commands,
                &dead_trunk_mesh,
                &dead_branch_mesh,
                &dead_mat,
                Vec3::new(x, ground_y, z),
                rot_y,
                COLLIDER_RADIUS,
                COLLIDER_HALF_H,
            ),
            _ => unreachable!(),
        }

        counts[variant_idx] += 1;
        placed += 1;
    }

    bevy::log::info!(
        "tree_variants: {} trees placed ({} pine, {} oak, {} palm, {} dead) in {} attempts",
        placed,
        counts[0],
        counts[1],
        counts[2],
        counts[3],
        attempts,
    );
}

// ---------------------------------------------------------------------------
// Pine
// ---------------------------------------------------------------------------
//
// Trunk: cylinder r=0.3, h=5 — centred at Y=2.5
// Foliage layers (bottom up, each cone centred at its own height):
//   cone0 Cone(r=2.5, h=2.5) — bottom of foliage at Y=4 (trunk top), centre Y=4+1.25=5.25
//   cone1 Cone(r=2.0, h=2.5) — centre Y=5.25+1.5=6.75  (overlap 1.25 each side keeps tiers flush)
//   cone2 Cone(r=1.5, h=2.5) — centre Y=6.75+1.5=8.25
//   cone3 Cone(r=1.0, h=2.0) — centre Y=8.25+1.5=9.75 (tighter at apex)

#[allow(clippy::too_many_arguments)]
fn spawn_pine(
    commands: &mut Commands,
    trunk_mesh: &Handle<Mesh>,
    cone0: &Handle<Mesh>,
    cone1: &Handle<Mesh>,
    cone2: &Handle<Mesh>,
    cone3: &Handle<Mesh>,
    trunk_mat: &Handle<StandardMaterial>,
    foliage_mat: &Handle<StandardMaterial>,
    origin: Vec3,
    yaw: f32,
    col_r: f32,
    col_hh: f32,
) {
    // Collider centred at mid-height (2.5 m up): half-height = col_hh=2.5
    let col_centre_y = col_hh;

    let parent = commands.spawn((
        TreeVariant,
        Transform::from_translation(Vec3::new(origin.x, origin.y + col_centre_y, origin.z))
            .with_rotation(Quat::from_rotation_y(yaw)),
        Visibility::default(),
        RigidBody::Static,
        Collider::cylinder(col_r, col_hh * 2.0),
    )).id();

    // Parent is at world Y = origin.y + col_centre_y (= origin.y + 2.5).
    // All child positions below are in local space relative to that parent.

    // Trunk: cylinder h=5, world-centre = origin.y + 2.5 → local Y = 0.0
    let trunk = commands.spawn((
        Mesh3d(trunk_mesh.clone()),
        MeshMaterial3d(trunk_mat.clone()),
        Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
    )).id();

    // Cone tiers stacked above the trunk top (local Y = 2.5 = trunk-top-local).
    // Cone primitive in Bevy has tip at +Y/2 and base at -Y/2.
    let tier_step = 1.8_f32;
    let foliage_base_local = 2.5_f32; // trunk top in local space (world 5.0 - parent_offset 2.5)

    let tier0 = commands.spawn((
        Mesh3d(cone0.clone()),
        MeshMaterial3d(foliage_mat.clone()),
        // cone0 height=2.5, centre = foliage_base + 1.25
        Transform::from_translation(Vec3::new(0.0, foliage_base_local + 1.25, 0.0)),
    )).id();

    let tier1 = commands.spawn((
        Mesh3d(cone1.clone()),
        MeshMaterial3d(foliage_mat.clone()),
        Transform::from_translation(Vec3::new(0.0, foliage_base_local + 1.25 + tier_step, 0.0)),
    )).id();

    let tier2 = commands.spawn((
        Mesh3d(cone2.clone()),
        MeshMaterial3d(foliage_mat.clone()),
        Transform::from_translation(Vec3::new(0.0, foliage_base_local + 1.25 + tier_step * 2.0, 0.0)),
    )).id();

    let tier3 = commands.spawn((
        Mesh3d(cone3.clone()),
        MeshMaterial3d(foliage_mat.clone()),
        // cone3 height=2.0, centre offset = 1.0
        Transform::from_translation(Vec3::new(0.0, foliage_base_local + 1.0 + tier_step * 3.0, 0.0)),
    )).id();

    commands.entity(parent).add_children(&[trunk, tier0, tier1, tier2, tier3]);
}

// ---------------------------------------------------------------------------
// Oak
// ---------------------------------------------------------------------------
//
// Trunk: cylinder r=0.5, h=3 — centred at Y=1.5
// Crown: sphere r=2.5 — centred at Y=3+2.5=5.5 (just above trunk top)

fn spawn_oak(
    commands: &mut Commands,
    trunk_mesh: &Handle<Mesh>,
    crown_mesh: &Handle<Mesh>,
    trunk_mat: &Handle<StandardMaterial>,
    crown_mat: &Handle<StandardMaterial>,
    origin: Vec3,
    yaw: f32,
    col_r: f32,
    col_hh: f32,
) {
    let col_centre_y = col_hh;

    let parent = commands.spawn((
        TreeVariant,
        Transform::from_translation(Vec3::new(origin.x, origin.y + col_centre_y, origin.z))
            .with_rotation(Quat::from_rotation_y(yaw)),
        Visibility::default(),
        RigidBody::Static,
        Collider::cylinder(col_r, col_hh * 2.0),
    )).id();

    let trunk = commands.spawn((
        Mesh3d(trunk_mesh.clone()),
        MeshMaterial3d(trunk_mat.clone()),
        // trunk h=3, centred at Y=1.5 relative to origin (which is at col_centre_y=2.5)
        // local Y = 1.5 - 2.5 = -1.0
        Transform::from_translation(Vec3::new(0.0, -1.0, 0.0)),
    )).id();

    let crown = commands.spawn((
        Mesh3d(crown_mesh.clone()),
        MeshMaterial3d(crown_mat.clone()),
        // trunk top at Y=3 (world), which is Y=3-2.5=0.5 local; crown centre at 0.5+2.5=3.0 local
        Transform::from_translation(Vec3::new(0.0, 3.0, 0.0)),
    )).id();

    commands.entity(parent).add_children(&[trunk, crown]);
}

// ---------------------------------------------------------------------------
// Palm
// ---------------------------------------------------------------------------
//
// Trunk: cylinder r=0.25, h=6 — centred at Y=3
// Fronds: 6 thin cuboids (0.2 × 1.5 × 0.05) radiating out from trunk top (Y=6),
//         tilted ~30° down from horizontal (rotation around local X), then rotated
//         around Y by 0°, 60°, 120°, 180°, 240°, 300°.

fn spawn_palm(
    commands: &mut Commands,
    trunk_mesh: &Handle<Mesh>,
    frond_mesh: &Handle<Mesh>,
    trunk_mat: &Handle<StandardMaterial>,
    frond_mat: &Handle<StandardMaterial>,
    origin: Vec3,
    yaw: f32,
    col_r: f32,
    col_hh: f32,
) {
    let col_centre_y = col_hh;

    let parent = commands.spawn((
        TreeVariant,
        Transform::from_translation(Vec3::new(origin.x, origin.y + col_centre_y, origin.z))
            .with_rotation(Quat::from_rotation_y(yaw)),
        Visibility::default(),
        RigidBody::Static,
        Collider::cylinder(col_r, col_hh * 2.0),
    )).id();

    // Trunk centred at local Y = 3 - col_centre_y = 3 - 2.5 = 0.5
    let trunk = commands.spawn((
        Mesh3d(trunk_mesh.clone()),
        MeshMaterial3d(trunk_mat.clone()),
        Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
    )).id();

    let trunk_top_local = 6.0 - col_centre_y; // 3.5
    // frond half-length = 0.75 (half of 1.5 m), tilted 30° down from horizontal
    let tilt = 30.0_f32.to_radians(); // angle below horizontal
    // frond centre offset: radial = 0.75*cos(tilt), vertical = -0.75*sin(tilt)
    let radial_offset = 0.75 * tilt.cos();
    let vertical_offset = -0.75 * tilt.sin();

    let mut children: Vec<Entity> = vec![trunk];

    for i in 0..6u32 {
        let angle = i as f32 * std::f32::consts::TAU / 6.0;
        let dx = angle.sin() * radial_offset;
        let dz = angle.cos() * radial_offset;

        // Rotation: first tilt down from horizontal around local X, then rotate around Y
        // The frond cuboid is oriented along its Y axis (height=1.5), so we need to
        // rotate it so it points outward and down.
        // 1. Start with frond along +Y axis.
        // 2. Rotate -90° around Z to make it lie along +X.
        // 3. Tilt it down by `tilt` around Z.
        // 4. Rotate around Y by `angle`.
        let rot = Quat::from_rotation_y(angle)
            * Quat::from_rotation_z(-(std::f32::consts::FRAC_PI_2 - tilt));

        let frond = commands.spawn((
            Mesh3d(frond_mesh.clone()),
            MeshMaterial3d(frond_mat.clone()),
            Transform {
                translation: Vec3::new(dx, trunk_top_local + vertical_offset, dz),
                rotation: rot,
                scale: Vec3::ONE,
            },
        )).id();
        children.push(frond);
    }

    commands.entity(parent).add_children(&children);
}

// ---------------------------------------------------------------------------
// Dead tree
// ---------------------------------------------------------------------------
//
// Trunk: cylinder r=0.4, h=4 — centred at Y=2
// Branches: 5 thin cylinders (r=0.06, h=1.5) at the top of the trunk,
//           radiating outward at deterministic angles.

fn spawn_dead(
    commands: &mut Commands,
    trunk_mesh: &Handle<Mesh>,
    branch_mesh: &Handle<Mesh>,
    mat: &Handle<StandardMaterial>,
    origin: Vec3,
    yaw: f32,
    col_r: f32,
    col_hh: f32,
) {
    let col_centre_y = col_hh;

    let parent = commands.spawn((
        TreeVariant,
        Transform::from_translation(Vec3::new(origin.x, origin.y + col_centre_y, origin.z))
            .with_rotation(Quat::from_rotation_y(yaw)),
        Visibility::default(),
        RigidBody::Static,
        Collider::cylinder(col_r, col_hh * 2.0),
    )).id();

    // Trunk centred at local Y = 2 - col_centre_y = 2 - 2.5 = -0.5
    let trunk = commands.spawn((
        Mesh3d(trunk_mesh.clone()),
        MeshMaterial3d(mat.clone()),
        Transform::from_translation(Vec3::new(0.0, -0.5, 0.0)),
    )).id();

    let trunk_top_local = 4.0 - col_centre_y; // 1.5

    let mut children: Vec<Entity> = vec![trunk];

    // 5 branches at evenly-spaced azimuths (offset so they don't align with axes)
    let branch_half_h = 0.75_f32;
    // outward tilt from vertical: ~50° so branches droop convincingly
    let tilt_from_vertical = 50.0_f32.to_radians();

    for i in 0..5u32 {
        let azimuth = i as f32 * std::f32::consts::TAU / 5.0 + 0.4; // 0.4 rad offset

        // Branch centre: trunk_top + half_h along the branch direction
        let dx = azimuth.sin() * tilt_from_vertical.sin() * branch_half_h;
        let dy = tilt_from_vertical.cos() * branch_half_h;
        let dz = azimuth.cos() * tilt_from_vertical.sin() * branch_half_h;

        // Rotation: cylinder is along Y; we tilt it outward from vertical.
        // Rotate around Z by tilt_from_vertical, then rotate around Y by azimuth.
        let rot = Quat::from_rotation_y(azimuth)
            * Quat::from_rotation_z(tilt_from_vertical);

        let branch = commands.spawn((
            Mesh3d(branch_mesh.clone()),
            MeshMaterial3d(mat.clone()),
            Transform {
                translation: Vec3::new(
                    trunk_top_local * azimuth.sin() * tilt_from_vertical.sin() * 0.5 + dx,
                    trunk_top_local + dy,
                    trunk_top_local * azimuth.cos() * tilt_from_vertical.sin() * 0.5 + dz,
                ),
                rotation: rot,
                scale: Vec3::ONE,
            },
        )).id();
        children.push(branch);
    }

    commands.entity(parent).add_children(&children);
}
