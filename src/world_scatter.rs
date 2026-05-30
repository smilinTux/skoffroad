// Sprint 78 — World Scatter Pass
//
// WorldScatterPlugin fills the large 720 m terrain (X/Z in [-360, +360]) with
// natural ground cover that makes the open world feel alive everywhere —
// not just at the mode-area biome hubs.
//
// Item types and target counts:
//   • Mixed trees (conifer cone+trunk / round-canopy sphere+trunk) — ~300
//   • Rocks / boulders  (grey granite spheres, varied scale)       — ~200
//   • Grass / shrub clumps (low cuboid/cone tufts)                 — ~500
//   • Bushes (small sphere canopies)                               — ~100
//   • Dead / fallen logs (horizontal cylinders)                    — ~50
//
// Total target: ~1 150 entities — all PropLod-culled (250 m, 8-frame cadence).
//
// Hub-avoidance exclusion zones (XZ centres + radii):
//   Hillclimb  (-150, -210)  r = 90
//   RockCrawl  (120, 0)      r = 55
//   RockCrawl  (-80, 80)     r = 55
//   RockCrawl  (60, -120)    r = 55
//   Obstacle   (-10, 230)    r = 80
//   Trail      (90, 8)       r = 80
//   Spawn      (0, 0)        r = 65
//
// Rules followed (no exceptions):
//   - NO BorderRadius in any spawn tuple
//   - NO Unicode glyphs in any string
//   - Colliders ONLY on trees (trunks) + boulders + logs; none on grass/shrubs/bushes
//   - RigidBody::Static only on collider-bearing items
//   - PropLod on EVERY entity
//   - Y-snapped via terrain_height_at(x, z)

use bevy::prelude::*;
use avian3d::prelude::*;

use crate::terrain::terrain_height_at;
use crate::prop_lod::PropLod;

// ---------------------------------------------------------------------------
// Marker component
// ---------------------------------------------------------------------------

/// Applied to every entity spawned by WorldScatterPlugin.
#[derive(Component)]
pub struct WorldScatterProp;

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct WorldScatterPlugin;

impl Plugin for WorldScatterPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Startup,
            (
                spawn_world_trees,
                spawn_world_boulders,
                spawn_world_grass,
                spawn_world_bushes,
                spawn_world_logs,
            ),
        );
    }
}

// ---------------------------------------------------------------------------
// Terrain bounds
// ---------------------------------------------------------------------------

const TERRAIN_HALF: f32 = 360.0; // terrain spans [-360, +360]

// ---------------------------------------------------------------------------
// Hub exclusion zones — (cx, cz, radius_sq)
// We store radius_sq to avoid a sqrt per candidate.
// ---------------------------------------------------------------------------

struct Hub {
    cx: f32,
    cz: f32,
    r_sq: f32,
}

const HUBS: &[Hub] = &[
    Hub { cx: -150.0, cz: -210.0, r_sq: 90.0 * 90.0 },   // Hillclimb
    Hub { cx:  120.0, cz:    0.0, r_sq: 55.0 * 55.0 },   // RockCrawl A
    Hub { cx:  -80.0, cz:   80.0, r_sq: 55.0 * 55.0 },   // RockCrawl B
    Hub { cx:   60.0, cz: -120.0, r_sq: 55.0 * 55.0 },   // RockCrawl C
    Hub { cx:  -10.0, cz:  230.0, r_sq: 80.0 * 80.0 },   // Obstacle
    Hub { cx:   90.0, cz:    8.0, r_sq: 80.0 * 80.0 },   // Trail
    Hub { cx:    0.0, cz:    0.0, r_sq: 65.0 * 65.0 },   // Spawn / Meadow
];

/// Returns true if (x, z) is inside any hub exclusion zone.
#[inline]
fn in_hub(x: f32, z: f32) -> bool {
    for h in HUBS {
        let dx = x - h.cx;
        let dz = z - h.cz;
        if dx * dx + dz * dz < h.r_sq {
            return true;
        }
    }
    false
}

// ---------------------------------------------------------------------------
// Deterministic RNG — same hash pattern as biome_dressing.rs
// ---------------------------------------------------------------------------

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

/// Map rng output to [lo, hi].
#[inline]
fn rng_range(a: i32, b: i32, salt: u32, lo: f32, hi: f32) -> f32 {
    lo + rng(a, b, salt) * (hi - lo)
}

/// Sample a world-space (x, z) position from index pair (i, j) with given salt.
/// Covers the whole terrain with an 8 m margin from the edge.
#[inline]
fn sample_pos(i: i32, j: i32, salt_x: u32, salt_z: u32) -> (f32, f32) {
    let margin = 8.0_f32;
    let span = (TERRAIN_HALF - margin) * 2.0;
    let x = -TERRAIN_HALF + margin + rng(i, j, salt_x) * span;
    let z = -TERRAIN_HALF + margin + rng(i, j, salt_z) * span;
    (x, z)
}

/// Y position snapped to terrain with a vertical offset of `half_h`.
#[inline]
fn snap_y(x: f32, z: f32, half_h: f32) -> f32 {
    terrain_height_at(x, z) + half_h
}

// ---------------------------------------------------------------------------
// Material colours — natural palette that complements biome_dressing.rs
// ---------------------------------------------------------------------------

// Conifer (cone canopy)
const CON_CANOPY_A: Color = Color::srgb(0.10, 0.30, 0.12); // darker pine
const CON_CANOPY_B: Color = Color::srgb(0.16, 0.40, 0.18); // lighter pine
const CON_BARK:     Color = Color::srgb(0.30, 0.20, 0.10);

// Round-canopy oak / broadleaf
const OAK_CANOPY_A: Color = Color::srgb(0.22, 0.46, 0.14);
const OAK_CANOPY_B: Color = Color::srgb(0.28, 0.54, 0.20);
const OAK_BARK:     Color = Color::srgb(0.36, 0.24, 0.12);

// Boulders — granite grey
const BOULDER_A: Color = Color::srgb(0.52, 0.52, 0.54);
const BOULDER_B: Color = Color::srgb(0.44, 0.44, 0.46);

// Grass / shrub tufts
const GRASS_A: Color = Color::srgb(0.28, 0.58, 0.18);
const GRASS_B: Color = Color::srgb(0.34, 0.52, 0.22);
const GRASS_C: Color = Color::srgb(0.40, 0.48, 0.25);

// Bushes
const BUSH_A: Color = Color::srgb(0.18, 0.38, 0.12);
const BUSH_B: Color = Color::srgb(0.24, 0.44, 0.16);

// Fallen logs — weathered bark
const LOG_COLOR: Color = Color::srgb(0.40, 0.30, 0.20);

// ---------------------------------------------------------------------------
// System 1 — World Trees (~300 entities)
//
// Half conifer (cone + trunk = 2 entities each) and half round-canopy
// (sphere + trunk = 2 entities each). That's ~150 conifers + ~150 round
// = 300 trees = 600 entity parts. Hub-avoidance keeps density low near hubs.
// ---------------------------------------------------------------------------

pub fn spawn_world_trees(
    mut commands:  Commands,
    mut meshes:    ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // --- Materials ---
    let con_canopy_a = materials.add(StandardMaterial {
        base_color: CON_CANOPY_A,
        perceptual_roughness: 0.88,
        metallic: 0.0,
        ..default()
    });
    let con_canopy_b = materials.add(StandardMaterial {
        base_color: CON_CANOPY_B,
        perceptual_roughness: 0.90,
        metallic: 0.0,
        ..default()
    });
    let con_bark = materials.add(StandardMaterial {
        base_color: CON_BARK,
        perceptual_roughness: 0.94,
        metallic: 0.0,
        ..default()
    });
    let oak_canopy_a = materials.add(StandardMaterial {
        base_color: OAK_CANOPY_A,
        perceptual_roughness: 0.86,
        metallic: 0.0,
        ..default()
    });
    let oak_canopy_b = materials.add(StandardMaterial {
        base_color: OAK_CANOPY_B,
        perceptual_roughness: 0.84,
        metallic: 0.0,
        ..default()
    });
    let oak_bark = materials.add(StandardMaterial {
        base_color: OAK_BARK,
        perceptual_roughness: 0.95,
        metallic: 0.0,
        ..default()
    });

    let con_trunk_mesh = meshes.add(Cylinder::new(0.18, 2.0));
    let oak_trunk_mesh = meshes.add(Cylinder::new(0.22, 2.2));

    let mut spawned = 0usize;

    // --- 150 conifers ---
    let mut attempts = 0i32;
    let mut placed = 0i32;
    while placed < 150 && attempts < 3000 {
        let (x, z) = sample_pos(placed, attempts, 0xF101, 0xF102);
        attempts += 1;
        if in_hub(x, z) {
            continue;
        }

        let tree_h = rng_range(placed, attempts, 0xF103, 3.5, 8.0);
        let trunk_h = tree_h * 0.35;
        let cone_h  = tree_h * 0.70;
        let cone_r  = cone_h * 0.30;
        let trunk_r = 0.14 + rng(placed, attempts, 0xF104) * 0.08;

        let ground = terrain_height_at(x, z);
        let trunk_y = ground + trunk_h / 2.0;
        let cone_y  = ground + trunk_h + cone_h / 2.0;

        let cone_mesh = meshes.add(Cone { radius: cone_r, height: cone_h });
        let canopy_mat = if placed % 2 == 0 { con_canopy_a.clone() } else { con_canopy_b.clone() };

        // Trunk — with collider
        commands.spawn((
            WorldScatterProp,
            PropLod { half_height: trunk_h / 2.0 },
            Mesh3d(con_trunk_mesh.clone()),
            MeshMaterial3d(con_bark.clone()),
            Transform::from_xyz(x, trunk_y, z),
            RigidBody::Static,
            Collider::cylinder(trunk_r, trunk_h / 2.0),
        ));
        // Canopy cone — with collider (approximate)
        commands.spawn((
            WorldScatterProp,
            PropLod { half_height: cone_h / 2.0 },
            Mesh3d(cone_mesh),
            MeshMaterial3d(canopy_mat),
            Transform::from_xyz(x, cone_y, z),
            RigidBody::Static,
            Collider::cylinder(cone_r * 0.50, cone_h / 2.0),
        ));
        spawned += 2;
        placed += 1;
    }

    // --- 150 round-canopy (oak/broadleaf) trees ---
    let mut attempts = 0i32;
    let mut placed = 0i32;
    while placed < 150 && attempts < 3000 {
        let (x, z) = sample_pos(placed, attempts + 1000, 0xF201, 0xF202);
        attempts += 1;
        if in_hub(x, z) {
            continue;
        }

        let tree_h  = rng_range(placed, attempts, 0xF203, 3.0, 7.0);
        let trunk_h = tree_h * 0.50;
        let canopy_r = 1.2 + rng(placed, attempts, 0xF204) * 1.0;
        let trunk_rr = 0.16 + rng(placed, attempts, 0xF205) * 0.10;

        let ground   = terrain_height_at(x, z);
        let trunk_y  = ground + trunk_h / 2.0;
        let canopy_y = ground + trunk_h + canopy_r * 0.70;

        let canopy_mesh = meshes.add(Sphere::new(canopy_r));
        let canopy_mat  = if placed % 2 == 0 { oak_canopy_a.clone() } else { oak_canopy_b.clone() };

        // Trunk — with collider
        commands.spawn((
            WorldScatterProp,
            PropLod { half_height: trunk_h / 2.0 },
            Mesh3d(oak_trunk_mesh.clone()),
            MeshMaterial3d(oak_bark.clone()),
            Transform::from_xyz(x, trunk_y, z),
            RigidBody::Static,
            Collider::cylinder(trunk_rr, trunk_h / 2.0),
        ));
        // Canopy sphere — with collider
        commands.spawn((
            WorldScatterProp,
            PropLod { half_height: canopy_r },
            Mesh3d(canopy_mesh),
            MeshMaterial3d(canopy_mat),
            Transform::from_xyz(x, canopy_y, z),
            RigidBody::Static,
            Collider::sphere(canopy_r * 0.65),
        ));
        spawned += 2;
        placed += 1;
    }

    info!(
        "world_scatter: Trees — {} entities ({} conifers + {} oaks, 2 parts each). Colliders on all trunks + canopies.",
        spawned,
        spawned / 2 / 2,
        spawned / 2 / 2
    );
}

// ---------------------------------------------------------------------------
// System 2 — World Boulders (~200 entities)
//
// Grey granite spheres of varied scale. Some clustered (2-3 together).
// All have colliders (RigidBody::Static).
// ---------------------------------------------------------------------------

pub fn spawn_world_boulders(
    mut commands:  Commands,
    mut meshes:    ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let boulder_a = materials.add(StandardMaterial {
        base_color: BOULDER_A,
        perceptual_roughness: 0.97,
        metallic: 0.0,
        ..default()
    });
    let boulder_b = materials.add(StandardMaterial {
        base_color: BOULDER_B,
        perceptual_roughness: 0.95,
        metallic: 0.0,
        ..default()
    });

    let boulder_mesh = meshes.add(Sphere::new(1.0)); // scale per boulder

    let mut spawned = 0usize;

    // 120 solitary boulders
    let mut attempts = 0i32;
    let mut placed = 0i32;
    while placed < 120 && attempts < 2400 {
        let (x, z) = sample_pos(placed, attempts, 0xF301, 0xF302);
        attempts += 1;
        if in_hub(x, z) {
            continue;
        }

        let r = rng_range(placed, attempts, 0xF303, 0.40, 1.20);
        let by = snap_y(x, z, r * 0.45); // half-buried

        let mat = if placed % 2 == 0 { boulder_a.clone() } else { boulder_b.clone() };

        commands.spawn((
            WorldScatterProp,
            PropLod { half_height: r },
            Mesh3d(boulder_mesh.clone()),
            MeshMaterial3d(mat),
            Transform {
                translation: Vec3::new(x, by, z),
                scale: Vec3::new(
                    r,
                    r * (0.65 + rng(placed, attempts, 0xF304) * 0.35),
                    r * (0.75 + rng(placed, attempts, 0xF305) * 0.35),
                ),
                rotation: Quat::from_rotation_y(rng(placed, attempts, 0xF306) * std::f32::consts::TAU),
            },
            RigidBody::Static,
            Collider::sphere(r * 0.72),
        ));
        spawned += 1;
        placed += 1;
    }

    // 20 clusters of 3-4 boulders each (up to 80 extra boulders)
    let mut attempts = 0i32;
    let mut cluster = 0i32;
    while cluster < 20 && attempts < 800 {
        let (cx, cz) = sample_pos(cluster, attempts + 2000, 0xF401, 0xF402);
        attempts += 1;
        if in_hub(cx, cz) {
            continue;
        }

        let count_in_cluster = 3 + (rng(cluster, attempts, 0xF403) * 2.0) as i32; // 3 or 4
        for k in 0..count_in_cluster {
            let off_x = (rng(cluster, k, 0xF404) - 0.5) * 5.0;
            let off_z = (rng(cluster, k, 0xF405) - 0.5) * 5.0;
            let bx = (cx + off_x).clamp(-TERRAIN_HALF + 4.0, TERRAIN_HALF - 4.0);
            let bz = (cz + off_z).clamp(-TERRAIN_HALF + 4.0, TERRAIN_HALF - 4.0);

            let r = rng_range(cluster, k, 0xF406, 0.30, 0.90);
            let by = snap_y(bx, bz, r * 0.45);
            let mat = if (cluster + k) % 2 == 0 { boulder_a.clone() } else { boulder_b.clone() };

            commands.spawn((
                WorldScatterProp,
                PropLod { half_height: r },
                Mesh3d(boulder_mesh.clone()),
                MeshMaterial3d(mat),
                Transform {
                    translation: Vec3::new(bx, by, bz),
                    scale: Vec3::splat(r),
                    rotation: Quat::from_rotation_y(
                        rng(cluster, k, 0xF407) * std::f32::consts::TAU,
                    ),
                },
                RigidBody::Static,
                Collider::sphere(r * 0.75),
            ));
            spawned += 1;
        }
        cluster += 1;
    }

    info!(
        "world_scatter: Boulders — {} entities (120 solitary + clustered). Colliders on all.",
        spawned
    );
}

// ---------------------------------------------------------------------------
// System 3 — World Grass / Shrub Clumps (~500 entities)
//
// Small low cuboid tufts and fat cone shrubs. NO collider, NO RigidBody.
// ---------------------------------------------------------------------------

pub fn spawn_world_grass(
    mut commands:  Commands,
    mut meshes:    ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let grass_mat_a = materials.add(StandardMaterial {
        base_color: GRASS_A,
        perceptual_roughness: 0.92,
        metallic: 0.0,
        ..default()
    });
    let grass_mat_b = materials.add(StandardMaterial {
        base_color: GRASS_B,
        perceptual_roughness: 0.90,
        metallic: 0.0,
        ..default()
    });
    let grass_mat_c = materials.add(StandardMaterial {
        base_color: GRASS_C,
        perceptual_roughness: 0.93,
        metallic: 0.0,
        ..default()
    });

    // Three mesh shapes for variety
    let tuft_mesh   = meshes.add(Cuboid::new(0.50, 0.35, 0.35)); // flat tuft
    let blade_mesh  = meshes.add(Cuboid::new(0.08, 0.60, 0.06)); // tall blade
    let shrub_mesh  = meshes.add(Cone { radius: 0.30, height: 0.40 }); // fat shrub

    let mut spawned = 0usize;

    // 500 grass / shrub items
    let mut attempts = 0i32;
    let mut placed = 0i32;
    while placed < 500 && attempts < 8000 {
        let (x, z) = sample_pos(placed, attempts, 0xF501, 0xF502);
        attempts += 1;
        if in_hub(x, z) {
            continue;
        }

        let yaw = rng(placed, attempts, 0xF503) * std::f32::consts::TAU;
        let variety = (placed % 3) as u32;

        let (mesh, half_h) = match variety {
            0 => (tuft_mesh.clone(),  0.175_f32),
            1 => (blade_mesh.clone(), 0.30_f32),
            _ => (shrub_mesh.clone(), 0.20_f32),
        };

        let mat = match placed % 3 {
            0 => grass_mat_a.clone(),
            1 => grass_mat_b.clone(),
            _ => grass_mat_c.clone(),
        };

        let gy = snap_y(x, z, half_h);

        // No RigidBody, no Collider — pure visual
        commands.spawn((
            WorldScatterProp,
            PropLod { half_height: half_h },
            Mesh3d(mesh),
            MeshMaterial3d(mat),
            Transform {
                translation: Vec3::new(x, gy, z),
                rotation: Quat::from_rotation_y(yaw),
                ..default()
            },
        ));
        spawned += 1;
        placed += 1;
    }

    info!(
        "world_scatter: Grass/shrubs — {} entities (tufts, blades, shrub cones). NO colliders.",
        spawned
    );
}

// ---------------------------------------------------------------------------
// System 4 — World Bushes (~100 entities)
//
// Small sphere canopies only (no trunks). NO collider, NO RigidBody.
// ---------------------------------------------------------------------------

pub fn spawn_world_bushes(
    mut commands:  Commands,
    mut meshes:    ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let bush_mat_a = materials.add(StandardMaterial {
        base_color: BUSH_A,
        perceptual_roughness: 0.86,
        metallic: 0.0,
        ..default()
    });
    let bush_mat_b = materials.add(StandardMaterial {
        base_color: BUSH_B,
        perceptual_roughness: 0.88,
        metallic: 0.0,
        ..default()
    });

    // Two sphere sizes; scaled per-instance
    let bush_mesh = meshes.add(Sphere::new(1.0));

    let mut spawned = 0usize;

    let mut attempts = 0i32;
    let mut placed = 0i32;
    while placed < 100 && attempts < 2000 {
        let (x, z) = sample_pos(placed, attempts + 3000, 0xF601, 0xF602);
        attempts += 1;
        if in_hub(x, z) {
            continue;
        }

        let r = rng_range(placed, attempts, 0xF603, 0.40, 0.90);
        let by = snap_y(x, z, r * 0.55);
        let mat = if placed % 2 == 0 { bush_mat_a.clone() } else { bush_mat_b.clone() };

        // Pure visual — no collider
        commands.spawn((
            WorldScatterProp,
            PropLod { half_height: r },
            Mesh3d(bush_mesh.clone()),
            MeshMaterial3d(mat),
            Transform {
                translation: Vec3::new(x, by, z),
                scale: Vec3::new(r, r * 0.70, r),
                ..default()
            },
        ));
        spawned += 1;
        placed += 1;
    }

    info!(
        "world_scatter: Bushes — {} sphere canopies. NO colliders.",
        spawned
    );
}

// ---------------------------------------------------------------------------
// System 5 — World Fallen Logs (~50 entities)
//
// Horizontal cylinders, weathered bark colour.
// Colliders + RigidBody::Static.
// ---------------------------------------------------------------------------

pub fn spawn_world_logs(
    mut commands:  Commands,
    mut meshes:    ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let log_mat = materials.add(StandardMaterial {
        base_color: LOG_COLOR,
        perceptual_roughness: 0.96,
        metallic: 0.0,
        ..default()
    });

    let mut spawned = 0usize;

    let mut attempts = 0i32;
    let mut placed = 0i32;
    while placed < 50 && attempts < 1500 {
        let (x, z) = sample_pos(placed, attempts + 4000, 0xF701, 0xF702);
        attempts += 1;
        if in_hub(x, z) {
            continue;
        }

        let log_l = rng_range(placed, attempts, 0xF703, 1.5, 4.5);
        let log_r = rng_range(placed, attempts, 0xF704, 0.15, 0.28);
        let yaw   = rng(placed, attempts, 0xF705) * std::f32::consts::TAU;

        let log_mesh = meshes.add(Cylinder::new(log_r, log_l));
        let ly = snap_y(x, z, log_r * 0.65);

        commands.spawn((
            WorldScatterProp,
            PropLod { half_height: log_r },
            Mesh3d(log_mesh),
            MeshMaterial3d(log_mat.clone()),
            Transform {
                translation: Vec3::new(x, ly, z),
                // Rotate upright cylinder to lie horizontally, then yaw
                rotation: Quat::from_rotation_y(yaw)
                    * Quat::from_rotation_z(std::f32::consts::FRAC_PI_2),
                ..default()
            },
            RigidBody::Static,
            Collider::cylinder(log_r, log_l / 2.0),
        ));
        spawned += 1;
        placed += 1;
    }

    info!(
        "world_scatter: Logs — {} horizontal cylinders. Colliders on all.",
        spawned
    );
}
