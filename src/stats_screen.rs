// Full-screen stats overlay — hold Tab to show, release to hide.
//
// Layout: dark overlay → centred 700×500 panel → three sections:
//   Driving stats | Achievement badges | Recent events

use bevy::prelude::*;

use crate::damage::DamageState;
use crate::events::{EventLog, GameEvent};
use crate::hud::SessionStats;

// ---- Plugin -----------------------------------------------------------------

pub struct StatsScreenPlugin;

impl Plugin for StatsScreenPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_stats_screen)
           .add_systems(Update, (update_stats_screen, toggle_stats_screen));
    }
}

// ---- Components -------------------------------------------------------------

#[derive(Component)]
struct StatsRoot;

/// Individual text rows identified by role.
#[derive(Component, Clone, Copy)]
enum StatsRow {
    Distance,
    MaxSpeed,
    MeanSpeed,
    MaxTilt,
    SessionTime,
    Damage,
    Impacts,
    // Achievements (7 badges, indexed 0..6)
    Badge(usize),
    // Recent events (5 rows, indexed 0..4)
    Event(usize),
}

// ---- Colors -----------------------------------------------------------------

const OVERLAY_BG: Color = Color::srgba(0.0, 0.0, 0.0, 0.85);
const PANEL_BG:   Color = Color::srgba(0.06, 0.07, 0.10, 0.97);
const TITLE_COL:  Color = Color::srgb(0.9, 0.85, 0.4);
const LABEL_COL:  Color = Color::srgb(0.75, 0.78, 0.82);
const VALUE_COL:  Color = Color::WHITE;
const EARNED_COL: Color = Color::srgb(0.3, 0.92, 0.3);
const UNEARNED:   Color = Color::srgb(0.42, 0.42, 0.44);
const SECTION_COL: Color = Color::srgb(0.55, 0.75, 0.95);

// ---- Startup: build UI tree -------------------------------------------------

fn spawn_stats_screen(mut commands: Commands) {
    // Full-screen dark overlay; hidden by default.
    let root = commands.spawn((
        StatsRoot,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            top: Val::Px(0.0),
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            display: Display::None,
            ..default()
        },
        BackgroundColor(OVERLAY_BG),
        ZIndex(200),
    )).id();

    // Centred content panel.
    let panel = commands.spawn((
        Node {
            width: Val::Px(700.0),
            height: Val::Px(500.0),
            flex_direction: FlexDirection::Column,
            padding: UiRect::all(Val::Px(20.0)),
            row_gap: Val::Px(6.0),
            ..default()
        },
        BackgroundColor(PANEL_BG),
    )).id();
    commands.entity(root).add_child(panel);

    // Title row.
    let title = text_node(&mut commands, "SESSION STATS", 22.0, TITLE_COL);
    commands.entity(panel).add_child(title);

    // Spacer.
    let gap = commands.spawn(Node { height: Val::Px(4.0), ..default() }).id();
    commands.entity(panel).add_child(gap);

    // ---- Driving section ----
    let drv_head = text_node(&mut commands, "-- DRIVING --", 14.0, SECTION_COL);
    commands.entity(panel).add_child(drv_head);

    let stats_rows: &[(StatsRow, &str)] = &[
        (StatsRow::Distance,    "Distance:      --"),
        (StatsRow::MaxSpeed,    "Max speed:     --"),
        (StatsRow::MeanSpeed,   "Mean speed:    --"),
        (StatsRow::MaxTilt,     "Max tilt:      --"),
        (StatsRow::SessionTime, "Session time:  --"),
        (StatsRow::Damage,      "Damage:        --"),
        (StatsRow::Impacts,     "Impacts taken: --"),
    ];

    for (row, placeholder) in stats_rows {
        let e = commands.spawn((
            *row,
            Text::new(*placeholder),
            TextFont { font_size: 13.0, ..default() },
            TextColor(LABEL_COL),
        )).id();
        commands.entity(panel).add_child(e);
    }

    // ---- Achievements section ----
    let gap2 = commands.spawn(Node { height: Val::Px(4.0), ..default() }).id();
    commands.entity(panel).add_child(gap2);
    let ach_head = text_node(&mut commands, "-- ACHIEVEMENTS --", 14.0, SECTION_COL);
    commands.entity(panel).add_child(ach_head);

    let badge_labels = [
        "[ ] Sub-mile rookie",
        "[ ] Marathoner",
        "[ ] Air time pilot",
        "[ ] Daredevil",
        "[ ] Demolition derby",
        "[ ] Time traveler",
        "[ ] Waypoint hunter",
    ];

    for (i, label) in badge_labels.iter().enumerate() {
        let e = commands.spawn((
            StatsRow::Badge(i),
            Text::new(*label),
            TextFont { font_size: 13.0, ..default() },
            TextColor(UNEARNED),
        )).id();
        commands.entity(panel).add_child(e);
    }

    // ---- Recent events section ----
    let gap3 = commands.spawn(Node { height: Val::Px(4.0), ..default() }).id();
    commands.entity(panel).add_child(gap3);
    let ev_head = text_node(&mut commands, "-- RECENT EVENTS --", 14.0, SECTION_COL);
    commands.entity(panel).add_child(ev_head);

    for i in 0..5usize {
        let e = commands.spawn((
            StatsRow::Event(i),
            Text::new(""),
            TextFont { font_size: 12.0, ..default() },
            TextColor(LABEL_COL),
        )).id();
        commands.entity(panel).add_child(e);
    }

    // Hint at the bottom.
    let gap4 = commands.spawn(Node { height: Val::Px(4.0), ..default() }).id();
    commands.entity(panel).add_child(gap4);
    let hint = text_node(&mut commands, "Hold TAB to keep open", 11.0, Color::srgb(0.45, 0.45, 0.48));
    commands.entity(panel).add_child(hint);
}

// Helper: plain text node without a StatsRow marker.
fn text_node(commands: &mut Commands, s: &str, size: f32, color: Color) -> Entity {
    commands.spawn((
        Text::new(s),
        TextFont { font_size: size, ..default() },
        TextColor(color),
    )).id()
}

// ---- Update: populate numbers -----------------------------------------------

fn update_stats_screen(
    stats:   Res<SessionStats>,
    damage:  Res<DamageState>,
    log:     Res<EventLog>,
    // TimeOfDay available for day/night detection.
    tod:     Option<Res<crate::sky::TimeOfDay>>,
    // Waypoint is an optional resource (parallel agent may or may not insert it).
    // We look it up by name via reflection-free field access if it exists.
    // Since compass.rs is a stub, we can't name the type; guard with Option<Res<crate::compass::Waypoint>>.
    // The compass stub exports no Waypoint — so we skip: the achievement stays stubbed.
    mut rows: Query<(&StatsRow, &mut Text, &mut TextColor)>,
) {
    // ---- Pre-compute achievement booleans ----

    let mut airtime_count   = 0u32;
    let mut big_tilt_count  = 0u32;
    let mut hard_impact_count = 0u32;

    for (_, ev) in &log.events {
        match ev {
            GameEvent::Airtime { .. }       => airtime_count += 1,
            GameEvent::BigTilt { .. }       => big_tilt_count += 1,
            GameEvent::HardImpact { .. }    => hard_impact_count += 1,
            _ => {}
        }
    }

    // Day/night: earned if TimeOfDay.t has apparently crossed both 0.25 and 0.75.
    // We can only observe the current t, not history — skip (always false).
    let time_traveler = false;
    let _ = tod; // suppress unused warning; kept for future use

    let badges_earned = [
        stats.distance_m >= 1_600.0,                  // 0: Sub-mile rookie
        stats.distance_m >= 5_000.0,                  // 1: Marathoner
        airtime_count >= 1,                            // 2: Air time pilot
        big_tilt_count >= 3,                           // 3: Daredevil
        hard_impact_count >= 5,                        // 4: Demolition derby
        time_traveler,                                 // 5: Time traveler (stubbed)
        false,                                         // 6: Waypoint hunter (compass stub)
    ];

    let badge_names = [
        "Sub-mile rookie",
        "Marathoner",
        "Air time pilot",
        "Daredevil",
        "Demolition derby",
        "Time traveler",
        "Waypoint hunter",
    ];

    // ---- Driving stats strings ----

    let elapsed_u = stats.elapsed_s as u32;
    let sess_m = elapsed_u / 60;
    let sess_s = elapsed_u % 60;

    let dist_m = stats.distance_m;
    let dist_str = {
        let base = format!("{:.1} m", dist_m);
        if dist_m >= 200.0 {
            let miles = dist_m / 1609.34;
            format!("Distance:      {} ({:.2} mi)", base, miles)
        } else {
            format!("Distance:      {}", base)
        }
    };

    let max_speed_mph = stats.max_speed_mps * 2.237;
    let max_spd_str = format!(
        "Max speed:     {:.1} mph ({:.1} m/s)",
        max_speed_mph, stats.max_speed_mps
    );

    let mean_speed_mph = if stats.elapsed_s > 0.0 {
        (dist_m / stats.elapsed_s) * 2.237
    } else {
        0.0
    };
    let mean_str = format!("Mean speed:    {:.1} mph", mean_speed_mph);

    let tilt_str   = format!("Max tilt:      {:.1}\u{b0}", stats.max_tilt_deg);
    let time_str   = format!("Session time:  {:02}:{:02}", sess_m, sess_s);
    let damage_str = format!("Damage:        {:.0}%", damage.damage * 100.0);
    let impact_str = format!("Impacts taken: {}", damage.impact_count);

    // ---- Recent events ----

    // Collect all events, take the last 5 (newest at bottom).
    let all: Vec<_> = log.events.iter().collect();
    let start = if all.len() > 5 { all.len() - 5 } else { 0 };
    let recent: Vec<_> = all[start..].iter().copied().collect();

    // ---- Write to text nodes ----

    for (row, mut text, mut color) in &mut rows {
        match row {
            StatsRow::Distance    => { text.0 = dist_str.clone(); color.0 = VALUE_COL; }
            StatsRow::MaxSpeed    => { text.0 = max_spd_str.clone(); color.0 = VALUE_COL; }
            StatsRow::MeanSpeed   => { text.0 = mean_str.clone(); color.0 = VALUE_COL; }
            StatsRow::MaxTilt     => { text.0 = tilt_str.clone(); color.0 = VALUE_COL; }
            StatsRow::SessionTime => { text.0 = time_str.clone(); color.0 = VALUE_COL; }
            StatsRow::Damage      => { text.0 = damage_str.clone(); color.0 = VALUE_COL; }
            StatsRow::Impacts     => { text.0 = impact_str.clone(); color.0 = VALUE_COL; }

            StatsRow::Badge(i) => {
                let idx = *i;
                if idx < badges_earned.len() {
                    let earned = badges_earned[idx];
                    let check  = if earned { "[x]" } else { "[ ]" };
                    text.0    = format!("{} {}", check, badge_names[idx]);
                    color.0   = if earned { EARNED_COL } else { UNEARNED };
                }
            }

            StatsRow::Event(i) => {
                let idx = *i;
                if idx < recent.len() {
                    let (ts, ev) = recent[idx];
                    let ts_s = *ts as u32;
                    let mm = ts_s / 60;
                    let ss = ts_s % 60;
                    text.0  = format!("[{:02}:{:02}] {}", mm, ss, event_desc(ev));
                    color.0 = event_color(ev);
                } else {
                    text.0  = String::new();
                    color.0 = Color::NONE;
                }
            }
        }
    }
}

// ---- Toggle: hold Tab = show, release = hide --------------------------------

fn toggle_stats_screen(
    keys: Res<ButtonInput<KeyCode>>,
    mut root_q: Query<&mut Node, With<StatsRoot>>,
) {
    let show = keys.pressed(KeyCode::Tab);
    for mut node in &mut root_q {
        node.display = if show { Display::Flex } else { Display::None };
    }
}

// ---- Event helpers ----------------------------------------------------------

fn event_desc(ev: &GameEvent) -> String {
    match ev {
        GameEvent::HardImpact { v }         => format!("Hard impact {:.1} m/s", v.abs()),
        GameEvent::BigTilt { tilt_deg }     => format!("Big tilt {:.1}\u{b0}", tilt_deg),
        GameEvent::DistanceMilestone { km } => format!("{:.1} km milestone", *km as f32 * 0.5),
        GameEvent::SpeedMilestone { mph }   => format!("{} mph milestone", mph),
        GameEvent::BrakeStop { from_mph }   => format!("Stopped from {:.0} mph", from_mph),
        GameEvent::Airtime { duration_s }   => format!("Airtime {:.1} s", duration_s),
    }
}

fn event_color(ev: &GameEvent) -> Color {
    match ev {
        GameEvent::HardImpact { .. } | GameEvent::BigTilt { .. }
            => Color::srgb(0.95, 0.42, 0.30),
        GameEvent::DistanceMilestone { .. } | GameEvent::SpeedMilestone { .. }
            => Color::srgb(0.4, 0.9, 0.4),
        GameEvent::BrakeStop { .. }
            => Color::srgb(0.85, 0.85, 0.85),
        GameEvent::Airtime { .. }
            => Color::srgb(0.95, 0.85, 0.2),
    }
}
