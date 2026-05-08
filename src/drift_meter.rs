// Drift meter for skoffroad (Sprint 30).
//
// While chassis lateral velocity > 4 m/s AND grounded, drift score builds at
// (lateral_speed * 5.0) pts/s.  When drift ends cleanly (lateral drops below
// threshold, chassis goes airborne, or hard impact), the accumulated score is
// awarded as XP if it exceeds 50 pts.  A HardImpact wipes the score instead.
//
// HUD: bottom-right corner, hidden while inactive; text colour pulses from
//       green → yellow → orange → red as score climbs.
//
// Public API:
//   DriftMeterPlugin
//   DriftMeterState (resource)

use bevy::prelude::*;
use avian3d::prelude::LinearVelocity;

use crate::vehicle::{Chassis, VehicleRoot};
use crate::airtime::AirtimeStats;
use crate::events::{EventLog, GameEvent};
use crate::xp::XpState;

// ---- Grace period for "drift still visible in HUD after it ends" (seconds) --
const HUD_GRACE_S: f32 = 1.0;

// ---- Public resource ---------------------------------------------------------

#[derive(Resource, Default, Clone, Copy)]
pub struct DriftMeterState {
    pub current_score: u32,
    pub drift_active: bool,
    pub last_awarded_t: f32,
}

// ---- Internal components -----------------------------------------------------

#[derive(Component)]
struct DriftHudRoot;

#[derive(Component)]
struct DriftHudText;

// ---- Plugin ------------------------------------------------------------------

pub struct DriftMeterPlugin;

impl Plugin for DriftMeterPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DriftMeterState>()
            .add_systems(Startup, spawn_drift_hud)
            .add_systems(Update, (tick_drift, check_drift_end, update_hud).chain());
    }
}

// ---- Startup: spawn HUD ------------------------------------------------------

fn spawn_drift_hud(mut commands: Commands) {
    let root = commands.spawn((
        DriftHudRoot,
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(460.0),
            right: Val::Px(14.0),
            display: Display::None, // hidden until drifting
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::FlexEnd,
            ..default()
        },
    )).id();

    let text = commands.spawn((
        DriftHudText,
        Text::new("DRIFT 0"),
        TextFont { font_size: 18.0, ..default() },
        TextColor(Color::srgb(1.0, 0.9, 0.0)), // default yellow
    )).id();

    commands.entity(root).add_child(text);
}

// ---- tick_drift: accumulate score while sliding ------------------------------

fn tick_drift(
    time: Res<Time>,
    vehicle: Option<Res<VehicleRoot>>,
    chassis_q: Query<(&Transform, &LinearVelocity), With<Chassis>>,
    airtime: Option<Res<AirtimeStats>>,
    mut state: ResMut<DriftMeterState>,
) {
    let Some(vehicle) = vehicle else { return };
    let Ok((transform, lin_vel)) = chassis_q.get(vehicle.chassis) else { return };

    let airborne = airtime.map(|a| a.airborne).unwrap_or(false);

    // Project world-space linear velocity onto the chassis's local right axis.
    let chassis_right = transform.rotation * Vec3::X;
    let vel_world = Vec3::new(lin_vel.x, lin_vel.y, lin_vel.z);
    let lateral = vel_world.dot(chassis_right);
    let lateral_abs = lateral.abs();

    let dt = time.delta_secs();

    if !airborne && lateral_abs > 4.0 {
        state.drift_active = true;
        // Scale: 10 m/s → 50 pts/s; general: lateral * 5.0 pts/s
        let gain = (lateral_abs * 5.0 * dt) as u32;
        state.current_score = state.current_score.saturating_add(gain);
    } else {
        state.drift_active = false;
    }
}

// ---- check_drift_end: award XP on falling edge, wipe on hard impact ---------

fn check_drift_end(
    time: Res<Time>,
    event_log: Option<Res<EventLog>>,
    mut state: ResMut<DriftMeterState>,
    mut xp: Option<ResMut<XpState>>,
    // was_drifting: true on previous frame (falling-edge detection)
    mut was_drifting: Local<bool>,
    // last_log_len: how many EventLog entries we have seen, for HardImpact polling
    mut last_log_len: Local<usize>,
) {
    let now = time.elapsed_secs();

    // ---- Falling-edge: drift just ended cleanly ------------------------------
    if *was_drifting && !state.drift_active {
        if state.current_score > 50 {
            let score = state.current_score;
            if let Some(ref mut xp_state) = xp {
                xp_state.total_xp   = xp_state.total_xp.saturating_add(score as u64);
                xp_state.session_xp = xp_state.session_xp.saturating_add(score as u64);
                xp_state.last_gain   = score as i32;
                xp_state.last_gain_t = now;
            }
            info!("DRIFT! +{} XP", score);
            state.last_awarded_t = now;
        }
        state.current_score = 0;
    }
    *was_drifting = state.drift_active;

    // ---- HardImpact: scan newly appended EventLog entries -------------------
    if let Some(log) = event_log {
        let current_len = log.events.len();
        if current_len > *last_log_len {
            let new_count = current_len - *last_log_len;
            // Iterate only the newly added tail (newest-last ordering).
            for (_, ev) in log.events.iter().rev().take(new_count).collect::<Vec<_>>().iter().rev() {
                if matches!(ev, GameEvent::HardImpact { .. }) {
                    // Impact wipes accumulated drift score (drift broken).
                    state.current_score = 0;
                    state.drift_active = false;
                }
            }
        }
        *last_log_len = current_len;
    }
}

// ---- update_hud: show/hide + colour pulse ------------------------------------

fn update_hud(
    time: Res<Time>,
    state: Res<DriftMeterState>,
    mut root_q: Query<&mut Node, With<DriftHudRoot>>,
    mut text_q: Query<(&mut Text, &mut TextColor), With<DriftHudText>>,
) {
    let now = time.elapsed_secs();

    // Show while actively drifting or within the 1-second grace window after.
    let recently_drifted = !state.drift_active
        && state.current_score == 0
        && (now - state.last_awarded_t) < HUD_GRACE_S;
    let show = state.drift_active || state.current_score > 0 || recently_drifted;

    for mut node in &mut root_q {
        node.display = if show { Display::Flex } else { Display::None };
    }

    if !show {
        return;
    }

    // Colour pulse by accumulated score.
    let color = if state.current_score < 100 {
        // green
        Color::srgb(0.3, 1.0, 0.3)
    } else if state.current_score < 300 {
        // yellow
        Color::srgb(1.0, 0.9, 0.0)
    } else if state.current_score < 600 {
        // orange
        Color::srgb(1.0, 0.5, 0.05)
    } else {
        // red
        Color::srgb(1.0, 0.15, 0.1)
    };

    for (mut text, mut text_color) in &mut text_q {
        text.0 = format!("DRIFT {}", state.current_score);
        text_color.0 = color;
    }
}
