// Session summary modal for skoffroad — Sprint 29.
//
// F9 toggles a centered 520×440 dark modal that shows all session stats:
// distance, top speed, total airtime, biggest jump, wheelies, gems,
// XP / level, and medal counts.
//
// Public API:
//   SessionSummaryPlugin

use bevy::prelude::*;

use crate::airtime::AirtimeStats;
use crate::collectibles::CollectibleCount;
use crate::hud::SessionStats;
use crate::medals::{Medal, MedalChallenge, MedalsState};
use crate::progression::ProgressionState;
use crate::wheelie::WheelieStats;
use crate::xp::XpState;

// ---- Public plugin ----------------------------------------------------------

pub struct SessionSummaryPlugin;

impl Plugin for SessionSummaryPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SessionSummaryState>()
            .add_systems(Startup, spawn_modal)
            .add_systems(Update, (toggle_with_f9, update_text));
    }
}

// ---- Resource ---------------------------------------------------------------

/// Tracks whether the session-summary modal is currently open.
#[derive(Resource, Default)]
pub struct SessionSummaryState {
    pub open: bool,
}

// ---- Components -------------------------------------------------------------

/// Marker on the modal root node.
#[derive(Component)]
struct SummaryModalRoot;

/// Tag enum identifying each text row so `update_text` can target them.
#[derive(Component)]
enum SummaryRow {
    Distance,
    TopSpeed,
    TotalAirtime,
    BiggestJump,
    Wheelies,
    Gems,
    XpLevel,
    Medals,
}

// ---- Colors -----------------------------------------------------------------

const MODAL_BG: Color = Color::srgba(0.05, 0.05, 0.08, 0.96);
const TITLE_COLOR: Color = Color::srgb(1.0, 0.88, 0.10);
const ROW_COLOR: Color = Color::srgb(0.88, 0.88, 0.90);
const FOOTER_COLOR: Color = Color::srgb(0.45, 0.45, 0.50);

const MODAL_W: f32 = 520.0;
const MODAL_H: f32 = 440.0;

// ---- Startup: spawn modal ---------------------------------------------------

fn spawn_modal(mut commands: Commands) {
    // Root: centered, initially hidden via Display::None.
    let root = commands
        .spawn((
            SummaryModalRoot,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Percent(50.0),
                top: Val::Percent(50.0),
                // Offset by half dimensions to truly centre.
                margin: UiRect {
                    left: Val::Px(-(MODAL_W / 2.0)),
                    top: Val::Px(-(MODAL_H / 2.0)),
                    ..default()
                },
                width: Val::Px(MODAL_W),
                height: Val::Px(MODAL_H),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                padding: UiRect::axes(Val::Px(28.0), Val::Px(24.0)),
                row_gap: Val::Px(10.0),
                display: Display::None,
                ..default()
            },
            BackgroundColor(MODAL_BG),
            ZIndex(500),
        ))
        .id();

    // Title
    let title = commands
        .spawn((
            Text::new("SESSION SUMMARY"),
            TextFont {
                font_size: 22.0,
                ..default()
            },
            TextColor(TITLE_COLOR),
        ))
        .id();

    // Helper: spawn a stat row with an initial placeholder string.
    let spawn_row = |commands: &mut Commands, tag: SummaryRow, label: &str| -> Entity {
        commands
            .spawn((
                tag,
                Text::new(label),
                TextFont {
                    font_size: 15.0,
                    ..default()
                },
                TextColor(ROW_COLOR),
                Node {
                    width: Val::Px(MODAL_W - 56.0),
                    ..default()
                },
            ))
            .id()
    };

    let row_distance = spawn_row(&mut commands, SummaryRow::Distance,    "Distance:        0.0 km");
    let row_speed    = spawn_row(&mut commands, SummaryRow::TopSpeed,    "Top Speed:       0 mph");
    let row_airtime  = spawn_row(&mut commands, SummaryRow::TotalAirtime,"Total Airtime:   0.0s");
    let row_jump     = spawn_row(&mut commands, SummaryRow::BiggestJump, "Biggest Jump:    0.0s");
    let row_wheelie  = spawn_row(&mut commands, SummaryRow::Wheelies,    "Wheelies:        0");
    let row_gems     = spawn_row(&mut commands, SummaryRow::Gems,        "Gems:            0");
    let row_xp       = spawn_row(&mut commands, SummaryRow::XpLevel,     "XP / Level:      0 / 1");
    let row_medals   = spawn_row(&mut commands, SummaryRow::Medals,      "Medals:          0 gold, 0 silver, 0 bronze");

    // Footer hint
    let footer = commands
        .spawn((
            Text::new("F9 close"),
            TextFont {
                font_size: 11.0,
                ..default()
            },
            TextColor(FOOTER_COLOR),
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(14.0),
                ..default()
            },
        ))
        .id();

    commands.entity(root).add_children(&[
        title,
        row_distance,
        row_speed,
        row_airtime,
        row_jump,
        row_wheelie,
        row_gems,
        row_xp,
        row_medals,
        footer,
    ]);
}

// ---- Toggle system ----------------------------------------------------------

fn toggle_with_f9(
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<SessionSummaryState>,
    mut root_q: Query<&mut Node, With<SummaryModalRoot>>,
) {
    if keys.just_pressed(KeyCode::F9) {
        state.open = !state.open;
        for mut node in &mut root_q {
            node.display = if state.open {
                Display::Flex
            } else {
                Display::None
            };
        }
    }
}

// ---- Update-text system -----------------------------------------------------

fn update_text(
    state: Res<SessionSummaryState>,
    session: Option<Res<SessionStats>>,
    airtime: Option<Res<AirtimeStats>>,
    wheelie: Option<Res<WheelieStats>>,
    gems: Option<Res<CollectibleCount>>,
    xp: Option<Res<XpState>>,
    progression: Option<Res<ProgressionState>>,
    medals_res: Option<Res<MedalsState>>,
    mut rows: Query<(&SummaryRow, &mut Text)>,
) {
    // Only update while the modal is open — avoids unnecessary string work.
    if !state.open {
        return;
    }

    // --- Gather values (fall back to zero-defaults when resource absent) ---

    let distance_km = session
        .as_ref()
        .map(|s| s.distance_m / 1000.0)
        .unwrap_or(0.0);
    let top_speed_mph = session
        .as_ref()
        .map(|s| s.max_speed_mps * 2.237)
        .unwrap_or(0.0);

    let total_air_s = airtime
        .as_ref()
        .map(|a| a.session_total_air_s)
        .unwrap_or(0.0);
    let max_air_s = airtime
        .as_ref()
        .map(|a| a.max_air_s)
        .unwrap_or(0.0);

    let wheelie_count = wheelie
        .as_ref()
        .map(|w| w.wheelie_count)
        .unwrap_or(0);
    let longest_wheelie_s = wheelie
        .as_ref()
        .map(|w| w.longest_wheelie_s)
        .unwrap_or(0.0);

    let gem_count = gems
        .as_ref()
        .map(|g| g.collected)
        .unwrap_or(0);

    let session_xp = xp
        .as_ref()
        .map(|x| x.session_xp)
        .unwrap_or(0);
    let level = progression
        .as_ref()
        .map(|p| p.level)
        .unwrap_or(1);

    // Count gold / silver / bronze from the medals HashMap.
    let (gold, silver, bronze) = if let Some(ref m) = medals_res {
        let mut g = 0u32;
        let mut s = 0u32;
        let mut b = 0u32;
        // Iterate over all tracked challenges and tally by tier.
        for challenge in [
            MedalChallenge::CourseTime,
            MedalChallenge::RaceVsRivals,
            MedalChallenge::GemCollector,
            MedalChallenge::Airtime,
            MedalChallenge::TopSpeed,
        ] {
            match m.best.get(&challenge).copied().unwrap_or(Medal::None) {
                Medal::Gold   => g += 1,
                Medal::Silver => s += 1,
                Medal::Bronze => b += 1,
                Medal::None   => {}
            }
        }
        (g, s, b)
    } else {
        (0, 0, 0)
    };

    // --- Write strings into row nodes ----------------------------------------

    for (row, mut text) in &mut rows {
        text.0 = match row {
            SummaryRow::Distance => {
                format!("Distance:        {:.1} km", distance_km)
            }
            SummaryRow::TopSpeed => {
                format!("Top Speed:       {:.0} mph", top_speed_mph)
            }
            SummaryRow::TotalAirtime => {
                format!("Total Airtime:   {:.1}s", total_air_s)
            }
            SummaryRow::BiggestJump => {
                format!("Biggest Jump:    {:.1}s", max_air_s)
            }
            SummaryRow::Wheelies => {
                format!("Wheelies:        {}  (best {:.1}s)", wheelie_count, longest_wheelie_s)
            }
            SummaryRow::Gems => {
                format!("Gems:            {}", gem_count)
            }
            SummaryRow::XpLevel => {
                format!("XP / Level:      {} / {}", session_xp, level)
            }
            SummaryRow::Medals => {
                format!("Medals:          {} gold, {} silver, {} bronze", gold, silver, bronze)
            }
        };
    }
}
