// Sprint 49 — P2P Multiplayer (position sync only)
//
// Architecture: P2P over WebRTC via bevy_matchbox / matchbox_socket.
// No server authority — peers broadcast chassis state at 20 Hz.
// Each remote peer is rendered as a semi-transparent "ghost" chassis.
//
// State machine:
//   Disconnected → Connecting → InRoom | Failed
//
// UI panel toggled by I (the letter I, for "Internet").
// (P is taken by photo-mode/pursuit; K is taken by transmission/skidmarks.)
//
// Packet layout (per peer, per tick, ~56 bytes wire-size with bincode):
//   Vec3  translation  (12 bytes)
//   Quat  rotation     (16 bytes)
//   Vec3  linear_vel   (12 bytes)
//   Vec3  angular_vel  (12 bytes)
//   u8    paint_index  ( 1 byte)
//   u8    variant_disc ( 1 byte)
//                       --------
//                       54 bytes  → ~54 × 20 = ~8.6 Kbps per direction per peer
//
// ICE / signaling constants are at the top of the file for easy swapping.

// ---------------------------------------------------------------------------
// ICE / signaling constants
// ---------------------------------------------------------------------------

/// Default STUN servers (Google, Cloudflare, Twilio).
const STUN_URLS: &[&str] = &[
    "stun:stun.l.google.com:19302",
    "stun:stun.cloudflare.com:3478",
    "stun:global.stun.twilio.com:3478",
];

/// Default signaling server URL — overridable via env var or settings.
const DEFAULT_SIGNALING_URL: &str = "wss://signaling.skoffroad.skworld.io/skoffroad-1";

/// Env var that overrides the signaling server URL.
const ENV_SIGNALING_URL: &str = "SKOFFROAD_SIGNALING_URL";

/// Env vars for TURN override.
const ENV_TURN_URL:      &str = "SKOFFROAD_TURN_URL";
const ENV_TURN_USERNAME: &str = "SKOFFROAD_TURN_USERNAME";
const ENV_TURN_PASSWORD: &str = "SKOFFROAD_TURN_PASSWORD";

/// Storage key for saved signaling URL.
const STORAGE_SIGNALING_KEY: &str = "signaling.json";

/// Storage key for saved TURN config.
const STORAGE_TURN_KEY: &str = "turn.json";

/// Send chassis state at this rate (Hz).
const SEND_HZ: f32 = 20.0;

/// Seconds between ghost-transform ticks used for lerp smoothing target.
const LERP_DURATION_S: f32 = 0.05;

/// Ghost material alpha.
const GHOST_ALPHA: f32 = 0.55;

/// Channel index for our unreliable state channel.
const CHANNEL_STATE: usize = 0;

/// Channel index for reliable signaling (voice SDP / ICE candidates).
pub const CHANNEL_VOICE_SIGNAL: usize = 1;

/// Channel index for reliable recovery messages (Sprint 55).
pub const CHANNEL_RECOVERY: usize = 2;

// ---------------------------------------------------------------------------
// Packet-kind prefix byte — sits at byte 0 of every CHANNEL_STATE message
// so that future packet types can be dispatched without breaking the existing
// chassis decoder.  Voice signaling rides CHANNEL_VOICE_SIGNAL instead and
// does NOT use this prefix; it is included here for documentation parity.
// ---------------------------------------------------------------------------

/// The first byte of every CHANNEL_STATE datagram identifies its kind.
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MessageKind {
    /// Existing chassis-state broadcast (bytes 1..N are a `ChassisPacket`).
    Game  = 0,
    /// Reserved — voice signaling rides CHANNEL_VOICE_SIGNAL, not this channel.
    Voice = 1,
}

use bevy::prelude::*;
use bevy_matchbox::prelude::*;
use bevy_matchbox::matchbox_socket::RtcIceServerConfig;
use bincode::{Decode, Encode};

use crate::buddy_recovery::{
    self as br, HookKind, RecoveryKind, RecoveryState as BuddyRecoveryState,
};
use crate::camera_modes::{CameraMode, CameraModesState};
use crate::livery::LiveryState;
use crate::platform_storage;
use crate::spectate::{SpectateButton, SpectateState};
use crate::variants::VehicleVariant;
use crate::vehicle::Chassis;

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct MultiplayerPlugin;

impl Plugin for MultiplayerPlugin {
    fn build(&self, app: &mut App) {
        app
            // Resources
            .insert_resource(MultiplayerState::default())
            .insert_resource(MultiplayerConfig::load())
            .insert_resource(SendTimer(Timer::from_seconds(
                1.0 / SEND_HZ,
                TimerMode::Repeating,
            )))
            .insert_resource(PeerGhosts::default())
            // Startup: build the UI panel (hidden by default)
            .add_systems(Startup, spawn_panel)
            // Update systems
            .add_systems(
                Update,
                (
                    toggle_panel,
                    update_socket,
                    send_chassis_state,
                    recv_chassis_state,
                    lerp_ghosts,
                    cleanup_disconnected_ghosts,
                    update_panel_ui,
                    handle_connect_button,
                    handle_config_inputs,
                    handle_recovery_attach_buttons,
                )
                    .chain(),
            );
    }
}

// ---------------------------------------------------------------------------
// Public state machine
// ---------------------------------------------------------------------------

/// Re-export PeerId so voice.rs (and other plugins) don't need a direct
/// `bevy_matchbox` import just for the type.
pub use bevy_matchbox::prelude::PeerId;

/// Current multiplayer connection state, readable by other plugins.
#[derive(Resource, Default)]
pub enum MultiplayerState {
    #[default]
    Disconnected,
    Connecting {
        since_secs: f32,
    },
    InRoom {
        peer_count: u8,
    },
    Failed {
        reason:     String,
        since_secs: f32,
    },
}

// ---------------------------------------------------------------------------
// Configuration (signaling URL, TURN, room code) — persisted via platform_storage
// ---------------------------------------------------------------------------

/// Runtime-editable config for the multiplayer panel.
#[derive(Resource, Clone)]
pub struct MultiplayerConfig {
    /// Full signaling URL including room segment, e.g. `wss://…/skoffroad-1`
    pub signaling_url:   String,
    /// Optional TURN URL (e.g. `turn:myserver.com:3478`)
    pub turn_url:        String,
    pub turn_username:   String,
    pub turn_password:   String,
}

impl MultiplayerConfig {
    /// Load config from environment variables → platform_storage → defaults.
    pub fn load() -> Self {
        // --- signaling URL ---
        let signaling_url = resolve_signaling_url();

        // --- TURN ---
        let (turn_url, turn_username, turn_password) = resolve_turn();

        Self {
            signaling_url,
            turn_url,
            turn_username,
            turn_password,
        }
    }

    /// Persist changed TURN / signaling config to platform storage.
    pub fn save(&self) {
        let signaling_json = serde_json::json!({ "url": self.signaling_url }).to_string();
        let _ = platform_storage::write_string(STORAGE_SIGNALING_KEY, &signaling_json);

        if !self.turn_url.is_empty() {
            let turn_json = serde_json::json!({
                "url":      self.turn_url,
                "username": self.turn_username,
                "password": self.turn_password,
            })
            .to_string();
            let _ = platform_storage::write_string(STORAGE_TURN_KEY, &turn_json);
        }
    }
}

// ---------------------------------------------------------------------------
// Packet format
// ---------------------------------------------------------------------------

/// Chassis snapshot broadcast to peers at 20 Hz.
/// Packet version: v2 (Sprint 53 added camera_mode byte).
#[derive(Encode, Decode, Clone, Copy, Debug)]
pub struct ChassisPacket {
    /// World translation [x, y, z]
    pub translation:  [f32; 3],
    /// World rotation as quaternion [x, y, z, w]
    pub rotation:     [f32; 4],
    /// Linear velocity [x, y, z]
    pub linear_vel:   [f32; 3],
    /// Angular velocity [x, y, z]
    pub angular_vel:  [f32; 3],
    /// Paint preset index (LiveryState::current)
    pub paint_index:  u8,
    /// VehicleVariant discriminant (0 = JeepTJ, 1 = FordBronco, …)
    pub variant_disc: u8,
    /// Active camera mode (mirrors CameraMode discriminant: 0=Chase, 1=WheelFL,
    /// 2=WheelFR, 3=FirstPerson, 4=FreeOrbit).  Used by spectate.rs.
    pub camera_mode:  u8,
}

// ---------------------------------------------------------------------------
// Ghost cars — one per remote peer
// ---------------------------------------------------------------------------

/// Maps peer IDs to their ghost entity and latest received state.
#[derive(Resource, Default)]
struct PeerGhosts {
    /// peer_id → (ghost_entity, target_transform, current_transform, lerp_t)
    entries: std::collections::HashMap<PeerId, GhostEntry>,
}

struct GhostEntry {
    entity:      Entity,
    /// Transform we are lerping *toward* (set on each received packet)
    target:      Transform,
    /// Current smoothed transform (starts equal to target on first packet)
    current:     Transform,
    /// 0.0 = at old position, 1.0 = arrived at target (reset to 0 on new packet)
    lerp_t:      f32,
    /// Last received paint index (used to tint ghost body)
    paint:       u8,
    /// Last received variant discriminant
    variant:     u8,
    /// Last received camera mode (0=Chase, 1=WheelFL, 2=WheelFR, 3=FirstPerson, 4=FreeOrbit)
    camera_mode: u8,
}

// ---------------------------------------------------------------------------
// Timer for 20 Hz send rate
// ---------------------------------------------------------------------------

#[derive(Resource)]
struct SendTimer(Timer);

// ---------------------------------------------------------------------------
// UI components
// ---------------------------------------------------------------------------

#[derive(Component)]
struct MpPanel;

#[derive(Component)]
struct MpPanelRoot;

#[derive(Component)]
enum MpText {
    Status,
    PeerCount,
    RoomCode,
    Help,
    ConnectButtonLabel,
}

#[derive(Component)]
struct ConnectButton;

/// Container for the peer list inside the I-panel (rows are rebuilt each frame).
#[derive(Component)]
struct PeerListContainer;

/// Button that initiates a recovery attach to a peer.
#[derive(Component)]
struct RecoveryAttachButton {
    peer_id:   PeerId,
    peer_hook: HookKind,
    our_hook:  HookKind,
    kind:      RecoveryKind,
}

// ---------------------------------------------------------------------------
// UI colours (match the settings.rs palette)
// ---------------------------------------------------------------------------

const OVERLAY_BG:  Color = Color::srgba(0.0, 0.0, 0.0, 0.55);
const PANEL_BG:    Color = Color::srgba(0.05, 0.05, 0.07, 0.88);
const COLOR_TITLE: Color = Color::srgb(1.0, 0.9, 0.3);
const COLOR_BODY:  Color = Color::srgb(0.85, 0.85, 0.85);
const COLOR_HELP:  Color = Color::srgb(0.55, 0.55, 0.55);
const COLOR_BTN:   Color = Color::srgb(0.22, 0.44, 0.22);

// ---------------------------------------------------------------------------
// Startup: spawn panel (hidden)
// ---------------------------------------------------------------------------

fn spawn_panel(mut commands: Commands) {
    // Full-screen dim backdrop — children hold the panel
    let root = commands
        .spawn((
            MpPanelRoot,
            Node {
                width:           Val::Percent(100.0),
                height:          Val::Percent(100.0),
                position_type:   PositionType::Absolute,
                align_items:     AlignItems::Center,
                justify_content: JustifyContent::Center,
                display:         Display::None, // hidden until I is pressed
                ..default()
            },
            BackgroundColor(OVERLAY_BG),
        ))
        .id();

    // Centred panel
    let panel = commands
        .spawn((
            MpPanel,
            Node {
                width:          Val::Px(420.0),
                flex_direction: FlexDirection::Column,
                padding:        UiRect::all(Val::Px(24.0)),
                row_gap:        Val::Px(10.0),
                ..default()
            },
            BackgroundColor(PANEL_BG),
        ))
        .id();

    let title = commands
        .spawn((
            Text::new("MULTIPLAYER"),
            TextFont { font_size: 30.0, ..default() },
            TextColor(COLOR_TITLE),
        ))
        .id();

    let status = commands
        .spawn((
            MpText::Status,
            Text::new("Status: Disconnected"),
            TextFont { font_size: 16.0, ..default() },
            TextColor(COLOR_BODY),
        ))
        .id();

    let peer_count = commands
        .spawn((
            MpText::PeerCount,
            Text::new(""),
            TextFont { font_size: 16.0, ..default() },
            TextColor(COLOR_BODY),
        ))
        .id();

    let room_label = commands
        .spawn((
            MpText::RoomCode,
            Text::new("Room: skoffroad-1"),
            TextFont { font_size: 14.0, ..default() },
            TextColor(COLOR_BODY),
        ))
        .id();

    // Connect / Disconnect button row
    let btn = commands
        .spawn((
            ConnectButton,
            Button,
            Node {
                padding: UiRect::axes(Val::Px(16.0), Val::Px(8.0)),
                ..default()
            },
            BackgroundColor(COLOR_BTN),
        ))
        .with_children(|p| {
            p.spawn((
                MpText::ConnectButtonLabel,
                Text::new("Connect"),
                TextFont { font_size: 15.0, ..default() },
                TextColor(COLOR_BODY),
            ));
        })
        .id();

    // Peer list container — populated dynamically by update_panel_ui.
    let peer_list = commands
        .spawn((
            PeerListContainer,
            Node {
                flex_direction: FlexDirection::Column,
                row_gap:        Val::Px(4.0),
                ..default()
            },
        ))
        .id();

    let help = commands
        .spawn((
            MpText::Help,
            Text::new(
                "[ I ] close panel\n\
                 Set SKOFFROAD_SIGNALING_URL to override signaling server.\n\
                 Set SKOFFROAD_TURN_URL / USERNAME / PASSWORD for TURN.",
            ),
            TextFont { font_size: 12.0, ..default() },
            TextColor(COLOR_HELP),
        ))
        .id();

    commands
        .entity(panel)
        .add_children(&[title, status, peer_count, room_label, btn, peer_list, help]);
    commands.entity(root).add_children(&[panel]);
}

// ---------------------------------------------------------------------------
// Toggle panel with I key
// ---------------------------------------------------------------------------

fn toggle_panel(
    keys:      Res<ButtonInput<KeyCode>>,
    mut roots: Query<&mut Node, With<MpPanelRoot>>,
) {
    if keys.just_pressed(KeyCode::KeyI) {
        for mut node in &mut roots {
            node.display = match node.display {
                Display::None => Display::Flex,
                _             => Display::None,
            };
        }
    }
}

// ---------------------------------------------------------------------------
// Update socket state machine
// ---------------------------------------------------------------------------

fn update_socket(
    mut mp:    ResMut<MultiplayerState>,
    socket:    Option<ResMut<MatchboxSocket>>,
    time:      Res<Time>,
    mut ghosts: ResMut<PeerGhosts>,
    mut commands: Commands,
    mut meshes:   ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let Some(mut socket) = socket else {
        // No socket resource: we are Disconnected (or Failed).
        // Advance the since_secs timer in Failed state.
        match &mut *mp {
            MultiplayerState::Failed { since_secs, .. } => {
                *since_secs += time.delta_secs();
                // Auto-clear after 10 s so the status resets gracefully.
                if *since_secs > 10.0 {
                    *mp = MultiplayerState::Disconnected;
                }
            }
            MultiplayerState::Connecting { since_secs } => {
                // Lost socket resource while connecting — mark failed.
                let elapsed = *since_secs;
                *mp = MultiplayerState::Failed {
                    reason: format!("Lost connection after {elapsed:.1}s"),
                    since_secs: 0.0,
                };
            }
            _ => {}
        }
        return;
    };

    // Drain peer-state events (new peers join / leave)
    for (peer_id, peer_state) in socket.update_peers() {
        match peer_state {
            PeerState::Connected => {
                info!("multiplayer: peer connected: {peer_id:?}");
                // Spawn ghost chassis for this peer
                let ghost = spawn_ghost(
                    peer_id,
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                );
                ghosts.entries.insert(
                    peer_id,
                    GhostEntry {
                        entity:      ghost,
                        target:      Transform::default(),
                        current:     Transform::default(),
                        lerp_t:      1.0, // already at target
                        paint:       0,
                        variant:     0,
                        camera_mode: 0,
                    },
                );
            }
            PeerState::Disconnected => {
                info!("multiplayer: peer disconnected: {peer_id:?}");
                if let Some(entry) = ghosts.entries.remove(&peer_id) {
                    commands.entity(entry.entity).despawn();
                }
            }
        }
    }

    // Compute connected peer count
    let peer_count = socket.connected_peers().count() as u8;

    // Update state machine
    match &mut *mp {
        MultiplayerState::Connecting { since_secs } => {
            *since_secs += time.delta_secs();
            if peer_count > 0 {
                *mp = MultiplayerState::InRoom { peer_count };
            } else if *since_secs > 30.0 {
                // Timeout after 30 s without any peer.
                *mp = MultiplayerState::InRoom { peer_count: 0 };
                // Stay in room — signaling succeeded, just waiting for other players.
            }
        }
        MultiplayerState::InRoom { peer_count: pc } => {
            *pc = peer_count;
        }
        _ => {
            // Socket exists but state says disconnected/failed — promote to Connecting.
            *mp = MultiplayerState::Connecting { since_secs: 0.0 };
        }
    }
}

// ---------------------------------------------------------------------------
// Send our chassis state at 20 Hz
// ---------------------------------------------------------------------------

fn send_chassis_state(
    time:       Res<Time>,
    mut timer:  ResMut<SendTimer>,
    mut socket: Option<ResMut<MatchboxSocket>>,
    chassis_q:  Query<(&Transform, &avian3d::prelude::LinearVelocity, &avian3d::prelude::AngularVelocity), With<Chassis>>,
    livery:     Res<LiveryState>,
    variant:    Res<VehicleVariant>,
    mp:         Res<MultiplayerState>,
    cam_modes:  Res<CameraModesState>,
) {
    // Only send while in a room or connecting
    match &*mp {
        MultiplayerState::InRoom { .. } | MultiplayerState::Connecting { .. } => {}
        _ => return,
    }

    timer.0.tick(time.delta());
    if !timer.0.just_finished() {
        return;
    }

    let Some(ref mut socket) = socket else { return };

    let Ok((tf, lin_vel, ang_vel)) = chassis_q.single() else { return };

    let packet = ChassisPacket {
        translation:  tf.translation.to_array(),
        rotation:     [tf.rotation.x, tf.rotation.y, tf.rotation.z, tf.rotation.w],
        linear_vel:   lin_vel.0.to_array(),
        angular_vel:  ang_vel.0.to_array(),
        paint_index:  livery.current,
        variant_disc: variant_to_disc(*variant),
        camera_mode:  cam_mode_to_u8(cam_modes.mode),
    };

    let bytes = match bincode::encode_to_vec(packet, bincode::config::standard()) {
        Ok(b)  => b,
        Err(e) => { warn!("multiplayer: encode error: {e}"); return; }
    };

    let peers: Vec<PeerId> = socket.connected_peers().collect();
    for peer in peers {
        socket.channel_mut(CHANNEL_STATE).send(bytes.clone().into(), peer);
    }
}

// ---------------------------------------------------------------------------
// Receive chassis state from peers
// ---------------------------------------------------------------------------

fn recv_chassis_state(
    mut socket:  Option<ResMut<MatchboxSocket>>,
    mut ghosts:  ResMut<PeerGhosts>,
    mp:          Res<MultiplayerState>,
    mut spectate: ResMut<SpectateState>,
) {
    match &*mp {
        MultiplayerState::InRoom { .. } | MultiplayerState::Connecting { .. } => {}
        _ => return,
    }

    let Some(ref mut socket) = socket else { return };

    for (peer_id, bytes) in socket.channel_mut(CHANNEL_STATE).receive() {
        let Ok((pkt, _)) = bincode::decode_from_slice::<ChassisPacket, _>(
            &bytes,
            bincode::config::standard(),
        ) else {
            warn!("multiplayer: decode error from {peer_id:?}");
            continue;
        };

        let t = Vec3::from_array(pkt.translation);
        let r = Quat::from_array(pkt.rotation);
        let new_target = Transform {
            translation: t,
            rotation:    r,
            scale:       Vec3::ONE,
        };

        if let Some(entry) = ghosts.entries.get_mut(&peer_id) {
            // Snap current to old target before resetting (keeps lerp from
            // ever teleporting backward on the very first packet).
            entry.current = Transform {
                translation: entry.current.translation,
                rotation:    entry.current.rotation,
                scale:       Vec3::ONE,
            };
            entry.target      = new_target;
            entry.lerp_t      = 0.0; // restart lerp
            entry.paint       = pkt.paint_index;
            entry.variant     = pkt.variant_disc;
            entry.camera_mode = pkt.camera_mode;
        }

        // Propagate camera mode to SpectateState if we are watching this peer.
        if spectate.target_peer == Some(peer_id) {
            spectate.target_cam_mode = pkt.camera_mode;
        }
    }
}

// ---------------------------------------------------------------------------
// Lerp ghost transforms toward their received targets
// ---------------------------------------------------------------------------

fn lerp_ghosts(
    time:       Res<Time>,
    mut ghosts: ResMut<PeerGhosts>,
    mut xforms: Query<&mut Transform, Without<Chassis>>,
) {
    let dt = time.delta_secs();

    for entry in ghosts.entries.values_mut() {
        // Advance lerp parameter: complete in LERP_DURATION_S seconds.
        entry.lerp_t = (entry.lerp_t + dt / LERP_DURATION_S).min(1.0);
        let t = entry.lerp_t;

        entry.current.translation = entry.current.translation.lerp(entry.target.translation, t);
        entry.current.rotation    = entry.current.rotation.slerp(entry.target.rotation, t);

        if let Ok(mut tf) = xforms.get_mut(entry.entity) {
            *tf = entry.current;
        }
    }
}

// ---------------------------------------------------------------------------
// Despawn ghosts for peers we no longer track (safety cleanup)
// ---------------------------------------------------------------------------

fn cleanup_disconnected_ghosts(
    mut commands: Commands,
    mut ghosts:   ResMut<PeerGhosts>,
    existing:     Query<Entity, With<GhostMarker>>,
) {
    let tracked: std::collections::HashSet<Entity> =
        ghosts.entries.values().map(|e| e.entity).collect();

    for entity in &existing {
        if !tracked.contains(&entity) {
            commands.entity(entity).despawn();
        }
    }

    // Also prune stale entries whose entity has already been despawned.
    ghosts.entries.retain(|_, entry| existing.get(entry.entity).is_ok());
}

// ---------------------------------------------------------------------------
// Update panel UI text
// ---------------------------------------------------------------------------

fn update_panel_ui(
    mp:        Res<MultiplayerState>,
    cfg:       Res<MultiplayerConfig>,
    ghosts:    Res<PeerGhosts>,
    spectate:  Res<SpectateState>,
    recovery:  Option<Res<BuddyRecoveryState>>,
    mut texts: Query<(&MpText, &mut Text, &mut TextColor)>,
    peer_list_q: Query<(Entity, &Children), With<PeerListContainer>>,
    mut commands: Commands,
) {
    // Always update peer rows (they change whenever ghosts / spectate change).
    // Only skip text updates when neither mp nor cfg changed.
    let texts_changed = mp.is_changed() || cfg.is_changed();

    if texts_changed {
        let (status_str, peer_str, status_color, btn_label) = match &*mp {
            MultiplayerState::Disconnected => (
                "● Disconnected — click Connect to join the room".to_string(),
                String::new(),
                Color::srgb(0.85, 0.40, 0.40),
                "Connect",
            ),
            MultiplayerState::Connecting { since_secs } => (
                format!("● Connecting… ({since_secs:.1}s)"),
                String::new(),
                Color::srgb(0.95, 0.80, 0.30),
                "Cancel",
            ),
            MultiplayerState::InRoom { peer_count } => {
                let extra = if *peer_count == 0 {
                    "  (alone — share the page URL to invite friends)"
                } else {
                    ""
                };
                (
                    format!("● In room{extra}"),
                    format!("Peers: {peer_count}"),
                    Color::srgb(0.45, 0.85, 0.45),
                    "Disconnect",
                )
            }
            MultiplayerState::Failed { reason, since_secs } => (
                format!("● Failed ({since_secs:.0}s) — {reason}"),
                String::new(),
                Color::srgb(0.95, 0.40, 0.40),
                "Retry",
            ),
        };

        for (label, mut text, mut color) in &mut texts {
            match label {
                MpText::Status => {
                    text.0 = status_str.clone();
                    color.0 = status_color;
                }
                MpText::PeerCount => text.0 = peer_str.clone(),
                MpText::RoomCode  => {
                    let room = cfg.signaling_url.rsplit('/').next().unwrap_or("?");
                    text.0 = format!("Room: {room}");
                }
                MpText::ConnectButtonLabel => text.0 = btn_label.to_string(),
                MpText::Help => { /* static */ }
            }
        }
    }

    // Rebuild peer list rows if ghosts, spectate, or recovery state changed.
    let recovery_changed = recovery.as_ref().map(|r| r.is_changed()).unwrap_or(false);
    if !ghosts.is_changed() && !spectate.is_changed() && !recovery_changed {
        return;
    }

    let Ok((list_entity, children)) = peer_list_q.single() else { return };

    // Despawn all existing child rows.
    for child in children.iter() {
        commands.entity(child).despawn();
    }

    // Re-spawn one row per connected ghost.
    let spectating   = spectate.target_peer;
    let recovering   = recovery.as_ref().and_then(|r| r.active.as_ref().map(|c| c.peer_id));
    let btn_color    = Color::srgb(0.18, 0.36, 0.50);
    let btn_active   = Color::srgb(0.72, 0.48, 0.10);
    let btn_recovery = Color::srgb(0.50, 0.25, 0.05);

    let mut new_children: Vec<Entity> = Vec::new();

    for (peer_id, _entry) in &ghosts.entries {
        let id_str   = format!("{peer_id:?}");
        let short_id: String = id_str.chars().filter(|c| c.is_alphanumeric()).take(8).collect();

        // --- Row 1: peer label + spectate button ---
        let is_spectating   = spectating == Some(*peer_id);
        let btn_label       = if is_spectating { "Exit" } else { "Spectate" };
        let row_btn_color   = if is_spectating { btn_active } else { btn_color };

        let label_text = commands
            .spawn((
                Text::new(format!("Peer {short_id}")),
                TextFont { font_size: 13.0, ..default() },
                TextColor(COLOR_BODY),
            ))
            .id();

        let btn_text = commands
            .spawn((
                Text::new(btn_label),
                TextFont { font_size: 13.0, ..default() },
                TextColor(COLOR_BODY),
            ))
            .id();

        let spectate_btn = commands
            .spawn((
                SpectateButton { peer_id: *peer_id },
                Button,
                Node {
                    padding: UiRect::axes(Val::Px(10.0), Val::Px(4.0)),
                    ..default()
                },
                BackgroundColor(row_btn_color),
            ))
            .add_child(btn_text)
            .id();

        let row1 = commands
            .spawn((
                Node {
                    flex_direction:  FlexDirection::Row,
                    align_items:     AlignItems::Center,
                    justify_content: JustifyContent::SpaceBetween,
                    width:           Val::Percent(100.0),
                    ..default()
                },
            ))
            .add_children(&[label_text, spectate_btn])
            .id();

        new_children.push(row1);

        // --- Row 2: Attach Winch buttons (skip if already in recovery with this peer) ---
        if recovering != Some(*peer_id) {
            // Winch row: [Front] [Rear] [Cage]
            let winch_label = commands
                .spawn((
                    Text::new("Winch →"),
                    TextFont { font_size: 12.0, ..default() },
                    TextColor(Color::srgb(0.9, 0.75, 0.2)),
                ))
                .id();

            let mut winch_buttons: Vec<Entity> = vec![winch_label];
            for hook in [HookKind::FrontHook, HookKind::RearHook, HookKind::CageHook] {
                let btn_txt = commands
                    .spawn((
                        Text::new(hook.label()),
                        TextFont { font_size: 12.0, ..default() },
                        TextColor(COLOR_BODY),
                    ))
                    .id();
                let btn = commands
                    .spawn((
                        RecoveryAttachButton {
                            peer_id:   *peer_id,
                            peer_hook: hook,
                            our_hook:  HookKind::FrontHook,
                            kind:      RecoveryKind::Winch,
                        },
                        Button,
                        Node {
                            padding: UiRect::axes(Val::Px(6.0), Val::Px(3.0)),
                            margin:  UiRect::left(Val::Px(4.0)),
                            ..default()
                        },
                        BackgroundColor(btn_recovery),
                    ))
                    .add_child(btn_txt)
                    .id();
                winch_buttons.push(btn);
            }

            let winch_row = commands
                .spawn((Node {
                    flex_direction: FlexDirection::Row,
                    align_items:    AlignItems::Center,
                    column_gap:     Val::Px(2.0),
                    padding:        UiRect::left(Val::Px(12.0)),
                    ..default()
                },))
                .add_children(&winch_buttons)
                .id();
            new_children.push(winch_row);

            // Tow strap row
            let tow_label = commands
                .spawn((
                    Text::new("Tow →"),
                    TextFont { font_size: 12.0, ..default() },
                    TextColor(Color::srgb(1.0, 0.5, 0.15)),
                ))
                .id();

            let mut tow_buttons: Vec<Entity> = vec![tow_label];
            for hook in [HookKind::FrontHook, HookKind::RearHook, HookKind::CageHook] {
                let btn_txt = commands
                    .spawn((
                        Text::new(hook.label()),
                        TextFont { font_size: 12.0, ..default() },
                        TextColor(COLOR_BODY),
                    ))
                    .id();
                let btn = commands
                    .spawn((
                        RecoveryAttachButton {
                            peer_id:   *peer_id,
                            peer_hook: hook,
                            our_hook:  HookKind::RearHook,
                            kind:      RecoveryKind::TowStrap,
                        },
                        Button,
                        Node {
                            padding: UiRect::axes(Val::Px(6.0), Val::Px(3.0)),
                            margin:  UiRect::left(Val::Px(4.0)),
                            ..default()
                        },
                        BackgroundColor(btn_recovery),
                    ))
                    .add_child(btn_txt)
                    .id();
                tow_buttons.push(btn);
            }

            let tow_row = commands
                .spawn((Node {
                    flex_direction: FlexDirection::Row,
                    align_items:    AlignItems::Center,
                    column_gap:     Val::Px(2.0),
                    padding:        UiRect::left(Val::Px(12.0)),
                    ..default()
                },))
                .add_children(&tow_buttons)
                .id();
            new_children.push(tow_row);
        } else {
            // Show "Recovery active" label
            let active_label = commands
                .spawn((
                    Text::new("  [Recovery active — Esc to detach]"),
                    TextFont { font_size: 12.0, ..default() },
                    TextColor(Color::srgb(0.4, 0.9, 0.4)),
                ))
                .id();
            new_children.push(active_label);
        }
    }

    if !new_children.is_empty() {
        commands.entity(list_entity).add_children(&new_children);
    }
}

// ---------------------------------------------------------------------------
// Connect / Disconnect button handler
// ---------------------------------------------------------------------------

fn handle_connect_button(
    interaction_q: Query<&Interaction, (Changed<Interaction>, With<ConnectButton>)>,
    cfg:    Res<MultiplayerConfig>,
    mut commands: Commands,
    mut mp: ResMut<MultiplayerState>,
) {
    for interaction in &interaction_q {
        if *interaction != Interaction::Pressed {
            continue;
        }

        match &*mp {
            MultiplayerState::Disconnected | MultiplayerState::Failed { .. } => {
                // Build and open socket
                let builder = build_socket_builder(&cfg);
                commands.open_socket(builder);
                let url = cfg.signaling_url.clone();
                *mp = MultiplayerState::Connecting { since_secs: 0.0 };
                info!("multiplayer: connecting to {url}");
            }
            MultiplayerState::Connecting { .. } | MultiplayerState::InRoom { .. } => {
                // Disconnect
                commands.close_socket();
                *mp = MultiplayerState::Disconnected;
                info!("multiplayer: disconnected by user");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Config input handling (TURN / signaling URL edited via env vars on this
// sprint; live text-field editing is done in the next UI sprint)
// ---------------------------------------------------------------------------

fn handle_config_inputs(
    // Placeholder — live text-field editing requires egui or Bevy text input
    // which we'll add in Sprint 50 UI pass. For now, config is loaded at
    // startup from env vars / platform_storage.
) {
}

// ---------------------------------------------------------------------------
// Ghost spawning helper
// ---------------------------------------------------------------------------

/// Marker component for ghost chassis entities.
#[derive(Component)]
pub struct GhostMarker {
    pub peer_id: PeerId,
}

/// Spawn a semi-transparent ghost chassis and a peer-ID label above it.
fn spawn_ghost(
    peer_id:   PeerId,
    commands:  &mut Commands,
    meshes:    &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) -> Entity {
    // Short peer ID label (first 8 hex chars)
    let id_str = format!("{peer_id:?}");
    let short_id: String = id_str.chars().filter(|c| c.is_alphanumeric()).take(8).collect();

    // Ghost body material: slightly translucent, neutral grey with a faint
    // tint that will be updated when paint packets arrive.
    let ghost_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(0.55, 0.55, 0.60, GHOST_ALPHA),
        alpha_mode: AlphaMode::Blend,
        ..default()
    });

    // Simple box silhouette representing the chassis (same proportions as
    // the Jeep TJ body used elsewhere in vehicle.rs).
    let body_mesh = meshes.add(Cuboid::new(1.80, 0.70, 3.80));

    // Root ghost entity
    let root = commands
        .spawn((
            GhostMarker { peer_id },
            Transform::default(),
            Visibility::default(),
            InheritedVisibility::default(),
            ViewVisibility::default(),
        ))
        .id();

    // Body mesh child
    let body = commands
        .spawn((
            Mesh3d(body_mesh),
            MeshMaterial3d(ghost_mat),
            Transform::from_xyz(0.0, 0.35, 0.0),
        ))
        .id();

    // Peer ID text label above the ghost
    let label = commands
        .spawn((
            Text2d::new(short_id),
            TextFont { font_size: 14.0, ..default() },
            TextColor(Color::srgba(1.0, 1.0, 0.7, 0.9)),
            Transform::from_xyz(0.0, 1.5, 0.0),
        ))
        .id();

    commands.entity(root).add_children(&[body, label]);
    root
}

// ---------------------------------------------------------------------------
// Socket builder — applies ICE / TURN config
// ---------------------------------------------------------------------------

fn build_socket_builder(cfg: &MultiplayerConfig) -> WebRtcSocketBuilder {
    // Assemble ICE servers: always include STUN, optionally add TURN.
    let mut stun_urls: Vec<String> = STUN_URLS.iter().map(|s| s.to_string()).collect();

    let ice_server = if !cfg.turn_url.is_empty() {
        // Use TURN + STUN together via multiple RtcIceServerConfig entries.
        // matchbox_socket 0.14 takes a single RtcIceServerConfig, so we put
        // all URLs in one config (STUN URLs go in `urls`, TURN is separate).
        // For full STUN+TURN, put TURN URL alongside STUN URLs and supply creds.
        stun_urls.push(cfg.turn_url.clone());
        RtcIceServerConfig {
            urls:       stun_urls,
            username:   Some(cfg.turn_username.clone()).filter(|s| !s.is_empty()),
            credential: Some(cfg.turn_password.clone()).filter(|s| !s.is_empty()),
        }
    } else {
        RtcIceServerConfig {
            urls:       stun_urls,
            username:   None,
            credential: None,
        }
    };

    WebRtcSocketBuilder::new(&cfg.signaling_url)
        .ice_server(ice_server)
        .add_channel(ChannelConfig::unreliable()) // channel 0: chassis state
        .add_channel(ChannelConfig::reliable())   // channel 1: voice SDP / ICE signaling
        .add_channel(ChannelConfig::reliable())   // channel 2: recovery (Sprint 55)
        .add_channel(ChannelConfig::reliable())   // channel 3: hillclimb leaderboard (Sprint 56)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Priority-resolved signaling URL.
fn resolve_signaling_url() -> String {
    // 1. Env var
    #[cfg(not(target_arch = "wasm32"))]
    if let Ok(url) = std::env::var(ENV_SIGNALING_URL) {
        if !url.is_empty() {
            return url;
        }
    }

    // 2. platform_storage
    if let Some(json) = platform_storage::read_string(STORAGE_SIGNALING_KEY) {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&json) {
            if let Some(url) = v.get("url").and_then(|u| u.as_str()) {
                if !url.is_empty() {
                    return url.to_string();
                }
            }
        }
    }

    // 3. Default
    DEFAULT_SIGNALING_URL.to_string()
}

/// Priority-resolved TURN config: (url, username, password).
fn resolve_turn() -> (String, String, String) {
    // 1. Env vars
    #[cfg(not(target_arch = "wasm32"))]
    {
        let url  = std::env::var(ENV_TURN_URL).unwrap_or_default();
        let user = std::env::var(ENV_TURN_USERNAME).unwrap_or_default();
        let pass = std::env::var(ENV_TURN_PASSWORD).unwrap_or_default();
        if !url.is_empty() {
            return (url, user, pass);
        }
    }

    // 2. platform_storage
    if let Some(json) = platform_storage::read_string(STORAGE_TURN_KEY) {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&json) {
            let url  = v.get("url").and_then(|x| x.as_str()).unwrap_or("").to_string();
            let user = v.get("username").and_then(|x| x.as_str()).unwrap_or("").to_string();
            let pass = v.get("password").and_then(|x| x.as_str()).unwrap_or("").to_string();
            if !url.is_empty() {
                return (url, user, pass);
            }
        }
    }

    // 3. None — STUN-only fallback
    (String::new(), String::new(), String::new())
}

/// Map CameraMode to its wire byte (matches spectate.rs decoder).
fn cam_mode_to_u8(mode: CameraMode) -> u8 {
    match mode {
        CameraMode::Chase       => 0,
        CameraMode::WheelFL     => 1,
        CameraMode::WheelFR     => 2,
        CameraMode::FirstPerson => 3,
        CameraMode::FreeOrbit   => 4,
    }
}

/// Map VehicleVariant to its wire discriminant (u8).
fn variant_to_disc(v: VehicleVariant) -> u8 {
    match v {
        VehicleVariant::JeepTJ      => 0,
        VehicleVariant::FordBronco  => 1,
        VehicleVariant::Pickup      => 2,
        VehicleVariant::Hummer      => 3,
        VehicleVariant::Buggy       => 4,
        VehicleVariant::HighlandSK  => 5,
        VehicleVariant::DuneSkipper => 6,
        VehicleVariant::HaulerSK    => 7,
    }
}

// ---------------------------------------------------------------------------
// Recovery attach button handler (Sprint 55)
// ---------------------------------------------------------------------------

fn handle_recovery_attach_buttons(
    interaction_q: Query<(&Interaction, &RecoveryAttachButton), Changed<Interaction>>,
    mut recovery:  Option<ResMut<BuddyRecoveryState>>,
    mut socket:    Option<ResMut<MatchboxSocket>>,
) {
    for (interaction, btn) in &interaction_q {
        if *interaction != Interaction::Pressed { continue; }

        let Some(ref mut recovery) = recovery else { continue };
        let Some(ref mut socket)   = socket   else { continue };

        br::request_recovery(
            btn.our_hook,
            btn.peer_id,
            btn.peer_hook,
            btn.kind,
            recovery,
            socket,
        );
    }
}
