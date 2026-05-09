// Sprint 55 — Peer-to-peer buddy recovery (winch + tow strap).
//
// Two recovery mechanics:
//   1. Tow Strap — rigid 4 m link; pulls victim when rescuer drives.
//   2. Winch     — variable-length cable; rescuer holds U to retract (~0.5 m/s).
//
// Hook points are orange spheres on each chassis (chassis-local space).
// Networking uses matchbox channel 2 (reliable, ordered) with RecoveryMessage.
// In single-player the existing winch.rs terrain-anchor flow is untouched.
//
// Physics authority: per-client — each side applies force to its own chassis.
// Cable render: thin cylinder from local-hook to remote-hook via Ghost transform.
//
// UI:
//   I-panel rows (added below and called from multiplayer.rs's update_panel_ui)
//   show Attach Winch / Attach Tow buttons per peer.
//   Bottom-right HUD shows active connection + "U: retract  Esc: detach".

use bevy::prelude::*;
use avian3d::prelude::*;
use bincode::{Decode, Encode};

use crate::multiplayer::{GhostMarker, PeerId};
use crate::vehicle::{Chassis, VehicleRoot};

// ---------------------------------------------------------------------------
// Channel
// ---------------------------------------------------------------------------

/// Matchbox channel index for reliable recovery signaling.
pub const CHANNEL_RECOVERY: usize = 2;

// ---------------------------------------------------------------------------
// Hook definitions
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq, Encode, Decode)]
pub enum HookKind {
    FrontHook,
    RearHook,
    CageHook,
}

impl HookKind {
    pub fn label(self) -> &'static str {
        match self {
            HookKind::FrontHook => "Front",
            HookKind::RearHook  => "Rear",
            HookKind::CageHook  => "Cage",
        }
    }
}

/// Chassis-local positions for each hook (shared by all variants).
pub const SKRAMBLER_HOOKS: &[(HookKind, Vec3)] = &[
    (HookKind::FrontHook, Vec3::new(0.0, -0.20, -2.10)),
    (HookKind::RearHook,  Vec3::new(0.0, -0.20,  2.30)),
    (HookKind::CageHook,  Vec3::new(0.0,  1.30,  0.00)),
];

/// World-space position of a hook given the chassis transform.
fn hook_world_pos(chassis_tf: &Transform, hook: HookKind) -> Vec3 {
    for (k, local) in SKRAMBLER_HOOKS {
        if *k == hook {
            return chassis_tf.transform_point(*local);
        }
    }
    chassis_tf.translation
}

// ---------------------------------------------------------------------------
// Recovery kind
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq, Encode, Decode)]
pub enum RecoveryKind {
    Winch,
    TowStrap,
}

// ---------------------------------------------------------------------------
// Network messages (channel 2, reliable)
//
// We do NOT encode PeerId in the messages — the socket tells us the sender.
// Variants use tuple fields (not struct fields) for broadest bincode compat.
// attach_id is a u32 local handle matching Accept to Request.
// ---------------------------------------------------------------------------

/// Wire-format tag byte for each message kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Encode, Decode)]
#[repr(u8)]
enum MsgTag {
    Request = 0,
    Accept  = 1,
    Reject  = 2,
    Retract = 3,
    Detach  = 4,
}

/// Tuple payload for a Request message.
#[derive(Clone, Copy, Debug, Encode, Decode)]
struct MsgRequest {
    target_hook: HookKind,
    our_hook:    HookKind,
    kind:        RecoveryKind,
    attach_id:   u32,
}

/// Tuple payload for messages that carry only an attach_id.
#[derive(Clone, Copy, Debug, Encode, Decode)]
struct MsgAttachId {
    attach_id: u32,
}

/// Decoded recovery message (constructed after parsing tag + payload).
#[derive(Clone, Debug)]
pub enum RecoveryMessage {
    Request { target_hook: HookKind, our_hook: HookKind, kind: RecoveryKind, attach_id: u32 },
    Accept  { attach_id: u32 },
    Reject,
    Retract { attach_id: u32 },
    Detach  { attach_id: u32 },
}

impl RecoveryMessage {
    /// Serialize to bytes: 1-byte tag + payload.
    pub fn encode_bytes(&self) -> Result<Vec<u8>, bincode::error::EncodeError> {
        let cfg = bincode::config::standard();
        let mut out = Vec::with_capacity(16);
        match self {
            RecoveryMessage::Request { target_hook, our_hook, kind, attach_id } => {
                out.extend(bincode::encode_to_vec(MsgTag::Request, cfg)?);
                out.extend(bincode::encode_to_vec(MsgRequest {
                    target_hook: *target_hook,
                    our_hook: *our_hook,
                    kind: *kind,
                    attach_id: *attach_id,
                }, cfg)?);
            }
            RecoveryMessage::Accept { attach_id } => {
                out.extend(bincode::encode_to_vec(MsgTag::Accept, cfg)?);
                out.extend(bincode::encode_to_vec(MsgAttachId { attach_id: *attach_id }, cfg)?);
            }
            RecoveryMessage::Reject => {
                out.extend(bincode::encode_to_vec(MsgTag::Reject, cfg)?);
            }
            RecoveryMessage::Retract { attach_id } => {
                out.extend(bincode::encode_to_vec(MsgTag::Retract, cfg)?);
                out.extend(bincode::encode_to_vec(MsgAttachId { attach_id: *attach_id }, cfg)?);
            }
            RecoveryMessage::Detach { attach_id } => {
                out.extend(bincode::encode_to_vec(MsgTag::Detach, cfg)?);
                out.extend(bincode::encode_to_vec(MsgAttachId { attach_id: *attach_id }, cfg)?);
            }
        }
        Ok(out)
    }

    /// Deserialize from bytes.
    pub fn decode_bytes(bytes: &[u8]) -> Result<Self, bincode::error::DecodeError> {
        let cfg = bincode::config::standard();
        let (tag, consumed): (MsgTag, usize) = bincode::decode_from_slice(bytes, cfg)?;
        let rest = &bytes[consumed..];
        match tag {
            MsgTag::Request => {
                let (p, _): (MsgRequest, usize) = bincode::decode_from_slice(rest, cfg)?;
                Ok(RecoveryMessage::Request {
                    target_hook: p.target_hook,
                    our_hook:    p.our_hook,
                    kind:        p.kind,
                    attach_id:   p.attach_id,
                })
            }
            MsgTag::Accept => {
                let (p, _): (MsgAttachId, usize) = bincode::decode_from_slice(rest, cfg)?;
                Ok(RecoveryMessage::Accept { attach_id: p.attach_id })
            }
            MsgTag::Reject => {
                Ok(RecoveryMessage::Reject)
            }
            MsgTag::Retract => {
                let (p, _): (MsgAttachId, usize) = bincode::decode_from_slice(rest, cfg)?;
                Ok(RecoveryMessage::Retract { attach_id: p.attach_id })
            }
            MsgTag::Detach => {
                let (p, _): (MsgAttachId, usize) = bincode::decode_from_slice(rest, cfg)?;
                Ok(RecoveryMessage::Detach { attach_id: p.attach_id })
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct BuddyRecoveryPlugin;

impl Plugin for BuddyRecoveryPlugin {
    fn build(&self, app: &mut App) {
        app
            .insert_resource(RecoveryState::default())
            .add_systems(Startup, spawn_recovery_hud)
            // Hook spheres need VehicleRoot to exist — defer to PostStartup.
            .add_systems(PostStartup, spawn_hook_spheres)
            .add_systems(
                Update,
                (
                    recv_recovery_messages,
                    handle_retract_key,
                    handle_detach_key,
                    apply_tow_strap_force,
                    update_cable_visual,
                    update_recovery_hud,
                )
                    .run_if(resource_exists::<VehicleRoot>),
            )
            .add_systems(
                PhysicsSchedule,
                apply_recovery_force
                    .after(PhysicsStepSystems::NarrowPhase)
                    .before(PhysicsStepSystems::Solver),
            );
    }
}

// ---------------------------------------------------------------------------
// State resource
// ---------------------------------------------------------------------------

#[derive(Resource, Default)]
pub struct RecoveryState {
    /// Active peer-to-peer connection.
    pub active:           Option<RecoveryConnection>,
    /// Current cable length (winch shortens; tow strap is fixed).
    pub cable_len:        f32,
    /// Request we sent, waiting for Accept/Reject.
    pub pending_request:  Option<PendingRequest>,
}

pub struct RecoveryConnection {
    pub peer_id:    PeerId,
    pub our_hook:   HookKind,
    pub peer_hook:  HookKind,
    pub kind:       RecoveryKind,
    /// True if we initiated (are the rescuer).
    pub rescuer:    bool,
    pub attach_id:  u32,
    /// Flag set by recv_recovery_messages when a Retract arrives.
    pub retracting: bool,
}

pub struct PendingRequest {
    pub peer_id:   PeerId,
    pub our_hook:  HookKind,
    pub peer_hook: HookKind,
    pub kind:      RecoveryKind,
    pub attach_id: u32,
}

// ---------------------------------------------------------------------------
// Physics constants
// ---------------------------------------------------------------------------

const TOW_STRAP_LEN:     f32 = 4.0;
const WINCH_INITIAL_LEN: f32 = 4.0;
const WINCH_RETRACT_SPEED: f32 = 0.5;
/// Newtons — enough to drag a 1500 kg chassis at ~0.5 m/s against terrain
const RECOVERY_FORCE:    f32 = 10_000.0;
const HOOK_RADIUS:       f32 = 0.08;
const CABLE_RADIUS:      f32 = 0.035;

// ---------------------------------------------------------------------------
// Attach-id counter
// ---------------------------------------------------------------------------

fn next_attach_id() -> u32 {
    use std::sync::atomic::{AtomicU32, Ordering};
    static COUNTER: AtomicU32 = AtomicU32::new(1);
    COUNTER.fetch_add(1, Ordering::Relaxed)
}

// ---------------------------------------------------------------------------
// Startup: spawn orange hook spheres as children of the chassis
// ---------------------------------------------------------------------------

#[derive(Component)]
pub struct HookSphere {
    pub hook: HookKind,
}

fn spawn_hook_spheres(
    mut commands:  Commands,
    mut meshes:    ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    vehicle:       Option<Res<VehicleRoot>>,
) {
    let Some(vehicle) = vehicle else { return };

    let mesh = meshes.add(Sphere::new(HOOK_RADIUS));
    let mat  = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.55, 0.0),
        emissive:   LinearRgba::new(0.6, 0.28, 0.0, 1.0),
        ..default()
    });

    for (hook, local_pos) in SKRAMBLER_HOOKS {
        let sphere = commands
            .spawn((
                HookSphere { hook: *hook },
                Mesh3d(mesh.clone()),
                MeshMaterial3d(mat.clone()),
                Transform::from_translation(*local_pos),
            ))
            .id();
        commands.entity(vehicle.chassis).add_child(sphere);
    }
}

// ---------------------------------------------------------------------------
// HUD — bottom-right overlay
// ---------------------------------------------------------------------------

#[derive(Component)]
struct RecoveryHud;

#[derive(Component)]
struct RecoveryHudText;

fn spawn_recovery_hud(mut commands: Commands) {
    let panel = commands
        .spawn((
            RecoveryHud,
            Node {
                position_type:  PositionType::Absolute,
                right:          Val::Px(16.0),
                bottom:         Val::Px(48.0),
                padding:        UiRect::all(Val::Px(10.0)),
                flex_direction: FlexDirection::Column,
                row_gap:        Val::Px(4.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.72)),
            Visibility::Hidden,
        ))
        .id();

    let text = commands
        .spawn((
            RecoveryHudText,
            Text::new(""),
            TextFont  { font_size: 14.0, ..default() },
            TextColor(Color::srgb(1.0, 0.85, 0.3)),
        ))
        .id();

    commands.entity(panel).add_child(text);
}

fn update_recovery_hud(
    state:     Res<RecoveryState>,
    mut panel: Query<&mut Visibility, With<RecoveryHud>>,
    mut text:  Query<&mut Text, With<RecoveryHudText>>,
) {
    let Some(conn) = &state.active else {
        for mut vis in &mut panel { *vis = Visibility::Hidden; }
        return;
    };

    for mut vis in &mut panel { *vis = Visibility::Visible; }

    let kind_label = match conn.kind {
        RecoveryKind::Winch    => "Winching",
        RecoveryKind::TowStrap => "Tow Strap",
    };
    let id_str   = format!("{:?}", conn.peer_id);
    let short_id: String = id_str.chars().filter(|c| c.is_alphanumeric()).take(8).collect();
    let hint = match conn.kind {
        RecoveryKind::Winch    => "\n[U] retract  [Esc] detach",
        RecoveryKind::TowStrap => "\n[Esc] detach",
    };

    for mut t in &mut text {
        t.0 = format!("{kind_label} {short_id} ({:.1} m){hint}", state.cable_len);
    }
}

// ---------------------------------------------------------------------------
// Receive messages from channel 2
// ---------------------------------------------------------------------------

fn recv_recovery_messages(
    mut socket: Option<ResMut<bevy_matchbox::prelude::MatchboxSocket>>,
    mut state:  ResMut<RecoveryState>,
) {
    let Some(ref mut socket) = socket else { return };

    let messages: Vec<(PeerId, Vec<u8>)> = socket
        .channel_mut(CHANNEL_RECOVERY)
        .receive()
        .into_iter()
        .map(|(p, b)| (p, b.to_vec()))
        .collect();

    for (peer_id, bytes) in messages {
        let msg = match RecoveryMessage::decode_bytes(&bytes) {
            Ok(m) => m,
            Err(e) => {
                warn!("buddy_recovery: decode error from {peer_id:?}: {e:?}");
                continue;
            }
        };

        match msg {
            RecoveryMessage::Request { target_hook, our_hook: peer_our_hook, kind, attach_id } => {
                // Auto-accept (v1 — no confirmation UI)
                if state.active.is_some() {
                    // Already connected — send Reject
                    if let Ok(b) = RecoveryMessage::Reject.encode_bytes() {
                        socket.channel_mut(CHANNEL_RECOVERY).send(b.into(), peer_id);
                    }
                    continue;
                }

                state.active = Some(RecoveryConnection {
                    peer_id,
                    our_hook:   target_hook,  // peer targeted this hook on us
                    peer_hook:  peer_our_hook, // peer's own hook
                    kind,
                    rescuer:    false,
                    attach_id,
                    retracting: false,
                });
                state.cable_len = if kind == RecoveryKind::Winch {
                    WINCH_INITIAL_LEN
                } else {
                    TOW_STRAP_LEN
                };

                // Accept
                if let Ok(b) = (RecoveryMessage::Accept { attach_id }).encode_bytes() {
                    socket.channel_mut(CHANNEL_RECOVERY).send(b.into(), peer_id);
                }
            }

            RecoveryMessage::Accept { attach_id } => {
                if let Some(pending) = state.pending_request.take() {
                    if pending.peer_id == peer_id && pending.attach_id == attach_id {
                        state.active = Some(RecoveryConnection {
                            peer_id,
                            our_hook:   pending.our_hook,
                            peer_hook:  pending.peer_hook,
                            kind:       pending.kind,
                            rescuer:    true,
                            attach_id,
                            retracting: false,
                        });
                        state.cable_len = if pending.kind == RecoveryKind::Winch {
                            WINCH_INITIAL_LEN
                        } else {
                            TOW_STRAP_LEN
                        };
                    }
                }
            }

            RecoveryMessage::Reject => {
                if state.pending_request.as_ref().map(|p| p.peer_id) == Some(peer_id) {
                    state.pending_request = None;
                }
            }

            RecoveryMessage::Retract { attach_id } => {
                if let Some(conn) = &mut state.active {
                    if conn.peer_id == peer_id && conn.attach_id == attach_id && !conn.rescuer {
                        conn.retracting = true;
                    }
                }
            }

            RecoveryMessage::Detach { .. } => {
                if state.active.as_ref().map(|c| c.peer_id) == Some(peer_id) {
                    state.active = None;
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// U key — rescuer retracts winch
// ---------------------------------------------------------------------------

fn handle_retract_key(
    keys:       Res<ButtonInput<KeyCode>>,
    mut state:  ResMut<RecoveryState>,
    mut socket: Option<ResMut<bevy_matchbox::prelude::MatchboxSocket>>,
    time:       Res<Time>,
) {
    // Scope borrows carefully
    let should_retract = {
        let Some(conn) = &state.active else { return };
        conn.kind == RecoveryKind::Winch && conn.rescuer && keys.pressed(KeyCode::KeyU)
    };
    if !should_retract { return; }

    let dt = time.delta_secs();
    state.cable_len = (state.cable_len - WINCH_RETRACT_SPEED * dt).max(0.5);

    let (peer_id, attach_id) = {
        let conn = state.active.as_ref().unwrap();
        (conn.peer_id, conn.attach_id)
    };

    if let Some(ref mut socket) = socket {
        if let Ok(b) = (RecoveryMessage::Retract { attach_id }).encode_bytes() {
            socket.channel_mut(CHANNEL_RECOVERY).send(b.into(), peer_id);
        }
    }
}

// ---------------------------------------------------------------------------
// Esc key — detach
// ---------------------------------------------------------------------------

fn handle_detach_key(
    keys:       Res<ButtonInput<KeyCode>>,
    mut state:  ResMut<RecoveryState>,
    mut socket: Option<ResMut<bevy_matchbox::prelude::MatchboxSocket>>,
) {
    if !keys.just_pressed(KeyCode::Escape) { return; }
    let Some(conn) = state.active.take() else { return };

    if let Some(ref mut socket) = socket {
        if let Ok(b) = (RecoveryMessage::Detach { attach_id: conn.attach_id }).encode_bytes() {
            socket.channel_mut(CHANNEL_RECOVERY).send(b.into(), conn.peer_id);
        }
    }
    // state.active already taken to None above
}

// ---------------------------------------------------------------------------
// Physics — apply pull force to local chassis
// ---------------------------------------------------------------------------

fn apply_recovery_force(
    mut state:     ResMut<RecoveryState>,
    vehicle:       Option<Res<VehicleRoot>>,
    mut chassis_q: Query<(Forces, &Transform), With<Chassis>>,
    ghosts_q:      Query<(&GhostMarker, &Transform)>,
) {
    let Some(vehicle) = vehicle else { return };
    let cable_len = state.cable_len;
    let Some(conn) = &mut state.active else { return };

    let Ok((mut forces, chassis_tf)) = chassis_q.get_mut(vehicle.chassis) else { return };

    let ghost_tf = ghosts_q.iter().find_map(|(gm, tf)| {
        if gm.peer_id == conn.peer_id { Some(*tf) } else { None }
    });
    let Some(ghost_tf) = ghost_tf else {
        conn.retracting = false;
        return;
    };

    let our_pos  = hook_world_pos(chassis_tf, conn.our_hook);
    let peer_pos = hook_world_pos(&ghost_tf,  conn.peer_hook);

    let delta = peer_pos - our_pos;
    let dist  = delta.length();
    if dist < 0.01 { conn.retracting = false; return; }
    let dir = delta / dist;

    let should_pull = match conn.kind {
        RecoveryKind::Winch => {
            if conn.rescuer {
                // Rescuer is pulled toward victim when cable taut
                dist > cable_len
            } else {
                // Victim is pulled toward rescuer when Retract arrives
                conn.retracting
            }
        }
        RecoveryKind::TowStrap => {
            // Both sides feel pull when strap would stretch beyond limit
            dist > TOW_STRAP_LEN
        }
    };

    if should_pull {
        let vel        = forces.linear_velocity();
        let speed_proj = vel.dot(dir);
        let scale = ((0.5 - speed_proj) / 0.5).clamp(0.0, 1.0);
        forces.apply_force(dir * RECOVERY_FORCE * scale);
    }

    conn.retracting = false;
}

// ---------------------------------------------------------------------------
// Tow strap: small countering pull on the rescuer chassis (Newton's 3rd)
// ---------------------------------------------------------------------------

fn apply_tow_strap_force(
    state:         Res<RecoveryState>,
    vehicle:       Option<Res<VehicleRoot>>,
    mut chassis_q: Query<(Forces, &Transform), With<Chassis>>,
    ghosts_q:      Query<(&GhostMarker, &Transform)>,
) {
    let Some(vehicle) = vehicle else { return };
    let Some(conn) = &state.active else { return };
    if conn.kind != RecoveryKind::TowStrap || !conn.rescuer { return; }

    let Ok((mut forces, chassis_tf)) = chassis_q.get_mut(vehicle.chassis) else { return };
    let ghost_tf = ghosts_q.iter().find_map(|(gm, tf)| {
        if gm.peer_id == conn.peer_id { Some(*tf) } else { None }
    });
    let Some(ghost_tf) = ghost_tf else { return };

    let our_pos  = hook_world_pos(chassis_tf, conn.our_hook);
    let peer_pos = hook_world_pos(&ghost_tf,  conn.peer_hook);
    let delta = peer_pos - our_pos;
    let dist  = delta.length();
    if dist <= TOW_STRAP_LEN || dist < 0.01 { return; }

    // Light drag on rescuer so it's slowed but can still drive
    let dir = delta / dist;
    let vel = forces.linear_velocity();
    let speed = vel.dot(dir);
    let scale = ((0.4 - speed) / 0.4).clamp(0.0, 1.0);
    forces.apply_force(dir * RECOVERY_FORCE * 0.25 * scale);
}

// ---------------------------------------------------------------------------
// Cable visual — straight cylinder from our hook to peer hook
// ---------------------------------------------------------------------------

#[derive(Component)]
struct RecoveryCable;

fn update_cable_visual(
    state:       Res<RecoveryState>,
    vehicle:     Res<VehicleRoot>,
    chassis_q:   Query<&Transform, With<Chassis>>,
    ghosts_q:    Query<(&GhostMarker, &Transform)>,
    mut commands: Commands,
    mut meshes:   ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut cable_ent: Local<Option<Entity>>,
    existing:    Query<Entity, With<RecoveryCable>>,
) {
    let Ok(chassis_tf) = chassis_q.get(vehicle.chassis) else { return };

    let Some(conn) = &state.active else {
        if let Some(ent) = cable_ent.take() {
            if existing.get(ent).is_ok() {
                commands.entity(ent).despawn();
            }
        }
        return;
    };

    let ghost_tf = ghosts_q.iter().find_map(|(gm, tf)| {
        if gm.peer_id == conn.peer_id { Some(*tf) } else { None }
    });
    let Some(ghost_tf) = ghost_tf else { return };

    let a = hook_world_pos(chassis_tf, conn.our_hook);
    let b = hook_world_pos(&ghost_tf,  conn.peer_hook);

    let cable_vec = b - a;
    let length = cable_vec.length();
    if length < 0.01 { return; }

    let midpoint  = (a + b) * 0.5;
    let cable_dir = cable_vec / length;
    let rotation  = Quat::from_rotation_arc(Vec3::Y, cable_dir);

    // Update existing entity if still alive
    if let Some(ent) = *cable_ent {
        if existing.get(ent).is_ok() {
            commands.entity(ent).insert(Transform {
                translation: midpoint,
                rotation,
                scale: Vec3::new(1.0, length, 1.0),
            });
            return;
        }
    }

    // Spawn new cable cylinder (height = 1.0, scaled by cable length)
    let cable_color = match conn.kind {
        RecoveryKind::Winch    => Color::srgb(0.9, 0.75, 0.2),  // gold
        RecoveryKind::TowStrap => Color::srgb(1.0, 0.35, 0.1),  // orange-red
    };

    let mesh = meshes.add(Cylinder::new(CABLE_RADIUS, 1.0));
    let mat  = materials.add(StandardMaterial {
        base_color:           cable_color,
        perceptual_roughness: 0.4,
        ..default()
    });

    let ent = commands
        .spawn((
            RecoveryCable,
            Mesh3d(mesh),
            MeshMaterial3d(mat),
            Transform {
                translation: midpoint,
                rotation,
                scale: Vec3::new(1.0, length, 1.0),
            },
            Visibility::default(),
        ))
        .id();

    *cable_ent = Some(ent);
}

// ---------------------------------------------------------------------------
// Public API — called by multiplayer.rs UI button handler
// ---------------------------------------------------------------------------

/// Initiate a recovery connection to a remote peer.
pub fn request_recovery(
    our_hook:  HookKind,
    peer_id:   PeerId,
    peer_hook: HookKind,
    kind:      RecoveryKind,
    state:     &mut RecoveryState,
    socket:    &mut bevy_matchbox::prelude::MatchboxSocket,
) {
    if state.active.is_some() || state.pending_request.is_some() { return; }

    let attach_id = next_attach_id();
    let msg = RecoveryMessage::Request {
        target_hook: peer_hook,
        our_hook,
        kind,
        attach_id,
    };

    if let Ok(bytes) = msg.encode_bytes() {
        socket.channel_mut(CHANNEL_RECOVERY).send(bytes.into(), peer_id);
    }

    state.pending_request = Some(PendingRequest {
        peer_id,
        our_hook,
        peer_hook,
        kind,
        attach_id,
    });
}
