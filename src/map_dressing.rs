// Sprint 69 — Map Dressing
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
// Coordinates sourced from:
//   hillclimb_tiers.rs  : START_X = -150, TIER_Z = [-180, -210, -240]
//   rock_crawl_trail.rs : SECTION_CX/CZ = (120,0), (-80,80), (60,-120)
//   obstacle_course.rs  : START_X = -60, COURSE_Z = [200, 230, 260]
//   trail_rides.rs      : spawn_x / spawn_z per manifest (we use origin)

use bevy::prelude::*;
use avian3d::prelude::*;

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

const HC_START_X:  f32 = -150.0;
const HC_TIER_Z:   [f32; 3] = [-180.0, -210.0, -240.0];

// Approximate summit X (START_X + 8 segments × ~16 m run each ≈ 130 m).
const HC_SUMMIT_X: f32 = HC_START_X + 130.0;

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

    let wood_mat  = materials.add(StandardMaterial {
        base_color: WOOD_COLOR,
        perceptual_roughness: 0.92,
        ..default()
    });
    let rubber_mat = materials.add(StandardMaterial {
        base_color: RUBBER_COLOR,
        perceptual_roughness: 0.98,
        ..default()
    });
    let dirt_mat = materials.add(StandardMaterial {
        base_color: DIRT_COLOR,
        perceptual_roughness: 0.97,
        ..default()
    });

    let mut prop_count = 0usize;

    for (tier_idx, &tz) in HC_TIER_Z.iter().enumerate() {
        let emissive_rgba = color_to_linear_rgba(sign_colors[tier_idx]);
        let sign_mat = materials.add(StandardMaterial {
            base_color: Color::srgb(0.85, 0.78, 0.60),
            emissive: emissive_rgba,
            perceptual_roughness: 0.75,
            ..default()
        });

        let flag_color = flag_colors[tier_idx];
        let flag_mat = materials.add(StandardMaterial {
            base_color: flag_color,
            perceptual_roughness: 0.5,
            emissive: color_to_linear_rgba(flag_color) * 0.4,
            ..default()
        });

        // ---- SIGN at start gate ----
        let sign_x  = HC_START_X - 3.0;
        let sign_y  = 2.2;
        let sign_z  = tz;

        // Sign body
        commands.spawn((
            MapProp,
            Mesh3d(sign_body_mesh.clone()),
            MeshMaterial3d(sign_mat.clone()),
            Transform::from_xyz(sign_x, sign_y, sign_z),
            RigidBody::Static,
            Collider::cuboid(1.2, 0.45, 0.05),
        ));
        // Left post
        commands.spawn((
            MapProp,
            Mesh3d(sign_post_mesh.clone()),
            MeshMaterial3d(wood_mat.clone()),
            Transform::from_xyz(sign_x - 0.9, sign_y - 0.9, sign_z),
            RigidBody::Static,
            Collider::cylinder(0.06, 0.8),
        ));
        // Right post
        commands.spawn((
            MapProp,
            Mesh3d(sign_post_mesh.clone()),
            MeshMaterial3d(wood_mat.clone()),
            Transform::from_xyz(sign_x + 0.9, sign_y - 0.9, sign_z),
            RigidBody::Static,
            Collider::cylinder(0.06, 0.8),
        ));
        prop_count += 3;

        // ---- TIRE STACKS — left and right of start gate ----
        for &side in &[-1.0_f32, 1.0_f32] {
            let base_x = HC_START_X - 1.5;
            let base_z = tz + side * 7.5;
            // Row of 2 on ground, 1 on top.
            let offsets: [(f32, f32, f32); 3] = [
                (-0.38, 0.35, 0.0),
                ( 0.38, 0.35, 0.0),
                ( 0.0,  1.05, 0.0),
            ];
            for (dx, dy, dz) in offsets {
                commands.spawn((
                    MapProp,
                    Mesh3d(tire_mesh.clone()),
                    MeshMaterial3d(rubber_mat.clone()),
                    Transform::from_xyz(base_x + dx, dy, base_z + dz),
                    RigidBody::Static,
                    Collider::sphere(0.35),
                ));
                prop_count += 1;
            }
        }

        // ---- SUMMIT FLAG ----
        let sum_x = HC_SUMMIT_X;
        let sum_y_base = 0.5; // lifted slightly above nominal terrain
        // Pole
        commands.spawn((
            MapProp,
            Mesh3d(flag_pole_mesh.clone()),
            MeshMaterial3d(metal_mat(&mut materials)),
            Transform::from_xyz(sum_x, sum_y_base + 1.75, tz),
            RigidBody::Static,
            Collider::cylinder(0.05, 1.75),
        ));
        // Flag body (at top of pole)
        commands.spawn((
            MapProp,
            Mesh3d(flag_body_mesh.clone()),
            MeshMaterial3d(flag_mat.clone()),
            Transform::from_xyz(sum_x + 0.5, sum_y_base + 3.7, tz),
            RigidBody::Static,
            Collider::cuboid(0.4, 0.225, 0.03),
        ));
        prop_count += 2;
    }

    // ---- DIRT BARRIERS between adjacent tiers ----
    // Between tier 0 (Z=-180) and tier 1 (Z=-210): midpoint Z=-195
    // Between tier 1 (Z=-210) and tier 2 (Z=-240): midpoint Z=-225
    for &mid_z in &[-195.0_f32, -225.0_f32] {
        let barrier_cx = HC_START_X + 64.0; // roughly mid-slope
        commands.spawn((
            MapProp,
            Mesh3d(barrier_mesh.clone()),
            MeshMaterial3d(dirt_mat.clone()),
            Transform::from_xyz(barrier_cx, 1.0, mid_z),
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
    let rock_mat = materials.add(StandardMaterial {
        base_color: GRANITE_COLOR,
        perceptual_roughness: 0.97,
        ..default()
    });
    let wood_mat = materials.add(StandardMaterial {
        base_color: WOOD_COLOR,
        perceptual_roughness: 0.90,
        ..default()
    });
    let sign_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.82, 0.72, 0.52),
        perceptual_roughness: 0.80,
        emissive: LinearRgba::rgb(0.15, 0.10, 0.04),
        ..default()
    });
    let sign_post_mat = materials.add(StandardMaterial {
        base_color: WOOD_COLOR,
        perceptual_roughness: 0.92,
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
        let scy = 0.0_f32; // we ignore terrain_height_at to keep the plugin simple

        // Cairn: 3 spheres bottom row + 1 on top.
        let cairn_offsets: [(f32, f32, f32); 4] = [
            (-0.28, 0.22, -0.18),
            ( 0.28, 0.22,  0.15),
            ( 0.0,  0.22,  0.28),
            ( 0.0,  0.60,  0.05),
        ];
        for (dx, dy, dz) in cairn_offsets {
            commands.spawn((
                MapProp,
                Mesh3d(cairn_sphere_mesh.clone()),
                MeshMaterial3d(rock_mat.clone()),
                Transform::from_xyz(start_x + dx, scy + dy, cz + dz + 4.0),
                RigidBody::Static,
                Collider::sphere(0.22),
            ));
            prop_count += 1;
        }

        // Trail sign.
        let sign_x = start_x - 1.5;
        let sign_y = 2.0_f32;
        // Body
        commands.spawn((
            MapProp,
            Mesh3d(sign_body_mesh.clone()),
            MeshMaterial3d(sign_mat.clone()),
            Transform::from_xyz(sign_x, scy + sign_y, cz),
            RigidBody::Static,
            Collider::cuboid(1.1, 0.35, 0.05),
        ));
        // Left post
        commands.spawn((
            MapProp,
            Mesh3d(sign_post_mesh.clone()),
            MeshMaterial3d(sign_post_mat.clone()),
            Transform::from_xyz(sign_x - 0.85, scy + sign_y - 0.8, cz),
            RigidBody::Static,
            Collider::cylinder(0.055, 0.7),
        ));
        // Right post
        commands.spawn((
            MapProp,
            Mesh3d(sign_post_mesh.clone()),
            MeshMaterial3d(sign_post_mat.clone()),
            Transform::from_xyz(sign_x + 0.85, scy + sign_y - 0.8, cz),
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
        let by             = 0.10_f32;

        // 4 planks spanning the approach.
        for i in 0..4usize {
            let px = bridge_base_x + (i as f32) * 3.8;
            commands.spawn((
                MapProp,
                Mesh3d(plank_mesh.clone()),
                MeshMaterial3d(wood_mat.clone()),
                Transform::from_xyz(px, by + 0.55, bz),
                RigidBody::Static,
                Collider::cuboid(1.8, 0.06, 0.275),
            ));
            prop_count += 1;
        }

        // 4 short post legs under the planks.
        let leg_positions: [(f32, f32); 4] = [
            (bridge_base_x,          bz - 1.2),
            (bridge_base_x,          bz + 1.2),
            (bridge_base_x + 11.4,   bz - 1.2),
            (bridge_base_x + 11.4,   bz + 1.2),
        ];
        for (lx, lz) in leg_positions {
            commands.spawn((
                MapProp,
                Mesh3d(leg_mesh.clone()),
                MeshMaterial3d(wood_mat.clone()),
                Transform::from_xyz(lx, by + 0.28, lz),
                RigidBody::Static,
                Collider::cylinder(0.07, 0.28),
            ));
            prop_count += 1;
        }
    }

    // ---- LOOSE BOULDERS between sections for visual continuity ----
    // Deterministic positions (no rand — constant array).
    let boulder_data: [(f32, f32, f32); 7] = [
        ( 50.0, 0.35,  35.0),
        (-20.0, 0.42,  50.0),
        ( 80.0, 0.28, -30.0),
        (-50.0, 0.50,  -5.0),
        ( 30.0, 0.38,  65.0),
        (-10.0, 0.30, -80.0),
        ( 10.0, 0.45,  20.0),
    ];
    let boulder_radii: [f32; 7] = [0.55, 0.70, 0.42, 0.65, 0.50, 0.60, 0.38];

    for (i, &(bx, by, bz)) in boulder_data.iter().enumerate() {
        let r = boulder_radii[i];
        let sphere_mesh = meshes.add(Sphere::new(r));
        commands.spawn((
            MapProp,
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
//   • Tire wall (8 spheres per side × 2 sides = 16 spheres/level)
//   • Finish-line tower (thin post + platform + checkered flag = 3 entities)
//   • Countdown post at the start (1 cylinder)
//   • Course-name sign at the start (body + 2 posts = 3 entities)
//
// Total: 3 × (16 + 3 + 1 + 3) = 3 × 23 = 69 entities, all with colliders.

const OC_START_X: f32 = -60.0;
const OC_COURSE_Z: [f32; 3] = [200.0, 230.0, 260.0];
// Finish X (level 0: -60 + 10*8 + 12 = 32; approx used here)
const OC_FINISH_X: [f32; 3] = [32.0, 50.0, 60.0];
const OC_NAMES: [&str; 3] = ["OBSTACLE BEGINNER", "OBSTACLE INTERMEDIATE", "OBSTACLE EXPERT"];

fn spawn_obstacle_course_dressing(
    mut commands:  Commands,
    mut meshes:    ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let rubber_mat = materials.add(StandardMaterial {
        base_color: RUBBER_COLOR,
        perceptual_roughness: 0.98,
        ..default()
    });
    let countdown_mat = materials.add(StandardMaterial {
        base_color: COUNTDOWN_RED,
        perceptual_roughness: 0.55,
        emissive: LinearRgba::rgb(0.30, 0.02, 0.02),
        ..default()
    });
    let sign_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.80, 0.70, 0.50),
        perceptual_roughness: 0.78,
        emissive: LinearRgba::rgb(0.12, 0.08, 0.02),
        ..default()
    });
    let sign_post_mat = materials.add(StandardMaterial {
        base_color: WOOD_COLOR,
        perceptual_roughness: 0.92,
        ..default()
    });
    let tower_mat = materials.add(StandardMaterial {
        base_color: METAL_COLOR,
        perceptual_roughness: 0.65,
        ..default()
    });
    let check_white_mat = materials.add(StandardMaterial {
        base_color: CHECK_WHITE,
        perceptual_roughness: 0.5,
        ..default()
    });
    let check_dark_mat = materials.add(StandardMaterial {
        base_color: CHECK_DARK,
        perceptual_roughness: 0.5,
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
        // 8 spheres per side, spaced every 12 m from start to roughly mid-course.
        for &side in &[-1.0_f32, 1.0_f32] {
            let wall_z = cz + side * 9.5; // ±9.5 m from course centre
            for i in 0..8usize {
                let tx = OC_START_X + (i as f32) * 11.0 + 6.0;
                commands.spawn((
                    MapProp,
                    Mesh3d(tire_mesh.clone()),
                    MeshMaterial3d(rubber_mat.clone()),
                    // Half-buried: y = 0.2 (radius 0.4, half below ground level).
                    Transform::from_xyz(tx, 0.20, wall_z),
                    RigidBody::Static,
                    Collider::sphere(0.40),
                ));
                prop_count += 1;
            }
        }

        // ---- FINISH-LINE TOWER ----
        let tower_x = finish_x + 2.0;
        // Tall post
        commands.spawn((
            MapProp,
            Mesh3d(tower_post_mesh.clone()),
            MeshMaterial3d(tower_mat.clone()),
            Transform::from_xyz(tower_x, 2.5, cz),
            RigidBody::Static,
            Collider::cuboid(0.09, 2.5, 0.09),
        ));
        // Platform on top
        commands.spawn((
            MapProp,
            Mesh3d(platform_mesh.clone()),
            MeshMaterial3d(tower_mat.clone()),
            Transform::from_xyz(tower_x, 5.075, cz),
            RigidBody::Static,
            Collider::cuboid(1.2, 0.075, 1.2),
        ));
        // Checkered flag (white tile on top of platform)
        commands.spawn((
            MapProp,
            Mesh3d(flag_check_mesh.clone()),
            MeshMaterial3d(if level % 2 == 0 { check_white_mat.clone() } else { check_dark_mat.clone() }),
            Transform::from_xyz(tower_x, 5.45, cz),
            RigidBody::Static,
            Collider::cuboid(0.60, 0.30, 0.03),
        ));
        prop_count += 3;

        // ---- COUNTDOWN POST at start ----
        commands.spawn((
            MapProp,
            Mesh3d(countdown_mesh.clone()),
            MeshMaterial3d(countdown_mat.clone()),
            Transform::from_xyz(OC_START_X - 4.0, 1.0, cz),
            RigidBody::Static,
            Collider::cylinder(0.12, 1.0),
        ));
        prop_count += 1;

        // ---- COURSE-NAME SIGN at start ----
        let sign_x = OC_START_X - 6.0;
        let sign_y = 2.2_f32;
        // Body
        commands.spawn((
            MapProp,
            Mesh3d(sign_body_mesh.clone()),
            MeshMaterial3d(sign_mat.clone()),
            Transform::from_xyz(sign_x, sign_y, cz + 7.0),
            RigidBody::Static,
            Collider::cuboid(1.3, 0.375, 0.05),
        ));
        // Left post
        commands.spawn((
            MapProp,
            Mesh3d(sign_post_mesh_oc.clone()),
            MeshMaterial3d(sign_post_mat.clone()),
            Transform::from_xyz(sign_x - 0.95, sign_y - 0.85, cz + 7.0),
            RigidBody::Static,
            Collider::cylinder(0.06, 0.75),
        ));
        // Right post
        commands.spawn((
            MapProp,
            Mesh3d(sign_post_mesh_oc.clone()),
            MeshMaterial3d(sign_post_mat.clone()),
            Transform::from_xyz(sign_x + 0.95, sign_y - 0.85, cz + 7.0),
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
// Area 4 — Trail Rides  (stub — filled in next commit)
// ---------------------------------------------------------------------------

fn spawn_trail_ride_dressing(
    _commands:  Commands,
    _meshes:    ResMut<Assets<Mesh>>,
    _materials: ResMut<Assets<StandardMaterial>>,
) {
}

// ---------------------------------------------------------------------------
// Area 5 — Default Spawn Area  (stub — filled in next commit)
// ---------------------------------------------------------------------------

fn spawn_spawn_area_dressing(
    _commands:  Commands,
    _meshes:    ResMut<Assets<Mesh>>,
    _materials: ResMut<Assets<StandardMaterial>>,
) {
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Quick emissive colour helper — converts a Bevy `Color` to LinearRgba.
fn color_to_linear_rgba(c: Color) -> LinearRgba {
    let lc = c.to_linear();
    LinearRgba::new(lc.red, lc.green, lc.blue, lc.alpha)
}

/// Create a one-off metal StandardMaterial (avoids borrowing issues in loops).
fn metal_mat(materials: &mut Assets<StandardMaterial>) -> Handle<StandardMaterial> {
    materials.add(StandardMaterial {
        base_color: METAL_COLOR,
        perceptual_roughness: 0.65,
        metallic: 0.55,
        ..default()
    })
}
