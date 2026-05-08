// Rival/race HUD: position (1st/2nd/...), gap to leader, lap counter.
//
// Reads RaceState; renders a top-right panel with a 4-row leaderboard.
// Placed below the session-stats panel (hud.rs top-right) at top: 220 px.
//
// Public API:
//   RivalHudPlugin
//   RivalHudRoot
//   RivalHudRow(usize)
//   RivalHudCell { Pos, Name, Gap }

use bevy::prelude::*;

use crate::race::{RacePhase, RaceState};

// ---- Plugin -----------------------------------------------------------------

pub struct RivalHudPlugin;

impl Plugin for RivalHudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_rival_hud)
            .add_systems(Update, update_rival_hud);
    }
}

// ---- Marker components ------------------------------------------------------

/// Marks the root container of the leaderboard panel.
#[derive(Component)]
pub struct RivalHudRoot;

/// Marks each data row (0 = row for rank 1, 1 = row for rank 2, …).
#[derive(Component)]
pub struct RivalHudRow(pub usize);

/// Marks the individual text nodes inside each row.
#[derive(Component)]
pub enum RivalHudCell {
    Pos,
    Name,
    Gap,
}

// ---- Constants --------------------------------------------------------------

const BG: Color = Color::srgba(0.05, 0.05, 0.07, 0.75);
const PANEL_WIDTH: f32 = 200.0;
const PANEL_TOP: f32 = 220.0;
const PANEL_RIGHT: f32 = 14.0;

const COLOR_PLAYER: Color = Color::srgb(1.0, 0.9, 0.3);
const COLOR_OTHER: Color = Color::WHITE;
const COLOR_HEADER: Color = Color::srgb(0.75, 0.75, 0.75);

const POSITION_LABELS: [&str; 4] = ["1st", "2nd", "3rd", "4th"];
const NUM_ROWS: usize = 4;

// ---- Startup: spawn UI tree -------------------------------------------------

fn spawn_rival_hud(mut commands: Commands) {
    // Root panel — starts hidden (Display::None) until a race begins.
    let root = commands
        .spawn((
            RivalHudRoot,
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(PANEL_TOP),
                right: Val::Px(PANEL_RIGHT),
                width: Val::Px(PANEL_WIDTH),
                flex_direction: FlexDirection::Column,
                padding: UiRect::axes(Val::Px(10.0), Val::Px(8.0)),
                display: Display::None,
                ..default()
            },
            BackgroundColor(BG),
            ZIndex(10),
        ))
        .id();

    // Header row — static label.
    let header = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                padding: UiRect::bottom(Val::Px(4.0)),
                ..default()
            },
        ))
        .id();

    let header_text = commands
        .spawn((
            Text::new("POSITION"),
            TextFont { font_size: 12.0, ..default() },
            TextColor(COLOR_HEADER),
        ))
        .id();

    commands.entity(header).add_child(header_text);
    commands.entity(root).add_child(header);

    // Data rows 0..NUM_ROWS.
    for i in 0..NUM_ROWS {
        let row = commands
            .spawn((
                RivalHudRow(i),
                Node {
                    width: Val::Percent(100.0),
                    flex_direction: FlexDirection::Row,
                    justify_content: JustifyContent::SpaceBetween,
                    padding: UiRect::vertical(Val::Px(2.0)),
                    ..default()
                },
            ))
            .id();

        // Position cell — fixed width so columns align.
        let pos_node = commands
            .spawn((
                RivalHudRow(i),
                RivalHudCell::Pos,
                Text::new(""),
                TextFont { font_size: 13.0, ..default() },
                TextColor(COLOR_OTHER),
                Node {
                    width: Val::Px(36.0),
                    ..default()
                },
            ))
            .id();

        // Name cell — takes remaining space.
        let name_node = commands
            .spawn((
                RivalHudRow(i),
                RivalHudCell::Name,
                Text::new(""),
                TextFont { font_size: 13.0, ..default() },
                TextColor(COLOR_OTHER),
                Node {
                    flex_grow: 1.0,
                    ..default()
                },
            ))
            .id();

        // Gap cell — right-aligned.
        let gap_node = commands
            .spawn((
                RivalHudRow(i),
                RivalHudCell::Gap,
                Text::new(""),
                TextFont { font_size: 13.0, ..default() },
                TextColor(COLOR_OTHER),
                Node {
                    width: Val::Px(60.0),
                    justify_content: JustifyContent::FlexEnd,
                    ..default()
                },
            ))
            .id();

        commands.entity(row).add_children(&[pos_node, name_node, gap_node]);
        commands.entity(root).add_child(row);
    }
}

// ---- Per-frame update -------------------------------------------------------

fn update_rival_hud(
    race: Res<RaceState>,
    mut root_q: Query<&mut Node, With<RivalHudRoot>>,
    // Query all RivalHudRow + RivalHudCell text nodes together.
    // We tag each cell node with RivalHudRow(i) so we can group by row index.
    mut cell_q: Query<(&RivalHudRow, &RivalHudCell, &mut Text, &mut TextColor, &mut Node),
        Without<RivalHudRoot>>,
) {
    // --- Show / hide root based on phase ------------------------------------
    let Ok(mut root_node) = root_q.single_mut() else {
        return;
    };

    let visible = matches!(
        race.phase,
        RacePhase::Countdown | RacePhase::Active | RacePhase::Finished
    );
    root_node.display = if visible { Display::Flex } else { Display::None };

    if !visible {
        return;
    }

    // --- Compute leader progress for gap calculation ------------------------
    // leaderboard[0] is the current leader (highest progress / lap).
    let leader_progress = race.leaderboard.first().map(|e| e.progress).unwrap_or(0.0);
    let leader_lap = race.leaderboard.first().map(|e| e.lap).unwrap_or(0);

    // --- Update each row ----------------------------------------------------
    for (row_marker, cell_kind, mut text, mut color, mut node) in &mut cell_q {
        let row_idx = row_marker.0;

        // Hide rows beyond the leaderboard length.
        if row_idx >= race.leaderboard.len() {
            node.display = Display::None;
            text.0.clear();
            continue;
        }

        node.display = Display::Flex;

        let entry = &race.leaderboard[row_idx];
        let row_color = if entry.is_player { COLOR_PLAYER } else { COLOR_OTHER };

        match cell_kind {
            RivalHudCell::Pos => {
                text.0 = POSITION_LABELS[row_idx].to_string();
                color.0 = row_color;
            }
            RivalHudCell::Name => {
                // Truncate name to 4 characters.
                let name: String = entry.name.chars().take(4).collect();
                text.0 = name;
                color.0 = row_color;
            }
            RivalHudCell::Gap => {
                let gap_str = if let Some(ft) = entry.finish_time_s {
                    // Finished — show finish time as M:SS.cc
                    format_race_time(ft)
                } else if row_idx == 0 {
                    // Leader — show current lap progress.
                    format!("L{}/{}", entry.lap + 1, race.total_laps)
                } else {
                    // Not finished, not leader.
                    if entry.lap < leader_lap {
                        // On a different lap behind leader.
                        let lap_diff = leader_lap - entry.lap;
                        format!("-{}L", lap_diff)
                    } else {
                        // Same lap — gap in meters (negative = behind leader).
                        let gap_m = (entry.progress - leader_progress).floor() as i32;
                        format!("{}m", gap_m)
                    }
                };
                text.0 = gap_str;
                color.0 = row_color;
            }
        }
    }
}

// ---- Helpers ----------------------------------------------------------------

/// Format a race time (seconds) as `M:SS.cc`.
fn format_race_time(total_s: f32) -> String {
    let minutes = (total_s / 60.0) as u32;
    let secs = (total_s % 60.0) as u32;
    let cents = ((total_s % 1.0) * 100.0) as u32;
    format!("{}:{:02}.{:02}", minutes, secs, cents)
}
