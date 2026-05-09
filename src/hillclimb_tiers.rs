// Sprint 56 — Multiplayer Hillclimb Tiers
//
// Three parallel hillclimb tracks at different Z positions from the existing
// single-track hillclimb (hillclimb.rs / hillclimb_track.rs):
//
//   Tier 0 — Beginner      Z = -180  (gentler grade: 20°–35°)
//   Tier 1 — Intermediate  Z = -210  (moderate grade: 25°–45°)
//   Tier 2 — Expert        Z = -240  (steep grade: 30°–55°)
//
// Per-tier timer + personal best:
//   Timer starts when the chassis enters the tier's start gate corridor.
//   Stops when the chassis reaches within 8 m XZ of the finish gate AND has
//   gained ≥ FINISH_ELEV_GAIN metres from the attempt start elevation.
//   Personal bests stored to platform_storage["hillclimb.json"].
//
// Leaderboard panel (L key):
//   Shows local + peer best times per tier.
//   Peer times arrive on CHANNEL_LEADERBOARD (reliable channel 3).
//
// Auto-spectate:
//   Uses SpectateState.auto_target to follow the currently leading peer.
//
// Public API:
//   HillclimbTiersPlugin
//   HillclimbTiersState  (Resource)
//   CHANNEL_LEADERBOARD  (usize)

use bevy::prelude::*;
use avian3d::prelude::*;
use bincode::{Decode, Encode};
use bevy_matchbox::prelude::*;

use crate::multiplayer::{GhostMarker, PeerId};
use crate::platform_storage;
use crate::spectate::SpectateState;
use crate::terrain::terrain_height_at;
use crate::vehicle::{Chassis, VehicleRoot};
use crate::notifications::NotificationQueue;

// ---------------------------------------------------------------------------
// Channel constant (socket builder adds channel 3 for leaderboard)
// ---------------------------------------------------------------------------

/// Reliable channel 3: hillclimb tier leaderboard broadcasts.
pub const CHANNEL_LEADERBOARD: usize = 3;

// ---------------------------------------------------------------------------
// Tier layout constants
// ---------------------------------------------------------------------------

pub const NUM_TIERS: usize = 3;
pub const TIER_NAMES: [&str; NUM_TIERS] = ["Beginner", "Intermediate", "Expert"];

/// Track direction runs along +X, starting at world X = -150.
const START_X: f32 = -150.0;

/// +1 m above terrain at startup for the chassis to sit cleanly.
const START_Y_LIFT: f32 = 1.0;

/// Z centre-lines for each tier track.
const TIER_Z: [f32; NUM_TIERS] = [-180.0, -210.0, -240.0];

const TIER_WIDTH: f32 = 10.0;

const NUM_SEGS: usize = 8;
const SEG_LEN:  f32   = 18.0;
const SLAB_THICK: f32 = 1.5;
const WALL_H:    f32  = 10.0;
const WALL_THICK: f32 = 0.6;
const POST_W: f32 = 0.4;
const POST_H: f32 = 4.0;
const BEAM_H: f32 = 0.4;

/// Elevation gain required before a finish gate crossing counts.
const FINISH_ELEV_GAIN: f32 = 50.0;

/// XZ proximity to finish gate position to trigger finish.
const FINISH_RADIUS: f32 = 8.0;

/// XZ proximity to start gate to auto-start the timer.
const START_RADIUS: f32 = 6.0;

/// Attempt timeout before auto-abort.
const ATTEMPT_TIMEOUT_S: f32 = 300.0;

// Per-tier pitch grades (degrees) for 8 segments.
const GRADES: [[f32; NUM_SEGS]; NUM_TIERS] = [
    [20.0, 22.0, 25.0, 28.0, 30.0, 32.0, 28.0, 22.0], // Beginner
    [25.0, 28.0, 32.0, 38.0, 42.0, 45.0, 38.0, 30.0], // Intermediate
    [30.0, 33.0, 38.0, 45.0, 50.0, 55.0, 42.0, 35.0], // Expert
];

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct HillclimbTiersPlugin;

impl Plugin for HillclimbTiersPlugin {
    fn build(&self, app: &mut App) {
        app
            .insert_resource(HillclimbTiersState::load())
            .insert_resource(TierLayout::default())
            .add_systems(Startup, (init_tier_layout, spawn_tier_tracks, spawn_tier_ui).chain())
            .add_systems(
                Update,
                (
                    tick_tier_timer,
                    check_tier_start,
                    check_tier_finish,
                    update_peer_attempts,
                    recv_leaderboard_packets,
                    update_tier_hud,
                    rebuild_leaderboard_content,
                    toggle_leaderboard_panel,
                )
                    .chain()
                    .run_if(resource_exists::<VehicleRoot>),
            );
    }
}

// ---------------------------------------------------------------------------
// TierLayout — computed at startup, read at runtime
// ---------------------------------------------------------------------------

/// Positions of start and finish gates per tier (populated in init_tier_layout).
#[derive(Resource, Default)]
pub struct TierLayout {
    pub start_pos:  [Vec3; NUM_TIERS],
    pub finish_pos: [Vec3; NUM_TIERS],
}

// ---------------------------------------------------------------------------
// Leaderboard wire packet
// ---------------------------------------------------------------------------

#[derive(Encode, Decode, Clone, Copy, Debug)]
pub struct LeaderboardPacket {
    /// 0 = Beginner, 1 = Intermediate, 2 = Expert.
    pub tier: u8,
    /// Personal best seconds. f32::MAX = no best yet.
    pub best_time_s: f32,
    /// True if sender is currently mid-attempt on this tier.
    pub is_active: bool,
    /// Current attempt elapsed (valid only when is_active is true).
    pub current_elapsed_s: f32,
}

// ---------------------------------------------------------------------------
// Public resource
// ---------------------------------------------------------------------------

#[derive(Clone, Default, Debug)]
pub struct TierRecord {
    pub best_s: Option<f32>,
    pub last_s: Option<f32>,
}

#[derive(Clone, Debug)]
pub struct TierAttempt {
    pub tier:      usize,
    pub elapsed_s: f32,
    /// Chassis Y at the start of this attempt.
    pub start_y:   f32,
}

#[derive(Clone, Debug)]
pub struct PeerEntry {
    pub peer_id:        PeerId,
    pub bests:          [Option<f32>; NUM_TIERS],
    pub is_active:      bool,
    pub active_tier:    usize,
    pub active_elapsed: f32,
}

#[derive(Clone, Debug)]
struct PeerAttemptTrack {
    peer_id:   PeerId,
    #[allow(dead_code)]
    tier:      usize,
    elapsed_s: f32,
}

#[derive(Resource)]
pub struct HillclimbTiersState {
    pub records:      [TierRecord; NUM_TIERS],
    pub active:       Option<TierAttempt>,
    pub peer_entries: Vec<PeerEntry>,
    peer_tracks:      Vec<PeerAttemptTrack>,
    needs_save:       bool,
    needs_broadcast:  bool,
}

impl Default for HillclimbTiersState {
    fn default() -> Self {
        Self {
            records:         Default::default(),
            active:          None,
            peer_entries:    Vec::new(),
            peer_tracks:     Vec::new(),
            needs_save:      false,
            needs_broadcast: false,
        }
    }
}

impl HillclimbTiersState {
    pub fn load() -> Self {
        let mut s = Self::default();
        if let Some(text) = platform_storage::read_string("hillclimb.json") {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
                // Tier format: { "tiers": [ {"best_s": …, "last_s": …}, … ] }
                if let Some(arr) = v.get("tiers").and_then(|t| t.as_array()) {
                    for (i, entry) in arr.iter().enumerate() {
                        if i >= NUM_TIERS { break; }
                        s.records[i].best_s = entry.get("best_s")
                            .and_then(|x| x.as_f64())
                            .map(|x| x as f32);
                        s.records[i].last_s = entry.get("last_s")
                            .and_then(|x| x.as_f64())
                            .map(|x| x as f32);
                    }
                }
                // Legacy single-tier: { "best_time_s": … } — migrate to Expert slot.
                else if let Some(best) = v.get("best_time_s").and_then(|x| x.as_f64()) {
                    s.records[2].best_s = Some(best as f32);
                    info!("hillclimb_tiers: migrated legacy best_time_s to Expert tier");
                }
            }
        }
        s
    }

    fn save(&self) {
        let tiers: Vec<serde_json::Value> = self.records.iter().map(|r| {
            serde_json::json!({ "best_s": r.best_s, "last_s": r.last_s })
        }).collect();
        let json = serde_json::json!({ "tiers": tiers }).to_string();
        match platform_storage::write_string("hillclimb.json", &json) {
            Ok(()) => info!("hillclimb_tiers: saved to hillclimb.json"),
            Err(e) => warn!("hillclimb_tiers: save failed: {}", e),
        }
    }
}

// ---------------------------------------------------------------------------
// Gate marker components
// ---------------------------------------------------------------------------

#[derive(Component)]
#[allow(dead_code)]
struct TierStartGate { tier: usize }

#[derive(Component)]
#[allow(dead_code)]
struct TierFinishGate { tier: usize }

// ---------------------------------------------------------------------------
// HUD / Leaderboard UI markers
// ---------------------------------------------------------------------------

#[derive(Component)]
struct TierHudRoot;

#[derive(Component)]
struct TierHudTitle;

#[derive(Component)]
struct TierHudTimer;

#[derive(Component)]
struct TierHudBest;

#[derive(Component)]
struct LeaderboardPanelRoot;

#[derive(Component)]
struct LeaderboardContent;

// ---------------------------------------------------------------------------
// Startup: init_tier_layout — compute start/finish positions (no commands needed)
// ---------------------------------------------------------------------------

fn init_tier_layout(mut layout: ResMut<TierLayout>) {
    for tier in 0..NUM_TIERS {
        let tz = TIER_Z[tier];
        let terrain_y = terrain_height_at(START_X, tz);
        let start_y   = terrain_y + START_Y_LIFT;
        layout.start_pos[tier] = Vec3::new(START_X, start_y, tz);

        // Compute finish position by simulating the slope geometry.
        let mut foot = Vec3::new(START_X, start_y, tz);
        for i in 0..NUM_SEGS {
            let pitch_rad = GRADES[tier][i].to_radians();
            let run  = SEG_LEN * pitch_rad.cos();
            let rise = SEG_LEN * pitch_rad.sin();
            foot.x += run;
            foot.y += rise;
        }
        layout.finish_pos[tier] = foot;

        info!(
            "hillclimb_tiers: tier {} ({}) layout: start=({:.0},{:.0},{:.0}) finish=({:.0},{:.0},{:.0})",
            tier, TIER_NAMES[tier],
            layout.start_pos[tier].x, layout.start_pos[tier].y, layout.start_pos[tier].z,
            layout.finish_pos[tier].x, layout.finish_pos[tier].y, layout.finish_pos[tier].z,
        );
    }
}

// ---------------------------------------------------------------------------
// Startup: spawn_tier_tracks
// ---------------------------------------------------------------------------

fn spawn_tier_tracks(
    layout:        Res<TierLayout>,
    mut commands:  Commands,
    mut meshes:    ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let tier_colors = [
        Color::srgb(0.30, 0.55, 0.30), // Beginner: green-tinted
        Color::srgb(0.55, 0.45, 0.25), // Intermediate: amber-tinted
        Color::srgb(0.55, 0.28, 0.22), // Expert: red-tinted
    ];

    let wall_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.28, 0.24, 0.22),
        perceptual_roughness: 0.97,
        ..default()
    });

    let gate_start_color  = Color::srgb(1.0,  0.85, 0.0);
    let gate_finish_color = Color::srgb(0.1,  0.85, 0.2);
    let half_w = TIER_WIDTH * 0.5;
    let post_off = half_w - POST_W * 0.5;

    let post_mesh = meshes.add(Cuboid::new(POST_W, POST_H, POST_W));

    for tier in 0..NUM_TIERS {
        let sp = layout.start_pos[tier];
        let start_y = sp.y;
        let tz = TIER_Z[tier];

        let slope_mat = materials.add(StandardMaterial {
            base_color: tier_colors[tier],
            perceptual_roughness: 0.95,
            ..default()
        });

        // ----------------------------------------------------------------
        // Slope segments
        // ----------------------------------------------------------------
        let mut foot = sp;

        struct SegDatum { centre: Vec3, pitch: f32 }
        let mut segs: Vec<SegDatum> = Vec::with_capacity(NUM_SEGS);

        for i in 0..NUM_SEGS {
            let pitch = GRADES[tier][i].to_radians();
            let run  = SEG_LEN * pitch.cos();
            let rise = SEG_LEN * pitch.sin();

            let centre = Vec3::new(
                foot.x + run  * 0.5,
                foot.y + rise * 0.5,
                tz,
            );
            let rot = Quat::from_rotation_x(-pitch);

            let mesh = meshes.add(Cuboid::new(SEG_LEN, SLAB_THICK, TIER_WIDTH));

            commands.spawn((
                Mesh3d(mesh),
                MeshMaterial3d(slope_mat.clone()),
                Transform { translation: centre, rotation: rot, scale: Vec3::ONE },
                RigidBody::Static,
                Collider::cuboid(SEG_LEN * 0.5, SLAB_THICK * 0.5, TIER_WIDTH * 0.5),
            ));

            segs.push(SegDatum { centre, pitch });

            foot.x += run;
            foot.y += rise;
        }

        // ----------------------------------------------------------------
        // Side walls
        // ----------------------------------------------------------------
        let wall_mesh = meshes.add(Cuboid::new(SEG_LEN, WALL_H, WALL_THICK));

        for seg in &segs {
            let wall_base_y = seg.centre.y + SLAB_THICK * 0.5;
            let wall_cy     = wall_base_y + WALL_H * 0.5;
            let rot = Quat::from_rotation_x(-seg.pitch);

            for side in [-1.0_f32, 1.0_f32] {
                let wc = Vec3::new(seg.centre.x, wall_cy, tz + side * half_w);
                commands.spawn((
                    Mesh3d(wall_mesh.clone()),
                    MeshMaterial3d(wall_mat.clone()),
                    Transform { translation: wc, rotation: rot, scale: Vec3::ONE },
                    RigidBody::Static,
                    Collider::cuboid(SEG_LEN * 0.5, WALL_H * 0.5, WALL_THICK * 0.5),
                ));
            }
        }

        // ----------------------------------------------------------------
        // Start gate
        // ----------------------------------------------------------------
        let start_mat = materials.add(StandardMaterial {
            base_color: gate_start_color,
            perceptual_roughness: 0.5,
            emissive: LinearRgba::rgb(0.3, 0.2, 0.0),
            ..default()
        });
        let beam_start = meshes.add(Cuboid::new(TIER_WIDTH, BEAM_H, BEAM_H));

        let sg = commands.spawn((
            TierStartGate { tier },
            Transform::from_translation(Vec3::new(START_X, start_y, tz)),
            Visibility::default(),
        )).id();
        let lp = commands.spawn((
            Mesh3d(post_mesh.clone()),
            MeshMaterial3d(start_mat.clone()),
            Transform::from_xyz(0.0, POST_H * 0.5, -post_off),
            RigidBody::Static,
            Collider::cuboid(POST_W * 0.5, POST_H * 0.5, POST_W * 0.5),
        )).id();
        let rp = commands.spawn((
            Mesh3d(post_mesh.clone()),
            MeshMaterial3d(start_mat.clone()),
            Transform::from_xyz(0.0, POST_H * 0.5, post_off),
            RigidBody::Static,
            Collider::cuboid(POST_W * 0.5, POST_H * 0.5, POST_W * 0.5),
        )).id();
        let bm = commands.spawn((
            Mesh3d(beam_start),
            MeshMaterial3d(start_mat.clone()),
            Transform::from_xyz(0.0, POST_H + BEAM_H * 0.5, 0.0),
            RigidBody::Static,
            Collider::cuboid(TIER_WIDTH * 0.5, BEAM_H * 0.5, BEAM_H * 0.5),
        )).id();
        commands.entity(sg).add_children(&[lp, rp, bm]);

        // ----------------------------------------------------------------
        // Finish gate
        // ----------------------------------------------------------------
        let finish_mat = materials.add(StandardMaterial {
            base_color: gate_finish_color,
            perceptual_roughness: 0.5,
            emissive: LinearRgba::rgb(0.0, 0.25, 0.05),
            ..default()
        });
        let beam_finish = meshes.add(Cuboid::new(TIER_WIDTH, BEAM_H, BEAM_H));
        let fp = layout.finish_pos[tier];

        let fg = commands.spawn((
            TierFinishGate { tier },
            Transform::from_translation(fp),
            Visibility::default(),
        )).id();
        let flp = commands.spawn((
            Mesh3d(post_mesh.clone()),
            MeshMaterial3d(finish_mat.clone()),
            Transform::from_xyz(0.0, POST_H * 0.5, -post_off),
            RigidBody::Static,
            Collider::cuboid(POST_W * 0.5, POST_H * 0.5, POST_W * 0.5),
        )).id();
        let frp = commands.spawn((
            Mesh3d(post_mesh.clone()),
            MeshMaterial3d(finish_mat.clone()),
            Transform::from_xyz(0.0, POST_H * 0.5, post_off),
            RigidBody::Static,
            Collider::cuboid(POST_W * 0.5, POST_H * 0.5, POST_W * 0.5),
        )).id();
        let fbm = commands.spawn((
            Mesh3d(beam_finish),
            MeshMaterial3d(finish_mat.clone()),
            Transform::from_xyz(0.0, POST_H + BEAM_H * 0.5, 0.0),
            RigidBody::Static,
            Collider::cuboid(TIER_WIDTH * 0.5, BEAM_H * 0.5, BEAM_H * 0.5),
        )).id();
        commands.entity(fg).add_children(&[flp, frp, fbm]);
    }

    info!("hillclimb_tiers: spawned {} tier tracks", NUM_TIERS);
}

// ---------------------------------------------------------------------------
// Startup: spawn UI (HUD + leaderboard panel)
// ---------------------------------------------------------------------------

fn spawn_tier_ui(mut commands: Commands) {
    let bg = Color::srgba(0.03, 0.05, 0.10, 0.90);

    // ---- Tier active-attempt HUD (top-center, right of the main hillclimb HUD) ----
    let hud_root = commands.spawn((
        TierHudRoot,
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(8.0),
            // Offset right of the main hillclimb HUD (which is at 50% - 140px).
            // We place ours at 50% + 150px (140px wide hud + 10px gap).
            left: Val::Percent(50.0),
            margin: UiRect { left: Val::Px(150.0), ..default() },
            width: Val::Px(270.0),
            min_height: Val::Px(70.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            padding: UiRect::axes(Val::Px(10.0), Val::Px(6.0)),
            row_gap: Val::Px(2.0),
            ..default()
        },
        BackgroundColor(bg),
        ZIndex(42),
        Visibility::Hidden,
    )).id();

    let title = commands.spawn((
        TierHudTitle,
        Text::new("TIER HILLCLIMB"),
        TextFont { font_size: 12.0, ..default() },
        TextColor(Color::srgb(1.0, 0.75, 0.15)),
    )).id();

    let timer = commands.spawn((
        TierHudTimer,
        Text::new("00:00.00"),
        TextFont { font_size: 22.0, ..default() },
        TextColor(Color::WHITE),
    )).id();

    let best = commands.spawn((
        TierHudBest,
        Text::new("Best: --"),
        TextFont { font_size: 12.0, ..default() },
        TextColor(Color::srgb(0.6, 0.6, 0.6)),
    )).id();

    commands.entity(hud_root).add_children(&[title, timer, best]);

    // ---- Leaderboard panel (bottom-right, toggled with L) ----
    let lb_root = commands.spawn((
        LeaderboardPanelRoot,
        Node {
            position_type:   PositionType::Absolute,
            bottom:          Val::Px(12.0),
            right:           Val::Px(12.0),
            width:           Val::Px(300.0),
            flex_direction:  FlexDirection::Column,
            padding:         UiRect::all(Val::Px(10.0)),
            row_gap:         Val::Px(4.0),
            ..default()
        },
        BackgroundColor(Color::srgba(0.03, 0.05, 0.10, 0.92)),
        ZIndex(50),
        Visibility::Hidden,
    )).id();

    let lb_title = commands.spawn((
        Text::new("HILLCLIMB LEADERBOARD  [L]"),
        TextFont { font_size: 13.0, ..default() },
        TextColor(Color::srgb(1.0, 0.82, 0.2)),
    )).id();

    let lb_content = commands.spawn((
        LeaderboardContent,
        Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(2.0),
            ..default()
        },
    )).id();

    commands.entity(lb_root).add_children(&[lb_title, lb_content]);
}

// ---------------------------------------------------------------------------
// System: tick_tier_timer — advance elapsed + save/broadcast when dirty
// ---------------------------------------------------------------------------

fn tick_tier_timer(
    time:       Res<Time>,
    mut state:  ResMut<HillclimbTiersState>,
    mut socket: Option<ResMut<MatchboxSocket>>,
) {
    let dt = time.delta_secs();

    if let Some(ref mut attempt) = state.active {
        attempt.elapsed_s += dt;
        if attempt.elapsed_s >= ATTEMPT_TIMEOUT_S {
            warn!("hillclimb_tiers: tier {} attempt timed out", attempt.tier);
            state.active = None;
        }
    }

    if state.needs_save {
        state.save();
        state.needs_save = false;
    }

    if state.needs_broadcast {
        state.needs_broadcast = false;
        if let Some(ref mut sock) = socket {
            let peers: Vec<PeerId> = sock.connected_peers().collect();
            if !peers.is_empty() {
                for tier in 0..NUM_TIERS {
                    let best = state.records[tier].best_s.unwrap_or(f32::MAX);
                    let (is_active, elapsed) = state.active
                        .as_ref()
                        .filter(|a| a.tier == tier)
                        .map(|a| (true, a.elapsed_s))
                        .unwrap_or((false, 0.0));

                    let pkt = LeaderboardPacket {
                        tier:              tier as u8,
                        best_time_s:       best,
                        is_active,
                        current_elapsed_s: elapsed,
                    };

                    if let Ok(bytes) = bincode::encode_to_vec(pkt, bincode::config::standard()) {
                        for &peer in &peers {
                            sock.channel_mut(CHANNEL_LEADERBOARD).send(bytes.clone().into(), peer);
                        }
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// System: check_tier_start — detect chassis entering a tier start corridor
// ---------------------------------------------------------------------------

fn check_tier_start(
    vehicle:   Res<VehicleRoot>,
    chassis_q: Query<&Transform, With<Chassis>>,
    layout:    Res<TierLayout>,
    mut state: ResMut<HillclimbTiersState>,
    mut notifs: ResMut<NotificationQueue>,
) {
    if state.active.is_some() { return; }

    let Ok(tf) = chassis_q.get(vehicle.chassis) else { return };
    let pos = tf.translation;

    for tier in 0..NUM_TIERS {
        let sp = layout.start_pos[tier];
        let dx = pos.x - sp.x;
        let dz = (pos.z - sp.z).abs();
        let xz_dist = (dx * dx + (pos.z - sp.z) * (pos.z - sp.z)).sqrt();

        // Enter when chassis is within START_RADIUS on XZ of start gate
        // and has just crossed the gate (dx >= 0).
        if xz_dist <= START_RADIUS && dx >= 0.0 {
            state.active = Some(TierAttempt {
                tier,
                elapsed_s: 0.0,
                start_y:   pos.y,
            });
            notifs.push(
                format!("{} HILLCLIMB STARTED", TIER_NAMES[tier]),
                Color::srgb(1.0, 0.75, 0.15),
            );
            info!(
                "hillclimb_tiers: started {} tier at ({:.1},{:.1},{:.1})",
                TIER_NAMES[tier], pos.x, pos.y, pos.z
            );
            let _ = dz; // consumed above in the check
            break;
        }
    }
}

// ---------------------------------------------------------------------------
// System: check_tier_finish — detect chassis reaching the finish gate
// ---------------------------------------------------------------------------

fn check_tier_finish(
    vehicle:   Res<VehicleRoot>,
    chassis_q: Query<&Transform, With<Chassis>>,
    layout:    Res<TierLayout>,
    mut state: ResMut<HillclimbTiersState>,
    mut notifs: ResMut<NotificationQueue>,
) {
    let Ok(tf) = chassis_q.get(vehicle.chassis) else { return };

    let Some(attempt) = state.active.clone() else { return };

    // Flip detection
    let up = tf.rotation * Vec3::Y;
    if up.y < -0.3 {
        notifs.push("TIER HILLCLIMB FAILED — flipped", Color::srgb(1.0, 0.25, 0.1));
        info!("hillclimb_tiers: tier {} failed — flipped", attempt.tier);
        state.active = None;
        return;
    }

    let pos = tf.translation;
    let elev_gained = pos.y - attempt.start_y;
    if elev_gained < FINISH_ELEV_GAIN { return; }

    let fp = layout.finish_pos[attempt.tier];
    let dx = pos.x - fp.x;
    let dz = pos.z - fp.z;
    let xz_dist = (dx * dx + dz * dz).sqrt();

    if xz_dist <= FINISH_RADIUS {
        let elapsed = attempt.elapsed_s;
        let tier    = attempt.tier;
        state.active = None;

        state.records[tier].last_s = Some(elapsed);
        let is_best = state.records[tier].best_s.map(|b| elapsed < b).unwrap_or(true);
        if is_best {
            state.records[tier].best_s = Some(elapsed);
            info!("hillclimb_tiers: tier {} NEW BEST {:.2}s", tier, elapsed);
            notifs.push(
                format!("{} TIER — NEW BEST! {}", TIER_NAMES[tier], fmt_time(elapsed)),
                Color::srgb(0.1, 1.0, 0.4),
            );
        } else {
            let best = state.records[tier].best_s.unwrap_or(elapsed);
            notifs.push(
                format!(
                    "{} TIER — {} (Best: {})",
                    TIER_NAMES[tier], fmt_time(elapsed), fmt_time(best)
                ),
                Color::srgb(0.4, 0.85, 1.0),
            );
        }
        state.needs_save      = true;
        state.needs_broadcast = true;
    }
}

// ---------------------------------------------------------------------------
// System: update_peer_attempts — track ghost positions for auto-spectate
// ---------------------------------------------------------------------------

fn update_peer_attempts(
    ghosts:    Query<(&GhostMarker, &Transform)>,
    mut state: ResMut<HillclimbTiersState>,
    mut spec:  ResMut<SpectateState>,
) {
    let mut still_active: Vec<PeerAttemptTrack> = Vec::new();

    for (ghost, tf) in &ghosts {
        let pos = tf.translation;
        let pid = ghost.peer_id;

        if let Some(tier) = tier_containing(pos) {
            let elapsed = state.peer_tracks
                .iter()
                .find(|p| p.peer_id == pid)
                .map(|p| p.elapsed_s)
                .unwrap_or(0.0);
            still_active.push(PeerAttemptTrack { peer_id: pid, tier, elapsed_s: elapsed });
        }
    }

    state.peer_tracks = still_active;

    // Auto-spectate: pick the leading peer (lowest elapsed_s).
    if state.active.is_none() && spec.target_peer.is_none() {
        let best = state.peer_tracks
            .iter()
            .min_by(|a, b| a.elapsed_s.partial_cmp(&b.elapsed_s)
                .unwrap_or(std::cmp::Ordering::Equal))
            .map(|p| p.peer_id);
        spec.auto_target = best;
    } else if state.active.is_some() {
        spec.auto_target = None;
    }
}

// ---------------------------------------------------------------------------
// System: recv_leaderboard_packets
// ---------------------------------------------------------------------------

fn recv_leaderboard_packets(
    mut socket: Option<ResMut<MatchboxSocket>>,
    mut state:  ResMut<HillclimbTiersState>,
) {
    let Some(ref mut sock) = socket else { return };

    for (peer_id, bytes) in sock.channel_mut(CHANNEL_LEADERBOARD).receive() {
        let Ok((pkt, _)) = bincode::decode_from_slice::<LeaderboardPacket, _>(
            &bytes,
            bincode::config::standard(),
        ) else {
            warn!("hillclimb_tiers: leaderboard decode error from {:?}", peer_id);
            continue;
        };

        let tier = pkt.tier as usize;
        if tier >= NUM_TIERS { continue; }

        let entry = if let Some(e) = state.peer_entries.iter_mut().find(|e| e.peer_id == peer_id) {
            e
        } else {
            state.peer_entries.push(PeerEntry {
                peer_id,
                bests:          [None; NUM_TIERS],
                is_active:      false,
                active_tier:    0,
                active_elapsed: 0.0,
            });
            state.peer_entries.last_mut().unwrap()
        };

        entry.bests[tier]    = if pkt.best_time_s < f32::MAX { Some(pkt.best_time_s) } else { None };
        entry.is_active      = pkt.is_active;
        entry.active_tier    = tier;
        entry.active_elapsed = pkt.current_elapsed_s;
    }
}

// ---------------------------------------------------------------------------
// System: update_tier_hud — show active tier attempt info
// ---------------------------------------------------------------------------

fn update_tier_hud(
    state:    Res<HillclimbTiersState>,
    mut hud_vis_q: Query<&mut Visibility, With<TierHudRoot>>,
    mut title_q:   Query<&mut Text, (With<TierHudTitle>, Without<TierHudTimer>, Without<TierHudBest>)>,
    mut timer_q:   Query<&mut Text, (With<TierHudTimer>, Without<TierHudTitle>, Without<TierHudBest>)>,
    mut best_q:    Query<&mut Text, (With<TierHudBest>,  Without<TierHudTitle>, Without<TierHudTimer>)>,
) {
    let is_active = state.active.is_some();

    for mut vis in hud_vis_q.iter_mut() {
        *vis = if is_active { Visibility::Visible } else { Visibility::Hidden };
    }

    if let Some(ref attempt) = state.active {
        let tier    = attempt.tier;
        let elapsed = attempt.elapsed_s;

        for mut txt in title_q.iter_mut() {
            txt.0 = format!("{} HILLCLIMB", TIER_NAMES[tier]);
        }
        for mut txt in timer_q.iter_mut() {
            txt.0 = fmt_time(elapsed);
        }
        for mut txt in best_q.iter_mut() {
            txt.0 = match state.records[tier].best_s {
                Some(b) => format!("Best: {}", fmt_time(b)),
                None    => "Best: --".to_string(),
            };
        }
    }
}

// ---------------------------------------------------------------------------
// System: rebuild_leaderboard_content — populate the leaderboard panel
// ---------------------------------------------------------------------------

fn rebuild_leaderboard_content(
    state:        Res<HillclimbTiersState>,
    content_q:    Query<Entity, With<LeaderboardContent>>,
    mut commands: Commands,
) {
    let Ok(content_entity) = content_q.single() else { return };
    commands.entity(content_entity).despawn_related::<Children>();

    let mut rows: Vec<Entity> = Vec::new();

    let tier_colors = [
        Color::srgb(0.4, 0.9, 0.4),
        Color::srgb(0.9, 0.75, 0.2),
        Color::srgb(0.9, 0.35, 0.25),
    ];

    for tier in 0..NUM_TIERS {
        // Tier header
        let hdr = commands.spawn((
            Text::new(format!("── {} ──", TIER_NAMES[tier])),
            TextFont { font_size: 12.0, ..default() },
            TextColor(tier_colors[tier]),
        )).id();
        rows.push(hdr);

        // Local player
        let local_best_str = state.records[tier].best_s
            .map(fmt_time)
            .unwrap_or_else(|| "--".to_string());

        let local_label = if state.active.as_ref().map(|a| a.tier == tier).unwrap_or(false) {
            let el = state.active.as_ref().map(|a| a.elapsed_s).unwrap_or(0.0);
            format!("  YOU  {} (pb {})", fmt_time(el), local_best_str)
        } else {
            format!("  YOU  {}", local_best_str)
        };

        let local_row = commands.spawn((
            Text::new(local_label),
            TextFont { font_size: 11.0, ..default() },
            TextColor(Color::srgb(0.85, 0.85, 0.85)),
        )).id();
        rows.push(local_row);

        // Peer rows
        for pe in &state.peer_entries {
            let peer_best_str = pe.bests[tier]
                .map(fmt_time)
                .unwrap_or_else(|| "--".to_string());

            let peer_label = if pe.is_active && pe.active_tier == tier {
                format!("  PEER  {} (pb {})", fmt_time(pe.active_elapsed), peer_best_str)
            } else {
                format!("  PEER  {}", peer_best_str)
            };

            let peer_row = commands.spawn((
                Text::new(peer_label),
                TextFont { font_size: 11.0, ..default() },
                TextColor(Color::srgb(0.65, 0.75, 0.95)),
            )).id();
            rows.push(peer_row);
        }
    }

    if !rows.is_empty() {
        commands.entity(content_entity).add_children(&rows);
    }
}

// ---------------------------------------------------------------------------
// System: toggle_leaderboard_panel — L key
// ---------------------------------------------------------------------------

fn toggle_leaderboard_panel(
    keys:     Res<ButtonInput<KeyCode>>,
    mut lb_q: Query<&mut Visibility, With<LeaderboardPanelRoot>>,
) {
    if !keys.just_pressed(KeyCode::KeyL) { return; }
    for mut vis in lb_q.iter_mut() {
        *vis = match *vis {
            Visibility::Hidden    => Visibility::Visible,
            Visibility::Visible   => Visibility::Hidden,
            Visibility::Inherited => Visibility::Visible,
        };
    }
}

// ---------------------------------------------------------------------------
// Helper: which tier corridor is a point inside?
// ---------------------------------------------------------------------------

fn tier_containing(pos: Vec3) -> Option<usize> {
    let half = TIER_WIDTH * 0.5;
    let max_run = SEG_LEN * NUM_SEGS as f32 + 10.0;

    for tier in 0..NUM_TIERS {
        let dz = (pos.z - TIER_Z[tier]).abs();
        let dx = pos.x - START_X;
        if dz <= half && dx >= -2.0 && dx <= max_run {
            return Some(tier);
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Time formatting (MM:SS.cc)
// ---------------------------------------------------------------------------

fn fmt_time(s: f32) -> String {
    let s    = s.max(0.0);
    let mins = (s / 60.0) as u32;
    let rem  = s - (mins as f32) * 60.0;
    let sec  = rem as u32;
    let cs   = ((rem % 1.0) * 100.0) as u32;
    format!("{:02}:{:02}.{:02}", mins, sec, cs)
}
