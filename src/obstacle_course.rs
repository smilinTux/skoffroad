// Sprint 64 — Obstacle Course mode
//
// Three procedurally-placed obstacle courses north of spawn (Z = +200 / +230 / +260):
//
//   Level 0 — Beginner      Z = +200  (8 obstacles, ~10 m spacing, 15° ramps)
//   Level 1 — Intermediate  Z = +230  (12 obstacles, ~7 m spacing, 25° ramps)
//   Level 2 — Expert        Z = +260  (16 obstacles, ~5 m spacing, 35° ramps)
//
// Each course runs along +X from a start gate to a finish gate.
// The obstacle sequence is deterministically seeded per level (LCG) so every
// player gets the same layout — fair for leaderboards.
//
// Obstacle types:
//   Log    — horizontal cylinder, brown, perpendicular to travel (+Z axis)
//   Boulder — sphere + non-uniform scale, grey/granite
//   Ramp   — cuboid tilted at the difficulty angle
//   Gate   — two upright posts + top bar (visual only; proximity detected)
//
// Timer logic mirrors hillclimb_tiers.rs:
//   Start when chassis crosses within START_RADIUS of the start gate
//   Stop  when chassis crosses within FINISH_RADIUS of the finish gate
//   Abort if chassis flips (up.y < –0.3) or 5-minute timeout
//   Personal best persisted to platform_storage["obstacle_course.json"]
//
// HUD (top-center, right of hillclimb HUD):
//   "OBSTACLE EXPERT  |  12.4s  |  6/16"
//   On finish: "FINISH  03:12.7  (best 02:58.3)" for ~5 s
//
// Public API:
//   ObstacleCoursePlugin
//   ObstacleCourseState  (Resource)

use bevy::prelude::*;
use avian3d::prelude::*;

use crate::platform_storage;
use crate::terrain::terrain_height_at;
use crate::vehicle::{Chassis, VehicleRoot};
use crate::notifications::NotificationQueue;

// ---------------------------------------------------------------------------
// Level constants
// ---------------------------------------------------------------------------

pub const NUM_LEVELS: usize = 3;
pub const LEVEL_NAMES: [&str; NUM_LEVELS] = ["Beginner", "Intermediate", "Expert"];

/// Z centre-line for each course (north of spawn).
pub const COURSE_Z: [f32; NUM_LEVELS] = [200.0, 230.0, 260.0];

/// X position of the start gate for each course.
const START_X: f32 = -60.0;

/// Number of obstacles per level.
const OBSTACLE_COUNT: [usize; NUM_LEVELS] = [8, 12, 16];

/// Spacing between obstacles (approx. metres along X).
const OBSTACLE_SPACING: [f32; NUM_LEVELS] = [10.0, 7.0, 5.0];

/// Log radius (cylinder half-height in Y → becomes radius after rotation).
const LOG_RADIUS: [f32; NUM_LEVELS] = [0.20, 0.30, 0.40];

/// Ramp pitch (degrees) per level.
const RAMP_PITCH_DEG: [f32; NUM_LEVELS] = [15.0, 25.0, 35.0];

/// Gate geometry (shared).
const GATE_POST_W: f32 = 0.4;
const GATE_POST_H: f32 = 4.5;
const GATE_BEAM_H: f32 = 0.4;
const GATE_WIDTH:  f32 = 12.0; // visual channel width
const GATE_POST_HALF: f32 = 5.5; // ± Z offset for posts

/// Course half-width (for finish-proximity check).
const COURSE_HALF_W: f32 = 8.0;

/// Proximity radii for gate detection.
const START_RADIUS: f32 = 5.0;
const FINISH_RADIUS: f32 = 6.0;

/// Gate-approach: only trigger start when chassis is moving into the course (dx ≥ 0).
const MIN_DX_FOR_START: f32 = -0.5;

/// Attempt timeout.
const ATTEMPT_TIMEOUT_S: f32 = 300.0;

/// How long the finish banner stays on screen (seconds).
const FINISH_BANNER_DURATION: f32 = 5.0;


// ---------------------------------------------------------------------------
// LCG — deterministic per-level RNG (same helper style as rock_crawl_trail.rs)
// ---------------------------------------------------------------------------

struct Lcg(u64);

impl Lcg {
    fn new(seed: u32) -> Self { Self(seed as u64) }

    fn next_f32(&mut self) -> f32 {
        self.0 = self.0
            .wrapping_mul(1_664_525)
            .wrapping_add(1_013_904_223)
            & 0xFFFF_FFFF;
        (self.0 as f32) / (u32::MAX as f32)
    }

    fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + self.next_f32() * (hi - lo)
    }

    fn signed(&mut self, half: f32) -> f32 { self.range(-half, half) }

    /// Pick a value in [0, n) as usize.
    fn pick(&mut self, n: usize) -> usize {
        (self.next_f32() * n as f32) as usize % n
    }
}

// ---------------------------------------------------------------------------
// Obstacle type enum (used both for placement and for cleared-tracking)
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq)]
enum ObstacleKind {
    Log,
    Boulder,
    Ramp,
    Gate,
}

// ---------------------------------------------------------------------------
// Resource: ObstacleCourseState
// ---------------------------------------------------------------------------

#[derive(Clone, Default)]
pub struct LevelRecord {
    pub best_s: Option<f32>,
    pub last_s: Option<f32>,
}

#[derive(Clone)]
pub struct ActiveAttempt {
    pub level:            usize,
    pub elapsed_s:        f32,
    /// Number of obstacles already cleared (chassis passed their X position).
    pub cleared:          usize,
    /// World-X positions of all spawned obstacles for this level.
    pub obstacle_x:       Vec<f32>,
    /// Which obstacle X positions have already been passed.
    pub obs_cleared:      Vec<bool>,
}

/// Finish-banner state (shown for FINISH_BANNER_DURATION seconds after finish).
#[derive(Clone)]
pub struct FinishBanner {
    pub level:     usize,
    pub elapsed_s: f32,
    pub best_s:    Option<f32>,
    pub remaining: f32,
}

#[derive(Resource)]
pub struct ObstacleCourseState {
    pub records:       [LevelRecord; NUM_LEVELS],
    pub active:        Option<ActiveAttempt>,
    pub finish_banner: Option<FinishBanner>,
    needs_save:        bool,
}

impl Default for ObstacleCourseState {
    fn default() -> Self {
        Self {
            records:       Default::default(),
            active:        None,
            finish_banner: None,
            needs_save:    false,
        }
    }
}

impl ObstacleCourseState {
    pub fn load() -> Self {
        let mut s = Self::default();
        if let Some(text) = platform_storage::read_string("obstacle_course.json") {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
                if let Some(arr) = v.get("levels").and_then(|t| t.as_array()) {
                    for (i, entry) in arr.iter().enumerate() {
                        if i >= NUM_LEVELS { break; }
                        s.records[i].best_s = entry.get("best_s")
                            .and_then(|x| x.as_f64())
                            .map(|x| x as f32);
                        s.records[i].last_s = entry.get("last_s")
                            .and_then(|x| x.as_f64())
                            .map(|x| x as f32);
                    }
                }
            }
        }
        s
    }

    fn save(&self) {
        let levels: Vec<serde_json::Value> = self.records.iter().map(|r| {
            serde_json::json!({ "best_s": r.best_s, "last_s": r.last_s })
        }).collect();
        let json = serde_json::json!({ "levels": levels }).to_string();
        match platform_storage::write_string("obstacle_course.json", &json) {
            Ok(()) => info!("obstacle_course: saved"),
            Err(e) => warn!("obstacle_course: save failed: {}", e),
        }
    }
}

// ---------------------------------------------------------------------------
// Layout resource (start / finish gate positions, gate-obstacle positions)
// ---------------------------------------------------------------------------

#[derive(Resource, Default)]
pub struct ObstacleCourseLayout {
    pub start_pos:  [Vec3; NUM_LEVELS],
    pub finish_pos: [Vec3; NUM_LEVELS],
    /// World-X positions of all obstacles per level (for "cleared" tracking).
    pub obstacle_x: [Vec<f32>; NUM_LEVELS],
}

// ---------------------------------------------------------------------------
// Component markers
// ---------------------------------------------------------------------------

#[derive(Component)]
#[allow(dead_code)]
struct OcStartGate { level: usize }

#[derive(Component)]
#[allow(dead_code)]
struct OcFinishGate { level: usize }

// ---------------------------------------------------------------------------
// HUD component markers
// ---------------------------------------------------------------------------

#[derive(Component)]
struct OcHudRoot;

#[derive(Component)]
struct OcHudLine1;

#[derive(Component)]
struct OcHudLine2;

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct ObstacleCoursePlugin;

impl Plugin for ObstacleCoursePlugin {
    fn build(&self, app: &mut App) {
        app
            .insert_resource(ObstacleCourseState::load())
            .insert_resource(ObstacleCourseLayout::default())
            .add_systems(
                Startup,
                (init_layout, spawn_courses, spawn_hud).chain(),
            )
            .add_systems(
                Update,
                (
                    tick_timer,
                    check_start,
                    check_finish,
                    update_gate_clears,
                    update_hud,
                )
                    .chain()
                    .run_if(resource_exists::<VehicleRoot>),
            );
    }
}

// ---------------------------------------------------------------------------
// Startup: init_layout — compute start / finish positions per level
// ---------------------------------------------------------------------------

fn init_layout(mut layout: ResMut<ObstacleCourseLayout>) {
    for level in 0..NUM_LEVELS {
        let cz = COURSE_Z[level];
        let ty = terrain_height_at(START_X, cz) + 1.0;
        layout.start_pos[level] = Vec3::new(START_X, ty, cz);

        // Finish X: start + spacing × obstacle_count + a few metres of margin
        let finish_x = START_X
            + OBSTACLE_SPACING[level] * OBSTACLE_COUNT[level] as f32
            + 12.0;
        let finish_y = terrain_height_at(finish_x, cz) + 1.0;
        layout.finish_pos[level] = Vec3::new(finish_x, finish_y, cz);

        info!(
            "obstacle_course: level {} ({}) start=({:.0},{:.0},{:.0}) finish=({:.0},{:.0},{:.0})",
            level, LEVEL_NAMES[level],
            layout.start_pos[level].x, layout.start_pos[level].y, layout.start_pos[level].z,
            layout.finish_pos[level].x, layout.finish_pos[level].y, layout.finish_pos[level].z,
        );
    }
}

// ---------------------------------------------------------------------------
// Startup: spawn_courses — gates + obstacles for all levels
// ---------------------------------------------------------------------------

fn spawn_courses(
    mut oc_layout: ResMut<ObstacleCourseLayout>,
    mut commands:  Commands,
    mut meshes:    ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Gate colours (start = yellow, finish = green — same as hillclimb)
    let start_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.85, 0.0),
        perceptual_roughness: 0.5,
        emissive: LinearRgba::rgb(0.3, 0.2, 0.0),
        ..default()
    });
    let finish_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.1, 0.85, 0.2),
        perceptual_roughness: 0.5,
        emissive: LinearRgba::rgb(0.0, 0.25, 0.05),
        ..default()
    });

    // Level accent colours for gate beams.
    let level_colors = [
        Color::srgb(0.30, 0.80, 0.30), // Beginner: green
        Color::srgb(0.90, 0.65, 0.15), // Intermediate: amber
        Color::srgb(0.90, 0.30, 0.20), // Expert: red
    ];

    // Shared post mesh (reused across all gates).
    let post_mesh = meshes.add(Cuboid::new(GATE_POST_W, GATE_POST_H, GATE_POST_W));
    let beam_mesh = meshes.add(Cuboid::new(GATE_WIDTH, GATE_BEAM_H, GATE_BEAM_H));

    for level in 0..NUM_LEVELS {
        let sp = oc_layout.start_pos[level];
        let fp = oc_layout.finish_pos[level];
        let cz = COURSE_Z[level];

        // ---- Start gate ----
        spawn_gate(
            &mut commands,
            sp,
            level,
            true,
            &post_mesh,
            &beam_mesh,
            &start_mat,
        );

        // ---- Finish gate ----
        spawn_gate(
            &mut commands,
            fp,
            level,
            false,
            &post_mesh,
            &beam_mesh,
            &finish_mat,
        );

        // ---- Obstacle placement (LCG seeded per level) ----
        // Seed chosen to be unique per level: 6400, 6401, 6402
        let mut lcg = Lcg::new(6400 + level as u32);

        // Available obstacle kinds per level (Expert has all four).
        let kinds_available: &[ObstacleKind] = match level {
            0 => &[ObstacleKind::Log, ObstacleKind::Ramp],
            1 => &[ObstacleKind::Log, ObstacleKind::Boulder, ObstacleKind::Ramp, ObstacleKind::Gate],
            _ => &[ObstacleKind::Log, ObstacleKind::Boulder, ObstacleKind::Ramp, ObstacleKind::Gate],
        };

        let count = OBSTACLE_COUNT[level];
        let spacing = OBSTACLE_SPACING[level];

        // Obstacle materials.
        let log_mat = materials.add(StandardMaterial {
            base_color: Color::srgb(0.55, 0.35, 0.20),
            perceptual_roughness: 0.9,
            ..default()
        });
        let boulder_mat = materials.add(StandardMaterial {
            base_color: Color::srgb(0.42, 0.40, 0.38),
            perceptual_roughness: 0.95,
            ..default()
        });
        let ramp_mat = materials.add(StandardMaterial {
            base_color: Color::srgb(0.38, 0.34, 0.30),
            perceptual_roughness: 0.92,
            ..default()
        });
        let gate_obs_mat = materials.add(StandardMaterial {
            base_color: level_colors[level],
            perceptual_roughness: 0.55,
            emissive: LinearRgba::rgb(0.05, 0.05, 0.05),
            ..default()
        });

        let mut obstacle_x: Vec<f32> = Vec::with_capacity(count);

        for i in 0..count {
            let ox = sp.x + (i as f32 + 1.0) * spacing;
            // Small lateral jitter (±2 m) so obstacles aren't perfectly inline.
            let oz = cz + lcg.signed(2.0);
            let terrain_y = terrain_height_at(ox, oz);

            let kind = kinds_available[lcg.pick(kinds_available.len())];

            match kind {
                ObstacleKind::Log => {
                    spawn_log(
                        &mut commands,
                        &mut meshes,
                        &log_mat,
                        Vec3::new(ox, terrain_y + LOG_RADIUS[level] + 0.05, oz),
                        LOG_RADIUS[level],
                        level,
                        &mut lcg,
                    );
                }
                ObstacleKind::Boulder => {
                    spawn_boulder(
                        &mut commands,
                        &mut meshes,
                        &boulder_mat,
                        Vec3::new(ox, terrain_y, oz),
                        level,
                        &mut lcg,
                    );
                }
                ObstacleKind::Ramp => {
                    spawn_ramp(
                        &mut commands,
                        &mut meshes,
                        &ramp_mat,
                        Vec3::new(ox, terrain_y, oz),
                        level,
                    );
                }
                ObstacleKind::Gate => {
                    spawn_obstacle_gate(
                        &mut commands,
                        &mut meshes,
                        &gate_obs_mat,
                        Vec3::new(ox, terrain_y, cz),
                    );
                }
            }
            // Record obstacle X for cleared-tracking (all types).
            obstacle_x.push(ox);
        }

        // Store all obstacle X positions per level for runtime cleared-tracking.
        oc_layout.obstacle_x[level] = obstacle_x;
    }

    info!("obstacle_course: spawned {} courses", NUM_LEVELS);
}

// ---------------------------------------------------------------------------
// Helpers: individual obstacle spawners
// ---------------------------------------------------------------------------

fn spawn_gate(
    commands:  &mut Commands,
    pos:       Vec3,
    level:     usize,
    is_start:  bool,
    post_mesh: &Handle<Mesh>,
    beam_mesh: &Handle<Mesh>,
    mat:       &Handle<StandardMaterial>,
) {
    let root = if is_start {
        commands.spawn((
            OcStartGate { level },
            Transform::from_translation(pos),
            Visibility::default(),
        )).id()
    } else {
        commands.spawn((
            OcFinishGate { level },
            Transform::from_translation(pos),
            Visibility::default(),
        )).id()
    };

    let post_hh = GATE_POST_H * 0.5;

    let lp = commands.spawn((
        Mesh3d(post_mesh.clone()),
        MeshMaterial3d(mat.clone()),
        Transform::from_xyz(0.0, post_hh, -GATE_POST_HALF),
        RigidBody::Static,
        Collider::cuboid(GATE_POST_W * 0.5, post_hh, GATE_POST_W * 0.5),
    )).id();

    let rp = commands.spawn((
        Mesh3d(post_mesh.clone()),
        MeshMaterial3d(mat.clone()),
        Transform::from_xyz(0.0, post_hh, GATE_POST_HALF),
        RigidBody::Static,
        Collider::cuboid(GATE_POST_W * 0.5, post_hh, GATE_POST_W * 0.5),
    )).id();

    let bm = commands.spawn((
        Mesh3d(beam_mesh.clone()),
        MeshMaterial3d(mat.clone()),
        Transform::from_xyz(0.0, GATE_POST_H + GATE_BEAM_H * 0.5, 0.0),
        // Beam is visual only — no collider so the chassis passes through.
    )).id();

    commands.entity(root).add_children(&[lp, rp, bm]);
}

fn spawn_log(
    commands:  &mut Commands,
    meshes:    &mut Assets<Mesh>,
    mat:       &Handle<StandardMaterial>,
    pos:       Vec3,
    radius:    f32,
    _level:    usize,
    lcg:       &mut Lcg,
) {
    // Log length varies slightly per level.
    let length = lcg.range(7.0, 11.0);

    // Cylinder in Bevy: height along Y. Rotate 90° around X so it runs along Z
    // (perpendicular to the course's +X direction of travel).
    let log_mesh = meshes.add(Cylinder::new(radius, length));
    let rotation = Quat::from_rotation_x(std::f32::consts::FRAC_PI_2);

    commands.spawn((
        Mesh3d(log_mesh),
        MeshMaterial3d(mat.clone()),
        Transform { translation: pos, rotation, scale: Vec3::ONE },
        RigidBody::Static,
        Collider::cylinder(radius, length * 0.5),
    ));
}

fn spawn_boulder(
    commands:  &mut Commands,
    meshes:    &mut Assets<Mesh>,
    mat:       &Handle<StandardMaterial>,
    base_pos:  Vec3,
    level:     usize,
    lcg:       &mut Lcg,
) {
    // Base radius grows with difficulty.
    let base_r = match level {
        0 => lcg.range(0.5, 0.9),
        1 => lcg.range(0.7, 1.2),
        _ => lcg.range(0.9, 1.5),
    };

    // Non-uniform scale for an irregular look.
    let sx = lcg.range(0.8, 1.3);
    let sy = lcg.range(0.7, 1.1);
    let sz = lcg.range(0.8, 1.2);

    let sphere_mesh = meshes.add(Sphere::new(base_r));
    let centre = Vec3::new(
        base_pos.x,
        base_pos.y + base_r * sy,
        base_pos.z,
    );
    let yaw = lcg.range(0.0, std::f32::consts::TAU);

    commands.spawn((
        Mesh3d(sphere_mesh),
        MeshMaterial3d(mat.clone()),
        Transform {
            translation: centre,
            rotation:    Quat::from_rotation_y(yaw),
            scale:       Vec3::new(sx, sy, sz),
        },
        RigidBody::Static,
        // Collider uses the unscaled base sphere; avian3d handles transform scale.
        Collider::sphere(base_r),
    ));
}

fn spawn_ramp(
    commands: &mut Commands,
    meshes:   &mut Assets<Mesh>,
    mat:      &Handle<StandardMaterial>,
    base_pos: Vec3,
    level:    usize,
) {
    let pitch_rad = RAMP_PITCH_DEG[level].to_radians();
    let ramp_len   = 5.0_f32;
    let ramp_w     = 8.0_f32;
    let ramp_thick = 0.3_f32;

    // Rotation: pitch around Z axis so the ramp slopes up along +X.
    let rotation = Quat::from_rotation_z(-pitch_rad);

    let run  = ramp_len * pitch_rad.cos();
    let rise = ramp_len * pitch_rad.sin();
    let centre = Vec3::new(
        base_pos.x + run * 0.5,
        base_pos.y + rise * 0.5 + ramp_thick * 0.5,
        base_pos.z,
    );

    let ramp_mesh = meshes.add(Cuboid::new(ramp_len, ramp_thick, ramp_w));

    commands.spawn((
        Mesh3d(ramp_mesh),
        MeshMaterial3d(mat.clone()),
        Transform { translation: centre, rotation, scale: Vec3::ONE },
        RigidBody::Static,
        Collider::cuboid(ramp_len * 0.5, ramp_thick * 0.5, ramp_w * 0.5),
    ));
}

fn spawn_obstacle_gate(
    commands: &mut Commands,
    meshes:   &mut Assets<Mesh>,
    mat:      &Handle<StandardMaterial>,
    pos:      Vec3,
) {
    // Narrow gate (visual only — chassis must pass between posts).
    const NARROW_POST_H: f32 = 4.0;
    const NARROW_POST_HALF: f32 = 3.5; // ± Z from centre
    const NARROW_BEAM_W: f32 = 7.5;

    let post_mesh = meshes.add(Cuboid::new(GATE_POST_W, NARROW_POST_H, GATE_POST_W));
    let beam_mesh = meshes.add(Cuboid::new(NARROW_BEAM_W, GATE_BEAM_H, GATE_BEAM_H));

    let root = commands.spawn((
        Transform::from_translation(pos),
        Visibility::default(),
    )).id();

    let post_hh = NARROW_POST_H * 0.5;

    let lp = commands.spawn((
        Mesh3d(post_mesh.clone()),
        MeshMaterial3d(mat.clone()),
        Transform::from_xyz(0.0, post_hh, -NARROW_POST_HALF),
        RigidBody::Static,
        Collider::cuboid(GATE_POST_W * 0.5, post_hh, GATE_POST_W * 0.5),
    )).id();

    let rp = commands.spawn((
        Mesh3d(post_mesh.clone()),
        MeshMaterial3d(mat.clone()),
        Transform::from_xyz(0.0, post_hh, NARROW_POST_HALF),
        RigidBody::Static,
        Collider::cuboid(GATE_POST_W * 0.5, post_hh, GATE_POST_W * 0.5),
    )).id();

    let bm = commands.spawn((
        Mesh3d(beam_mesh),
        MeshMaterial3d(mat.clone()),
        Transform::from_xyz(0.0, NARROW_POST_H + GATE_BEAM_H * 0.5, 0.0),
        // No collider on the beam — visual crossbar only.
    )).id();

    commands.entity(root).add_children(&[lp, rp, bm]);
}

// ---------------------------------------------------------------------------
// Startup: spawn HUD
// ---------------------------------------------------------------------------

fn spawn_hud(mut commands: Commands) {
    let bg = Color::srgba(0.03, 0.05, 0.10, 0.90);

    let root = commands.spawn((
        OcHudRoot,
        Node {
            position_type:   PositionType::Absolute,
            top:             Val::Px(8.0),
            // Placed at 50% + 440 px (right of hillclimb tier HUD at 150+270=420, +20 gap)
            left:            Val::Percent(50.0),
            margin:          UiRect { left: Val::Px(440.0), ..default() },
            width:           Val::Px(300.0),
            min_height:      Val::Px(52.0),
            flex_direction:  FlexDirection::Column,
            align_items:     AlignItems::Center,
            padding:         UiRect::axes(Val::Px(10.0), Val::Px(6.0)),
            row_gap:         Val::Px(2.0),
            ..default()
        },
        BackgroundColor(bg),
        ZIndex(43),
        Visibility::Hidden,
    )).id();

    let line1 = commands.spawn((
        OcHudLine1,
        Text::new("OBSTACLE COURSE"),
        TextFont { font_size: 13.0, ..default() },
        TextColor(Color::srgb(1.0, 0.65, 0.15)),
    )).id();

    let line2 = commands.spawn((
        OcHudLine2,
        Text::new("--"),
        TextFont { font_size: 18.0, ..default() },
        TextColor(Color::WHITE),
    )).id();

    commands.entity(root).add_children(&[line1, line2]);
}

// ---------------------------------------------------------------------------
// System: tick_timer
// ---------------------------------------------------------------------------

fn tick_timer(
    time:      Res<Time>,
    mut state: ResMut<ObstacleCourseState>,
) {
    let dt = time.delta_secs();

    if let Some(ref mut attempt) = state.active {
        attempt.elapsed_s += dt;
        if attempt.elapsed_s >= ATTEMPT_TIMEOUT_S {
            warn!("obstacle_course: level {} timed out", attempt.level);
            state.active = None;
        }
    }

    if let Some(ref mut banner) = state.finish_banner {
        banner.remaining -= dt;
        if banner.remaining <= 0.0 {
            state.finish_banner = None;
        }
    }

    if state.needs_save {
        state.save();
        state.needs_save = false;
    }
}

// ---------------------------------------------------------------------------
// System: check_start
// ---------------------------------------------------------------------------

fn check_start(
    vehicle:   Res<VehicleRoot>,
    chassis_q: Query<&Transform, With<Chassis>>,
    layout:    Res<ObstacleCourseLayout>,
    mut state: ResMut<ObstacleCourseState>,
    mut notifs: ResMut<NotificationQueue>,
) {
    if state.active.is_some() { return; }

    let Ok(tf) = chassis_q.get(vehicle.chassis) else { return };
    let pos = tf.translation;

    for level in 0..NUM_LEVELS {
        let sp = layout.start_pos[level];
        let dx = pos.x - sp.x;
        let dz = pos.z - sp.z;
        let xz = (dx * dx + dz * dz).sqrt();

        if xz <= START_RADIUS && dx >= MIN_DX_FOR_START {
            let obs_x = layout.obstacle_x[level].clone();
            let obs_len = obs_x.len();
            state.active = Some(ActiveAttempt {
                level,
                elapsed_s:   0.0,
                cleared:     0,
                obstacle_x:  obs_x,
                obs_cleared: vec![false; obs_len],
            });
            notifs.push(
                format!("{} OBSTACLE COURSE STARTED", LEVEL_NAMES[level]),
                Color::srgb(1.0, 0.65, 0.15),
            );
            info!(
                "obstacle_course: started level {} at ({:.1},{:.1},{:.1})",
                level, pos.x, pos.y, pos.z
            );
            break;
        }
    }
}

// ---------------------------------------------------------------------------
// System: check_finish
// ---------------------------------------------------------------------------

fn check_finish(
    vehicle:   Res<VehicleRoot>,
    chassis_q: Query<&Transform, With<Chassis>>,
    layout:    Res<ObstacleCourseLayout>,
    mut state: ResMut<ObstacleCourseState>,
    mut notifs: ResMut<NotificationQueue>,
) {
    let Ok(tf) = chassis_q.get(vehicle.chassis) else { return };

    // Flip detection (abort if upside down).
    let up = tf.rotation * Vec3::Y;
    if state.active.is_some() && up.y < -0.3 {
        let level = state.active.as_ref().map(|a| a.level).unwrap_or(0);
        notifs.push("OBSTACLE COURSE FAILED — flipped", Color::srgb(1.0, 0.25, 0.1));
        info!("obstacle_course: level {} aborted — flipped", level);
        state.active = None;
        return;
    }

    let Some(attempt) = state.active.clone() else { return };
    let pos = tf.translation;
    let fp = layout.finish_pos[attempt.level];

    // Check lateral corridor: must be roughly aligned with the course Z.
    let dz = (pos.z - COURSE_Z[attempt.level]).abs();
    if dz > COURSE_HALF_W { return; }

    let dx = pos.x - fp.x;
    let xz_dist = (dx * dx + (pos.z - fp.z) * (pos.z - fp.z)).sqrt();

    if xz_dist <= FINISH_RADIUS {
        let elapsed = attempt.elapsed_s;
        let level   = attempt.level;
        state.active = None;

        state.records[level].last_s = Some(elapsed);
        let is_best = state.records[level].best_s
            .map(|b| elapsed < b)
            .unwrap_or(true);
        if is_best {
            state.records[level].best_s = Some(elapsed);
        }
        state.needs_save = true;

        let best_s = state.records[level].best_s;

        // Notify.
        if is_best {
            notifs.push(
                format!("{} OBSTACLE — NEW BEST! {}", LEVEL_NAMES[level], fmt_time(elapsed)),
                Color::srgb(0.1, 1.0, 0.4),
            );
        } else {
            let best_str = best_s.map(fmt_time).unwrap_or_else(|| "--".to_string());
            notifs.push(
                format!(
                    "{} OBSTACLE — {} (Best: {})",
                    LEVEL_NAMES[level], fmt_time(elapsed), best_str
                ),
                Color::srgb(0.4, 0.85, 1.0),
            );
        }

        // Show finish banner.
        state.finish_banner = Some(FinishBanner {
            level,
            elapsed_s: elapsed,
            best_s,
            remaining: FINISH_BANNER_DURATION,
        });

        info!(
            "obstacle_course: level {} FINISHED in {:.2}s (best: {:?})",
            level, elapsed, best_s
        );
    }
}

// ---------------------------------------------------------------------------
// System: update_gate_clears
// Counts obstacles as "cleared" when the chassis passes 2 m past each obstacle.
// ---------------------------------------------------------------------------

fn update_gate_clears(
    vehicle:   Res<VehicleRoot>,
    chassis_q: Query<&Transform, With<Chassis>>,
    mut state: ResMut<ObstacleCourseState>,
) {
    let Ok(tf) = chassis_q.get(vehicle.chassis) else { return };
    let chassis_x = tf.translation.x;

    let Some(ref mut attempt) = state.active else { return };

    for (i, &obs_x) in attempt.obstacle_x.iter().enumerate() {
        if attempt.obs_cleared[i] { continue; }
        // Mark cleared when chassis has passed 2 m beyond the obstacle's centre.
        if chassis_x > obs_x + 2.0 {
            attempt.obs_cleared[i] = true;
            attempt.cleared += 1;
        }
    }
}

// ---------------------------------------------------------------------------
// System: update_hud
// ---------------------------------------------------------------------------

fn update_hud(
    state:    Res<ObstacleCourseState>,
    mut root_q: Query<&mut Visibility, With<OcHudRoot>>,
    mut line1_q: Query<&mut Text, (With<OcHudLine1>, Without<OcHudLine2>)>,
    mut line2_q: Query<&mut Text, (With<OcHudLine2>, Without<OcHudLine1>)>,
) {
    let Ok(mut vis) = root_q.single_mut() else { return };

    if let Some(ref banner) = state.finish_banner {
        *vis = Visibility::Visible;
        if let Ok(mut t) = line1_q.single_mut() {
            t.0 = format!("OBSTACLE {}  FINISH", LEVEL_NAMES[banner.level]);
        }
        if let Ok(mut t) = line2_q.single_mut() {
            let best_str = banner.best_s
                .map(fmt_time)
                .unwrap_or_else(|| "--".to_string());
            t.0 = format!(
                "{}  (best {})",
                fmt_time(banner.elapsed_s),
                best_str
            );
        }
        return;
    }

    if let Some(ref attempt) = state.active {
        *vis = Visibility::Visible;
        let total = OBSTACLE_COUNT[attempt.level];
        if let Ok(mut t) = line1_q.single_mut() {
            t.0 = format!(
                "OBSTACLE {}  |  {}/{}",
                LEVEL_NAMES[attempt.level].to_uppercase(),
                attempt.cleared,
                total
            );
        }
        if let Ok(mut t) = line2_q.single_mut() {
            t.0 = fmt_time(attempt.elapsed_s);
        }
    } else {
        *vis = Visibility::Hidden;
    }
}

// ---------------------------------------------------------------------------
// Time formatting (MM:SS.cc)
// ---------------------------------------------------------------------------

pub fn fmt_time(s: f32) -> String {
    let s    = s.max(0.0);
    let mins = (s / 60.0) as u32;
    let rem  = s - (mins as f32) * 60.0;
    let sec  = rem as u32;
    let cs   = ((rem % 1.0) * 100.0) as u32;
    format!("{:02}:{:02}.{:02}", mins, sec, cs)
}
