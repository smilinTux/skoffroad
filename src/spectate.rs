// Sprint 53 — Spectate mode
//
// Allows a player to slave their local Camera3d to a remote peer's ghost
// chassis, replicating the peer's active camera mode (Chase, Cockpit, etc.)
// while the local player's own chassis continues to drive normally.
//
// Public API:
//   SpectatePlugin
//   SpectateState  (Resource)
//
// Integration points:
//   multiplayer.rs – spawns SpectateButton per peer row in the I-panel;
//                    ghost entities carry GhostMarker { peer_id }.
//   camera_modes.rs – camera mode byte constants mirror CameraMode discriminants.
//
// Camera mode byte encoding (must match camera_modes.rs CameraMode order):
//   0 = Chase  (default)
//   1 = WheelFL
//   2 = WheelFR
//   3 = FirstPerson
//   4 = FreeOrbit

use bevy::prelude::*;

use crate::multiplayer::{GhostMarker, PeerId};
use crate::vehicle::Chassis;

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct SpectatePlugin;

impl Plugin for SpectatePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(SpectateState::default())
            .add_systems(Update, (handle_spectate_buttons, apply_spectate_camera).chain());
    }
}

// ---------------------------------------------------------------------------
// Public resource
// ---------------------------------------------------------------------------

/// Current spectate target (None = watching own chassis normally).
#[derive(Resource, Default)]
pub struct SpectateState {
    /// The peer whose ghost we are slaved to, if any.
    pub target_peer: Option<PeerId>,
    /// Latest camera mode byte received from that peer.
    pub target_cam_mode: u8,
}

// ---------------------------------------------------------------------------
// UI component markers (spawned by multiplayer.rs)
// ---------------------------------------------------------------------------

/// Marker on the Spectate / Exit button for a specific peer.
#[derive(Component)]
pub struct SpectateButton {
    pub peer_id: PeerId,
}

/// Marker on the "Exit Spectate" button that lives in the I-panel footer.
#[derive(Component)]
pub struct ExitSpectateButton;

// ---------------------------------------------------------------------------
// System: handle button interactions
// ---------------------------------------------------------------------------

fn handle_spectate_buttons(
    mut spectate:      ResMut<SpectateState>,
    keys:              Res<ButtonInput<KeyCode>>,
    spec_btns:         Query<(&Interaction, &SpectateButton), Changed<Interaction>>,
    exit_btns:         Query<&Interaction, (Changed<Interaction>, With<ExitSpectateButton>)>,
) {
    // I key closes panel (multiplayer.rs handles display:none) → also exit spectate.
    if keys.just_pressed(KeyCode::KeyI) {
        spectate.target_peer    = None;
        spectate.target_cam_mode = 0;
        return;
    }

    // "Exit Spectate" button
    for interaction in &exit_btns {
        if *interaction == Interaction::Pressed {
            spectate.target_peer    = None;
            spectate.target_cam_mode = 0;
        }
    }

    // Per-peer "Spectate" button
    for (interaction, btn) in &spec_btns {
        if *interaction == Interaction::Pressed {
            if spectate.target_peer == Some(btn.peer_id) {
                // Toggle off
                spectate.target_peer    = None;
                spectate.target_cam_mode = 0;
            } else {
                spectate.target_peer    = Some(btn.peer_id);
                spectate.target_cam_mode = 0; // will be refreshed from ghost entry
            }
        }
    }
}

// ---------------------------------------------------------------------------
// System: override Camera3d transform when spectating
// ---------------------------------------------------------------------------

fn apply_spectate_camera(
    spectate:  Res<SpectateState>,
    ghosts:    Query<(&GhostMarker, &Transform)>,
    mut cam_q: Query<&mut Transform, (With<Camera3d>, Without<Chassis>, Without<GhostMarker>)>,
    time:      Res<Time>,
) {
    let Some(target_id) = spectate.target_peer else { return };

    // Find the ghost entity for this peer.
    let Some((_, ghost_tf)) = ghosts.iter().find(|(g, _)| g.peer_id == target_id) else {
        return;
    };

    let Ok(mut cam) = cam_q.single_mut() else { return };

    let chassis_pos = ghost_tf.translation;
    let chassis_rot = ghost_tf.rotation;

    let chassis_right = chassis_rot * Vec3::X;
    let chassis_fwd   = chassis_rot * Vec3::NEG_Z;
    let chassis_up    = chassis_rot * Vec3::Y;

    let new_tf = match spectate.target_cam_mode {
        // Chase: 8 m behind, 3 m up, looking at chassis +1 m
        0 => {
            let cam_pos = chassis_pos - chassis_fwd * 8.0 + chassis_up * 3.0;
            let look_at = chassis_pos + chassis_up * 1.0;
            Transform::from_translation(cam_pos).looking_at(look_at, Vec3::Y)
        }

        // WheelFL
        1 => {
            let cam_pos = chassis_pos
                + chassis_right * (-2.2)
                + chassis_fwd   * 0.6
                + chassis_up    * 0.2;
            let look_target = chassis_pos
                + chassis_right * (-1.1)
                + chassis_fwd   * 1.4
                + chassis_up    * (-0.4);
            Transform::from_translation(cam_pos).looking_at(look_target, Vec3::Y)
        }

        // WheelFR
        2 => {
            let cam_pos = chassis_pos
                + chassis_right * 2.2
                + chassis_fwd   * 0.6
                + chassis_up    * 0.2;
            let look_target = chassis_pos
                + chassis_right * 1.1
                + chassis_fwd   * 1.4
                + chassis_up    * (-0.4);
            Transform::from_translation(cam_pos).looking_at(look_target, Vec3::Y)
        }

        // FirstPerson
        3 => {
            let cam_pos = chassis_pos
                + chassis_up  * 0.6
                + chassis_fwd * (-0.5);
            Transform::from_translation(cam_pos)
                .looking_at(cam_pos + chassis_fwd * 5.0, chassis_up)
        }

        // FreeOrbit
        4 | _ => {
            let angle   = time.elapsed_secs() * 0.3;
            let cam_pos = chassis_pos
                + Vec3::new(angle.cos() * 8.0, 4.0, angle.sin() * 8.0);
            let look_target = chassis_pos + Vec3::Y * 1.0;
            Transform::from_translation(cam_pos).looking_at(look_target, Vec3::Y)
        }
    };

    *cam = new_tf;
}
