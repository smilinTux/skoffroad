// Sprint 54 — Rock Crawl Trail mode
//
// Three pre-built technical sections placed deterministically on the terrain:
//
//   1. "Boulder Stairs"   at world (120, _, 0)   — 8 stair-step boulders
//   2. "Two-Log Bridge"   at world (-80, _, 80)  — parallel log cylinders over a ravine
//   3. "Off-Camber Traverse" at world (60, _, -120) — slanted ramp chain
//
// Each section has a start gate and a finish gate (coloured posts + crossbeam).
// A per-section timer starts when the local chassis crosses the start gate and
// stops at the finish gate.  Best time + last time are persisted via
// `crate::platform_storage` to "rock_crawl.json".
//
// Multiplayer auto-spectate: when a remote peer's ghost chassis enters any
// section's start–finish corridor, SpectateState.auto_target is set so the
// local player auto-spectates them (unless they are themselves mid-attempt or
// manually spectating).
//
// HUD: small top-left overlay shown during an active attempt.
//
// Public API:
//   RockCrawlTrailPlugin
//   RockCrawlTrailState (Resource)

use bevy::prelude::*;
use avian3d::prelude::*;

use crate::multiplayer::{GhostMarker, PeerId};
use crate::platform_storage;
use crate::spectate::SpectateState;
use crate::terrain::terrain_height_at;
use crate::vehicle::{Chassis, VehicleRoot};

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct RockCrawlTrailPlugin;

impl Plugin for RockCrawlTrailPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(RockCrawlTrailState::load())
            .add_systems(Startup, (spawn_sections, spawn_hud).chain())
            .add_systems(
                Update,
                (
                    tick_section_timers,
                    check_gate_crossings,
                    update_peer_spectate,
                    update_hud,
                )
                    .chain()
                    .run_if(resource_exists::<VehicleRoot>),
            );
    }
}

// ---------------------------------------------------------------------------
// Section definitions — all positions deterministic (no clock seed)
// ---------------------------------------------------------------------------

/// Number of trail sections.
const NUM_SECTIONS: usize = 3;

/// Names displayed in the HUD.
const SECTION_NAMES: [&str; NUM_SECTIONS] = [
    "Boulder Stairs",
    "Two-Log Bridge",
    "Off-Camber Traverse",
];

/// Approximate XZ centre of each section (used for gate placement & spectate
/// radius check).  Y is resolved at runtime via terrain_height_at.
const SECTION_CX: [f32; NUM_SECTIONS] = [120.0, -80.0, 60.0];
const SECTION_CZ: [f32; NUM_SECTIONS] = [0.0, 80.0, -120.0];

/// The "corridor" half-length along the section's main axis (metres).
/// A chassis within ±CORRIDOR_HALF of the section centre (on the longitudinal
/// axis) and within ±SECTION_WIDTH_HALF laterally is "in-section".
const CORRIDOR_HALF: [f32; NUM_SECTIONS] = [18.0, 10.0, 22.0];
const SECTION_WIDTH_HALF: f32 = 12.0;

/// Timeout in seconds: if a peer starts an attempt and doesn't finish, we
/// clear the auto-spectate target after this many seconds.
const ATTEMPT_TIMEOUT_S: f32 = 60.0;

// ---------------------------------------------------------------------------
// Gate geometry
// ---------------------------------------------------------------------------

const GATE_POST_W: f32 = 0.4;
const GATE_POST_H: f32 = 4.0;
const GATE_POST_D: f32 = 0.4;
const GATE_BEAM_W: f32 = 10.0; // spans the section channel
const GATE_BEAM_H: f32 = 0.4;
const GATE_BEAM_D: f32 = 0.4;

/// Half-width used for gate post placement (±5 m from centre-line).
const GATE_POST_HALF: f32 = 5.0;

const GATE_START_COLOR: Color = Color::srgb(1.0, 0.85, 0.0);  // yellow
const GATE_FINISH_COLOR: Color = Color::srgb(0.1, 0.85, 0.2); // green

// ---------------------------------------------------------------------------
// LCG helper (same style as rock_garden.rs — deterministic, no clock seed)
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
}

// ---------------------------------------------------------------------------
// Public resource
// ---------------------------------------------------------------------------

/// Per-section persistent record: best time and last time in seconds.
#[derive(Clone, Default)]
pub struct SectionRecord {
    pub best_s: Option<f32>,
    pub last_s: Option<f32>,
}

/// Active attempt (local chassis).
#[derive(Clone)]
pub struct ActiveAttempt {
    pub section_idx: usize,
    pub elapsed_s:   f32,
}

/// Peer attempt tracking for auto-spectate.
#[allow(dead_code)] // section_idx + elapsed_s reserved for spectate / leaderboard UI
struct PeerAttempt {
    peer_id:     PeerId,
    section_idx: usize,
    elapsed_s:   f32,
}

/// Main plugin resource.
#[derive(Resource)]
pub struct RockCrawlTrailState {
    pub records:       [SectionRecord; NUM_SECTIONS],
    pub active:        Option<ActiveAttempt>,
    peer_attempts:     Vec<PeerAttempt>,
    /// Dirty flag: write to storage on the next frame where this is true.
    needs_save:        bool,
}

impl Default for RockCrawlTrailState {
    fn default() -> Self {
        Self {
            records:       Default::default(),
            active:        None,
            peer_attempts: Vec::new(),
            needs_save:    false,
        }
    }
}

impl RockCrawlTrailState {
    /// Load from `platform_storage["rock_crawl.json"]`, falling back to default.
    pub fn load() -> Self {
        let mut s = Self::default();
        if let Some(text) = platform_storage::read_string("rock_crawl.json") {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
                if let Some(arr) = v.as_array() {
                    for (i, entry) in arr.iter().enumerate() {
                        if i >= NUM_SECTIONS { break; }
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

    /// Persist to `platform_storage["rock_crawl.json"]`.
    fn save(&self) {
        let arr: Vec<serde_json::Value> = self.records.iter().map(|r| {
            serde_json::json!({
                "best_s": r.best_s,
                "last_s": r.last_s,
            })
        }).collect();
        let text = serde_json::to_string(&arr).unwrap_or_default();
        let _ = platform_storage::write_string("rock_crawl.json", &text);
    }
}

// ---------------------------------------------------------------------------
// Gate marker components
// ---------------------------------------------------------------------------

/// Marker on a start-gate root entity, carrying its section index.
#[derive(Component)]
#[allow(dead_code)] // section is read by future per-gate trigger systems
struct RcStartGate {
    section: usize,
}

/// Marker on a finish-gate root entity, carrying its section index.
#[derive(Component)]
#[allow(dead_code)]
struct RcFinishGate {
    section: usize,
}

// ---------------------------------------------------------------------------
// HUD component markers
// ---------------------------------------------------------------------------

#[derive(Component)]
struct RcHudRoot;

#[derive(Component)]
struct RcHudSection;

#[derive(Component)]
struct RcHudTimer;

#[derive(Component)]
struct RcHudBest;

// ---------------------------------------------------------------------------
// Persistence key
// ---------------------------------------------------------------------------

#[allow(dead_code)] // wired in once persistence-on-finish lands in a follow-up
const STORAGE_KEY: &str = "rock_crawl.json";

// ---------------------------------------------------------------------------
// Startup: spawn all three sections
// ---------------------------------------------------------------------------

fn spawn_sections(
    mut commands:  Commands,
    mut meshes:    ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Shared gate materials.
    let start_mat = materials.add(StandardMaterial {
        base_color: GATE_START_COLOR,
        perceptual_roughness: 0.5,
        emissive: LinearRgba::rgb(0.3, 0.2, 0.0),
        ..default()
    });
    let finish_mat = materials.add(StandardMaterial {
        base_color: GATE_FINISH_COLOR,
        perceptual_roughness: 0.5,
        emissive: LinearRgba::rgb(0.0, 0.25, 0.05),
        ..default()
    });

    let post_mesh = meshes.add(Cuboid::new(GATE_POST_W, GATE_POST_H, GATE_POST_D));
    let beam_mesh = meshes.add(Cuboid::new(GATE_BEAM_W, GATE_BEAM_H, GATE_BEAM_D));

    // -----------------------------------------------------------------------
    // Section 0 — Boulder Stairs at (120, _, 0)
    // -----------------------------------------------------------------------
    spawn_boulder_stairs(
        &mut commands,
        &mut meshes,
        &mut materials,
    );

    let (s0_start, s0_finish) = section_gate_positions(0);
    spawn_gate(&mut commands, s0_start, 0, true,  &post_mesh, &beam_mesh, &start_mat);
    spawn_gate(&mut commands, s0_finish, 0, false, &post_mesh, &beam_mesh, &finish_mat);

    // -----------------------------------------------------------------------
    // Section 1 — Two-Log Bridge at (-80, _, 80)
    // -----------------------------------------------------------------------
    spawn_log_bridge(&mut commands, &mut meshes, &mut materials);

    let (s1_start, s1_finish) = section_gate_positions(1);
    spawn_gate(&mut commands, s1_start, 1, true,  &post_mesh, &beam_mesh, &start_mat);
    spawn_gate(&mut commands, s1_finish, 1, false, &post_mesh, &beam_mesh, &finish_mat);

    // -----------------------------------------------------------------------
    // Section 2 — Off-Camber Traverse at (60, _, -120)
    // -----------------------------------------------------------------------
    spawn_off_camber(&mut commands, &mut meshes, &mut materials);

    let (s2_start, s2_finish) = section_gate_positions(2);
    spawn_gate(&mut commands, s2_start, 2, true,  &post_mesh, &beam_mesh, &start_mat);
    spawn_gate(&mut commands, s2_finish, 2, false, &post_mesh, &beam_mesh, &finish_mat);

    bevy::log::info!("rock_crawl_trail: spawned 3 sections (Boulder Stairs, Two-Log Bridge, Off-Camber Traverse)");
}

/// Compute world-space start and finish gate positions for a section.
/// Start = section-centre minus CORRIDOR_HALF on the +X axis.
/// Finish = section-centre plus CORRIDOR_HALF on the +X axis.
/// (All three sections run along the +X axis for simplicity.)
fn section_gate_positions(idx: usize) -> (Vec3, Vec3) {
    let cx = SECTION_CX[idx];
    let cz = SECTION_CZ[idx];
    let half = CORRIDOR_HALF[idx];
    let base_y = terrain_height_at(cx, cz);
    let start  = Vec3::new(cx - half, base_y, cz);
    let finish = Vec3::new(cx + half, base_y, cz);
    (start, finish)
}

/// Spawn a gate (two posts + crossbeam) as a parented hierarchy.
fn spawn_gate(
    commands:   &mut Commands,
    pos:        Vec3,
    section:    usize,
    is_start:   bool,
    post_mesh:  &Handle<Mesh>,
    beam_mesh:  &Handle<Mesh>,
    mat:        &Handle<StandardMaterial>,
) {
    let root_id = if is_start {
        commands.spawn((
            RcStartGate { section },
            Transform::from_translation(pos),
            Visibility::default(),
        )).id()
    } else {
        commands.spawn((
            RcFinishGate { section },
            Transform::from_translation(pos),
            Visibility::default(),
        )).id()
    };

    let post_hh = GATE_POST_H * 0.5;

    // We use add_children with an iterator to avoid borrow conflicts.
    let left_post = commands.spawn((
        Mesh3d(post_mesh.clone()),
        MeshMaterial3d(mat.clone()),
        Transform::from_xyz(0.0, post_hh, -GATE_POST_HALF),
        RigidBody::Static,
        Collider::cuboid(GATE_POST_W * 0.5, post_hh, GATE_POST_D * 0.5),
    )).id();

    let right_post = commands.spawn((
        Mesh3d(post_mesh.clone()),
        MeshMaterial3d(mat.clone()),
        Transform::from_xyz(0.0, post_hh, GATE_POST_HALF),
        RigidBody::Static,
        Collider::cuboid(GATE_POST_W * 0.5, post_hh, GATE_POST_D * 0.5),
    )).id();

    let beam = commands.spawn((
        Mesh3d(beam_mesh.clone()),
        MeshMaterial3d(mat.clone()),
        Transform::from_xyz(0.0, GATE_POST_H + GATE_BEAM_H * 0.5, 0.0),
    )).id();

    commands.entity(root_id).add_children(&[left_post, right_post, beam]);
}

// ---------------------------------------------------------------------------
// Section 0: Boulder Stairs
// ---------------------------------------------------------------------------
// 8 stair-step boulders ascending from start to finish within the existing
// rock_garden zone at (120, _, 0).  LCG seed 54 (Sprint 54) — distinct from
// the rock_garden.rs seed (99) so they don't interfere.
// Boulders are Static RigidBodies so they don't move.

fn spawn_boulder_stairs(
    commands:  &mut Commands,
    meshes:    &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    const NUM_STAIRS: usize = 8;
    const START_X: f32 = 102.0; // just inside the start gate (cx - half = 102)
    const STEP_X:  f32 = 4.0;   // horizontal advance per stair
    const STEP_Y:  f32 = 0.6;   // height rise per stair (stair-step effect)
    const STAIR_CZ: f32 = 0.0;

    let rock_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.42, 0.37, 0.32),
        perceptual_roughness: 0.96,
        ..default()
    });

    let mut lcg = Lcg::new(54);

    for i in 0..NUM_STAIRS {
        let bx = START_X + i as f32 * STEP_X;
        let terrain_y = terrain_height_at(bx, STAIR_CZ);
        let by = terrain_y + i as f32 * STEP_Y + 0.5;
        let bz = STAIR_CZ + lcg.signed(1.2); // slight lateral scatter

        // Boulder size: roughly 1.0–1.8 m radius, getting slightly larger.
        let base_r = 0.8 + i as f32 * 0.08 + lcg.range(0.0, 0.3);
        // Slightly elongated box to look like a step slab.
        let half_x = base_r * 1.2;
        let half_y = base_r * 0.5;
        let half_z = base_r * 0.9 + lcg.range(0.0, 0.3);

        let mesh = meshes.add(Cuboid::new(half_x * 2.0, half_y * 2.0, half_z * 2.0));

        // Small random yaw so boulders look natural.
        let yaw = lcg.range(-0.25, 0.25);
        let rotation = Quat::from_rotation_y(yaw);

        commands.spawn((
            Mesh3d(mesh),
            MeshMaterial3d(rock_mat.clone()),
            Transform {
                translation: Vec3::new(bx, by, bz),
                rotation,
                scale: Vec3::ONE,
            },
            RigidBody::Static,
            Collider::cuboid(half_x, half_y, half_z),
        ));
    }

    bevy::log::info!("rock_crawl_trail: spawned {} boulder stairs at (120,_,0)", NUM_STAIRS);
}

// ---------------------------------------------------------------------------
// Section 1: Two-Log Bridge
// ---------------------------------------------------------------------------
// Two parallel cylinders spanning a small ravine at (-80, _, 80).
// Terrain is depressed 3 m beneath the logs (visual ravine only — we don't
// modify the actual terrain heightmap, but we spawn a static thin slab that
// acts as a floor several metres below so the vehicle doesn't fall through
// the world if it slips off).
// Logs: 0.4 m diameter, 12 m long, placed 0.6 m apart (inside edge gap = 0.2 m).

fn spawn_log_bridge(
    commands:  &mut Commands,
    meshes:    &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    const BRIDGE_CX: f32 = -80.0;
    const BRIDGE_CZ: f32 = 80.0;
    const LOG_LENGTH: f32 = 12.0;   // spans a 12 m gap
    const LOG_RADIUS: f32 = 0.20;   // 0.4 m diameter
    const LOG_SEP:    f32 = 0.65;   // centre-to-centre (so gap ≈ 0.25 m)
    const LOG_HEIGHT_ABOVE: f32 = 0.5; // logs float above mean terrain by this much

    let terrain_y = terrain_height_at(BRIDGE_CX, BRIDGE_CZ);
    let log_y     = terrain_y + LOG_HEIGHT_ABOVE;

    let log_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.55, 0.38, 0.22),
        perceptual_roughness: 0.9,
        ..default()
    });

    // Logs run along world X (same as section direction).
    // Cylinder mesh in Bevy: height is along Y; we rotate 90° around Z to make
    // it run along X.
    let log_mesh = meshes.add(Cylinder::new(LOG_RADIUS, LOG_LENGTH));
    let log_rot  = Quat::from_rotation_z(std::f32::consts::FRAC_PI_2);

    for side in [-1.0_f32, 1.0_f32] {
        let lz = BRIDGE_CZ + side * LOG_SEP * 0.5;
        commands.spawn((
            Mesh3d(log_mesh.clone()),
            MeshMaterial3d(log_mat.clone()),
            Transform {
                translation: Vec3::new(BRIDGE_CX, log_y, lz),
                rotation: log_rot,
                scale: Vec3::ONE,
            },
            RigidBody::Static,
            Collider::cylinder(LOG_RADIUS, LOG_LENGTH * 0.5),
        ));
    }

    // Ravine floor — static thin slab 3 m below log surface so a falling
    // vehicle is caught and the player can recover without a world reset.
    let floor_y = terrain_y - 2.5;
    let floor_mesh = meshes.add(Cuboid::new(LOG_LENGTH + 4.0, 0.4, 8.0));
    let floor_mat  = materials.add(StandardMaterial {
        base_color: Color::srgb(0.30, 0.25, 0.20),
        perceptual_roughness: 0.98,
        ..default()
    });
    commands.spawn((
        Mesh3d(floor_mesh),
        MeshMaterial3d(floor_mat),
        Transform::from_xyz(BRIDGE_CX, floor_y, BRIDGE_CZ),
        RigidBody::Static,
        Collider::cuboid((LOG_LENGTH + 4.0) * 0.5, 0.2, 4.0),
    ));

    // Approach ramp slabs (short 2 m ramps on each side bridging ground to log height).
    let ramp_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.40, 0.35, 0.28),
        perceptual_roughness: 0.95,
        ..default()
    });
    let ramp_angle = (LOG_HEIGHT_ABOVE / 2.0_f32).atan();
    for side in [-1.0_f32, 1.0_f32] {
        let ramp_cx = BRIDGE_CX + side * (LOG_LENGTH * 0.5 + 1.0);
        let ramp_y  = terrain_y + LOG_HEIGHT_ABOVE * 0.5;
        let ramp_mesh = meshes.add(Cuboid::new(2.0, 0.3, 4.0));
        let ramp_rot  = Quat::from_rotation_z(side * ramp_angle);
        commands.spawn((
            Mesh3d(ramp_mesh),
            MeshMaterial3d(ramp_mat.clone()),
            Transform {
                translation: Vec3::new(ramp_cx, ramp_y, BRIDGE_CZ),
                rotation: ramp_rot,
                scale: Vec3::ONE,
            },
            RigidBody::Static,
            Collider::cuboid(1.0, 0.15, 2.0),
        ));
    }

    bevy::log::info!("rock_crawl_trail: spawned two-log bridge at ({},{},{})", BRIDGE_CX, log_y, BRIDGE_CZ);
}

// ---------------------------------------------------------------------------
// Section 2: Off-Camber Traverse
// ---------------------------------------------------------------------------
// A chain of 8 cuboid ramp segments rotated 25–30° on their Z axis (bank
// angle), running along +X from the section start. Style mirrors ramps.rs.

fn spawn_off_camber(
    commands:  &mut Commands,
    meshes:    &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    const NUM_SEGS: usize = 8;
    const START_X:  f32   = 38.0; // cx - CORRIDOR_HALF[2] = 60 - 22
    const SEG_LEN:  f32   = 5.5;  // length of each slab (m)
    const SEG_W:    f32   = 8.0;  // width
    const SEG_H:    f32   = 0.35; // thickness
    const OC_CZ:    f32   = -120.0;

    let ramp_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.38, 0.34, 0.30),
        perceptual_roughness: 0.93,
        ..default()
    });

    // Bank angles alternate left/right to force constant steering correction.
    // Deterministic — no random.
    let bank_angles_deg: [f32; NUM_SEGS] = [25.0, -28.0, 27.0, -30.0, 26.0, -27.0, 29.0, -25.0];

    let mut lcg = Lcg::new(542); // distinct seed

    for i in 0..NUM_SEGS {
        let seg_cx = START_X + (i as f32 + 0.5) * SEG_LEN;
        let terrain_y = terrain_height_at(seg_cx, OC_CZ);
        let seg_y = terrain_y + SEG_H * 0.5 + lcg.range(0.0, 0.15);

        let bank = bank_angles_deg[i].to_radians();
        // Z-axis rotation produces the camber (bank).
        let rotation = Quat::from_rotation_z(bank);

        let mesh = meshes.add(Cuboid::new(SEG_LEN, SEG_H, SEG_W));
        commands.spawn((
            Mesh3d(mesh),
            MeshMaterial3d(ramp_mat.clone()),
            Transform {
                translation: Vec3::new(seg_cx, seg_y, OC_CZ),
                rotation,
                scale: Vec3::ONE,
            },
            RigidBody::Static,
            Collider::cuboid(SEG_LEN * 0.5, SEG_H * 0.5, SEG_W * 0.5),
        ));
    }

    bevy::log::info!("rock_crawl_trail: spawned off-camber traverse ({} segments) at (60,_,-120)", NUM_SEGS);
}

// ---------------------------------------------------------------------------
// Startup: spawn HUD (hidden by default, top-left corner)
// ---------------------------------------------------------------------------

fn spawn_hud(mut commands: Commands) {
    let bg = Color::srgba(0.04, 0.06, 0.14, 0.88);

    let root = commands.spawn((
        RcHudRoot,
        Node {
            position_type:   PositionType::Absolute,
            top:             Val::Px(12.0),
            left:            Val::Px(12.0),
            width:           Val::Px(270.0),
            min_height:      Val::Px(62.0),
            flex_direction:  FlexDirection::Column,
            align_items:     AlignItems::FlexStart,
            padding:         UiRect::axes(Val::Px(10.0), Val::Px(6.0)),
            row_gap:         Val::Px(2.0),
            ..default()
        },
        BackgroundColor(bg),
        ZIndex(45),
        Visibility::Hidden,
    )).id();

    let section_lbl = commands.spawn((
        RcHudSection,
        Text::new("Section: —"),
        TextFont { font_size: 12.0, ..default() },
        TextColor(Color::srgb(1.0, 0.82, 0.2)),
    )).id();

    let timer_lbl = commands.spawn((
        RcHudTimer,
        Text::new("00:00.0"),
        TextFont { font_size: 22.0, ..default() },
        TextColor(Color::WHITE),
    )).id();

    let best_lbl = commands.spawn((
        RcHudBest,
        Text::new("Best: --"),
        TextFont { font_size: 12.0, ..default() },
        TextColor(Color::srgb(0.6, 0.8, 0.6)),
    )).id();

    commands.entity(root).add_children(&[section_lbl, timer_lbl, best_lbl]);
}

// ---------------------------------------------------------------------------
// System: tick timers
// ---------------------------------------------------------------------------

fn tick_section_timers(
    time:      Res<Time>,
    mut state: ResMut<RockCrawlTrailState>,
) {
    let dt = time.delta_secs();

    if let Some(ref mut attempt) = state.active {
        attempt.elapsed_s += dt;
    }

    for pa in state.peer_attempts.iter_mut() {
        pa.elapsed_s += dt;
    }

    // Save if flagged dirty.
    if state.needs_save {
        state.save();
        state.needs_save = false;
    }
}

// ---------------------------------------------------------------------------
// Helper: AABB-based in-section check (local axis-aligned corridor)
// ---------------------------------------------------------------------------

/// Returns Some(section_idx) if `pos` is inside that section's corridor.
fn section_containing(pos: Vec3) -> Option<usize> {
    for idx in 0..NUM_SECTIONS {
        let cx = SECTION_CX[idx];
        let cz = SECTION_CZ[idx];
        let half = CORRIDOR_HALF[idx];

        let dx = (pos.x - cx).abs();
        let dz = (pos.z - cz).abs();

        if dx <= half && dz <= SECTION_WIDTH_HALF {
            return Some(idx);
        }
    }
    None
}

/// Returns true if `pos` is past the finish gate of `section_idx` (x > finish_x).
fn past_finish(pos: Vec3, section_idx: usize) -> bool {
    let cx   = SECTION_CX[section_idx];
    let half = CORRIDOR_HALF[section_idx];
    let cz   = SECTION_CZ[section_idx];
    let dz   = (pos.z - cz).abs();
    pos.x >= cx + half && dz <= SECTION_WIDTH_HALF
}

/// Returns true if `pos` is before the start gate of `section_idx` (x < start_x).
fn before_start(pos: Vec3, section_idx: usize) -> bool {
    let cx   = SECTION_CX[section_idx];
    let half = CORRIDOR_HALF[section_idx];
    let cz   = SECTION_CZ[section_idx];
    let dz   = (pos.z - cz).abs();
    pos.x <= cx - half && dz <= SECTION_WIDTH_HALF
}

// ---------------------------------------------------------------------------
// System: gate crossing detection (local player)
// ---------------------------------------------------------------------------

fn check_gate_crossings(
    vehicle:   Res<VehicleRoot>,
    chassis_q: Query<&Transform, With<Chassis>>,
    mut state: ResMut<RockCrawlTrailState>,
) {
    let Ok(tf) = chassis_q.get(vehicle.chassis) else { return };
    let pos = tf.translation;

    match state.active.clone() {
        None => {
            // Not in an attempt — check if we just entered a start gate.
            if let Some(idx) = section_containing(pos) {
                // Entered from the start side (pos.x near start gate edge).
                // We start the timer as soon as the chassis is inside the corridor
                // (crossing the start gate plane means pos.x > start_x for +X sections).
                let cx   = SECTION_CX[idx];
                let half = CORRIDOR_HALF[idx];
                let start_x = cx - half;
                if pos.x >= start_x {
                    state.active = Some(ActiveAttempt { section_idx: idx, elapsed_s: 0.0 });
                    bevy::log::info!(
                        "rock_crawl_trail: started attempt on section {} ({})",
                        idx, SECTION_NAMES[idx]
                    );
                }
            }
        }

        Some(ref attempt) => {
            let idx = attempt.section_idx;
            let elapsed = attempt.elapsed_s;

            // Timeout check.
            if elapsed >= ATTEMPT_TIMEOUT_S {
                bevy::log::info!(
                    "rock_crawl_trail: attempt on section {} timed out ({:.1}s)",
                    idx, elapsed
                );
                state.active = None;
                return;
            }

            // Finish check — chassis has crossed past the finish gate.
            if past_finish(pos, idx) {
                bevy::log::info!(
                    "rock_crawl_trail: FINISHED section {} ({}) in {:.2}s",
                    idx, SECTION_NAMES[idx], elapsed
                );

                // Update record.
                state.records[idx].last_s = Some(elapsed);
                let is_best = state.records[idx].best_s
                    .map(|b| elapsed < b)
                    .unwrap_or(true);
                if is_best {
                    state.records[idx].best_s = Some(elapsed);
                }
                state.needs_save = true;
                state.active = None;
                return;
            }

            // Abort if the player reversed out of the start gate.
            if before_start(pos, idx) {
                bevy::log::info!(
                    "rock_crawl_trail: attempt on section {} aborted (reversed out)",
                    idx
                );
                state.active = None;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// System: peer auto-spectate
// ---------------------------------------------------------------------------

fn update_peer_spectate(
    ghosts:    Query<(&GhostMarker, &Transform)>,
    mut state: ResMut<RockCrawlTrailState>,
    mut spec:  ResMut<SpectateState>,
) {
    // Build new peer_attempts list from ghost positions.
    let mut still_active: Vec<PeerAttempt> = Vec::new();

    for (ghost, tf) in &ghosts {
        let pos = tf.translation;
        let pid = ghost.peer_id;

        // Check if this peer is inside any section corridor.
        if let Some(idx) = section_containing(pos) {
            // Is this peer already tracked?
            if let Some(existing) = state.peer_attempts.iter().find(|p| p.peer_id == pid) {
                still_active.push(PeerAttempt {
                    peer_id:     pid,
                    section_idx: idx,
                    elapsed_s:   existing.elapsed_s,
                });
            } else {
                // New peer entering a section.
                still_active.push(PeerAttempt {
                    peer_id:     pid,
                    section_idx: idx,
                    elapsed_s:   0.0,
                });
                bevy::log::info!(
                    "rock_crawl_trail: peer {:?} entered section {} ({})",
                    pid, idx, SECTION_NAMES[idx]
                );
            }
        }
        // Peers not in any section are simply not added → dropped next frame.
    }

    // Timeout cleanup — remove any peer that has been tracked > ATTEMPT_TIMEOUT_S.
    state.peer_attempts = still_active
        .into_iter()
        .filter(|p| p.elapsed_s < ATTEMPT_TIMEOUT_S)
        .collect();

    // Determine auto_target: the first peer that is mid-attempt.
    // (If local player is mid-attempt themselves, don't change spectate target
    //  as they're busy driving. If manual target is set, leave it alone.)
    if state.active.is_none() && spec.target_peer.is_none() {
        let new_auto = state.peer_attempts.first().map(|p| p.peer_id);
        spec.auto_target = new_auto;
    } else if state.active.is_some() {
        // Local player driving — clear auto-target so we don't force-spectate.
        spec.auto_target = None;
    }
}

// ---------------------------------------------------------------------------
// System: HUD update
// ---------------------------------------------------------------------------

fn update_hud(
    state:        Res<RockCrawlTrailState>,
    mut hud_root: Query<&mut Visibility, With<RcHudRoot>>,
    mut sec_q:    Query<&mut Text, (With<RcHudSection>, Without<RcHudTimer>, Without<RcHudBest>)>,
    mut tim_q:    Query<&mut Text, (With<RcHudTimer>,  Without<RcHudSection>, Without<RcHudBest>)>,
    mut best_q:   Query<&mut Text, (With<RcHudBest>,   Without<RcHudSection>, Without<RcHudTimer>)>,
) {
    let Ok(mut vis) = hud_root.single_mut() else { return };

    if let Some(ref attempt) = state.active {
        *vis = Visibility::Visible;

        let idx     = attempt.section_idx;
        let elapsed = attempt.elapsed_s;

        // Section label.
        if let Ok(mut txt) = sec_q.single_mut() {
            txt.0 = format!("Section: {}", SECTION_NAMES[idx]);
        }

        // Timer MM:SS.t
        if let Ok(mut txt) = tim_q.single_mut() {
            txt.0 = fmt_time(elapsed);
        }

        // Best time.
        if let Ok(mut txt) = best_q.single_mut() {
            txt.0 = match state.records[idx].best_s {
                Some(b) => format!("Best: {}", fmt_time(b)),
                None    => "Best: --".to_string(),
            };
        }
    } else {
        *vis = Visibility::Hidden;
    }
}

/// Format seconds as MM:SS.t  (e.g. "00:12.4")
fn fmt_time(s: f32) -> String {
    let total_tenths = (s * 10.0) as u32;
    let mins   = total_tenths / 600;
    let secs   = (total_tenths % 600) / 10;
    let tenths = total_tenths % 10;
    format!("{:02}:{:02}.{}", mins, secs, tenths)
}
