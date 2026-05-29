// Sprint 69 — Map Dressing  (Sprint 76: Y-snap + PBR upgrades + LOD)
//
// MapDressingPlugin spawns themed visual props (with Static colliders) at
// startup for each gameplay area so the world feels like a real place instead
// of generic procedural terrain.
//
// Five Startup systems, one per area:
//
//   Area 1: Hillclimb Tiers   (Z = -180 / -210 / -240)
//   Area 2: Rock Crawl Trail  (sections at (120,0), (-80,80), (60,-120))
//   Area 3: Obstacle Course   (Z = +200 / +230 / +260)
//   Area 4: Trail Rides       (origin + 150 m line of markers)
//   Area 5: Default spawn     (origin — ranger hut, weather station, rigs, campfires)
//
// All props are pure visuals built from Bevy primitive meshes (Cuboid /
// Cylinder / Sphere / Cone).  Each has a Collider so vehicles can hit them.
// The MapProp marker component lets future cleanup systems despawn them
// in bulk.
//
// Sprint 76 changes
// -----------------
// 1. Y-SNAPPING: every prop's Y is computed as terrain_height_at(x,z) +
//    prop_half_height so it sits flush with the ground instead of floating or
//    clipping.  terrain_height_at() is the public sampler exposed by terrain.rs.
//
// 2. PBR MATERIAL UPGRADES: proper perceptual_roughness / metallic per type:
//    - Wood posts/signs:     roughness 0.92, metallic 0.0  (unchanged, explicit)
//    - Rubber tires:         roughness 0.98, metallic 0.0  (very rough, dark)
//    - Granite/rock:         roughness 0.97, metallic 0.0
//    - Metal poles/towers:   roughness 0.40, metallic 0.75 (shinier metal)
//    - Flags:                roughness 0.50, emissive × 0.5 (slight pop)
//    - Countdown/emissive:   roughness 0.45, boosted emissive
//    - Log cabin walls:      roughness 0.95, metallic 0.0
//    - Flame/fire:           roughness 0.35, emissive (high)
//
// 3. LOD: every prop also gets a PropLod component so the PropLodPlugin
//    distance-culling system can toggle Visibility::Hidden beyond 250 m.
//    Colliders stay active at all distances.
//
// 4. PLACEMENT FIXES:
//    - Hillclimb signs moved to Z ± 12 (was ±7.5) to clear the racing line.
//    - Summit flags shifted +8 m in X from HC_START_X+130 to avoid the
//      actual finish-gate collision box.
//    - Dirt barriers moved from mid-slope (cx+64) to cx+90, clear of the
//      active lane.
//    - Obstacle course tire wall extended from 8 to 9 tires per side but each
//      pushed ±11 m laterally (was ±9.5 m) so they don't edge the drivable
//      line.
//    - Rock crawl cairns offset another 1.5 m sideways (dz+4 → dz+5.5) so
//      they don't clip into the section-entry corridor.
//    - Ranger hut parked rigs moved to avoid overlapping the spawn pad
//      (they were inside the hut footprint at Y=0.55 — now Y-snapped).
//
// Coordinates sourced from:
//   hillclimb_tiers.rs  : START_X = -150, TIER_Z = [-180, -210, -240]
//   rock_crawl_trail.rs : SECTION_CX/CZ = (120,0), (-80,80), (60,-120)
//   obstacle_course.rs  : START_X = -60, COURSE_Z = [200, 230, 260]
//   trail_rides.rs      : spawn_x / spawn_z per manifest (we use origin)

use bevy::prelude::*;
use avian3d::prelude::*;

use crate::terrain::terrain_height_at;
use crate::prop_lod::PropLod;

// ---------------------------------------------------------------------------
// Marker component
// ---------------------------------------------------------------------------

/// Applied to every prop spawned by MapDressingPlugin so future cleanup is
/// easy: `commands.entity(e).despawn()` on all `With<MapProp>` queries.
#[derive(Component)]
pub struct MapProp;

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct MapDressingPlugin;

impl Plugin for MapDressingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Startup,
            (
                spawn_hillclimb_dressing,
                spawn_rock_crawl_dressing,
                spawn_obstacle_course_dressing,
                spawn_trail_ride_dressing,
                spawn_spawn_area_dressing,
            ),
        );
    }
}

// ---------------------------------------------------------------------------
// Shared palette colours
// ---------------------------------------------------------------------------

/// Near-black ground / dark rubber.
const RUBBER_COLOR: Color = Color::srgb(0.07, 0.06, 0.05);
/// Brown post/wood.
const WOOD_COLOR:   Color = Color::srgb(0.45, 0.30, 0.18);
/// Dirt brown barrier.
const DIRT_COLOR:   Color = Color::srgb(0.38, 0.26, 0.14);
/// Granite grey.
const GRANITE_COLOR: Color = Color::srgb(0.52, 0.50, 0.48);
/// Log-cabin brown.
const LOG_BROWN:    Color = Color::srgb(0.40, 0.28, 0.16);
/// Metal / utility grey.
const METAL_COLOR:  Color = Color::srgb(0.55, 0.55, 0.58);
/// Flame orange (emissive).
const FLAME_COLOR:  Color = Color::srgb(1.0, 0.45, 0.05);
/// Checkered white/dark helper.
const CHECK_WHITE:  Color = Color::srgb(0.95, 0.95, 0.95);
const CHECK_DARK:   Color = Color::srgb(0.15, 0.15, 0.15);
/// Countdown red.
const COUNTDOWN_RED: Color = Color::srgb(0.80, 0.10, 0.10);

// Tier label emissive colours (Beginner=green, Intermediate=amber, Expert=red).
const SIGN_BEGINNER:     Color = Color::srgb(0.0,  0.75, 0.20);
const SIGN_INTERMEDIATE: Color = Color::srgb(0.85, 0.50, 0.0);
const SIGN_EXPERT:       Color = Color::srgb(0.85, 0.10, 0.05);
const FLAG_BEGINNER:     Color = Color::srgb(0.20, 0.75, 0.20);
const FLAG_INTERMEDIATE: Color = Color::srgb(0.85, 0.55, 0.0);
const FLAG_EXPERT:       Color = Color::srgb(0.85, 0.15, 0.10);

// Tier-level trail-marker bright colours.
const TRAIL_MARKER_COLOR: Color = Color::srgb(0.10, 0.55, 0.90);

// ---------------------------------------------------------------------------
// Y-snap helper
// ---------------------------------------------------------------------------

/// Compute the world-Y for a prop whose mesh half_height is `half_h`, placed
/// at world (x, z).  The prop's geometric centre lands at ground + half_h so
/// the bottom face sits exactly on the terrain surface.
#[inline]
fn snap_y(x: f32, z: f32, half_h: f32) -> f32 {
    terrain_height_at(x, z) + half_h
}

// ---------------------------------------------------------------------------
// Area 1 — Hillclimb Tiers
// ---------------------------------------------------------------------------
//
// Props per tier (×3 tiers = 3 signs, 6 tire stacks = 18 spheres, 3 flags,
// 2 dirt barriers):
//   • Hand-painted plywood SIGN at start gate (sign body + 2 posts)    → 3 entities/tier
//   • Tire stack pyramid either side of start (3 spheres × 2 stacks)   → 6 spheres/tier
//   • Summit FLAG at tier top (pole + flag body)                        → 2 entities/tier
// Plus 2 DIRT BARRIERS between adjacent tiers.
//
// Prop count: 3×(3+6+2) + 2 = 33 + 2 = 35 entities
// Collider count: all 35 have colliders.
//
// Sprint 76 placement fixes:
//   - Tire stacks moved from side ±7.5 to ±12 m off centre so they don't
//     crowd the racing line (~6 m wide at the start gate).
//   - Summit flags shifted to HC_SUMMIT_X + 8 so they sit behind the gate.
//   - Dirt barriers at HC_START_X + 90 (was +64) to be off the active lane.

const HC_START_X:  f32 = -150.0;
const HC_TIER_Z:   [f32; 3] = [-180.0, -210.0, -240.0];

// Approximate summit X (START_X + 8 segments × ~16 m run each ≈ 130 m).
const HC_SUMMIT_X: f32 = HC_START_X + 138.0; // +8 m offset from old value

fn spawn_hillclimb_dressing(
    mut commands:  Commands,
    mut meshes:    ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let sign_colors: [Color; 3] = [SIGN_BEGINNER, SIGN_INTERMEDIATE, SIGN_EXPERT];
    let flag_colors: [Color; 3] = [FLAG_BEGINNER, FLAG_INTERMEDIATE, FLAG_EXPERT];

    // Shared meshes.
    let sign_body_mesh = meshes.add(Cuboid::new(2.4, 0.9, 0.10));
    let sign_post_mesh = meshes.add(Cylinder::new(0.06, 1.6));
    let tire_mesh      = meshes.add(Sphere::new(0.35));
    let flag_pole_mesh = meshes.add(Cylinder::new(0.05, 3.5));
    let flag_body_mesh = meshes.add(Cuboid::new(0.8, 0.45, 0.06));
    let barrier_mesh   = meshes.add(Cuboid::new(28.0, 1.2, 1.2));

    // Sprint 76 upgraded materials -------------------------------------------
    let wood_mat  = materials.add(StandardMaterial {
        base_color: WOOD_COLOR,
        perceptual_roughness: 0.92,
        metallic: 0.0,
        ..default()
    });
    let rubber_mat = materials.add(StandardMaterial {
        base_color: RUBBER_COLOR,
        perceptual_roughness: 0.98,
        metallic: 0.0,
        ..default()
    });
    let dirt_mat = materials.add(StandardMaterial {
        base_color: DIRT_COLOR,
        perceptual_roughness: 0.97,
        metallic: 0.0,
        ..default()
    });

    let mut prop_count = 0usize;

    for (tier_idx, &tz) in HC_TIER_Z.iter().enumerate() {
        let emissive_rgba = color_to_linear_rgba(sign_colors[tier_idx]);
        // Sprint 76: sign panels — warm cream base + boosted emissive so they
        // read at a distance; perceptual_roughness 0.75 (same as before but
        // metallic explicitly 0).
        let sign_mat = materials.add(StandardMaterial {
            base_color: Color::srgb(0.85, 0.78, 0.60),
            emissive: emissive_rgba,
            perceptual_roughness: 0.75,
            metallic: 0.0,
            ..default()
        });

        let flag_color = flag_colors[tier_idx];
        // Sprint 76: flags — slightly emissive (×0.6 instead of ×0.4) for pop.
        let flag_mat = materials.add(StandardMaterial {
            base_color: flag_color,
            perceptual_roughness: 0.50,
            metallic: 0.0,
            emissive: color_to_linear_rgba(flag_color) * 0.6,
            ..default()
        });

        // ---- SIGN at start gate ----
        // Sprint 76: sign_x 3 m before start gate; Y-snapped.
        let sign_x  = HC_START_X - 3.0;
        let sign_z  = tz;
        // Sign body: half-height = 0.45; posts are 1.6 tall cylinders so
        // their base half_height for the snap is 0.8.
        let sign_body_y = snap_y(sign_x, sign_z, 0.45 + 1.6); // body sits on top of posts
        // Post base Y.
        let post_y = snap_y(sign_x, sign_z, 0.8);

        // Sign body
        commands.spawn((
            MapProp,
            PropLod { half_height: 0.45 },
            Mesh3d(sign_body_mesh.clone()),
            MeshMaterial3d(sign_mat.clone()),
            Transform::from_xyz(sign_x, sign_body_y, sign_z),
            RigidBody::Static,
            Collider::cuboid(1.2, 0.45, 0.05),
        ));
        // Left post
        commands.spawn((
            MapProp,
            PropLod { half_height: 0.8 },
            Mesh3d(sign_post_mesh.clone()),
            MeshMaterial3d(wood_mat.clone()),
            Transform::from_xyz(sign_x - 0.9, post_y, sign_z),
            RigidBody::Static,
            Collider::cylinder(0.06, 0.8),
        ));
        // Right post
        commands.spawn((
            MapProp,
            PropLod { half_height: 0.8 },
            Mesh3d(sign_post_mesh.clone()),
            MeshMaterial3d(wood_mat.clone()),
            Transform::from_xyz(sign_x + 0.9, post_y, sign_z),
            RigidBody::Static,
            Collider::cylinder(0.06, 0.8),
        ));
        prop_count += 3;

        // ---- TIRE STACKS — left and right of start gate ----
        // Sprint 76: moved from ±7.5 to ±12 m so they clear the racing line.
        for &side in &[-1.0_f32, 1.0_f32] {
            let base_x = HC_START_X - 1.5;
            let base_z = tz + side * 12.0; // was 7.5
            // Row of 2 on ground, 1 on top.
            let ground_y = terrain_height_at(base_x, base_z);
            let offsets: [(f32, f32, f32); 3] = [
                (-0.38, ground_y + 0.35, 0.0),
                ( 0.38, ground_y + 0.35, 0.0),
                ( 0.0,  ground_y + 1.05, 0.0),
            ];
            for (dx, ey, dz) in offsets {
                commands.spawn((
                    MapProp,
                    PropLod { half_height: 0.35 },
                    Mesh3d(tire_mesh.clone()),
                    MeshMaterial3d(rubber_mat.clone()),
                    Transform::from_xyz(base_x + dx, ey, base_z + dz),
                    RigidBody::Static,
                    Collider::sphere(0.35),
                ));
                prop_count += 1;
            }
        }

        // ---- SUMMIT FLAG ----
        let sum_x = HC_SUMMIT_X;
        let sum_ground = terrain_height_at(sum_x, tz);
        let flag_pole_half_h = 1.75;
        let pole_y = sum_ground + flag_pole_half_h;
        // Flag body sits at top of pole + 0.225 (half flag height).
        let flag_y = sum_ground + 3.5 + 0.225;

        // Sprint 76: metal pole with better metallic params.
        let metal_m = metal_mat(&mut materials);
        // Pole
        commands.spawn((
            MapProp,
            PropLod { half_height: flag_pole_half_h },
            Mesh3d(flag_pole_mesh.clone()),
            MeshMaterial3d(metal_m),
            Transform::from_xyz(sum_x, pole_y, tz),
            RigidBody::Static,
            Collider::cylinder(0.05, flag_pole_half_h),
        ));
        // Flag body (at top of pole)
        commands.spawn((
            MapProp,
            PropLod { half_height: 0.225 },
            Mesh3d(flag_body_mesh.clone()),
            MeshMaterial3d(flag_mat.clone()),
            Transform::from_xyz(sum_x + 0.5, flag_y, tz),
            RigidBody::Static,
            Collider::cuboid(0.4, 0.225, 0.03),
        ));
        prop_count += 2;
    }

    // ---- DIRT BARRIERS between adjacent tiers ----
    // Sprint 76: moved from cx+64 to cx+90 to clear the drivable lane.
    for &mid_z in &[-195.0_f32, -225.0_f32] {
        let barrier_cx = HC_START_X + 90.0; // was +64
        let bary = snap_y(barrier_cx, mid_z, 0.6);
        commands.spawn((
            MapProp,
            PropLod { half_height: 0.6 },
            Mesh3d(barrier_mesh.clone()),
            MeshMaterial3d(dirt_mat.clone()),
            Transform::from_xyz(barrier_cx, bary, mid_z),
            RigidBody::Static,
            Collider::cuboid(14.0, 0.6, 0.6),
        ));
        prop_count += 1;
    }

    info!(
        "map_dressing: Area 1 (Hillclimb Tiers) — {} props, all with colliders",
        prop_count
    );
}

// ---------------------------------------------------------------------------
// Area 2 — Rock Crawl Trail
// ---------------------------------------------------------------------------
//
// Sections: (120,0)=Boulder Stairs, (-80,80)=Two-Log Bridge, (60,-120)=Off-Camber
//
// Props:
//   • Cairn at each section start  (4 spheres × 3)          = 12 spheres
//   • Approach bridge at Two-Log Bridge (4 planks + 4 legs) =  8 entities
//   • Trail sign at each section (body + 2 posts × 3)       =  9 entities
//   • Loose boulders between sections (7 spheres)           =  7 spheres
//
// Total: 36 entities, all with colliders.
//
// Sprint 76 placement fix:
//   - Cairns offset from dz+4 to dz+5.5 so they clear the corridor entry.

const RC_CX: [f32; 3] = [120.0, -80.0,  60.0];
const RC_CZ: [f32; 3] = [  0.0,  80.0, -120.0];
const RC_NAMES: [&str; 3] = ["BOULDER STAIRS", "TWO-LOG BRIDGE", "OFF-CAMBER"];
// Corridor half-lengths from rock_crawl_trail.rs.
const RC_HALF: [f32; 3] = [18.0, 10.0, 22.0];

fn spawn_rock_crawl_dressing(
    mut commands:  Commands,
    mut meshes:    ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Sprint 76 upgraded materials -------------------------------------------
    let rock_mat = materials.add(StandardMaterial {
        base_color: GRANITE_COLOR,
        perceptual_roughness: 0.97,
        metallic: 0.0,
        ..default()
    });
    let wood_mat = materials.add(StandardMaterial {
        base_color: WOOD_COLOR,
        perceptual_roughness: 0.90,
        metallic: 0.0,
        ..default()
    });
    // Trail sign: warm sandy colour with subtle emissive for readability.
    let sign_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.82, 0.72, 0.52),
        perceptual_roughness: 0.78,
        metallic: 0.0,
        emissive: LinearRgba::rgb(0.20, 0.12, 0.04),
        ..default()
    });
    let sign_post_mat = materials.add(StandardMaterial {
        base_color: WOOD_COLOR,
        perceptual_roughness: 0.92,
        metallic: 0.0,
        ..default()
    });

    let cairn_sphere_mesh  = meshes.add(Sphere::new(0.22));
    let sign_body_mesh     = meshes.add(Cuboid::new(2.2, 0.7, 0.10));
    let sign_post_mesh     = meshes.add(Cylinder::new(0.055, 1.4));
    let plank_mesh         = meshes.add(Cuboid::new(3.6, 0.12, 0.55));
    let leg_mesh           = meshes.add(Cylinder::new(0.07, 0.6));

    let mut prop_count = 0usize;

    // ---- CAIRN + TRAIL SIGN at each section start ----
    for (sec, (&cx, &cz)) in RC_CX.iter().zip(RC_CZ.iter()).enumerate() {
        let start_x = cx - RC_HALF[sec] - 2.0; // just before the start gate

        // Cairn: 3 spheres bottom row + 1 on top.
        // Sprint 76: terrain-snapped, and moved from dz+4 to dz+5.5 to clear
        // the section entry corridor.
        let cairn_z = cz + 5.5; // was +4.0
        let cairn_ground = terrain_height_at(start_x, cairn_z);
        let cairn_offsets: [(f32, f32, f32); 4] = [
            (-0.28, cairn_ground + 0.22, -0.18),
            ( 0.28, cairn_ground + 0.22,  0.15),
            ( 0.0,  cairn_ground + 0.22,  0.28),
            ( 0.0,  cairn_ground + 0.60,  0.05),
        ];
        for (dx, ey, dz) in cairn_offsets {
            commands.spawn((
                MapProp,
                PropLod { half_height: 0.22 },
                Mesh3d(cairn_sphere_mesh.clone()),
                MeshMaterial3d(rock_mat.clone()),
                Transform::from_xyz(start_x + dx, ey, cairn_z + dz),
                RigidBody::Static,
                Collider::sphere(0.22),
            ));
            prop_count += 1;
        }

        // Trail sign — Y-snapped.
        let sign_x = start_x - 1.5;
        let sign_post_y = snap_y(sign_x, cz, 0.7); // half post height (1.4/2)
        let sign_body_y = terrain_height_at(sign_x, cz) + 1.4 + 0.35; // top of post + half panel

        // Body
        commands.spawn((
            MapProp,
            PropLod { half_height: 0.35 },
            Mesh3d(sign_body_mesh.clone()),
            MeshMaterial3d(sign_mat.clone()),
            Transform::from_xyz(sign_x, sign_body_y, cz),
            RigidBody::Static,
            Collider::cuboid(1.1, 0.35, 0.05),
        ));
        // Left post
        commands.spawn((
            MapProp,
            PropLod { half_height: 0.7 },
            Mesh3d(sign_post_mesh.clone()),
            MeshMaterial3d(sign_post_mat.clone()),
            Transform::from_xyz(sign_x - 0.85, sign_post_y, cz),
            RigidBody::Static,
            Collider::cylinder(0.055, 0.7),
        ));
        // Right post
        commands.spawn((
            MapProp,
            PropLod { half_height: 0.7 },
            Mesh3d(sign_post_mesh.clone()),
            MeshMaterial3d(sign_post_mat.clone()),
            Transform::from_xyz(sign_x + 0.85, sign_post_y, cz),
            RigidBody::Static,
            Collider::cylinder(0.055, 0.7),
        ));
        prop_count += 3;

        let _ = RC_NAMES[sec]; // used for reference / future text overlay
    }

    // ---- APPROACH BRIDGE at Two-Log Bridge section ----
    // Section 1: (-80, _, 80). Approach bridge on the entry side (X < -90).
    {
        let bridge_base_x = RC_CX[1] - RC_HALF[1] - 8.0; // ~-98 m
        let bz             = RC_CZ[1]; // 80

        // 4 planks spanning the approach — Y-snapped per plank.
        for i in 0..4usize {
            let px = bridge_base_x + (i as f32) * 3.8;
            let plank_y = snap_y(px, bz, 0.06);
            commands.spawn((
                MapProp,
                PropLod { half_height: 0.06 },
                Mesh3d(plank_mesh.clone()),
                MeshMaterial3d(wood_mat.clone()),
                Transform::from_xyz(px, plank_y, bz),
                RigidBody::Static,
                Collider::cuboid(1.8, 0.06, 0.275),
            ));
            prop_count += 1;
        }

        // 4 short post legs under the planks — Y-snapped.
        let leg_positions: [(f32, f32); 4] = [
            (bridge_base_x,          bz - 1.2),
            (bridge_base_x,          bz + 1.2),
            (bridge_base_x + 11.4,   bz - 1.2),
            (bridge_base_x + 11.4,   bz + 1.2),
        ];
        for (lx, lz) in leg_positions {
            let leg_y = snap_y(lx, lz, 0.30);
            commands.spawn((
                MapProp,
                PropLod { half_height: 0.30 },
                Mesh3d(leg_mesh.clone()),
                MeshMaterial3d(wood_mat.clone()),
                Transform::from_xyz(lx, leg_y, lz),
                RigidBody::Static,
                Collider::cylinder(0.07, 0.28),
            ));
            prop_count += 1;
        }
    }

    // ---- LOOSE BOULDERS between sections for visual continuity ----
    // Deterministic positions (no rand — constant array).
    // Sprint 76: Y values are now terrain-snapped (the old fixed Y values are
    // ignored).
    let boulder_data: [(f32, f32); 7] = [
        ( 50.0,  35.0),
        (-20.0,  50.0),
        ( 80.0, -30.0),
        (-50.0,  -5.0),
        ( 30.0,  65.0),
        (-10.0, -80.0),
        ( 10.0,  20.0),
    ];
    let boulder_radii: [f32; 7] = [0.55, 0.70, 0.42, 0.65, 0.50, 0.60, 0.38];

    for (i, &(bx, bz)) in boulder_data.iter().enumerate() {
        let r = boulder_radii[i];
        let sphere_mesh = meshes.add(Sphere::new(r));
        let by = snap_y(bx, bz, r);
        commands.spawn((
            MapProp,
            PropLod { half_height: r },
            Mesh3d(sphere_mesh),
            MeshMaterial3d(rock_mat.clone()),
            Transform::from_xyz(bx, by, bz),
            RigidBody::Static,
            Collider::sphere(r),
        ));
        prop_count += 1;
    }

    info!(
        "map_dressing: Area 2 (Rock Crawl Trail) — {} props, all with colliders",
        prop_count
    );
}

// ---------------------------------------------------------------------------
// Area 3 — Obstacle Course
// ---------------------------------------------------------------------------
//
// START_X = -60, COURSE_Z = [200, 230, 260]
//
// Per level (×3):
//   • Tire wall (9 spheres per side × 2 sides = 18 spheres/level)   [was 8]
//   • Finish-line tower (thin post + platform + checkered flag = 3 entities)
//   • Countdown post at the start (1 cylinder)
//   • Course-name sign at the start (body + 2 posts = 3 entities)
//
// Total: 3 × (18 + 3 + 1 + 3) = 3 × 25 = 75 entities, all with colliders.
//
// Sprint 76 placement fix:
//   - Tire walls moved from ±9.5 to ±11 m off course centre so they don't
//     edge the drivable 8 m lane.
//   - Added 9th tire per side for a longer guard-rail effect.

const OC_START_X: f32 = -60.0;
const OC_COURSE_Z: [f32; 3] = [200.0, 230.0, 260.0];
// Finish X (approx); finish towers already sat just past finish so fine.
const OC_FINISH_X: [f32; 3] = [32.0, 50.0, 60.0];
const OC_NAMES: [&str; 3] = ["OBSTACLE BEGINNER", "OBSTACLE INTERMEDIATE", "OBSTACLE EXPERT"];

fn spawn_obstacle_course_dressing(
    mut commands:  Commands,
    mut meshes:    ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Sprint 76 upgraded materials -------------------------------------------
    let rubber_mat = materials.add(StandardMaterial {
        base_color: RUBBER_COLOR,
        perceptual_roughness: 0.98,
        metallic: 0.0,
        ..default()
    });
    // Countdown post: brighter emissive for a beacon-like appearance.
    let countdown_mat = materials.add(StandardMaterial {
        base_color: COUNTDOWN_RED,
        perceptual_roughness: 0.45,
        metallic: 0.1,
        emissive: LinearRgba::rgb(0.55, 0.03, 0.03),
        ..default()
    });
    // Sign panels: sandy, slightly warmer emissive for at-distance readability.
    let sign_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.80, 0.70, 0.50),
        perceptual_roughness: 0.75,
        metallic: 0.0,
        emissive: LinearRgba::rgb(0.18, 0.10, 0.03),
        ..default()
    });
    let sign_post_mat = materials.add(StandardMaterial {
        base_color: WOOD_COLOR,
        perceptual_roughness: 0.92,
        metallic: 0.0,
        ..default()
    });
    // Tower: proper metallic — shinier like painted steel.
    let tower_mat = materials.add(StandardMaterial {
        base_color: METAL_COLOR,
        perceptual_roughness: 0.40,
        metallic: 0.75,
        ..default()
    });
    // Checkered flag squares — matte fabric look.
    let check_white_mat = materials.add(StandardMaterial {
        base_color: CHECK_WHITE,
        perceptual_roughness: 0.65,
        metallic: 0.0,
        ..default()
    });
    let check_dark_mat = materials.add(StandardMaterial {
        base_color: CHECK_DARK,
        perceptual_roughness: 0.65,
        metallic: 0.0,
        ..default()
    });

    let tire_mesh          = meshes.add(Sphere::new(0.40));
    let countdown_mesh     = meshes.add(Cylinder::new(0.12, 2.0));
    let sign_body_mesh     = meshes.add(Cuboid::new(2.6, 0.75, 0.10));
    let sign_post_mesh_oc  = meshes.add(Cylinder::new(0.06, 1.5));
    let tower_post_mesh    = meshes.add(Cuboid::new(0.18, 5.0, 0.18));
    let platform_mesh      = meshes.add(Cuboid::new(2.4, 0.15, 2.4));
    let flag_check_mesh    = meshes.add(Cuboid::new(1.2, 0.60, 0.06));

    let mut prop_count = 0usize;

    for (level, &cz) in OC_COURSE_Z.iter().enumerate() {
        let finish_x = OC_FINISH_X[level];

        // ---- TIRE WALL — left and right boundary ----
        // Sprint 76: 9 tires (was 8), walls at ±11 m (was ±9.5).
        for &side in &[-1.0_f32, 1.0_f32] {
            let wall_z = cz + side * 11.0; // was 9.5
            for i in 0..9usize {               // was 8
                let tx = OC_START_X + (i as f32) * 11.0 + 6.0;
                let tire_y = snap_y(tx, wall_z, 0.20); // half-buried: 0.4 r, 0.2 below
                commands.spawn((
                    MapProp,
                    PropLod { half_height: 0.40 },
                    Mesh3d(tire_mesh.clone()),
                    MeshMaterial3d(rubber_mat.clone()),
                    Transform::from_xyz(tx, tire_y, wall_z),
                    RigidBody::Static,
                    Collider::sphere(0.40),
                ));
                prop_count += 1;
            }
        }

        // ---- FINISH-LINE TOWER ----
        let tower_x = finish_x + 2.0;
        let tower_ground = terrain_height_at(tower_x, cz);
        // Tall post: half-height = 2.5.
        commands.spawn((
            MapProp,
            PropLod { half_height: 2.5 },
            Mesh3d(tower_post_mesh.clone()),
            MeshMaterial3d(tower_mat.clone()),
            Transform::from_xyz(tower_x, tower_ground + 2.5, cz),
            RigidBody::Static,
            Collider::cuboid(0.09, 2.5, 0.09),
        ));
        // Platform on top (0.075 half-height)
        commands.spawn((
            MapProp,
            PropLod { half_height: 0.075 },
            Mesh3d(platform_mesh.clone()),
            MeshMaterial3d(tower_mat.clone()),
            Transform::from_xyz(tower_x, tower_ground + 5.0 + 0.075, cz),
            RigidBody::Static,
            Collider::cuboid(1.2, 0.075, 1.2),
        ));
        // Checkered flag (0.30 half-height)
        commands.spawn((
            MapProp,
            PropLod { half_height: 0.30 },
            Mesh3d(flag_check_mesh.clone()),
            MeshMaterial3d(if level % 2 == 0 { check_white_mat.clone() } else { check_dark_mat.clone() }),
            Transform::from_xyz(tower_x, tower_ground + 5.15 + 0.30, cz),
            RigidBody::Static,
            Collider::cuboid(0.60, 0.30, 0.03),
        ));
        prop_count += 3;

        // ---- COUNTDOWN POST at start ----
        let cd_x = OC_START_X - 4.0;
        let cd_y = snap_y(cd_x, cz, 1.0);
        commands.spawn((
            MapProp,
            PropLod { half_height: 1.0 },
            Mesh3d(countdown_mesh.clone()),
            MeshMaterial3d(countdown_mat.clone()),
            Transform::from_xyz(cd_x, cd_y, cz),
            RigidBody::Static,
            Collider::cylinder(0.12, 1.0),
        ));
        prop_count += 1;

        // ---- COURSE-NAME SIGN at start ----
        let sign_x = OC_START_X - 6.0;
        let sign_sz = cz + 7.0;
        let sign_post_y = snap_y(sign_x, sign_sz, 0.75); // half of post h=1.5
        let sign_body_y = terrain_height_at(sign_x, sign_sz) + 1.5 + 0.375;
        // Body
        commands.spawn((
            MapProp,
            PropLod { half_height: 0.375 },
            Mesh3d(sign_body_mesh.clone()),
            MeshMaterial3d(sign_mat.clone()),
            Transform::from_xyz(sign_x, sign_body_y, sign_sz),
            RigidBody::Static,
            Collider::cuboid(1.3, 0.375, 0.05),
        ));
        // Left post
        commands.spawn((
            MapProp,
            PropLod { half_height: 0.75 },
            Mesh3d(sign_post_mesh_oc.clone()),
            MeshMaterial3d(sign_post_mat.clone()),
            Transform::from_xyz(sign_x - 0.95, sign_post_y, sign_sz),
            RigidBody::Static,
            Collider::cylinder(0.06, 0.75),
        ));
        // Right post
        commands.spawn((
            MapProp,
            PropLod { half_height: 0.75 },
            Mesh3d(sign_post_mesh_oc.clone()),
            MeshMaterial3d(sign_post_mat.clone()),
            Transform::from_xyz(sign_x + 0.95, sign_post_y, sign_sz),
            RigidBody::Static,
            Collider::cylinder(0.06, 0.75),
        ));
        prop_count += 3;

        let _ = OC_NAMES[level]; // used for reference
    }

    info!(
        "map_dressing: Area 3 (Obstacle Course) — {} props, all with colliders",
        prop_count
    );
}

// ---------------------------------------------------------------------------
// Area 4 — Trail Rides
// ---------------------------------------------------------------------------
//
// 5 trail markers in a line from spawn (origin) toward +X, spaced 30 m apart.
// Each marker: tall thin cylinder (pole) + small cuboid flag on top.
//
// Total: 5 × 2 = 10 entities, all with colliders.

fn spawn_trail_ride_dressing(
    mut commands:  Commands,
    mut meshes:    ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Sprint 76 upgraded materials -------------------------------------------
    // Metal poles: proper metallic.
    let pole_mat = materials.add(StandardMaterial {
        base_color: METAL_COLOR,
        perceptual_roughness: 0.40,
        metallic: 0.75,
        ..default()
    });
    // Trail flags: slightly emissive for readability (boosted from 0.18 to 0.30).
    let flag_mat = materials.add(StandardMaterial {
        base_color: TRAIL_MARKER_COLOR,
        perceptual_roughness: 0.45,
        metallic: 0.0,
        emissive: LinearRgba::rgb(0.07, 0.28, 0.60),
        ..default()
    });

    let pole_mesh = meshes.add(Cylinder::new(0.07, 3.0));
    let flag_mesh = meshes.add(Cuboid::new(0.55, 0.30, 0.06));

    const SPAWN_X: f32 = 15.0; // offset from origin so it's not inside the hut
    const SPAWN_Z: f32 = 8.0;
    const SPACING: f32 = 30.0;
    const NUM_MARKERS: usize = 5;

    let mut prop_count = 0usize;

    for i in 0..NUM_MARKERS {
        let mx = SPAWN_X + (i as f32) * SPACING;
        let mz = SPAWN_Z;

        // Pole: half-height = 1.5 (cylinder height 3.0 / 2).
        let pole_y = snap_y(mx, mz, 1.5);
        // Flag: sits at top of pole + half-flag-height.
        let flag_y = terrain_height_at(mx, mz) + 3.0 + 0.15;

        // Pole
        commands.spawn((
            MapProp,
            PropLod { half_height: 1.5 },
            Mesh3d(pole_mesh.clone()),
            MeshMaterial3d(pole_mat.clone()),
            Transform::from_xyz(mx, pole_y, mz),
            RigidBody::Static,
            Collider::cylinder(0.07, 1.5),
        ));
        // Flag
        commands.spawn((
            MapProp,
            PropLod { half_height: 0.15 },
            Mesh3d(flag_mesh.clone()),
            MeshMaterial3d(flag_mat.clone()),
            Transform::from_xyz(mx + 0.32, flag_y, mz),
            RigidBody::Static,
            Collider::cuboid(0.275, 0.15, 0.03),
        ));
        prop_count += 2;
    }

    info!(
        "map_dressing: Area 4 (Trail Rides) — {} props, all with colliders",
        prop_count
    );
}

// ---------------------------------------------------------------------------
// Area 5 — Default Spawn Area (origin)
// ---------------------------------------------------------------------------
//
// Props:
//   • Ranger hut: base + roof + door + window               = 4 entities
//   • Weather station: pole + 2 arms + wind vane            = 4 entities
//   • 3 parked Jeep silhouettes: body each                  = 3 entities
//   • 4 campfire spots: 3 logs + emissive cone              = 4 × 4 = 16 entities
//
// Total: 4 + 4 + 3 + 16 = 27 entities, all with colliders.
//
// Sprint 76 placement fix:
//   - Parked rigs Y-snapped (they were Y=0.55 fixed; hut at Z=-15 has
//     non-zero terrain_height so they were clipping).
//   - Campfire flames Y-snapped above the log pile.

fn spawn_spawn_area_dressing(
    mut commands:  Commands,
    mut meshes:    ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Sprint 76 upgraded materials -------------------------------------------
    let log_mat = materials.add(StandardMaterial {
        base_color: LOG_BROWN,
        perceptual_roughness: 0.95,
        metallic: 0.0,
        ..default()
    });
    // Weather station / metal details: shinier.
    let metal_mat_res = materials.add(StandardMaterial {
        base_color: METAL_COLOR,
        perceptual_roughness: 0.40,
        metallic: 0.75,
        ..default()
    });
    let rig_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.22, 0.18, 0.14),
        perceptual_roughness: 0.88,
        metallic: 0.05,
        ..default()
    });
    let fire_log_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.30, 0.18, 0.08),
        perceptual_roughness: 0.96,
        metallic: 0.0,
        ..default()
    });
    // Flame: brighter emissive — a good beacon.
    let flame_mat = materials.add(StandardMaterial {
        base_color: FLAME_COLOR,
        emissive: LinearRgba::rgb(2.0, 0.7, 0.05),
        perceptual_roughness: 0.35,
        metallic: 0.0,
        ..default()
    });
    // Window glass: slight reflection.
    let window_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(0.50, 0.70, 0.90, 0.65),
        alpha_mode: AlphaMode::Blend,
        perceptual_roughness: 0.05,
        metallic: 0.85,
        ..default()
    });

    // Meshes.
    let hut_base_mesh  = meshes.add(Cuboid::new(5.0, 3.0, 4.0));
    let hut_roof_mesh  = meshes.add(Cuboid::new(5.6, 0.5, 4.6));
    let hut_door_mesh  = meshes.add(Cuboid::new(0.9, 1.8, 0.15));
    let hut_win_mesh   = meshes.add(Cuboid::new(0.8, 0.7, 0.12));
    let w_pole_mesh    = meshes.add(Cylinder::new(0.06, 4.0));
    let w_arm_mesh     = meshes.add(Cuboid::new(1.2, 0.08, 0.08));
    let w_vane_mesh    = meshes.add(Cuboid::new(0.55, 0.25, 0.04));
    let rig_body_mesh  = meshes.add(Cuboid::new(2.8, 1.1, 1.6));
    let fire_log_mesh  = meshes.add(Cuboid::new(1.2, 0.14, 0.14));
    let flame_mesh     = meshes.add(Cone { radius: 0.22, height: 0.55 });

    let mut prop_count = 0usize;

    // ====== RANGER HUT (centre at (-20, 0, -15)) ======
    let hut_x = -20.0_f32;
    let hut_z = -15.0_f32;
    let hut_ground = terrain_height_at(hut_x, hut_z);

    // Base / walls: half-height = 1.5, sits on ground.
    commands.spawn((
        MapProp,
        PropLod { half_height: 1.5 },
        Mesh3d(hut_base_mesh.clone()),
        MeshMaterial3d(log_mat.clone()),
        Transform::from_xyz(hut_x, hut_ground + 1.5, hut_z),
        RigidBody::Static,
        Collider::cuboid(2.5, 1.5, 2.0),
    ));
    // Roof (slightly overhanging): sits on top of walls.
    commands.spawn((
        MapProp,
        PropLod { half_height: 0.25 },
        Mesh3d(hut_roof_mesh.clone()),
        MeshMaterial3d(log_mat.clone()),
        Transform {
            translation: Vec3::new(hut_x, hut_ground + 3.0 + 0.25, hut_z),
            rotation: Quat::from_rotation_z(0.35), // sloped
            scale: Vec3::ONE,
        },
        RigidBody::Static,
        Collider::cuboid(2.8, 0.25, 2.3),
    ));
    // Door
    commands.spawn((
        MapProp,
        PropLod { half_height: 0.9 },
        Mesh3d(hut_door_mesh.clone()),
        MeshMaterial3d(log_mat.clone()),
        Transform::from_xyz(hut_x + 1.5, hut_ground + 0.9, hut_z + 2.075),
        RigidBody::Static,
        Collider::cuboid(0.45, 0.9, 0.075),
    ));
    // Window
    commands.spawn((
        MapProp,
        PropLod { half_height: 0.35 },
        Mesh3d(hut_win_mesh.clone()),
        MeshMaterial3d(window_mat.clone()),
        Transform::from_xyz(hut_x - 1.0, hut_ground + 1.8, hut_z + 2.075),
        RigidBody::Static,
        Collider::cuboid(0.40, 0.35, 0.06),
    ));
    prop_count += 4;

    // ====== WEATHER STATION (centre at (-10, 0, -12)) ======
    let wx = -10.0_f32;
    let wz = -12.0_f32;
    let wground = terrain_height_at(wx, wz);

    // Vertical pole: half-height = 2.0.
    commands.spawn((
        MapProp,
        PropLod { half_height: 2.0 },
        Mesh3d(w_pole_mesh.clone()),
        MeshMaterial3d(metal_mat_res.clone()),
        Transform::from_xyz(wx, wground + 2.0, wz),
        RigidBody::Static,
        Collider::cylinder(0.06, 2.0),
    ));
    // Horizontal arm 1 (along X) at top of pole.
    commands.spawn((
        MapProp,
        PropLod { half_height: 0.04 },
        Mesh3d(w_arm_mesh.clone()),
        MeshMaterial3d(metal_mat_res.clone()),
        Transform::from_xyz(wx, wground + 3.9, wz),
        RigidBody::Static,
        Collider::cuboid(0.6, 0.04, 0.04),
    ));
    // Horizontal arm 2 (along Z).
    commands.spawn((
        MapProp,
        PropLod { half_height: 0.04 },
        Mesh3d(w_arm_mesh.clone()),
        MeshMaterial3d(metal_mat_res.clone()),
        Transform {
            translation: Vec3::new(wx, wground + 3.6, wz),
            rotation: Quat::from_rotation_y(std::f32::consts::FRAC_PI_2),
            scale: Vec3::ONE,
        },
        RigidBody::Static,
        Collider::cuboid(0.6, 0.04, 0.04),
    ));
    // Wind vane.
    commands.spawn((
        MapProp,
        PropLod { half_height: 0.125 },
        Mesh3d(w_vane_mesh.clone()),
        MeshMaterial3d(metal_mat_res.clone()),
        Transform::from_xyz(wx + 0.6, wground + 4.1, wz),
        RigidBody::Static,
        Collider::cuboid(0.275, 0.125, 0.02),
    ));
    prop_count += 4;

    // ====== PARKED RIGS (3 Jeep silhouettes) ======
    // Sprint 76: Y-snapped.  Original Y=0.55 was the half-height of the body
    // mesh; we now compute ground + 0.55.
    let rig_spawns: [(f32, f32); 3] = [
        (-12.0, -6.0),
        (-16.0,  2.0),
        ( -6.0, -3.0),
    ];
    let rig_yaws: [f32; 3] = [0.45, -0.80, 1.20];

    for (i, &(rx, rz)) in rig_spawns.iter().enumerate() {
        let ry = snap_y(rx, rz, 0.55); // half-height of rig body
        commands.spawn((
            MapProp,
            PropLod { half_height: 0.55 },
            Mesh3d(rig_body_mesh.clone()),
            MeshMaterial3d(rig_mat.clone()),
            Transform {
                translation: Vec3::new(rx, ry, rz),
                rotation: Quat::from_rotation_y(rig_yaws[i]),
                scale: Vec3::ONE,
            },
            RigidBody::Static,
            Collider::cuboid(1.4, 0.55, 0.8),
        ));
        prop_count += 1;
    }

    // ====== CAMPFIRES (4 spots) ======
    let campfire_spawns: [(f32, f32); 4] = [
        (-8.0,   5.0),
        (-24.0, -8.0),
        (-14.0, 10.0),
        ( -4.0, -8.0),
    ];

    // Log orientations (3 logs per fire = 3 different yaw angles).
    let log_yaws: [f32; 3] = [0.0, 1.05, -1.05];

    for &(cfx, cfz) in &campfire_spawns {
        let cf_ground = terrain_height_at(cfx, cfz);

        // 3 logs arranged in a star pattern — Y-snapped per log.
        for (j, &yaw) in log_yaws.iter().enumerate() {
            let offset_x = yaw.cos() * 0.3;
            let offset_z = yaw.sin() * 0.3;
            let log_ground = terrain_height_at(cfx + offset_x, cfz + offset_z);
            commands.spawn((
                MapProp,
                PropLod { half_height: 0.07 },
                Mesh3d(fire_log_mesh.clone()),
                MeshMaterial3d(fire_log_mat.clone()),
                Transform {
                    translation: Vec3::new(
                        cfx + offset_x,
                        log_ground + 0.07 + j as f32 * 0.06,
                        cfz + offset_z,
                    ),
                    rotation: Quat::from_rotation_y(yaw),
                    scale: Vec3::ONE,
                },
                RigidBody::Static,
                Collider::cuboid(0.6, 0.07, 0.07),
            ));
            prop_count += 1;
        }

        // Emissive flame cone on top of logs — Y-snapped above the log pile.
        commands.spawn((
            MapProp,
            PropLod { half_height: 0.275 },
            Mesh3d(flame_mesh.clone()),
            MeshMaterial3d(flame_mat.clone()),
            Transform::from_xyz(cfx, cf_ground + 0.20 + 0.275, cfz),
            RigidBody::Static,
            Collider::sphere(0.22),
        ));
        prop_count += 1;
    }

    info!(
        "map_dressing: Area 5 (Spawn Area) — {} props, all with colliders",
        prop_count
    );
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Quick emissive colour helper — converts a Bevy `Color` to LinearRgba.
fn color_to_linear_rgba(c: Color) -> LinearRgba {
    let lc = c.to_linear();
    LinearRgba::new(lc.red, lc.green, lc.blue, lc.alpha)
}

/// Create a one-off metal StandardMaterial with Sprint 76 PBR values.
/// metallic = 0.75, roughness = 0.40 for painted-steel look.
fn metal_mat(materials: &mut Assets<StandardMaterial>) -> Handle<StandardMaterial> {
    materials.add(StandardMaterial {
        base_color: METAL_COLOR,
        perceptual_roughness: 0.40,
        metallic: 0.75,
        ..default()
    })
}
