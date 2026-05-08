// Procedural buildings — 8 rural structures (4 small shacks + 4 barns) scattered
// around the 200 x 200 m terrain at deterministic LCG positions.
//
// Each building is assembled from plain cuboid primitives (no GLTF).  A single
// bounding-box collider on the parent entity lets vehicles bump into buildings
// without the cost of per-piece collision shapes.
//
// Public API:
//   BuildingsPlugin

use bevy::prelude::*;
use avian3d::prelude::*;

use crate::terrain::terrain_height_at;

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct BuildingsPlugin;

impl Plugin for BuildingsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_buildings);
    }
}

// ---------------------------------------------------------------------------
// Marker component
// ---------------------------------------------------------------------------

/// Marks the root entity of every spawned building.
#[derive(Component)]
pub struct Building;

// ---------------------------------------------------------------------------
// World / placement constants
// ---------------------------------------------------------------------------

const WORLD_HALF: f32 = 100.0; // ± half of the 200 m square
/// Minimum distance from the origin (player spawn) before placing a building.
const ORIGIN_CLEAR: f32 = 25.0;
/// Finite-difference step used to estimate local terrain slope.
const SLOPE_STEP: f32 = 5.0;
/// Max height difference over SLOPE_STEP before a site is rejected as too steep.
const MAX_HEIGHT_DELTA: f32 = 1.5;

// ---------------------------------------------------------------------------
// Building geometry constants
// ---------------------------------------------------------------------------

// Shack (5 × 3.5 × 5 m footprint)
const SHACK_W: f32 = 5.0;
const SHACK_H: f32 = 3.5;
const SHACK_D: f32 = 5.0;
const SHACK_WALL_T: f32 = 0.25; // wall thickness
// Roof angled cuboid dimensions
const SHACK_ROOF_W: f32 = 4.0; // width of each roof panel
const SHACK_ROOF_H: f32 = 0.25; // thickness of each roof panel (thin slab)
const SHACK_ROOF_D: f32 = 5.0; // depth matches shack depth
const SHACK_ROOF_PITCH_DEG: f32 = 40.0; // angle each panel tilts from horizontal
// Door opening on front face
const SHACK_DOOR_W: f32 = 1.0;
const SHACK_DOOR_H: f32 = 1.8;
const SHACK_DOOR_D: f32 = SHACK_WALL_T + 0.02; // slightly proud of wall

// Barn (8 × 5 × 12 m footprint)
const BARN_W: f32 = 8.0;
const BARN_H: f32 = 5.0;
const BARN_D: f32 = 12.0;
const BARN_WALL_T: f32 = 0.3;
// Gambrel roof — two pairs of angled panels (lower steep, upper shallow)
const BARN_ROOF_LOWER_W: f32 = 3.2; // width of each lower panel
const BARN_ROOF_LOWER_H: f32 = 0.25;
const BARN_ROOF_LOWER_D: f32 = 12.0;
const BARN_ROOF_LOWER_PITCH_DEG: f32 = 60.0;
const BARN_ROOF_UPPER_W: f32 = 2.4; // width of each upper panel
const BARN_ROOF_UPPER_H: f32 = 0.25;
const BARN_ROOF_UPPER_D: f32 = 12.0;
const BARN_ROOF_UPPER_PITCH_DEG: f32 = 20.0;
// Big barn door
const BARN_DOOR_W: f32 = 3.0;
const BARN_DOOR_H: f32 = 4.0;
const BARN_DOOR_D: f32 = BARN_WALL_T + 0.02;

// ---------------------------------------------------------------------------
// LCG — same parameters as obstacles.rs and ramps.rs for consistency
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
// Terrain-steepness guard
// ---------------------------------------------------------------------------

/// Returns true if the terrain at (x, z) is too steep to place a building.
fn terrain_too_steep(x: f32, z: f32) -> bool {
    let h0 = terrain_height_at(x, z);
    let hx = terrain_height_at(x + SLOPE_STEP, z);
    let hz = terrain_height_at(x, z + SLOPE_STEP);
    (hx - h0).abs() >= MAX_HEIGHT_DELTA || (hz - h0).abs() >= MAX_HEIGHT_DELTA
}

// ---------------------------------------------------------------------------
// Spawn system
// ---------------------------------------------------------------------------

fn spawn_buildings(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // ---------- shared materials ----------

    // Shack
    let shack_wall_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.55, 0.40, 0.25),
        perceptual_roughness: 0.92,
        ..default()
    });
    let shack_roof_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.35, 0.25, 0.20),
        perceptual_roughness: 0.88,
        ..default()
    });
    let shack_door_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.25, 0.18, 0.12),
        perceptual_roughness: 0.85,
        ..default()
    });

    // Barn
    let barn_wall_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.65, 0.18, 0.12),
        perceptual_roughness: 0.90,
        ..default()
    });
    let barn_roof_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.30, 0.20, 0.15),
        perceptual_roughness: 0.88,
        ..default()
    });
    let barn_door_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.20, 0.12, 0.08),
        perceptual_roughness: 0.85,
        ..default()
    });

    // ---------- shared meshes ----------

    // Shack walls (four slabs)
    let shack_front_back_mesh = meshes.add(Cuboid::new(SHACK_W, SHACK_H, SHACK_WALL_T));
    let shack_side_mesh = meshes.add(Cuboid::new(SHACK_WALL_T, SHACK_H, SHACK_D));
    let shack_roof_panel_mesh = meshes.add(Cuboid::new(SHACK_ROOF_W, SHACK_ROOF_H, SHACK_ROOF_D));
    let shack_door_mesh = meshes.add(Cuboid::new(SHACK_DOOR_W, SHACK_DOOR_H, SHACK_DOOR_D));

    // Barn walls
    let barn_front_back_mesh = meshes.add(Cuboid::new(BARN_W, BARN_H, BARN_WALL_T));
    let barn_side_mesh = meshes.add(Cuboid::new(BARN_WALL_T, BARN_H, BARN_D));
    let barn_roof_lower_mesh =
        meshes.add(Cuboid::new(BARN_ROOF_LOWER_W, BARN_ROOF_LOWER_H, BARN_ROOF_LOWER_D));
    let barn_roof_upper_mesh =
        meshes.add(Cuboid::new(BARN_ROOF_UPPER_W, BARN_ROOF_UPPER_H, BARN_ROOF_UPPER_D));
    let barn_door_mesh = meshes.add(Cuboid::new(BARN_DOOR_W, BARN_DOOR_H, BARN_DOOR_D));

    // ---------- LCG-driven placement ----------

    let mut lcg = Lcg::new(99);

    let mut placed = 0u32;
    let mut shacks = 0u32;
    let mut barns = 0u32;
    const TARGET_SHACKS: u32 = 4;
    const TARGET_BARNS: u32 = 4;

    // Iterate candidate positions until we have placed all 8 buildings.
    // Cap at 10_000 attempts to prevent an infinite loop if the terrain is
    // pathologically steep everywhere (should never happen in practice).
    let mut attempts = 0u32;
    while placed < 8 && attempts < 10_000 {
        attempts += 1;

        // Draw a position inside the ±100 m square.
        let x = (lcg.next_f32() * 2.0 - 1.0) * WORLD_HALF;
        let z = (lcg.next_f32() * 2.0 - 1.0) * WORLD_HALF;
        // Draw rotation (0, 90, 180, or 270 degrees).
        let rot_idx = (lcg.next_f32() * 4.0) as u32 % 4;

        // Skip if too close to the origin.
        if x * x + z * z < ORIGIN_CLEAR * ORIGIN_CLEAR {
            continue;
        }

        // Skip on steep terrain.
        if terrain_too_steep(x, z) {
            continue;
        }

        let yaw = rot_idx as f32 * std::f32::consts::FRAC_PI_2;
        let ground_y = terrain_height_at(x, z);

        // Alternate shacks and barns until each quota is filled.
        let place_shack = shacks < TARGET_SHACKS && (barns >= TARGET_BARNS || placed % 2 == 0);

        if place_shack {
            spawn_shack(
                &mut commands,
                &shack_front_back_mesh,
                &shack_side_mesh,
                &shack_roof_panel_mesh,
                &shack_door_mesh,
                &shack_wall_mat,
                &shack_roof_mat,
                &shack_door_mat,
                Vec3::new(x, ground_y, z),
                yaw,
            );
            shacks += 1;
        } else if barns < TARGET_BARNS {
            spawn_barn(
                &mut commands,
                &barn_front_back_mesh,
                &barn_side_mesh,
                &barn_roof_lower_mesh,
                &barn_roof_upper_mesh,
                &barn_door_mesh,
                &barn_wall_mat,
                &barn_roof_mat,
                &barn_door_mat,
                Vec3::new(x, ground_y, z),
                yaw,
            );
            barns += 1;
        } else {
            // Both quotas filled — break.
            break;
        }

        placed += 1;
    }

    bevy::log::info!(
        "buildings: {} shacks + {} barns placed ({} attempts)",
        shacks,
        barns,
        attempts,
    );
}

// ---------------------------------------------------------------------------
// Shack constructor
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
fn spawn_shack(
    commands: &mut Commands,
    front_back_mesh: &Handle<Mesh>,
    side_mesh: &Handle<Mesh>,
    roof_panel_mesh: &Handle<Mesh>,
    door_mesh: &Handle<Mesh>,
    wall_mat: &Handle<StandardMaterial>,
    roof_mat: &Handle<StandardMaterial>,
    door_mat: &Handle<StandardMaterial>,
    origin: Vec3,
    yaw: f32,
) {
    // The parent sits so that ground level == Y=0 in local space.
    // The floor of the shack is at Y=0; walls extend up to SHACK_H.
    // Bounding collider: half-extents cover walls + roof.
    let half_w = SHACK_W * 0.5;
    let half_d = SHACK_D * 0.5;
    // Roof apex is approximately at SHACK_H + SHACK_ROOF_W * sin(pitch) * 0.5.
    let roof_rise = (SHACK_ROOF_PITCH_DEG.to_radians().sin() * SHACK_ROOF_W * 0.5)
        + SHACK_ROOF_H * 0.5;
    let total_h = SHACK_H + roof_rise;
    let half_h = total_h * 0.5;

    let parent = commands
        .spawn((
            Building,
            Transform::from_translation(origin).with_rotation(Quat::from_rotation_y(yaw)),
            Visibility::default(),
            RigidBody::Static,
            Collider::cuboid(half_w, half_h, half_d),
        ))
        .id();

    // --- walls ---
    // Front wall (−Z face): centred at local (0, half_h_wall, −half_d)
    let wall_half_h = SHACK_H * 0.5;

    let front_wall = commands
        .spawn((
            Mesh3d(front_back_mesh.clone()),
            MeshMaterial3d(wall_mat.clone()),
            Transform::from_translation(Vec3::new(0.0, wall_half_h, -half_d)),
        ))
        .id();

    // Back wall (+Z face)
    let back_wall = commands
        .spawn((
            Mesh3d(front_back_mesh.clone()),
            MeshMaterial3d(wall_mat.clone()),
            Transform::from_translation(Vec3::new(0.0, wall_half_h, half_d)),
        ))
        .id();

    // Left wall (−X face)
    let left_wall = commands
        .spawn((
            Mesh3d(side_mesh.clone()),
            MeshMaterial3d(wall_mat.clone()),
            Transform::from_translation(Vec3::new(-half_w, wall_half_h, 0.0)),
        ))
        .id();

    // Right wall (+X face)
    let right_wall = commands
        .spawn((
            Mesh3d(side_mesh.clone()),
            MeshMaterial3d(wall_mat.clone()),
            Transform::from_translation(Vec3::new(half_w, wall_half_h, 0.0)),
        ))
        .id();

    // --- gable roof — two angled panels meeting at ridge ---
    // Each panel is a thin cuboid tilted around its outer long edge.
    // Pitch angle: SHACK_ROOF_PITCH_DEG from horizontal.
    let pitch = SHACK_ROOF_PITCH_DEG.to_radians();

    // Left panel tilts rightward (positive X roll) from the ridge down to the left eave.
    // Panel pivot is the outer eave edge at x = −half_w, y = SHACK_H.
    // We rotate around Z by −pitch so the panel slopes down to the left.
    // Panel centre in local (pre-tilt) space is at (−half_w/2 − SHACK_ROOF_W/4, ...).
    // After tilt the panel centre lands at:
    //   cx = −half_w/2 * cos(pitch)  (moves inward)
    //   cy = SHACK_H + half_w/2 * sin(pitch)  (rises)
    // For simplicity we position by rotating around the ridge apex.
    let panel_cx = (SHACK_ROOF_W * 0.5 - SHACK_ROOF_H * 0.5) * 0.5;
    let panel_cy_offset = (SHACK_ROOF_W * 0.5) * pitch.sin();

    // Left roof panel: tilts from ridge (x=0) down to left eave (x = −SHACK_ROOF_W/2 * cos(pitch))
    let left_roof = commands
        .spawn((
            Mesh3d(roof_panel_mesh.clone()),
            MeshMaterial3d(roof_mat.clone()),
            Transform {
                translation: Vec3::new(
                    -(panel_cx * pitch.cos()),
                    SHACK_H + panel_cy_offset - SHACK_ROOF_H * 0.5 * pitch.cos(),
                    0.0,
                ),
                rotation: Quat::from_rotation_z(-pitch),
                scale: Vec3::ONE,
            },
        ))
        .id();

    // Right roof panel: mirror of left
    let right_roof = commands
        .spawn((
            Mesh3d(roof_panel_mesh.clone()),
            MeshMaterial3d(roof_mat.clone()),
            Transform {
                translation: Vec3::new(
                    panel_cx * pitch.cos(),
                    SHACK_H + panel_cy_offset - SHACK_ROOF_H * 0.5 * pitch.cos(),
                    0.0,
                ),
                rotation: Quat::from_rotation_z(pitch),
                scale: Vec3::ONE,
            },
        ))
        .id();

    // --- door: dark cuboid centred on the front wall ---
    // Door sits flush with the outer face of the front wall.
    let door = commands
        .spawn((
            Mesh3d(door_mesh.clone()),
            MeshMaterial3d(door_mat.clone()),
            Transform::from_translation(Vec3::new(
                0.0,
                SHACK_DOOR_H * 0.5,
                -half_d - SHACK_DOOR_D * 0.5 + SHACK_WALL_T * 0.5,
            )),
        ))
        .id();

    commands.entity(parent).add_children(&[
        front_wall, back_wall, left_wall, right_wall, left_roof, right_roof, door,
    ]);
}

// ---------------------------------------------------------------------------
// Barn constructor
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
fn spawn_barn(
    commands: &mut Commands,
    front_back_mesh: &Handle<Mesh>,
    side_mesh: &Handle<Mesh>,
    roof_lower_mesh: &Handle<Mesh>,
    roof_upper_mesh: &Handle<Mesh>,
    door_mesh: &Handle<Mesh>,
    wall_mat: &Handle<StandardMaterial>,
    roof_mat: &Handle<StandardMaterial>,
    door_mat: &Handle<StandardMaterial>,
    origin: Vec3,
    yaw: f32,
) {
    let half_w = BARN_W * 0.5;
    let half_d = BARN_D * 0.5;

    // Gambrel roof: lower panels at 60°, upper panels at 20°.
    // Lower panel: starts at eave (x = ±half_w), width BARN_ROOF_LOWER_W.
    // Upper panel: starts where lower ends, width BARN_ROOF_UPPER_W.
    let lower_pitch = BARN_ROOF_LOWER_PITCH_DEG.to_radians();
    let upper_pitch = BARN_ROOF_UPPER_PITCH_DEG.to_radians();

    let lower_rise = BARN_ROOF_LOWER_W * lower_pitch.sin();
    let lower_run  = BARN_ROOF_LOWER_W * lower_pitch.cos();
    let upper_rise = BARN_ROOF_UPPER_W * upper_pitch.sin();

    // Total height from wall top to roof ridge.
    let roof_total_rise = lower_rise + upper_rise;
    let total_h = BARN_H + roof_total_rise;
    let half_h = total_h * 0.5;

    let parent = commands
        .spawn((
            Building,
            Transform::from_translation(origin).with_rotation(Quat::from_rotation_y(yaw)),
            Visibility::default(),
            RigidBody::Static,
            Collider::cuboid(half_w, half_h, half_d),
        ))
        .id();

    // --- walls ---
    let wall_half_h = BARN_H * 0.5;

    let front_wall = commands
        .spawn((
            Mesh3d(front_back_mesh.clone()),
            MeshMaterial3d(wall_mat.clone()),
            Transform::from_translation(Vec3::new(0.0, wall_half_h, -half_d)),
        ))
        .id();

    let back_wall = commands
        .spawn((
            Mesh3d(front_back_mesh.clone()),
            MeshMaterial3d(wall_mat.clone()),
            Transform::from_translation(Vec3::new(0.0, wall_half_h, half_d)),
        ))
        .id();

    let left_wall = commands
        .spawn((
            Mesh3d(side_mesh.clone()),
            MeshMaterial3d(wall_mat.clone()),
            Transform::from_translation(Vec3::new(-half_w, wall_half_h, 0.0)),
        ))
        .id();

    let right_wall = commands
        .spawn((
            Mesh3d(side_mesh.clone()),
            MeshMaterial3d(wall_mat.clone()),
            Transform::from_translation(Vec3::new(half_w, wall_half_h, 0.0)),
        ))
        .id();

    // --- gambrel roof ---
    // Lower-left panel: from x = −half_w tilting inward at lower_pitch.
    // Panel centre x = −half_w + lower_run/2, y offset = lower_rise/2
    let lower_left = commands
        .spawn((
            Mesh3d(roof_lower_mesh.clone()),
            MeshMaterial3d(roof_mat.clone()),
            Transform {
                translation: Vec3::new(
                    -(half_w - lower_run * 0.5),
                    BARN_H + lower_rise * 0.5,
                    0.0,
                ),
                rotation: Quat::from_rotation_z(-lower_pitch),
                scale: Vec3::ONE,
            },
        ))
        .id();

    // Lower-right panel: mirror
    let lower_right = commands
        .spawn((
            Mesh3d(roof_lower_mesh.clone()),
            MeshMaterial3d(roof_mat.clone()),
            Transform {
                translation: Vec3::new(
                    half_w - lower_run * 0.5,
                    BARN_H + lower_rise * 0.5,
                    0.0,
                ),
                rotation: Quat::from_rotation_z(lower_pitch),
                scale: Vec3::ONE,
            },
        ))
        .id();

    // Upper-left panel: starts where lower ends (x = −half_w + lower_run, y = BARN_H + lower_rise)
    // tilting inward at upper_pitch.
    let upper_base_x = half_w - lower_run;
    let upper_base_y = BARN_H + lower_rise;
    let upper_run    = BARN_ROOF_UPPER_W * upper_pitch.cos();

    let upper_left = commands
        .spawn((
            Mesh3d(roof_upper_mesh.clone()),
            MeshMaterial3d(roof_mat.clone()),
            Transform {
                translation: Vec3::new(
                    -(upper_base_x - upper_run * 0.5),
                    upper_base_y + upper_rise * 0.5,
                    0.0,
                ),
                rotation: Quat::from_rotation_z(-upper_pitch),
                scale: Vec3::ONE,
            },
        ))
        .id();

    // Upper-right panel: mirror
    let upper_right = commands
        .spawn((
            Mesh3d(roof_upper_mesh.clone()),
            MeshMaterial3d(roof_mat.clone()),
            Transform {
                translation: Vec3::new(
                    upper_base_x - upper_run * 0.5,
                    upper_base_y + upper_rise * 0.5,
                    0.0,
                ),
                rotation: Quat::from_rotation_z(upper_pitch),
                scale: Vec3::ONE,
            },
        ))
        .id();

    // --- big barn door on the front face ---
    let barn_door = commands
        .spawn((
            Mesh3d(door_mesh.clone()),
            MeshMaterial3d(door_mat.clone()),
            Transform::from_translation(Vec3::new(
                0.0,
                BARN_DOOR_H * 0.5,
                -half_d - BARN_DOOR_D * 0.5 + BARN_WALL_T * 0.5,
            )),
        ))
        .id();

    commands.entity(parent).add_children(&[
        front_wall,
        back_wall,
        left_wall,
        right_wall,
        lower_left,
        lower_right,
        upper_left,
        upper_right,
        barn_door,
    ]);
}
