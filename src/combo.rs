// Combo multiplier: when the player chains stunts (jumps, wheelies)
// within a 3-second window, the multiplier rises. Resets if no stunt
// in 3s or on hard impact. Multiplier shown in HUD; XP awards multiply.
//
// Public API:
//   ComboPlugin
//   ComboState (resource)

use bevy::prelude::*;

use crate::airtime::AirtimeStats;
use crate::events::{EventLog, GameEvent};
use crate::wheelie::WheelieStats;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const COMBO_WINDOW_S: f32 = 3.0;
const MIN_JUMP_AIR_S: f32 = 0.4;
const MIN_WHEELIE_S: f32  = 0.5;
const MAX_MULTIPLIER: u32 = 5;

// ---------------------------------------------------------------------------
// Public resource
// ---------------------------------------------------------------------------

/// Running combo state. Multiplier default is 1 (initialised via insert_resource).
#[derive(Resource, Default, Clone, Copy)]
pub struct ComboState {
    pub multiplier: u32,
    pub since_last_stunt_s: f32,
    pub last_stunt_t: f32,
}

// ---------------------------------------------------------------------------
// Private components
// ---------------------------------------------------------------------------

#[derive(Component)]
struct ComboIndicatorRoot;

#[derive(Component)]
struct ComboIndicatorText;

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct ComboPlugin;

impl Plugin for ComboPlugin {
    fn build(&self, app: &mut App) {
        // Initialise with multiplier = 1.
        app.insert_resource(ComboState {
            multiplier: 1,
            since_last_stunt_s: 0.0,
            last_stunt_t: 0.0,
        });

        app.add_systems(Startup, spawn_combo_hud)
           .add_systems(Update, (
               detect_stunts,
               decay_combo,
               check_hard_impact_reset,
               update_indicator,
           ));
    }
}

// ---------------------------------------------------------------------------
// Startup: spawn HUD indicator
// ---------------------------------------------------------------------------

fn spawn_combo_hud(mut commands: Commands) {
    // Small panel on the right side, top: 380, right: 14. Hidden initially.
    let root = commands.spawn((
        ComboIndicatorRoot,
        Node {
            position_type: PositionType::Absolute,
            top:   Val::Px(380.0),
            right: Val::Px(14.0),
            padding: UiRect::all(Val::Px(6.0)),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            display: Display::None, // hidden until combo > 1
            ..default()
        },
        BackgroundColor(Color::srgba(0.05, 0.05, 0.05, 0.80)),
    )).id();

    let label = commands.spawn((
        ComboIndicatorText,
        Text::new("x1"),
        TextFont { font_size: 16.0, ..default() },
        TextColor(Color::srgb(0.2, 0.95, 0.35)), // green
    )).id();

    commands.entity(root).add_child(label);
}

// ---------------------------------------------------------------------------
// System: detect_stunts
// ---------------------------------------------------------------------------

fn detect_stunts(
    airtime:  Option<Res<AirtimeStats>>,
    wheelie:  Option<Res<WheelieStats>>,
    time:     Res<Time>,
    mut combo: ResMut<ComboState>,
    mut was_airborne:       Local<bool>,
    mut prev_air_s:         Local<f32>,
    mut was_wheelie_active: Local<bool>,
    mut prev_wheelie_s:     Local<f32>,
) {
    let now = time.elapsed_secs();

    // ---- Jump landing detection ------------------------------------------------
    if let Some(airtime) = airtime {
        let currently_airborne = airtime.airborne;

        // While airborne, keep tracking how long we've been in the air.
        if currently_airborne {
            *prev_air_s = airtime.current_air_s;
        }

        // Falling edge: was airborne, now grounded.
        if *was_airborne && !currently_airborne {
            let air_s = *prev_air_s;
            if air_s >= MIN_JUMP_AIR_S {
                trigger_stunt(&mut combo, now);
            }
        }

        *was_airborne = currently_airborne;
    }

    // ---- Wheelie completion detection -----------------------------------------
    if let Some(wheelie) = wheelie {
        let currently_active = wheelie.in_wheelie;

        if currently_active {
            *prev_wheelie_s = wheelie.current_wheelie_s;
        }

        // Falling edge: was in wheelie, now ended.
        if *was_wheelie_active && !currently_active {
            let duration_s = *prev_wheelie_s;
            if duration_s >= MIN_WHEELIE_S {
                trigger_stunt(&mut combo, now);
            }
        }

        *was_wheelie_active = currently_active;
    }
}

/// Shared stunt-trigger logic: increment multiplier or start a fresh chain.
#[inline]
fn trigger_stunt(combo: &mut ComboState, now: f32) {
    if combo.since_last_stunt_s < COMBO_WINDOW_S && combo.multiplier > 1 {
        combo.multiplier = (combo.multiplier + 1).min(MAX_MULTIPLIER);
    } else {
        // Either first stunt, or chain was already broken — start at 2.
        combo.multiplier = 2;
    }
    combo.last_stunt_t = now;
    combo.since_last_stunt_s = 0.0;
    info!("COMBO x{}!", combo.multiplier);
}

// ---------------------------------------------------------------------------
// System: decay_combo
// ---------------------------------------------------------------------------

fn decay_combo(
    time:      Res<Time>,
    mut combo: ResMut<ComboState>,
) {
    combo.since_last_stunt_s += time.delta_secs();

    if combo.since_last_stunt_s > COMBO_WINDOW_S && combo.multiplier > 1 {
        combo.multiplier = 1;
        info!("combo broken");
    }
}

// ---------------------------------------------------------------------------
// System: check_hard_impact_reset
// ---------------------------------------------------------------------------

fn check_hard_impact_reset(
    event_log:     Option<Res<EventLog>>,
    mut combo:     ResMut<ComboState>,
    mut last_seen: Local<f32>,
) {
    let Some(event_log) = event_log else { return };

    let mut newest_ts = *last_seen;

    for (ts, ev) in &event_log.events {
        if *ts <= *last_seen {
            continue;
        }
        if *ts > newest_ts {
            newest_ts = *ts;
        }
        if let GameEvent::HardImpact { .. } = ev {
            combo.multiplier = 1;
            info!("combo reset: hard impact");
        }
    }

    *last_seen = newest_ts;
}

// ---------------------------------------------------------------------------
// System: update_indicator
// ---------------------------------------------------------------------------

fn update_indicator(
    combo:      Res<ComboState>,
    time:       Res<Time>,
    mut root_q: Query<&mut Node, With<ComboIndicatorRoot>>,
    mut text_q: Query<(&mut Text, &mut TextFont, &mut TextColor), With<ComboIndicatorText>>,
) {
    let visible = combo.multiplier > 1;

    for mut node in &mut root_q {
        node.display = if visible { Display::Flex } else { Display::None };
    }

    if !visible {
        return;
    }

    // Pulse: font_size oscillates slightly around 16 pt.
    let pulse = 16.0 + (time.elapsed_secs() * 6.0).sin() * 2.0;

    // Color by multiplier level.
    let color = match combo.multiplier {
        2 => Color::srgb(0.2,  0.95, 0.35), // green
        3 => Color::srgb(0.95, 0.90, 0.20), // yellow
        4 => Color::srgb(0.95, 0.55, 0.10), // orange
        _ => Color::srgb(0.95, 0.20, 0.15), // red (5x)
    };

    for (mut text, mut font, mut color_comp) in &mut text_q {
        text.0 = format!("x{}", combo.multiplier);
        font.font_size = pulse;
        color_comp.0 = color;
    }
}
