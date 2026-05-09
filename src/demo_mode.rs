// Demo mode: when no input has occurred for 30s, auto-drive the player
// vehicle along the race path so the game becomes its own attract loop.
// Any key press exits demo mode immediately.
//
// Public API:
//   DemoModePlugin
//   DemoModeState (resource)

use bevy::prelude::*;

use crate::ai_path::{PathFollower, RacePath};
use crate::vehicle::{Chassis, DriveInput, VehicleRoot};

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct DemoModePlugin;

impl Plugin for DemoModePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DemoModeState>()
            .add_systems(
                Update,
                (
                    ensure_player_path_follower,
                    tick_idle_timer,
                    apply_demo_steering,
                    manage_demo_indicator,
                )
                    .chain(),
            );
    }
}

// ---------------------------------------------------------------------------
// Public resource
// ---------------------------------------------------------------------------

#[derive(Resource, Default)]
pub struct DemoModeState {
    pub idle_time_s: f32,
    pub active: bool,
}

// ---------------------------------------------------------------------------
// Marker components
// ---------------------------------------------------------------------------

/// Marker on the demo-mode on-screen indicator text node.
#[derive(Component)]
struct DemoIndicator;

// ---------------------------------------------------------------------------
// System: ensure the player chassis has a PathFollower (run once via Local)
// ---------------------------------------------------------------------------

fn ensure_player_path_follower(
    mut commands: Commands,
    vehicle: Option<Res<VehicleRoot>>,
    chassis_q: Query<Entity, (With<Chassis>, Without<PathFollower>)>,
    mut done: Local<bool>,
) {
    if *done {
        return;
    }
    let Some(vehicle) = vehicle else { return };

    // Insert PathFollower only if the chassis entity lacks one.
    if chassis_q.get(vehicle.chassis).is_ok() {
        commands
            .entity(vehicle.chassis)
            .insert(PathFollower::default());
        info!("demo_mode: inserted PathFollower on player chassis");
        *done = true;
    } else {
        // chassis already has a PathFollower — nothing to do
        *done = true;
    }
}

// ---------------------------------------------------------------------------
// System: idle timer + demo activation
// ---------------------------------------------------------------------------

fn tick_idle_timer(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut state: ResMut<DemoModeState>,
) {
    let dt = time.delta_secs();

    // Any non-modifier key pressed → reset timer and deactivate.
    let any_key = keys.get_pressed().any(|k| !is_modifier(*k));

    if any_key {
        if state.active {
            info!("demo_mode: exited — key pressed");
        }
        state.idle_time_s = 0.0;
        state.active = false;
    } else {
        state.idle_time_s += dt;
        // On WASM the player drives immediately after landing on the page;
        // the attract loop is confusing and undesirable in a browser context.
        // cfg-gate the activation so the 30-second timeout is native-only.
        #[cfg(not(target_arch = "wasm32"))]
        if state.idle_time_s >= 30.0 && !state.active {
            state.active = true;
            info!("demo mode engaged");
        }
    }
}

/// Returns true for modifier keys that should not break idle.
#[inline]
fn is_modifier(key: KeyCode) -> bool {
    matches!(
        key,
        KeyCode::ShiftLeft
            | KeyCode::ShiftRight
            | KeyCode::ControlLeft
            | KeyCode::ControlRight
            | KeyCode::AltLeft
            | KeyCode::AltRight
            | KeyCode::SuperLeft
            | KeyCode::SuperRight
    )
}

// ---------------------------------------------------------------------------
// System: apply demo steering to DriveInput when active
// ---------------------------------------------------------------------------

fn apply_demo_steering(
    state: Res<DemoModeState>,
    vehicle: Option<Res<VehicleRoot>>,
    race_path: Option<Res<RacePath>>,
    chassis_q: Query<(&Transform, &PathFollower), With<Chassis>>,
    mut drive_input: ResMut<DriveInput>,
) {
    if !state.active {
        return;
    }

    let Some(vehicle) = vehicle else { return };
    let Some(race_path) = race_path else { return };

    if race_path.waypoints.is_empty() {
        return;
    }

    let Ok((transform, follower)) = chassis_q.get(vehicle.chassis) else { return };

    let chassis_pos = transform.translation;
    let chassis_rot = transform.rotation;
    let chassis_fwd = (chassis_rot * Vec3::NEG_Z).normalize();
    let chassis_up  = (chassis_rot * Vec3::Y).normalize();

    // Lookahead 10 m ahead on the race path.
    let target = race_path.lookahead(follower.current_idx, 10.0);

    // Signed angle to target (mirror ai_driver logic).
    let to_target = target - chassis_pos;
    let cross = chassis_fwd.cross(to_target.normalize_or_zero());
    let raw_angle_err = cross.y.clamp(-1.0, 1.0).asin();

    // steer_gain = 1.5 (same default as AiDriver)
    const STEER_GAIN: f32 = 1.5;
    const MAX_STEER: f32  = 30_f32 * std::f32::consts::PI / 180.0;

    // Compute normalised steer in [-1, 1] space (vehicle.rs reads it as a
    // fraction of MAX_STEER_ANGLE in the suspension system).
    let steer_angle = (raw_angle_err * STEER_GAIN).clamp(-MAX_STEER, MAX_STEER);
    let steer_norm  = steer_angle / MAX_STEER;

    // Suppress unused variable warning — chassis_up reserved for future
    // banked-turn compensation.
    let _ = chassis_up;

    drive_input.drive = 0.6;
    drive_input.steer = steer_norm;
    drive_input.brake = false;
}

// ---------------------------------------------------------------------------
// System: spawn / despawn / pulse the "DEMO MODE" indicator text
// ---------------------------------------------------------------------------

fn manage_demo_indicator(
    mut commands: Commands,
    state: Res<DemoModeState>,
    time: Res<Time>,
    indicator_q: Query<Entity, With<DemoIndicator>>,
    mut text_q: Query<&mut TextColor, With<DemoIndicator>>,
) {
    let exists = !indicator_q.is_empty();

    if state.active && !exists {
        // Spawn a centered top-of-screen text node.
        commands.spawn((
            DemoIndicator,
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(24.0),
                left: Val::Percent(50.0),
                // Shift left by roughly half the expected text width so it
                // appears centred (Bevy 0.18 has no built-in auto-centre for
                // absolutely-positioned nodes).
                margin: UiRect {
                    left: Val::Px(-240.0),
                    ..default()
                },
                ..default()
            },
            Text::new("DEMO MODE — press any key to play"),
            TextFont {
                font_size: 28.0,
                ..default()
            },
            TextColor(Color::srgba(1.0, 0.85, 0.0, 1.0)),
        ));
    } else if !state.active && exists {
        // Despawn all indicator nodes.
        for entity in indicator_q.iter() {
            commands.entity(entity).despawn();
        }
    } else if state.active && exists {
        // Pulse alpha: oscillate between 0.5 and 1.0 at ~1 Hz.
        let alpha = 0.75 + 0.25 * (time.elapsed_secs() * std::f32::consts::TAU).sin();
        for mut color in text_q.iter_mut() {
            color.0 = Color::srgba(1.0, 0.85, 0.0, alpha);
        }
    }
}
