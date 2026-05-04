// Live airtime counter for SandK Offroad.
//
// Detection: all 4 wheels must have is_grounded == false for the chassis to
// count as airborne. A 0.05 s grace period prevents false triggers from
// suspension flex.
//
// HUD: large amber text centred at the top of the screen (top: 160 px, below
// any wheelie banner). Only visible while airborne.
//   Line 1: "AIRTIME 0.83 s"  (28 pt amber)
//   Line 2: "BEST 1.42 s"     (12 pt grey)

use bevy::prelude::*;
use crate::vehicle::{Wheel, VehicleRoot};

// ---- Grace period (seconds) before a freshly ungrounded wheel counts --------
const GRACE_S: f32 = 0.05;
// ---- Minimum flight to count as a "real" airtime event ----------------------
const MIN_AIR_S: f32 = 0.3;

// ---- Public resource ---------------------------------------------------------

#[derive(Resource, Default)]
pub struct AirtimeStats {
    pub airborne: bool,
    pub current_air_s: f32,
    pub max_air_s: f32,
    pub session_total_air_s: f32,
    pub airtime_count: u32,
}

// ---- Internal state ----------------------------------------------------------

/// Tracks how long each wheel (by index 0-3) has been continuously ungrounded.
#[derive(Resource, Default)]
struct WheelAirTime([f32; 4]);

// ---- Components --------------------------------------------------------------

#[derive(Component)]
struct AirtimeHudRoot;

#[derive(Component)]
struct AirtimeMainText;

#[derive(Component)]
struct AirtimeBestText;

// ---- Plugin ------------------------------------------------------------------

pub struct AirtimePlugin;

impl Plugin for AirtimePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AirtimeStats>()
           .init_resource::<WheelAirTime>()
           .add_systems(Startup, spawn_airtime_hud)
           .add_systems(Update, (
               detect_airtime.run_if(resource_exists::<VehicleRoot>),
               update_airtime_hud,
           ));
    }
}

// ---- Startup: spawn HUD ------------------------------------------------------

fn spawn_airtime_hud(mut commands: Commands) {
    // Invisible root container, centred horizontally, near the top.
    let root = commands.spawn((
        AirtimeHudRoot,
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(160.0),
            left: Val::Percent(0.0),
            right: Val::Percent(0.0),
            display: Display::None, // hidden until airborne
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: Val::Px(4.0),
            ..default()
        },
    )).id();

    // Line 1: main airtime counter.
    let main_text = commands.spawn((
        AirtimeMainText,
        Text::new("AIRTIME 0.00 s"),
        TextFont { font_size: 28.0, ..default() },
        TextColor(Color::srgb(0.95, 0.85, 0.20)),
    )).id();

    // Line 2: best time.
    let best_text = commands.spawn((
        AirtimeBestText,
        Text::new("BEST 0.00 s"),
        TextFont { font_size: 12.0, ..default() },
        TextColor(Color::srgb(0.65, 0.65, 0.65)),
    )).id();

    commands.entity(root).add_children(&[main_text, best_text]);
}

// ---- Update: detect airborne state ------------------------------------------

fn detect_airtime(
    wheel_q: Query<&Wheel>,
    time: Res<Time>,
    mut wheel_air: ResMut<WheelAirTime>,
    mut stats: ResMut<AirtimeStats>,
) {
    let dt = time.delta_secs();

    // Update per-wheel ungrounded timer.
    for wheel in wheel_q.iter() {
        let i = wheel.index.min(3);
        if wheel.is_grounded {
            wheel_air.0[i] = 0.0;
        } else {
            wheel_air.0[i] += dt;
        }
    }

    // Chassis is "truly" airborne only when all 4 wheels have been
    // ungrounded for longer than the grace period.
    let truly_airborne = wheel_air.0.iter().all(|&t| t > GRACE_S);

    match (stats.airborne, truly_airborne) {
        // Transition: grounded -> airborne
        (false, true) => {
            stats.airborne = true;
            stats.current_air_s = 0.0;
        }
        // Transition: airborne -> grounded
        (true, false) => {
            if stats.current_air_s > MIN_AIR_S {
                stats.airtime_count += 1;
                stats.session_total_air_s += stats.current_air_s;
                if stats.current_air_s > stats.max_air_s {
                    stats.max_air_s = stats.current_air_s;
                }
            }
            stats.current_air_s = 0.0;
            stats.airborne = false;
        }
        // Sustained airborne
        (true, true) => {
            stats.current_air_s += dt;
        }
        // Sustained grounded — nothing to do
        (false, false) => {}
    }
}

// ---- Update: refresh HUD text -----------------------------------------------

fn update_airtime_hud(
    stats: Res<AirtimeStats>,
    mut root_q: Query<&mut Node, With<AirtimeHudRoot>>,
    mut main_q: Query<&mut Text, (With<AirtimeMainText>, Without<AirtimeBestText>)>,
    mut best_q: Query<&mut Text, (With<AirtimeBestText>, Without<AirtimeMainText>)>,
) {
    // Show/hide root.
    for mut node in &mut root_q {
        node.display = if stats.airborne { Display::Flex } else { Display::None };
    }

    if !stats.airborne {
        return;
    }

    for mut text in &mut main_q {
        text.0 = format!("AIRTIME {:.2} s", stats.current_air_s);
    }

    for mut text in &mut best_q {
        text.0 = format!("BEST {:.2} s", stats.max_air_s);
    }
}
