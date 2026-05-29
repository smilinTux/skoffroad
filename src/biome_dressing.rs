// Sprint 77 — Biome Dressing  (Phase B: density + biomes + landmarks)
//
// BiomeDressingPlugin places dense, biome-themed scatter and hero landmarks
// around each mode area so every zone has a distinct look and feel.
//
// Five biome zones:
//   1. ALPINE       — around Hillclimb (Z = -180 / -210 / -240)
//   2. RED-ROCK     — around Rock Crawl (sections at (120,0),(-80,80),(60,-120))
//   3. DESERT FLAT  — around Obstacle Course (Z = +200 / +230 / +260)
//   4. PINE FOREST  — around Trail Rides (origin + east corridor)
//   5. MEADOW       — around Spawn / Ranger area (origin)
//
// Five hero landmarks (one per area):
//   1. Radio / ski-lift tower on the Hillclimb ridge
//   2. Natural stone arch at Rock Crawl
//   3. Water tower on stilts at Obstacle Course
//   4. Fire lookout cabin on tall stilts at Trail Rides
//   5. Welcome archway sign at Spawn ("S&K OFFROAD PARK")
//
// Connecting trail ribbons: a series of flat darker quads Y-snapped to
// terrain linking spawn toward each mode area.
//
// Rules enforced:
//   - NO BorderRadius (not a bundle in Bevy 0.18)
//   - NO Unicode glyphs in text — ASCII only
//   - PropLod on every spawned entity (250 m cull, 8-frame cadence)
//   - Colliders on trees and boulders; NONE on grass, flowers, ferns, scrub
//   - RigidBody::Static only on collider-bearing items

use bevy::prelude::*;
use avian3d::prelude::*;

use crate::terrain::terrain_height_at;
use crate::prop_lod::PropLod;

// ---------------------------------------------------------------------------
// Marker component
// ---------------------------------------------------------------------------

/// Applied to every entity spawned by BiomeDressingPlugin.
#[derive(Component)]
pub struct BiomeProp;

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct BiomeDressingPlugin;

impl Plugin for BiomeDressingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Startup,
            (
                spawn_alpine_biome,
                spawn_redrock_biome,
                spawn_desert_biome,
                spawn_pine_forest_biome,
                spawn_meadow_biome,
                spawn_landmark_hillclimb,
                spawn_landmark_redrock,
                spawn_landmark_desert,
                spawn_landmark_trail,
                spawn_landmark_spawn,
                spawn_connecting_trails,
            ),
        );
    }
}

// ---------------------------------------------------------------------------
// Deterministic hash helper (mirrors scatter.rs pattern)
// ---------------------------------------------------------------------------

/// Deterministic float in [0, 1) from two integer seeds and a salt.
#[inline]
fn rng(a: i32, b: i32, salt: u32) -> f32 {
    let mut v = (a.wrapping_mul(374761393))
        .wrapping_add(b.wrapping_mul(668265263))
        .wrapping_add(salt as i32);
    v ^= v >> 13;
    v = v.wrapping_mul(1274126177);
    v ^= v >> 16;
    (v as u32) as f32 / u32::MAX as f32
}

/// Convenience: rng mapped to [-1, 1).
#[inline]
fn rng_signed(a: i32, b: i32, salt: u32) -> f32 {
    rng(a, b, salt) * 2.0 - 1.0
}

// ---------------------------------------------------------------------------
// Y-snap helper
// ---------------------------------------------------------------------------

#[inline]
fn snap_y(x: f32, z: f32, half_h: f32) -> f32 {
    terrain_height_at(x, z) + half_h
}

// ---------------------------------------------------------------------------
// Biome 1 — ALPINE  (Hillclimb area, Z ≈ -180 .. -240)
// ---------------------------------------------------------------------------
//
// Items (per scatter pass, seeded at 0xAA00):
//   • Conifer tree: cone (dark green) + trunk (cylinder, dark bark)   — collider
//   • Grey boulder: sphere                                            — collider
//   • Pale grass blade: thin flat cuboid                             — NO collider
//   • Snow-dusted rock: slightly lighter sphere                      — collider
//
// Centre: (-150, _, -210)  radius 80 m scatter ring.
// Item count: 30 conifers + 20 boulders + 40 grass + 10 snow rocks = 100 items
//             30×2 parts = 60 entities for trees + 20 + 40 + 10 = 130 total

const ALPINE_CX: f32 = -150.0;
const ALPINE_CZ: f32 = -210.0;
const ALPINE_R:  f32 = 80.0;

/// Dark forest green for alpine conifers.
const ALPINE_CANOPY: Color = Color::srgb(0.08, 0.28, 0.12);
/// Dull dark bark.
const ALPINE_BARK:   Color = Color::srgb(0.25, 0.16, 0.09);
/// Granite grey boulder.
const ALPINE_ROCK:   Color = Color::srgb(0.48, 0.47, 0.46);
/// Pale, cold grass — icy hue.
const ALPINE_GRASS:  Color = Color::srgb(0.70, 0.76, 0.60);
/// Snow-dusted rock: near white with slight cool tint.
const ALPINE_SNOW:   Color = Color::srgb(0.85, 0.87, 0.90);

fn spawn_alpine_biome(
    mut commands:  Commands,
    mut meshes:    ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let canopy_mat = materials.add(StandardMaterial {
        base_color: ALPINE_CANOPY,
        perceptual_roughness: 0.88,
        metallic: 0.0,
        ..default()
    });
    let bark_mat = materials.add(StandardMaterial {
        base_color: ALPINE_BARK,
        perceptual_roughness: 0.94,
        metallic: 0.0,
        ..default()
    });
    let rock_mat = materials.add(StandardMaterial {
        base_color: ALPINE_ROCK,
        perceptual_roughness: 0.97,
        metallic: 0.0,
        ..default()
    });
    let grass_mat = materials.add(StandardMaterial {
        base_color: ALPINE_GRASS,
        perceptual_roughness: 0.92,
        metallic: 0.0,
        ..default()
    });
    let snow_mat = materials.add(StandardMaterial {
        base_color: ALPINE_SNOW,
        perceptual_roughness: 0.80,
        metallic: 0.0,
        ..default()
    });

    // Conifer: cone (canopy) + cylinder (trunk)
    // We vary heights by seed so conifers look different.
    let trunk_mesh  = meshes.add(Cylinder::new(0.18, 1.8));
    let boulder_mesh = meshes.add(Sphere::new(0.70));
    let grass_mesh  = meshes.add(Cuboid::new(0.12, 0.55, 0.04));
    let snow_rock_mesh = meshes.add(Sphere::new(0.45));

    let mut count = 0usize;

    // --- 30 conifers ---
    for i in 0..30i32 {
        let angle = rng(i, 0, 0xAA01) * std::f32::consts::TAU;
        let dist  = rng(i, 0, 0xAA02) * ALPINE_R + 8.0;
        let ox = ALPINE_CX + angle.cos() * dist;
        let oz = ALPINE_CZ + angle.sin() * dist;

        // Height varies 3..7 m.
        let tree_h = 3.0 + rng(i, 1, 0xAA03) * 4.0;
        let cone_h = tree_h * 0.68;
        let trunk_h = tree_h * 0.36;
        let cone_r  = cone_h * 0.32;

        let cone_mesh = meshes.add(Cone { radius: cone_r, height: cone_h });

        let ground = terrain_height_at(ox, oz);
        let trunk_y = ground + trunk_h / 2.0;
        let cone_y  = ground + trunk_h + cone_h / 2.0;

        // Trunk
        commands.spawn((
            BiomeProp,
            PropLod { half_height: trunk_h / 2.0 },
            Mesh3d(trunk_mesh.clone()),
            MeshMaterial3d(bark_mat.clone()),
            Transform::from_xyz(ox, trunk_y, oz),
            RigidBody::Static,
            Collider::cylinder(0.18, trunk_h / 2.0),
        ));
        // Canopy
        commands.spawn((
            BiomeProp,
            PropLod { half_height: cone_h / 2.0 },
            Mesh3d(cone_mesh),
            MeshMaterial3d(canopy_mat.clone()),
            Transform::from_xyz(ox, cone_y, oz),
            RigidBody::Static,
            Collider::cylinder(cone_r * 0.55, cone_h / 2.0),
        ));
        count += 2;
    }

    // --- 20 boulders ---
    for i in 0..20i32 {
        let angle = rng(i, 2, 0xAA04) * std::f32::consts::TAU;
        let dist  = rng(i, 2, 0xAA05) * ALPINE_R + 5.0;
        let ox = ALPINE_CX + angle.cos() * dist;
        let oz = ALPINE_CZ + angle.sin() * dist;
        let r  = 0.40 + rng(i, 3, 0xAA06) * 0.50; // 0.4 .. 0.9 m radius

        let by = snap_y(ox, oz, r * 0.5); // half-buried
        commands.spawn((
            BiomeProp,
            PropLod { half_height: r },
            Mesh3d(boulder_mesh.clone()),
            MeshMaterial3d(rock_mat.clone()),
            Transform {
                translation: Vec3::new(ox, by, oz),
                scale: Vec3::new(1.0, 0.75, rng(i, 4, 0xAA07) * 0.5 + 0.75),
                ..default()
            },
            RigidBody::Static,
            Collider::sphere(r * 0.7),
        ));
        count += 1;
    }

    // --- 40 grass blades (NO collider, NO RigidBody) ---
    for i in 0..40i32 {
        let angle = rng(i, 5, 0xAA08) * std::f32::consts::TAU;
        let dist  = rng(i, 5, 0xAA09) * ALPINE_R;
        let ox = ALPINE_CX + angle.cos() * dist;
        let oz = ALPINE_CZ + angle.sin() * dist;

        let yaw = rng(i, 6, 0xAA0A) * std::f32::consts::TAU;
        let gy  = snap_y(ox, oz, 0.275);
        commands.spawn((
            BiomeProp,
            PropLod { half_height: 0.275 },
            Mesh3d(grass_mesh.clone()),
            MeshMaterial3d(grass_mat.clone()),
            Transform {
                translation: Vec3::new(ox, gy, oz),
                rotation: Quat::from_rotation_y(yaw),
                ..default()
            },
        ));
        count += 1;
    }

    // --- 10 snow-dusted rocks ---
    for i in 0..10i32 {
        let angle = rng(i, 7, 0xAA0B) * std::f32::consts::TAU;
        let dist  = rng(i, 7, 0xAA0C) * ALPINE_R * 0.7 + 10.0;
        let ox = ALPINE_CX + angle.cos() * dist;
        let oz = ALPINE_CZ + angle.sin() * dist;
        let r  = 0.35 + rng(i, 8, 0xAA0D) * 0.35;

        let sy = snap_y(ox, oz, r * 0.55);
        commands.spawn((
            BiomeProp,
            PropLod { half_height: r },
            Mesh3d(snow_rock_mesh.clone()),
            MeshMaterial3d(snow_mat.clone()),
            Transform::from_xyz(ox, sy, oz),
            RigidBody::Static,
            Collider::sphere(r * 0.75),
        ));
        count += 1;
    }

    info!(
        "biome_dressing: Alpine (Hillclimb) — {} entities (30 conifers × 2 parts, 20 boulders, 40 grass, 10 snow rocks). Colliders on conifers + boulders + snow rocks.",
        count
    );
}

// ---------------------------------------------------------------------------
// Biome 2 — RED-ROCK CANYON  (Rock Crawl sections)
// ---------------------------------------------------------------------------
//
// Three Rock Crawl section centres: (120,0), (-80,80), (60,-120).
// Scatter radius 55 m per centre.
//
// Items:
//   • Sandstone mesa chunk: stacked cuboids (warm orange/rust)   — collider
//   • Hoodoo spire: tapered stacked cylinders                   — collider
//   • Dead brush: small brown spiky cone                        — NO collider
//   • Red dirt patch: flat round disc (cylinder, very short)    — NO collider
//
// ~25 items per section × 3 sections = 75 items
// Mesa chunks = 3 entities each (3 stacked cuboids)
// Hoodoos = 2 entities each (2 cylinders)
// Total entities ≈ 3×(8×3 + 5×2 + 7 + 5) = 3×(24+10+7+5) = 138

const RC_CX_B: [f32; 3] = [120.0, -80.0,  60.0];
const RC_CZ_B: [f32; 3] = [  0.0,  80.0, -120.0];
const RC_R:    f32 = 55.0;

const RUST_ORANGE:   Color = Color::srgb(0.72, 0.33, 0.12);
const SANDSTONE:     Color = Color::srgb(0.80, 0.55, 0.30);
const DEAD_BRUSH:    Color = Color::srgb(0.48, 0.35, 0.18);
const RED_DIRT:      Color = Color::srgb(0.58, 0.26, 0.12);

fn spawn_redrock_biome(
    mut commands:  Commands,
    mut meshes:    ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mesa_mat = materials.add(StandardMaterial {
        base_color: RUST_ORANGE,
        perceptual_roughness: 0.95,
        metallic: 0.0,
        ..default()
    });
    let sandstone_mat = materials.add(StandardMaterial {
        base_color: SANDSTONE,
        perceptual_roughness: 0.90,
        metallic: 0.0,
        ..default()
    });
    let brush_mat = materials.add(StandardMaterial {
        base_color: DEAD_BRUSH,
        perceptual_roughness: 0.97,
        metallic: 0.0,
        ..default()
    });
    let dirt_mat = materials.add(StandardMaterial {
        base_color: RED_DIRT,
        perceptual_roughness: 0.99,
        metallic: 0.0,
        ..default()
    });

    let dirt_disc_mesh  = meshes.add(Cylinder::new(1.4, 0.06));
    let brush_cone_mesh = meshes.add(Cone { radius: 0.30, height: 0.55 });

    let mut count = 0usize;

    for sec in 0..3usize {
        let cx = RC_CX_B[sec];
        let cz = RC_CZ_B[sec];
        let salt_base = (0xBB00 + sec as u32 * 0x1000) as u32;

        // --- 8 mesa chunks per section ---
        for i in 0..8i32 {
            let angle = rng(i, sec as i32, salt_base + 1) * std::f32::consts::TAU;
            let dist  = rng(i, sec as i32, salt_base + 2) * RC_R + 6.0;
            let ox = cx + angle.cos() * dist;
            let oz = cz + angle.sin() * dist;

            // 3 stacked cuboids of decreasing size
            let w0 = 2.5 + rng(i, 0, salt_base + 3) * 2.0;
            let h0 = 0.8 + rng(i, 1, salt_base + 4) * 0.6;
            let w1 = w0 * 0.72;
            let h1 = 0.6 + rng(i, 2, salt_base + 5) * 0.4;
            let w2 = w1 * 0.65;
            let h2 = 0.4 + rng(i, 3, salt_base + 6) * 0.3;

            let ground = terrain_height_at(ox, oz);
            let y0 = ground + h0 / 2.0;
            let y1 = ground + h0 + h1 / 2.0;
            let y2 = ground + h0 + h1 + h2 / 2.0;

            let m0 = meshes.add(Cuboid::new(w0, h0, w0 * 0.70));
            let m1 = meshes.add(Cuboid::new(w1, h1, w1 * 0.70));
            let m2 = meshes.add(Cuboid::new(w2, h2, w2 * 0.70));

            // Choose alternating materials for visual variety
            let mat = if i % 2 == 0 { mesa_mat.clone() } else { sandstone_mat.clone() };

            commands.spawn((
                BiomeProp,
                PropLod { half_height: h0 / 2.0 },
                Mesh3d(m0),
                MeshMaterial3d(mat.clone()),
                Transform::from_xyz(ox, y0, oz),
                RigidBody::Static,
                Collider::cuboid(w0 / 2.0, h0 / 2.0, w0 * 0.35),
            ));
            commands.spawn((
                BiomeProp,
                PropLod { half_height: h1 / 2.0 },
                Mesh3d(m1),
                MeshMaterial3d(mat.clone()),
                Transform::from_xyz(ox, y1, oz),
                RigidBody::Static,
                Collider::cuboid(w1 / 2.0, h1 / 2.0, w1 * 0.35),
            ));
            commands.spawn((
                BiomeProp,
                PropLod { half_height: h2 / 2.0 },
                Mesh3d(m2),
                MeshMaterial3d(mat.clone()),
                Transform::from_xyz(ox, y2, oz),
                RigidBody::Static,
                Collider::cuboid(w2 / 2.0, h2 / 2.0, w2 * 0.35),
            ));
            count += 3;
        }

        // --- 5 hoodoo spires per section ---
        for i in 0..5i32 {
            let angle = rng(i, sec as i32 + 10, salt_base + 7) * std::f32::consts::TAU;
            let dist  = rng(i, sec as i32 + 10, salt_base + 8) * RC_R * 0.8 + 8.0;
            let ox = cx + angle.cos() * dist;
            let oz = cz + angle.sin() * dist;

            let base_r = 0.35 + rng(i, 5, salt_base + 9) * 0.25;
            let h_bot  = 1.2 + rng(i, 6, salt_base + 10) * 1.5;
            let h_top  = 0.6 + rng(i, 7, salt_base + 11) * 0.8;

            let ground = terrain_height_at(ox, oz);
            let bot_y = ground + h_bot / 2.0;
            let top_y = ground + h_bot + h_top / 2.0;

            let cyl_bot = meshes.add(Cylinder::new(base_r, h_bot));
            let cyl_top = meshes.add(Cylinder::new(base_r * 0.55, h_top));

            commands.spawn((
                BiomeProp,
                PropLod { half_height: h_bot / 2.0 },
                Mesh3d(cyl_bot),
                MeshMaterial3d(mesa_mat.clone()),
                Transform::from_xyz(ox, bot_y, oz),
                RigidBody::Static,
                Collider::cylinder(base_r, h_bot / 2.0),
            ));
            commands.spawn((
                BiomeProp,
                PropLod { half_height: h_top / 2.0 },
                Mesh3d(cyl_top),
                MeshMaterial3d(sandstone_mat.clone()),
                Transform::from_xyz(ox, top_y, oz),
                RigidBody::Static,
                Collider::cylinder(base_r * 0.55, h_top / 2.0),
            ));
            count += 2;
        }

        // --- 7 dead brush (NO collider) ---
        for i in 0..7i32 {
            let angle = rng(i, sec as i32 + 20, salt_base + 12) * std::f32::consts::TAU;
            let dist  = rng(i, sec as i32 + 20, salt_base + 13) * RC_R;
            let ox = cx + angle.cos() * dist;
            let oz = cz + angle.sin() * dist;

            let yaw = rng(i, 9, salt_base + 14) * std::f32::consts::TAU;
            let by  = snap_y(ox, oz, 0.275);
            commands.spawn((
                BiomeProp,
                PropLod { half_height: 0.275 },
                Mesh3d(brush_cone_mesh.clone()),
                MeshMaterial3d(brush_mat.clone()),
                Transform {
                    translation: Vec3::new(ox, by, oz),
                    rotation: Quat::from_rotation_y(yaw),
                    ..default()
                },
            ));
            count += 1;
        }

        // --- 5 red dirt patches (NO collider) ---
        for i in 0..5i32 {
            let angle = rng(i, sec as i32 + 30, salt_base + 15) * std::f32::consts::TAU;
            let dist  = rng(i, sec as i32 + 30, salt_base + 16) * RC_R;
            let ox = cx + angle.cos() * dist;
            let oz = cz + angle.sin() * dist;

            let dy = snap_y(ox, oz, 0.03);
            commands.spawn((
                BiomeProp,
                PropLod { half_height: 0.03 },
                Mesh3d(dirt_disc_mesh.clone()),
                MeshMaterial3d(dirt_mat.clone()),
                Transform::from_xyz(ox, dy, oz),
            ));
            count += 1;
        }
    }

    info!(
        "biome_dressing: Red-Rock Canyon (Rock Crawl) — {} entities across 3 sections. Colliders on mesa chunks + hoodoos.",
        count
    );
}

// ---------------------------------------------------------------------------
// Biome 3 — DESERT FLAT  (Obstacle Course, Z ≈ +200 .. +260)
// ---------------------------------------------------------------------------
//
// Centre: (-60 + 50, _, +230) ≈ (-10, _, +230).  Radius 65 m.
//
// Items:
//   • Saguaro cactus: main cylinder + 2 arm cylinders           — collider
//   • Low scrub: small fat cone                                 — NO collider
//   • Small sand rock: sphere, sand-colored                     — collider
//   • Sun-bleached log: squat horizontal cylinder               — collider
//
// 15 cacti × 3 parts = 45 + 20 scrub + 15 rocks + 10 logs = 90 entities

const DESERT_CX: f32 = -10.0;
const DESERT_CZ: f32 = 230.0;
const DESERT_R:  f32 = 65.0;

const CACTUS_GREEN: Color = Color::srgb(0.20, 0.45, 0.18);
const SCRUB_TAN:    Color = Color::srgb(0.62, 0.55, 0.35);
const SAND_ROCK:    Color = Color::srgb(0.75, 0.65, 0.48);
const BLEACHED_LOG: Color = Color::srgb(0.80, 0.74, 0.58);

fn spawn_desert_biome(
    mut commands:  Commands,
    mut meshes:    ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let cactus_mat = materials.add(StandardMaterial {
        base_color: CACTUS_GREEN,
        perceptual_roughness: 0.88,
        metallic: 0.0,
        ..default()
    });
    let scrub_mat = materials.add(StandardMaterial {
        base_color: SCRUB_TAN,
        perceptual_roughness: 0.96,
        metallic: 0.0,
        ..default()
    });
    let sand_rock_mat = materials.add(StandardMaterial {
        base_color: SAND_ROCK,
        perceptual_roughness: 0.94,
        metallic: 0.0,
        ..default()
    });
    let log_mat = materials.add(StandardMaterial {
        base_color: BLEACHED_LOG,
        perceptual_roughness: 0.93,
        metallic: 0.0,
        ..default()
    });

    let scrub_mesh = meshes.add(Cone { radius: 0.35, height: 0.45 });
    let sand_sphere = meshes.add(Sphere::new(0.40));

    let mut count = 0usize;

    // --- 15 saguaro cacti ---
    for i in 0..15i32 {
        let angle = rng(i, 0, 0xCC01) * std::f32::consts::TAU;
        let dist  = rng(i, 0, 0xCC02) * DESERT_R + 6.0;
        let ox = DESERT_CX + angle.cos() * dist;
        let oz = DESERT_CZ + angle.sin() * dist;

        // Main trunk
        let trunk_r  = 0.20 + rng(i, 1, 0xCC03) * 0.12;
        let trunk_h  = 2.5  + rng(i, 2, 0xCC04) * 2.5;
        let ground   = terrain_height_at(ox, oz);
        let trunk_y  = ground + trunk_h / 2.0;

        let trunk_mesh = meshes.add(Cylinder::new(trunk_r, trunk_h));
        commands.spawn((
            BiomeProp,
            PropLod { half_height: trunk_h / 2.0 },
            Mesh3d(trunk_mesh),
            MeshMaterial3d(cactus_mat.clone()),
            Transform::from_xyz(ox, trunk_y, oz),
            RigidBody::Static,
            Collider::cylinder(trunk_r, trunk_h / 2.0),
        ));
        count += 1;

        // Left arm
        let arm_h = trunk_h * 0.45;
        let arm_r = trunk_r * 0.62;
        let arm_attach_y = ground + trunk_h * 0.55;
        // Arm extends left (+X offset) and goes up
        let arm_lx = ox - trunk_r * 1.8;
        let arm_y  = arm_attach_y + arm_h * 0.3;
        let arm_mesh_l = meshes.add(Cylinder::new(arm_r, arm_h));
        commands.spawn((
            BiomeProp,
            PropLod { half_height: arm_h / 2.0 },
            Mesh3d(arm_mesh_l),
            MeshMaterial3d(cactus_mat.clone()),
            Transform {
                translation: Vec3::new(arm_lx, arm_y, oz),
                rotation: Quat::from_rotation_z(std::f32::consts::FRAC_PI_4 * 0.7),
                ..default()
            },
            RigidBody::Static,
            Collider::cylinder(arm_r, arm_h / 2.0),
        ));
        count += 1;

        // Right arm
        let arm_rx = ox + trunk_r * 1.8;
        let arm_mesh_r = meshes.add(Cylinder::new(arm_r, arm_h));
        commands.spawn((
            BiomeProp,
            PropLod { half_height: arm_h / 2.0 },
            Mesh3d(arm_mesh_r),
            MeshMaterial3d(cactus_mat.clone()),
            Transform {
                translation: Vec3::new(arm_rx, arm_y, oz),
                rotation: Quat::from_rotation_z(-std::f32::consts::FRAC_PI_4 * 0.7),
                ..default()
            },
            RigidBody::Static,
            Collider::cylinder(arm_r, arm_h / 2.0),
        ));
        count += 1;
    }

    // --- 20 scrub bushes (NO collider) ---
    for i in 0..20i32 {
        let angle = rng(i, 3, 0xCC05) * std::f32::consts::TAU;
        let dist  = rng(i, 3, 0xCC06) * DESERT_R;
        let ox = DESERT_CX + angle.cos() * dist;
        let oz = DESERT_CZ + angle.sin() * dist;

        let yaw = rng(i, 4, 0xCC07) * std::f32::consts::TAU;
        let sy  = snap_y(ox, oz, 0.225);
        commands.spawn((
            BiomeProp,
            PropLod { half_height: 0.225 },
            Mesh3d(scrub_mesh.clone()),
            MeshMaterial3d(scrub_mat.clone()),
            Transform {
                translation: Vec3::new(ox, sy, oz),
                rotation: Quat::from_rotation_y(yaw),
                ..default()
            },
        ));
        count += 1;
    }

    // --- 15 sand rocks ---
    for i in 0..15i32 {
        let angle = rng(i, 5, 0xCC08) * std::f32::consts::TAU;
        let dist  = rng(i, 5, 0xCC09) * DESERT_R + 4.0;
        let ox = DESERT_CX + angle.cos() * dist;
        let oz = DESERT_CZ + angle.sin() * dist;

        let r = 0.20 + rng(i, 6, 0xCC0A) * 0.30;
        let ry = snap_y(ox, oz, r * 0.55);
        commands.spawn((
            BiomeProp,
            PropLod { half_height: r },
            Mesh3d(sand_sphere.clone()),
            MeshMaterial3d(sand_rock_mat.clone()),
            Transform {
                translation: Vec3::new(ox, ry, oz),
                scale: Vec3::new(1.0, 0.7, 0.8 + rng(i, 7, 0xCC0B) * 0.4),
                ..default()
            },
            RigidBody::Static,
            Collider::sphere(r * 0.65),
        ));
        count += 1;
    }

    // --- 10 bleached logs ---
    for i in 0..10i32 {
        let angle = rng(i, 8, 0xCC0C) * std::f32::consts::TAU;
        let dist  = rng(i, 8, 0xCC0D) * DESERT_R * 0.75 + 5.0;
        let ox = DESERT_CX + angle.cos() * dist;
        let oz = DESERT_CZ + angle.sin() * dist;

        let log_l = 1.5 + rng(i, 9, 0xCC0E) * 2.5; // 1.5 .. 4.0 m
        let log_r = 0.15 + rng(i, 10, 0xCC0F) * 0.10;
        let yaw   = rng(i, 11, 0xCC10) * std::f32::consts::TAU;
        let log_mesh = meshes.add(Cylinder::new(log_r, log_l));
        let ly = snap_y(ox, oz, log_r * 0.7);
        commands.spawn((
            BiomeProp,
            PropLod { half_height: log_r },
            Mesh3d(log_mesh),
            MeshMaterial3d(log_mat.clone()),
            Transform {
                translation: Vec3::new(ox, ly, oz),
                rotation: Quat::from_rotation_y(yaw) * Quat::from_rotation_z(std::f32::consts::FRAC_PI_2),
                ..default()
            },
            RigidBody::Static,
            Collider::cylinder(log_r, log_l / 2.0),
        ));
        count += 1;
    }

    info!(
        "biome_dressing: Desert Flat (Obstacle Course) — {} entities (15 cacti × 3, 20 scrub, 15 rocks, 10 logs). Colliders on cacti + rocks + logs.",
        count
    );
}

// ---------------------------------------------------------------------------
// Biome 4 — PINE FOREST  (Trail Rides area, origin + east corridor)
// ---------------------------------------------------------------------------
//
// Centre: (90, _, 8)  radius 70 m  (east of spawn, along trail-marker line).
//
// Items:
//   • Dense conifer (taller): cone + trunk                      — collider
//   • Fallen log: horizontal cylinder                           — collider
//   • Fern: small flat green fan (2 quads rotated)              — NO collider
//   • Mossy rock: slightly greenish sphere                      — collider
//
// 25 conifers × 2 = 50 + 12 logs + 30 ferns + 10 mossy rocks = 102 entities

const PINE_CX: f32 = 90.0;
const PINE_CZ: f32 = 8.0;
const PINE_R:  f32 = 70.0;

const PINE_CANOPY: Color = Color::srgb(0.06, 0.22, 0.08);
const PINE_BARK:   Color = Color::srgb(0.28, 0.18, 0.10);
const LOG_GREY:    Color = Color::srgb(0.42, 0.36, 0.28);
const FERN_GREEN:  Color = Color::srgb(0.18, 0.52, 0.22);
const MOSSY_ROCK:  Color = Color::srgb(0.38, 0.48, 0.28);

fn spawn_pine_forest_biome(
    mut commands:  Commands,
    mut meshes:    ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let canopy_mat = materials.add(StandardMaterial {
        base_color: PINE_CANOPY,
        perceptual_roughness: 0.88,
        metallic: 0.0,
        ..default()
    });
    let bark_mat = materials.add(StandardMaterial {
        base_color: PINE_BARK,
        perceptual_roughness: 0.95,
        metallic: 0.0,
        ..default()
    });
    let log_mat = materials.add(StandardMaterial {
        base_color: LOG_GREY,
        perceptual_roughness: 0.96,
        metallic: 0.0,
        ..default()
    });
    let fern_mat = materials.add(StandardMaterial {
        base_color: FERN_GREEN,
        perceptual_roughness: 0.88,
        metallic: 0.0,
        ..default()
    });
    let mossy_mat = materials.add(StandardMaterial {
        base_color: MOSSY_ROCK,
        perceptual_roughness: 0.95,
        metallic: 0.0,
        ..default()
    });

    let trunk_mesh = meshes.add(Cylinder::new(0.20, 2.0));
    let fern_mesh  = meshes.add(Cuboid::new(0.80, 0.40, 0.04));
    let mossy_sphere = meshes.add(Sphere::new(0.50));

    let mut count = 0usize;

    // --- 25 pine trees ---
    for i in 0..25i32 {
        let angle = rng(i, 0, 0xDD01) * std::f32::consts::TAU;
        let dist  = rng(i, 0, 0xDD02) * PINE_R + 5.0;
        let ox = PINE_CX + angle.cos() * dist;
        let oz = PINE_CZ + angle.sin() * dist;

        let tree_h = 4.0 + rng(i, 1, 0xDD03) * 6.0; // 4..10 m
        let cone_h  = tree_h * 0.70;
        let trunk_h = tree_h * 0.38;
        let cone_r  = cone_h * 0.28;

        let cone_mesh = meshes.add(Cone { radius: cone_r, height: cone_h });
        let ground = terrain_height_at(ox, oz);
        let trunk_y = ground + trunk_h / 2.0;
        let cone_y  = ground + trunk_h + cone_h / 2.0;

        commands.spawn((
            BiomeProp,
            PropLod { half_height: trunk_h / 2.0 },
            Mesh3d(trunk_mesh.clone()),
            MeshMaterial3d(bark_mat.clone()),
            Transform::from_xyz(ox, trunk_y, oz),
            RigidBody::Static,
            Collider::cylinder(0.20, trunk_h / 2.0),
        ));
        commands.spawn((
            BiomeProp,
            PropLod { half_height: cone_h / 2.0 },
            Mesh3d(cone_mesh),
            MeshMaterial3d(canopy_mat.clone()),
            Transform::from_xyz(ox, cone_y, oz),
            RigidBody::Static,
            Collider::cylinder(cone_r * 0.55, cone_h / 2.0),
        ));
        count += 2;
    }

    // --- 12 fallen logs ---
    for i in 0..12i32 {
        let angle = rng(i, 2, 0xDD04) * std::f32::consts::TAU;
        let dist  = rng(i, 2, 0xDD05) * PINE_R * 0.85;
        let ox = PINE_CX + angle.cos() * dist;
        let oz = PINE_CZ + angle.sin() * dist;

        let log_l = 2.0 + rng(i, 3, 0xDD06) * 3.5;
        let log_r = 0.18 + rng(i, 4, 0xDD07) * 0.14;
        let yaw   = rng(i, 5, 0xDD08) * std::f32::consts::TAU;
        let log_mesh = meshes.add(Cylinder::new(log_r, log_l));
        let ly = snap_y(ox, oz, log_r * 0.65);
        commands.spawn((
            BiomeProp,
            PropLod { half_height: log_r },
            Mesh3d(log_mesh),
            MeshMaterial3d(log_mat.clone()),
            Transform {
                translation: Vec3::new(ox, ly, oz),
                rotation: Quat::from_rotation_y(yaw) * Quat::from_rotation_z(std::f32::consts::FRAC_PI_2),
                ..default()
            },
            RigidBody::Static,
            Collider::cylinder(log_r, log_l / 2.0),
        ));
        count += 1;
    }

    // --- 30 ferns (NO collider) — two crossed quads per fern ---
    for i in 0..30i32 {
        let angle = rng(i, 6, 0xDD09) * std::f32::consts::TAU;
        let dist  = rng(i, 6, 0xDD0A) * PINE_R;
        let ox = PINE_CX + angle.cos() * dist;
        let oz = PINE_CZ + angle.sin() * dist;

        let yaw = rng(i, 7, 0xDD0B) * std::f32::consts::TAU;
        let fy  = snap_y(ox, oz, 0.20);

        // Quad A
        commands.spawn((
            BiomeProp,
            PropLod { half_height: 0.20 },
            Mesh3d(fern_mesh.clone()),
            MeshMaterial3d(fern_mat.clone()),
            Transform {
                translation: Vec3::new(ox, fy, oz),
                rotation: Quat::from_rotation_y(yaw),
                ..default()
            },
        ));
        // Quad B (perpendicular)
        commands.spawn((
            BiomeProp,
            PropLod { half_height: 0.20 },
            Mesh3d(fern_mesh.clone()),
            MeshMaterial3d(fern_mat.clone()),
            Transform {
                translation: Vec3::new(ox, fy, oz),
                rotation: Quat::from_rotation_y(yaw + std::f32::consts::FRAC_PI_2),
                ..default()
            },
        ));
        count += 2;
    }

    // --- 10 mossy rocks ---
    for i in 0..10i32 {
        let angle = rng(i, 8, 0xDD0C) * std::f32::consts::TAU;
        let dist  = rng(i, 8, 0xDD0D) * PINE_R * 0.75 + 4.0;
        let ox = PINE_CX + angle.cos() * dist;
        let oz = PINE_CZ + angle.sin() * dist;

        let r = 0.35 + rng(i, 9, 0xDD0E) * 0.45;
        let my = snap_y(ox, oz, r * 0.55);
        commands.spawn((
            BiomeProp,
            PropLod { half_height: r },
            Mesh3d(mossy_sphere.clone()),
            MeshMaterial3d(mossy_mat.clone()),
            Transform {
                translation: Vec3::new(ox, my, oz),
                scale: Vec3::new(1.0 + rng(i, 10, 0xDD0F) * 0.4, 0.72, 1.0),
                ..default()
            },
            RigidBody::Static,
            Collider::sphere(r * 0.72),
        ));
        count += 1;
    }

    info!(
        "biome_dressing: Pine Forest (Trail Rides) — {} entities (25 conifers × 2, 12 logs, 30 ferns × 2 quads, 10 mossy rocks). Colliders on conifers + logs + mossy rocks.",
        count
    );
}

// ---------------------------------------------------------------------------
// Biome 5 — MEADOW  (Spawn / Ranger area, origin)
// ---------------------------------------------------------------------------
//
// Centre: (0, _, 0)  radius 50 m.
//
// Items:
//   • Wildflower: thin cuboid stem + tiny coloured cube on top   — NO collider
//   • Tall grass clump: 3 thin cuboids fanned out               — NO collider
//   • Pond: flat blue translucent disc (cylinder)               — NO collider
//   • Oak-style tree: sphere canopy + cylinder trunk            — collider
//
// 30 wildflowers (×2 parts) + 20 grass clumps (×3 parts) + 1 pond + 8 oaks (×2 parts)
// = 60 + 60 + 1 + 16 = 137 entities

const MEADOW_CX: f32 = 0.0;
const MEADOW_CZ: f32 = 0.0;
const MEADOW_R:  f32 = 50.0;

const MEADOW_GRASS: Color = Color::srgb(0.28, 0.60, 0.18);
const OAK_CANOPY:   Color = Color::srgb(0.22, 0.48, 0.14);
const OAK_BARK:     Color = Color::srgb(0.38, 0.26, 0.14);
const POND_BLUE:    Color = Color::srgba(0.25, 0.55, 0.85, 0.55);

// Wildflower colours — 5 distinct hues cycling by index.
const FLOWER_COLORS: [Color; 5] = [
    Color::srgb(0.90, 0.20, 0.30), // red
    Color::srgb(0.95, 0.80, 0.10), // yellow
    Color::srgb(0.55, 0.20, 0.85), // purple
    Color::srgb(1.00, 0.55, 0.10), // orange
    Color::srgb(0.85, 0.85, 0.95), // pale lavender
];

fn spawn_meadow_biome(
    mut commands:  Commands,
    mut meshes:    ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let grass_mat = materials.add(StandardMaterial {
        base_color: MEADOW_GRASS,
        perceptual_roughness: 0.92,
        metallic: 0.0,
        ..default()
    });
    let oak_canopy_mat = materials.add(StandardMaterial {
        base_color: OAK_CANOPY,
        perceptual_roughness: 0.86,
        metallic: 0.0,
        ..default()
    });
    let oak_bark_mat = materials.add(StandardMaterial {
        base_color: OAK_BARK,
        perceptual_roughness: 0.94,
        metallic: 0.0,
        ..default()
    });
    let pond_mat = materials.add(StandardMaterial {
        base_color: POND_BLUE,
        alpha_mode: AlphaMode::Blend,
        perceptual_roughness: 0.05,
        metallic: 0.20,
        ..default()
    });

    let stem_mesh  = meshes.add(Cuboid::new(0.04, 0.35, 0.04));
    let grass_blade = meshes.add(Cuboid::new(0.06, 0.55, 0.04));
    let pond_mesh  = meshes.add(Cylinder::new(6.0, 0.04));
    // oak_trunk: shared trunk mesh; each oak's canopy sphere is built per-tree (varying radius)
    let oak_trunk  = meshes.add(Cylinder::new(0.22, 2.2));

    // Flower petal meshes per colour — built once.
    let flower_mats: Vec<Handle<StandardMaterial>> = FLOWER_COLORS.iter().map(|&c| {
        materials.add(StandardMaterial {
            base_color: c,
            perceptual_roughness: 0.75,
            metallic: 0.0,
            emissive: {
                let lin = c.to_linear();
                LinearRgba::new(lin.red * 0.3, lin.green * 0.3, lin.blue * 0.3, lin.alpha)
            },
            ..default()
        })
    }).collect();
    let petal_mesh = meshes.add(Cuboid::new(0.12, 0.10, 0.06));

    let mut count = 0usize;

    // --- 1 pond (slightly south of hut to avoid overlapping ranger area) ---
    {
        let px = 18.0_f32;
        let pz = 12.0_f32;
        let py = terrain_height_at(px, pz) + 0.02;
        commands.spawn((
            BiomeProp,
            PropLod { half_height: 0.02 },
            Mesh3d(pond_mesh.clone()),
            MeshMaterial3d(pond_mat.clone()),
            Transform::from_xyz(px, py, pz),
        ));
        count += 1;
    }

    // --- 8 oak-style trees ---
    for i in 0..8i32 {
        let angle = rng(i, 0, 0xEE01) * std::f32::consts::TAU;
        let dist  = rng(i, 0, 0xEE02) * MEADOW_R * 0.8 + 12.0;
        // Avoid the ranger hut footprint at (-20, _, -15).
        let mut ox = MEADOW_CX + angle.cos() * dist;
        let mut oz = MEADOW_CZ + angle.sin() * dist;
        // Nudge away from ranger hut.
        if (ox + 20.0).abs() < 8.0 && (oz + 15.0).abs() < 8.0 {
            ox += 10.0;
            oz += 10.0;
        }

        let tree_h = 3.5 + rng(i, 1, 0xEE03) * 2.5;
        let trunk_h = tree_h * 0.50;
        let canopy_r = 1.4 + rng(i, 2, 0xEE04) * 0.8;

        let ground  = terrain_height_at(ox, oz);
        let trunk_y = ground + trunk_h / 2.0;
        let canopy_y = ground + trunk_h + canopy_r * 0.75;

        commands.spawn((
            BiomeProp,
            PropLod { half_height: trunk_h / 2.0 },
            Mesh3d(oak_trunk.clone()),
            MeshMaterial3d(oak_bark_mat.clone()),
            Transform::from_xyz(ox, trunk_y, oz),
            RigidBody::Static,
            Collider::cylinder(0.22, trunk_h / 2.0),
        ));
        let canopy_mesh = meshes.add(Sphere::new(canopy_r));
        commands.spawn((
            BiomeProp,
            PropLod { half_height: canopy_r },
            Mesh3d(canopy_mesh),
            MeshMaterial3d(oak_canopy_mat.clone()),
            Transform::from_xyz(ox, canopy_y, oz),
            RigidBody::Static,
            Collider::sphere(canopy_r * 0.70),
        ));
        count += 2;
    }

    // --- 30 wildflowers (stem + petal, NO collider) ---
    for i in 0..30i32 {
        let angle = rng(i, 3, 0xEE05) * std::f32::consts::TAU;
        let dist  = rng(i, 3, 0xEE06) * MEADOW_R;
        let ox = MEADOW_CX + angle.cos() * dist;
        let oz = MEADOW_CZ + angle.sin() * dist;

        let ground = terrain_height_at(ox, oz);
        let stem_y = ground + 0.175;
        let petal_y = ground + 0.35 + 0.05;

        let flower_idx = (i as usize) % FLOWER_COLORS.len();

        // Stem
        commands.spawn((
            BiomeProp,
            PropLod { half_height: 0.175 },
            Mesh3d(stem_mesh.clone()),
            MeshMaterial3d(grass_mat.clone()),
            Transform::from_xyz(ox, stem_y, oz),
        ));
        // Petal head
        commands.spawn((
            BiomeProp,
            PropLod { half_height: 0.05 },
            Mesh3d(petal_mesh.clone()),
            MeshMaterial3d(flower_mats[flower_idx].clone()),
            Transform::from_xyz(ox, petal_y, oz),
        ));
        count += 2;
    }

    // --- 20 tall grass clumps (3 blades each, NO collider) ---
    for i in 0..20i32 {
        let angle = rng(i, 4, 0xEE07) * std::f32::consts::TAU;
        let dist  = rng(i, 4, 0xEE08) * MEADOW_R;
        let ox = MEADOW_CX + angle.cos() * dist;
        let oz = MEADOW_CZ + angle.sin() * dist;

        let ground = terrain_height_at(ox, oz);

        for blade in 0..3i32 {
            let yaw = rng(blade, i, 0xEE09 + blade as u32) * std::f32::consts::TAU;
            let bx  = ox + rng_signed(blade, i, 0xEE0A) * 0.20;
            let bz  = oz + rng_signed(blade, i, 0xEE0B) * 0.20;
            let by  = ground + 0.275;
            commands.spawn((
                BiomeProp,
                PropLod { half_height: 0.275 },
                Mesh3d(grass_blade.clone()),
                MeshMaterial3d(grass_mat.clone()),
                Transform {
                    translation: Vec3::new(bx, by, bz),
                    rotation: Quat::from_rotation_y(yaw) * Quat::from_rotation_x(-0.15),
                    ..default()
                },
            ));
            count += 1;
        }
    }

    info!(
        "biome_dressing: Meadow (Spawn Area) — {} entities (1 pond, 8 oaks × 2, 30 flowers × 2, 20 grass clumps × 3). Colliders on oak trees only.",
        count
    );
}

// ---------------------------------------------------------------------------
// Landmark 1 — Radio / ski-lift tower  (Hillclimb ridge)
// ---------------------------------------------------------------------------
//
// Structure: 4 leg struts (angled outward cylinders) + central mast +
// 3 horizontal arm platforms + a blinking-red beacon cone at the apex.
// Total: 4 legs + 1 mast + 3 arms + 1 beacon = 9 entities, all with colliders.

const TOWER_X: f32 = -270.0; // ridge past summit
const TOWER_Z: f32 = -210.0;

fn spawn_landmark_hillclimb(
    mut commands:  Commands,
    mut meshes:    ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let metal_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.52, 0.54, 0.58),
        perceptual_roughness: 0.42,
        metallic: 0.80,
        ..default()
    });
    let beacon_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.90, 0.10, 0.10),
        emissive: LinearRgba::rgb(1.2, 0.05, 0.05),
        perceptual_roughness: 0.35,
        metallic: 0.10,
        ..default()
    });

    let ground = terrain_height_at(TOWER_X, TOWER_Z);

    // Central mast (half-height 9)
    let mast_h = 18.0_f32;
    let mast_mesh = meshes.add(Cuboid::new(0.28, mast_h, 0.28));
    commands.spawn((
        BiomeProp,
        PropLod { half_height: mast_h / 2.0 },
        Mesh3d(mast_mesh),
        MeshMaterial3d(metal_mat.clone()),
        Transform::from_xyz(TOWER_X, ground + mast_h / 2.0, TOWER_Z),
        RigidBody::Static,
        Collider::cuboid(0.14, mast_h / 2.0, 0.14),
    ));

    // 4 diagonal leg struts spreading to ±4 m at base
    let leg_offsets: [(f32, f32); 4] = [(4.0, 4.0), (-4.0, 4.0), (4.0, -4.0), (-4.0, -4.0)];
    let leg_mesh = meshes.add(Cuboid::new(0.16, 8.0, 0.16));
    for (dx, dz) in leg_offsets {
        let lx = TOWER_X + dx * 0.5;
        let lz = TOWER_Z + dz * 0.5;
        let tilt_x = (dz / 4.0).atan2(8.0_f32) * 0.5;
        let tilt_z = (dx / 4.0).atan2(8.0_f32) * 0.5;
        commands.spawn((
            BiomeProp,
            PropLod { half_height: 4.0 },
            Mesh3d(leg_mesh.clone()),
            MeshMaterial3d(metal_mat.clone()),
            Transform {
                translation: Vec3::new(lx, ground + 4.0, lz),
                rotation: Quat::from_euler(EulerRot::XYZ, tilt_x, 0.0, tilt_z),
                ..default()
            },
            RigidBody::Static,
            Collider::cuboid(0.08, 4.0, 0.08),
        ));
    }

    // 3 horizontal arms at 5 m, 10 m, 15 m height
    let arm_mesh = meshes.add(Cuboid::new(5.0, 0.18, 0.18));
    for arm_h_offset in [5.0_f32, 10.0, 15.0] {
        commands.spawn((
            BiomeProp,
            PropLod { half_height: 0.09 },
            Mesh3d(arm_mesh.clone()),
            MeshMaterial3d(metal_mat.clone()),
            Transform::from_xyz(TOWER_X, ground + arm_h_offset, TOWER_Z),
            RigidBody::Static,
            Collider::cuboid(2.5, 0.09, 0.09),
        ));
    }

    // Beacon at apex
    let beacon_mesh = meshes.add(Sphere::new(0.30));
    commands.spawn((
        BiomeProp,
        PropLod { half_height: 0.30 },
        Mesh3d(beacon_mesh),
        MeshMaterial3d(beacon_mat),
        Transform::from_xyz(TOWER_X, ground + mast_h + 0.30, TOWER_Z),
        RigidBody::Static,
        Collider::sphere(0.30),
    ));

    info!("biome_dressing: Landmark 1 — Radio tower at ({}, {}) — 9 entities, all with colliders.", TOWER_X, TOWER_Z);
}

// ---------------------------------------------------------------------------
// Landmark 2 — Natural stone arch  (Rock Crawl)
// ---------------------------------------------------------------------------
//
// Two leaning pillar stacks + a spanning keystone block overhead.
// Centre between Rock Crawl sections 0 and 2 (between (120,0) and (60,-120)).
// Total: 2 left-pillar cubes + 2 right-pillar cubes + 1 keystone = 5 entities.

const ARCH_X: f32 = 92.0;
const ARCH_Z: f32 = -55.0;

fn spawn_landmark_redrock(
    mut commands:  Commands,
    mut meshes:    ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let arch_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.68, 0.32, 0.16),
        perceptual_roughness: 0.95,
        metallic: 0.0,
        ..default()
    });

    let ground = terrain_height_at(ARCH_X, ARCH_Z);

    // Left pillar: two stacked cuboids leaning right (+X tilt)
    let lp_bot_mesh = meshes.add(Cuboid::new(2.4, 4.5, 2.0));
    let lp_top_mesh = meshes.add(Cuboid::new(1.8, 3.5, 1.6));
    commands.spawn((
        BiomeProp,
        PropLod { half_height: 2.25 },
        Mesh3d(lp_bot_mesh),
        MeshMaterial3d(arch_mat.clone()),
        Transform {
            translation: Vec3::new(ARCH_X - 5.0, ground + 2.25, ARCH_Z),
            rotation: Quat::from_rotation_z(0.08), // slight lean inward
            ..default()
        },
        RigidBody::Static,
        Collider::cuboid(1.2, 2.25, 1.0),
    ));
    commands.spawn((
        BiomeProp,
        PropLod { half_height: 1.75 },
        Mesh3d(lp_top_mesh),
        MeshMaterial3d(arch_mat.clone()),
        Transform {
            translation: Vec3::new(ARCH_X - 4.6, ground + 4.5 + 1.75, ARCH_Z),
            rotation: Quat::from_rotation_z(0.06),
            ..default()
        },
        RigidBody::Static,
        Collider::cuboid(0.9, 1.75, 0.8),
    ));

    // Right pillar: mirror tilt
    let rp_bot_mesh = meshes.add(Cuboid::new(2.4, 4.5, 2.0));
    let rp_top_mesh = meshes.add(Cuboid::new(1.8, 3.5, 1.6));
    commands.spawn((
        BiomeProp,
        PropLod { half_height: 2.25 },
        Mesh3d(rp_bot_mesh),
        MeshMaterial3d(arch_mat.clone()),
        Transform {
            translation: Vec3::new(ARCH_X + 5.0, ground + 2.25, ARCH_Z),
            rotation: Quat::from_rotation_z(-0.08),
            ..default()
        },
        RigidBody::Static,
        Collider::cuboid(1.2, 2.25, 1.0),
    ));
    commands.spawn((
        BiomeProp,
        PropLod { half_height: 1.75 },
        Mesh3d(rp_top_mesh),
        MeshMaterial3d(arch_mat.clone()),
        Transform {
            translation: Vec3::new(ARCH_X + 4.6, ground + 4.5 + 1.75, ARCH_Z),
            rotation: Quat::from_rotation_z(-0.06),
            ..default()
        },
        RigidBody::Static,
        Collider::cuboid(0.9, 1.75, 0.8),
    ));

    // Keystone spanning the gap (~10 m wide, 1.5 m thick)
    let keystone_mesh = meshes.add(Cuboid::new(10.0, 1.5, 2.0));
    commands.spawn((
        BiomeProp,
        PropLod { half_height: 0.75 },
        Mesh3d(keystone_mesh),
        MeshMaterial3d(arch_mat.clone()),
        Transform::from_xyz(ARCH_X, ground + 4.5 + 3.5 + 0.75, ARCH_Z),
        RigidBody::Static,
        Collider::cuboid(5.0, 0.75, 1.0),
    ));

    info!("biome_dressing: Landmark 2 — Stone arch at ({}, {}) — 5 entities, all with colliders.", ARCH_X, ARCH_Z);
}

// ---------------------------------------------------------------------------
// Landmark 3 — Water tower on stilts  (Obstacle Course)
// ---------------------------------------------------------------------------
//
// 4 stilts + cross-braces + cylindrical tank + conical cap + sign panel.
// Positioned north of obstacle courses at (-10, _, +270).
// Total: 4 stilts + 2 cross-braces + 1 tank + 1 cap + 1 sign = 9 entities.

const WTOWER_X: f32 = -10.0;
const WTOWER_Z: f32 = 275.0;

fn spawn_landmark_desert(
    mut commands:  Commands,
    mut meshes:    ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let metal_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.60, 0.58, 0.55),
        perceptual_roughness: 0.45,
        metallic: 0.70,
        ..default()
    });
    let tank_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.72, 0.65, 0.52),
        perceptual_roughness: 0.52,
        metallic: 0.30,
        ..default()
    });
    let sign_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.82, 0.70, 0.48),
        perceptual_roughness: 0.78,
        metallic: 0.0,
        emissive: LinearRgba::rgb(0.18, 0.12, 0.04),
        ..default()
    });

    let ground = terrain_height_at(WTOWER_X, WTOWER_Z);
    let stilt_h = 7.0_f32;

    // 4 stilts at ±2.5 m corners
    let stilt_mesh = meshes.add(Cylinder::new(0.18, stilt_h));
    let stilt_positions: [(f32, f32); 4] = [
        (WTOWER_X - 2.5, WTOWER_Z - 2.5),
        (WTOWER_X + 2.5, WTOWER_Z - 2.5),
        (WTOWER_X - 2.5, WTOWER_Z + 2.5),
        (WTOWER_X + 2.5, WTOWER_Z + 2.5),
    ];
    for (sx, sz) in stilt_positions {
        let sg = terrain_height_at(sx, sz);
        commands.spawn((
            BiomeProp,
            PropLod { half_height: stilt_h / 2.0 },
            Mesh3d(stilt_mesh.clone()),
            MeshMaterial3d(metal_mat.clone()),
            Transform::from_xyz(sx, sg + stilt_h / 2.0, sz),
            RigidBody::Static,
            Collider::cylinder(0.18, stilt_h / 2.0),
        ));
    }

    // 2 horizontal cross-braces (X and Z axes)
    let brace_mesh_x = meshes.add(Cuboid::new(5.5, 0.15, 0.15));
    let brace_mesh_z = meshes.add(Cuboid::new(0.15, 0.15, 5.5));
    let brace_y = ground + stilt_h * 0.5;
    commands.spawn((
        BiomeProp,
        PropLod { half_height: 0.075 },
        Mesh3d(brace_mesh_x),
        MeshMaterial3d(metal_mat.clone()),
        Transform::from_xyz(WTOWER_X, brace_y, WTOWER_Z),
        RigidBody::Static,
        Collider::cuboid(2.75, 0.075, 0.075),
    ));
    commands.spawn((
        BiomeProp,
        PropLod { half_height: 0.075 },
        Mesh3d(brace_mesh_z),
        MeshMaterial3d(metal_mat.clone()),
        Transform::from_xyz(WTOWER_X, brace_y, WTOWER_Z),
        RigidBody::Static,
        Collider::cuboid(0.075, 0.075, 2.75),
    ));

    // Tank cylinder on top of stilts
    let tank_r = 2.8_f32;
    let tank_h = 3.5_f32;
    let tank_mesh = meshes.add(Cylinder::new(tank_r, tank_h));
    let tank_y = ground + stilt_h + tank_h / 2.0;
    commands.spawn((
        BiomeProp,
        PropLod { half_height: tank_h / 2.0 },
        Mesh3d(tank_mesh),
        MeshMaterial3d(tank_mat.clone()),
        Transform::from_xyz(WTOWER_X, tank_y, WTOWER_Z),
        RigidBody::Static,
        Collider::cylinder(tank_r, tank_h / 2.0),
    ));

    // Conical cap on tank
    let cap_mesh = meshes.add(Cone { radius: tank_r + 0.2, height: 1.4 });
    let cap_y = ground + stilt_h + tank_h + 0.7;
    commands.spawn((
        BiomeProp,
        PropLod { half_height: 0.7 },
        Mesh3d(cap_mesh),
        MeshMaterial3d(metal_mat.clone()),
        Transform::from_xyz(WTOWER_X, cap_y, WTOWER_Z),
        RigidBody::Static,
        Collider::cylinder(tank_r + 0.2, 0.7),
    ));

    // Sign panel on front face of tank (ASCII text — no Unicode)
    let sign_mesh = meshes.add(Cuboid::new(2.8, 0.70, 0.10));
    let sign_y = tank_y + 0.5;
    commands.spawn((
        BiomeProp,
        PropLod { half_height: 0.35 },
        Mesh3d(sign_mesh),
        MeshMaterial3d(sign_mat),
        Transform::from_xyz(WTOWER_X, sign_y, WTOWER_Z - tank_r - 0.05),
        RigidBody::Static,
        Collider::cuboid(1.4, 0.35, 0.05),
    ));

    info!("biome_dressing: Landmark 3 — Water tower at ({}, {}) — 9 entities, all with colliders.", WTOWER_X, WTOWER_Z);
}

// ---------------------------------------------------------------------------
// Landmark 4 — Fire lookout cabin on stilts  (Trail Rides)
// ---------------------------------------------------------------------------
//
// 4 tall stilts + floor platform + 4 walls + roof + railing posts + ladder.
// Positioned east of spawn in the pine forest area at (160, _, 8).
// Total: 4 stilts + 1 floor + 4 walls + 1 roof + 4 railings + 1 ladder = 15 entities.

const LOOKOUT_X: f32 = 160.0;
const LOOKOUT_Z: f32 = 8.0;

fn spawn_landmark_trail(
    mut commands:  Commands,
    mut meshes:    ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let wood_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.42, 0.28, 0.14),
        perceptual_roughness: 0.93,
        metallic: 0.0,
        ..default()
    });
    let roof_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.30, 0.20, 0.10),
        perceptual_roughness: 0.96,
        metallic: 0.0,
        ..default()
    });

    let ground = terrain_height_at(LOOKOUT_X, LOOKOUT_Z);
    let stilt_h = 6.0_f32;
    let cabin_y = ground + stilt_h;

    // 4 stilts
    let stilt_mesh = meshes.add(Cylinder::new(0.20, stilt_h));
    for (dx, dz) in [(-2.0_f32, -2.0), (2.0, -2.0), (-2.0, 2.0), (2.0, 2.0)] {
        let sx = LOOKOUT_X + dx;
        let sz = LOOKOUT_Z + dz;
        let sg = terrain_height_at(sx, sz);
        commands.spawn((
            BiomeProp,
            PropLod { half_height: stilt_h / 2.0 },
            Mesh3d(stilt_mesh.clone()),
            MeshMaterial3d(wood_mat.clone()),
            Transform::from_xyz(sx, sg + stilt_h / 2.0, sz),
            RigidBody::Static,
            Collider::cylinder(0.20, stilt_h / 2.0),
        ));
    }

    // Floor platform
    let floor_mesh = meshes.add(Cuboid::new(5.0, 0.20, 5.0));
    commands.spawn((
        BiomeProp,
        PropLod { half_height: 0.10 },
        Mesh3d(floor_mesh),
        MeshMaterial3d(wood_mat.clone()),
        Transform::from_xyz(LOOKOUT_X, cabin_y + 0.10, LOOKOUT_Z),
        RigidBody::Static,
        Collider::cuboid(2.5, 0.10, 2.5),
    ));

    // 4 walls (front, back, left, right)
    let wall_fb = meshes.add(Cuboid::new(5.0, 2.5, 0.18)); // front/back
    let wall_lr = meshes.add(Cuboid::new(0.18, 2.5, 5.0)); // left/right
    let wall_y = cabin_y + 0.20 + 1.25;

    commands.spawn((
        BiomeProp,
        PropLod { half_height: 1.25 },
        Mesh3d(wall_fb.clone()),
        MeshMaterial3d(wood_mat.clone()),
        Transform::from_xyz(LOOKOUT_X, wall_y, LOOKOUT_Z - 2.5),
        RigidBody::Static,
        Collider::cuboid(2.5, 1.25, 0.09),
    ));
    commands.spawn((
        BiomeProp,
        PropLod { half_height: 1.25 },
        Mesh3d(wall_fb.clone()),
        MeshMaterial3d(wood_mat.clone()),
        Transform::from_xyz(LOOKOUT_X, wall_y, LOOKOUT_Z + 2.5),
        RigidBody::Static,
        Collider::cuboid(2.5, 1.25, 0.09),
    ));
    commands.spawn((
        BiomeProp,
        PropLod { half_height: 1.25 },
        Mesh3d(wall_lr.clone()),
        MeshMaterial3d(wood_mat.clone()),
        Transform::from_xyz(LOOKOUT_X - 2.5, wall_y, LOOKOUT_Z),
        RigidBody::Static,
        Collider::cuboid(0.09, 1.25, 2.5),
    ));
    commands.spawn((
        BiomeProp,
        PropLod { half_height: 1.25 },
        Mesh3d(wall_lr.clone()),
        MeshMaterial3d(wood_mat.clone()),
        Transform::from_xyz(LOOKOUT_X + 2.5, wall_y, LOOKOUT_Z),
        RigidBody::Static,
        Collider::cuboid(0.09, 1.25, 2.5),
    ));

    // Roof (slight overhang, sloped via rotation)
    let roof_mesh = meshes.add(Cuboid::new(5.6, 0.22, 5.6));
    let roof_y = cabin_y + 0.20 + 2.5 + 0.11;
    commands.spawn((
        BiomeProp,
        PropLod { half_height: 0.11 },
        Mesh3d(roof_mesh),
        MeshMaterial3d(roof_mat),
        Transform {
            translation: Vec3::new(LOOKOUT_X, roof_y, LOOKOUT_Z),
            rotation: Quat::from_rotation_z(0.15),
            ..default()
        },
        RigidBody::Static,
        Collider::cuboid(2.8, 0.11, 2.8),
    ));

    // 4 railing posts at platform corners
    let rail_mesh = meshes.add(Cylinder::new(0.06, 0.80));
    for (dx, dz) in [(-2.3_f32, -2.3), (2.3, -2.3), (-2.3, 2.3), (2.3, 2.3)] {
        commands.spawn((
            BiomeProp,
            PropLod { half_height: 0.40 },
            Mesh3d(rail_mesh.clone()),
            MeshMaterial3d(wood_mat.clone()),
            Transform::from_xyz(LOOKOUT_X + dx, cabin_y + 0.20 + 0.40, LOOKOUT_Z + dz),
            RigidBody::Static,
            Collider::cylinder(0.06, 0.40),
        ));
    }

    // Ladder rungs (3 rungs, front face)
    let rung_mesh = meshes.add(Cuboid::new(0.80, 0.06, 0.06));
    for rung in 0..3i32 {
        let ry = ground + 1.5 + (rung as f32) * 1.5;
        commands.spawn((
            BiomeProp,
            PropLod { half_height: 0.03 },
            Mesh3d(rung_mesh.clone()),
            MeshMaterial3d(wood_mat.clone()),
            Transform::from_xyz(LOOKOUT_X, ry, LOOKOUT_Z - 2.55),
            RigidBody::Static,
            Collider::cuboid(0.40, 0.03, 0.03),
        ));
    }

    info!("biome_dressing: Landmark 4 — Fire lookout cabin at ({}, {}) — 15 entities, all with colliders.", LOOKOUT_X, LOOKOUT_Z);
}

// ---------------------------------------------------------------------------
// Landmark 5 — Welcome archway sign  (Spawn area)
// ---------------------------------------------------------------------------
//
// Two large posts + a spanning arch beam + sign panel.
// Text (ASCII only): "S&K OFFROAD PARK"
// Positioned at (0, _, 25) facing south toward spawn point.
// Total: 2 posts + 1 beam + 1 sign panel = 4 entities.

const ARCH_SIGN_X: f32 = 0.0;
const ARCH_SIGN_Z: f32 = 25.0;

fn spawn_landmark_spawn(
    mut commands:  Commands,
    mut meshes:    ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let post_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.35, 0.24, 0.12),
        perceptual_roughness: 0.93,
        metallic: 0.0,
        ..default()
    });
    let beam_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.30, 0.20, 0.10),
        perceptual_roughness: 0.92,
        metallic: 0.0,
        ..default()
    });
    let sign_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.88, 0.76, 0.42),
        perceptual_roughness: 0.72,
        metallic: 0.0,
        emissive: LinearRgba::rgb(0.25, 0.18, 0.05),
        ..default()
    });

    let post_h = 6.0_f32;
    let ground_l = terrain_height_at(ARCH_SIGN_X - 6.0, ARCH_SIGN_Z);
    let ground_r = terrain_height_at(ARCH_SIGN_X + 6.0, ARCH_SIGN_Z);
    let ground_c = terrain_height_at(ARCH_SIGN_X, ARCH_SIGN_Z);

    // Left post
    let post_mesh = meshes.add(Cylinder::new(0.30, post_h));
    commands.spawn((
        BiomeProp,
        PropLod { half_height: post_h / 2.0 },
        Mesh3d(post_mesh.clone()),
        MeshMaterial3d(post_mat.clone()),
        Transform::from_xyz(ARCH_SIGN_X - 6.0, ground_l + post_h / 2.0, ARCH_SIGN_Z),
        RigidBody::Static,
        Collider::cylinder(0.30, post_h / 2.0),
    ));
    // Right post
    commands.spawn((
        BiomeProp,
        PropLod { half_height: post_h / 2.0 },
        Mesh3d(post_mesh),
        MeshMaterial3d(post_mat),
        Transform::from_xyz(ARCH_SIGN_X + 6.0, ground_r + post_h / 2.0, ARCH_SIGN_Z),
        RigidBody::Static,
        Collider::cylinder(0.30, post_h / 2.0),
    ));

    // Spanning beam
    let beam_mesh = meshes.add(Cuboid::new(12.8, 0.55, 0.55));
    commands.spawn((
        BiomeProp,
        PropLod { half_height: 0.275 },
        Mesh3d(beam_mesh),
        MeshMaterial3d(beam_mat),
        Transform::from_xyz(ARCH_SIGN_X, ground_c + post_h + 0.275, ARCH_SIGN_Z),
        RigidBody::Static,
        Collider::cuboid(6.4, 0.275, 0.275),
    ));

    // Sign panel hanging below beam — "S&K OFFROAD PARK" (ASCII-only)
    // The text is represented visually by the bright emissive panel.
    // No Text component used here — Bevy 0.18 default font renders tofu for
    // special chars; we use a plain emissive board as the visual identifier.
    let sign_panel = meshes.add(Cuboid::new(10.0, 0.90, 0.12));
    commands.spawn((
        BiomeProp,
        PropLod { half_height: 0.45 },
        Mesh3d(sign_panel),
        MeshMaterial3d(sign_mat),
        Transform::from_xyz(ARCH_SIGN_X, ground_c + post_h - 0.45 + 0.10, ARCH_SIGN_Z),
        RigidBody::Static,
        Collider::cuboid(5.0, 0.45, 0.06),
    ));

    info!("biome_dressing: Landmark 5 — Welcome arch 'S&K OFFROAD PARK' at ({}, {}) — 4 entities, all with colliders.", ARCH_SIGN_X, ARCH_SIGN_Z);
}

// ---------------------------------------------------------------------------
// Connecting trail ribbons
// ---------------------------------------------------------------------------
//
// A series of flat, darker dirt-quad markers leading from spawn (0, 0) toward
// each mode area.  Pure visual, no colliders.
// Each trail: 8 markers spaced ~20 m apart.
// Total: 4 trails × 8 markers = 32 entities.

const TRAIL_DIRT: Color = Color::srgb(0.32, 0.22, 0.12);

fn spawn_connecting_trails(
    mut commands:  Commands,
    mut meshes:    ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let dirt_mat = materials.add(StandardMaterial {
        base_color: TRAIL_DIRT,
        perceptual_roughness: 0.99,
        metallic: 0.0,
        ..default()
    });

    // Quad for trail marker: 2.5 m wide, 2 m long, 0.04 m thick.
    let quad_mesh = meshes.add(Cuboid::new(2.5, 0.04, 2.0));

    // Destinations (mode area centres):
    //   Hillclimb: (-150, _, -210)
    //   Rock Crawl: (120, _, 0)
    //   Obstacle Course: (-10, _, 230)
    //   Trail Rides: (160, _, 8) via pine forest
    let trails: [((f32, f32), (f32, f32)); 4] = [
        ((0.0, 0.0), (-150.0, -210.0)),
        ((0.0, 0.0), (120.0,   0.0  )),
        ((0.0, 0.0), (-10.0,  230.0 )),
        ((0.0, 0.0), (160.0,   8.0  )),
    ];

    let mut count = 0usize;
    const MARKERS_PER_TRAIL: usize = 8;

    for (ti, ((sx, sz), (ex, ez))) in trails.iter().enumerate() {
        let dx = (ex - sx) / (MARKERS_PER_TRAIL + 1) as f32;
        let dz = (ez - sz) / (MARKERS_PER_TRAIL + 1) as f32;

        // Yaw so the quad aligns to the direction of travel.
        let yaw = dz.atan2(dx);

        for step in 1..=(MARKERS_PER_TRAIL as i32) {
            let tx = sx + dx * step as f32 + rng_signed(step, ti as i32, 0xFF01) * 0.6;
            let tz = sz + dz * step as f32 + rng_signed(step, ti as i32, 0xFF02) * 0.6;
            let ty = terrain_height_at(tx, tz) + 0.022; // just above terrain

            commands.spawn((
                BiomeProp,
                PropLod { half_height: 0.022 },
                Mesh3d(quad_mesh.clone()),
                MeshMaterial3d(dirt_mat.clone()),
                Transform {
                    translation: Vec3::new(tx, ty, tz),
                    rotation: Quat::from_rotation_y(yaw),
                    ..default()
                },
            ));
            count += 1;
        }
    }

    info!(
        "biome_dressing: Connecting trails — {} dirt-path markers (4 trails × 8 markers). Pure visual, no colliders.",
        count
    );
}
