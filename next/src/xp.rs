// XP / score system for SandK Offroad.
//
// Awards XP for gameplay events detected in EventLog, wheelie_count,
// airtime_count, and waypoint reached_count. Renders a HUD panel top-center
// (top: 196 px, below airtime/wheelie banners) and a short-lived popup for
// each gain or penalty.

use bevy::prelude::*;

use crate::events::{EventLog, GameEvent};
use crate::wheelie::WheelieStats;
use crate::airtime::AirtimeStats;
use crate::compass::Waypoint;

// ---- XP award constants -------------------------------------------------------

const XP_DISTANCE_MILESTONE: i32  = 50;
const XP_SPEED_MILESTONE:     i32  = 20;
const XP_HARD_IMPACT:         i32  = -10;
const XP_BIG_TILT:            i32  = -5;
const XP_BRAKE_STOP:          i32  = 10;
const XP_AIRTIME:             i32  = 30;
const XP_WHEELIE:             i32  = 40;
const XP_WAYPOINT:            i32  = 100;

// ---- HUD geometry -------------------------------------------------------------

const PANEL_W:  f32 = 280.0;
const PANEL_TOP: f32 = 196.0;
const POPUP_DURATION: f32 = 1.5;

// ---- Public resource ----------------------------------------------------------

#[derive(Resource, Default)]
pub struct XpState {
    /// Total XP (all sessions — this session is the only one for now).
    pub total_xp: u64,
    /// XP gained this session.
    pub session_xp: u64,
    /// Most recent delta (positive = gain, negative = penalty).
    pub last_gain: i32,
    /// Elapsed time when last gain occurred (for popup fade).
    pub last_gain_t: f32,
}

// ---- Internal state -----------------------------------------------------------

#[derive(Default)]
struct EventCursor {
    /// How many events from EventLog we have already processed.
    processed: usize,
}

// ---- Components ---------------------------------------------------------------

#[derive(Component)] struct XpPanelRoot;
#[derive(Component)] struct XpTotalText;
#[derive(Component)] struct XpPopupText;

// ---- Plugin -------------------------------------------------------------------

pub struct XpPlugin;

impl Plugin for XpPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<XpState>()
            .add_systems(Startup, spawn_xp_hud)
            .add_systems(Update, (award_xp, update_xp_hud));
    }
}

// ---- Startup: spawn HUD -------------------------------------------------------

fn spawn_xp_hud(mut commands: Commands) {
    let root = commands.spawn((
        XpPanelRoot,
        Node {
            position_type:   PositionType::Absolute,
            top:             Val::Px(PANEL_TOP),
            left:            Val::Percent(50.0),
            margin:          UiRect { left: Val::Px(-(PANEL_W / 2.0)), ..default() },
            width:           Val::Px(PANEL_W),
            flex_direction:  FlexDirection::Column,
            align_items:     AlignItems::Center,
            padding:         UiRect::axes(Val::Px(12.0), Val::Px(8.0)),
            row_gap:         Val::Px(2.0),
            ..default()
        },
        BackgroundColor(Color::srgba(0.04, 0.04, 0.06, 0.80)),
    )).id();

    // Title row.
    let title = commands.spawn((
        Text::new("XP"),
        TextFont  { font_size: 12.0, ..default() },
        TextColor(Color::srgb(0.55, 0.55, 0.58)),
    )).id();

    // Big score number.
    let total = commands.spawn((
        XpTotalText,
        Text::new("0"),
        TextFont  { font_size: 28.0, ..default() },
        TextColor(Color::srgb(1.0, 0.72, 0.05)),
    )).id();

    // Popup delta (e.g. "+30 XP").
    let popup = commands.spawn((
        XpPopupText,
        Text::new(""),
        TextFont  { font_size: 14.0, ..default() },
        TextColor(Color::NONE),
    )).id();

    commands.entity(root).add_children(&[title, total, popup]);
}

// ---- Award system -------------------------------------------------------------

fn award_xp(
    time:      Res<Time>,
    log:       Option<Res<EventLog>>,
    wheelie:   Option<Res<WheelieStats>>,
    airtime:   Option<Res<AirtimeStats>>,
    waypoint:  Option<Res<Waypoint>>,
    mut state: ResMut<XpState>,
    mut cursor: Local<EventCursor>,
    mut last_wheelie:  Local<u32>,
    mut last_airtime:  Local<u32>,
    mut last_waypoint: Local<u32>,
) {
    let now = time.elapsed_secs();

    // ---- EventLog-based awards ------------------------------------------------
    // EventLog is a ring buffer capped at 8 entries (EventLog::CAP).
    // `cursor.processed` counts how many events we have consumed across the
    // entire session.  The deque always holds the *most recent* min(len, CAP)
    // events, so new events live at indices [len - new_count .. len).
    if let Some(log) = log {
        let total = log.events.len();
        if total > cursor.processed {
            let new_count = total - cursor.processed;
            // Iterate only the newly appended tail of the deque.
            for (_, ev) in log.events.iter().rev().take(new_count).collect::<Vec<_>>().iter().rev() {
                let delta = match ev {
                    GameEvent::DistanceMilestone { .. } => XP_DISTANCE_MILESTONE,
                    GameEvent::SpeedMilestone    { .. } => XP_SPEED_MILESTONE,
                    GameEvent::HardImpact        { .. } => XP_HARD_IMPACT,
                    GameEvent::BigTilt           { .. } => XP_BIG_TILT,
                    GameEvent::BrakeStop         { .. } => XP_BRAKE_STOP,
                    GameEvent::Airtime           { .. } => XP_AIRTIME,
                };
                apply_delta(&mut state, delta, now);
            }
            cursor.processed = total;
        }
    }

    // ---- Wheelie counter ------------------------------------------------------
    if let Some(ws) = wheelie {
        let current = ws.wheelie_count;
        if current > *last_wheelie {
            let diff = current - *last_wheelie;
            for _ in 0..diff {
                apply_delta(&mut state, XP_WHEELIE, now);
            }
            *last_wheelie = current;
        }
    }

    // ---- Airtime counter ------------------------------------------------------
    if let Some(at) = airtime {
        let current = at.airtime_count;
        if current > *last_airtime {
            let diff = current - *last_airtime;
            for _ in 0..diff {
                apply_delta(&mut state, XP_AIRTIME, now);
            }
            *last_airtime = current;
        }
    }

    // ---- Waypoint counter -----------------------------------------------------
    if let Some(wpt) = waypoint {
        let current = wpt.reached_count;
        if current > *last_waypoint {
            let diff = current - *last_waypoint;
            for _ in 0..diff {
                apply_delta(&mut state, XP_WAYPOINT, now);
            }
            *last_waypoint = current;
        }
    }
}

/// Apply a signed XP delta to XpState (saturating arithmetic).
#[inline]
fn apply_delta(state: &mut XpState, delta: i32, now: f32) {
    if delta >= 0 {
        let d = delta as u64;
        state.total_xp   = state.total_xp.saturating_add(d);
        state.session_xp = state.session_xp.saturating_add(d);
    } else {
        let d = (-delta) as u64;
        state.total_xp   = state.total_xp.saturating_sub(d);
        state.session_xp = state.session_xp.saturating_sub(d);
    }
    state.last_gain   = delta;
    state.last_gain_t = now;
}

// ---- HUD update system --------------------------------------------------------

fn update_xp_hud(
    state:   Res<XpState>,
    time:    Res<Time>,
    mut total_q: Query<(&mut Text, &mut TextColor), (With<XpTotalText>, Without<XpPopupText>)>,
    mut popup_q: Query<(&mut Text, &mut TextColor), (With<XpPopupText>, Without<XpTotalText>)>,
) {
    let now = time.elapsed_secs();

    // Update total score with comma formatting.
    for (mut text, _) in &mut total_q {
        text.0 = format_xp(state.total_xp);
    }

    // Update popup.
    let age = now - state.last_gain_t;
    for (mut text, mut color) in &mut popup_q {
        if state.last_gain != 0 && age < POPUP_DURATION {
            let alpha = (1.0 - age / POPUP_DURATION).clamp(0.0, 1.0);
            let (sign, abs, r, g, b) = if state.last_gain > 0 {
                ("+", state.last_gain as u32, 0.3_f32, 1.0_f32, 0.4_f32)
            } else {
                ("-", (-state.last_gain) as u32, 1.0_f32, 0.3_f32, 0.25_f32)
            };
            text.0 = format!("{}{} XP", sign, abs);
            color.0 = Color::linear_rgba(r, g, b, alpha);
        } else {
            text.0 = String::new();
            color.0 = Color::NONE;
        }
    }
}

/// Format a u64 score with commas: 12450 -> "12,450".
fn format_xp(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::with_capacity(s.len() + s.len() / 3);
    for (i, ch) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    out.chars().rev().collect()
}
