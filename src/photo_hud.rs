// photo_hud.rs — Sprint 40
//
// Visual overlay shown while photo mode is active.  Distinct from
// photomode.rs (which owns the PhotoMode resource and the toggle banner).
//
// Layout when active:
//   ┌─────────────────────────────────┐
//   │ PHOTO MODE           (top-left) │
//   │   rule-of-thirds grid           │
//   │                                 │
//   │ ISO 200 | f/8 | 1/250s (btm-l) │  Press F12 to capture  (btm-r) │
//   └─────────────────────────────────┘
//
// All geometry is built in Startup; visibility is toggled every Update
// frame to track PhotoMode.active.

use bevy::prelude::*;

use crate::photomode::PhotoMode;

// ---------------------------------------------------------------------------
// Public plugin
// ---------------------------------------------------------------------------

pub struct PhotoHudPlugin;

impl Plugin for PhotoHudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_photo_hud)
            .add_systems(Update, toggle_photo_hud_visibility);
    }
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Colour of the rule-of-thirds grid lines.
const GRID_LINE_COLOR: Color = Color::srgba(1.0, 1.0, 1.0, 0.4);

/// "PHOTO MODE" label colour.
const LABEL_PHOTO_MODE_COLOR: Color = Color::srgb(1.0, 0.95, 0.0); // yellow

/// Exposure / hint text colour.
const LABEL_INFO_COLOR: Color = Color::WHITE;

// ---------------------------------------------------------------------------
// Marker component
// ---------------------------------------------------------------------------

/// Marks the root entity of the photo-mode HUD overlay.
#[derive(Component)]
pub struct PhotoHudRoot;

// ---------------------------------------------------------------------------
// Startup: spawn the full overlay (hidden by default)
// ---------------------------------------------------------------------------

fn spawn_photo_hud(mut commands: Commands) {
    // -----------------------------------------------------------------------
    // Root — full-screen container, hidden until photo mode activates.
    // -----------------------------------------------------------------------
    let root = commands
        .spawn((
            PhotoHudRoot,
            Node {
                position_type: PositionType::Absolute,
                left:   Val::Px(0.0),
                top:    Val::Px(0.0),
                width:  Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            ZIndex(50),
            Visibility::Hidden,
        ))
        .id();

    // -----------------------------------------------------------------------
    // Rule-of-thirds grid
    // Four lines:
    //   H1 — horizontal at 33 %
    //   H2 — horizontal at 67 %
    //   V1 — vertical   at 33 %
    //   V2 — vertical   at 67 %
    // -----------------------------------------------------------------------

    // Horizontal line at 33 %
    let h1 = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left:   Val::Percent(0.0),
                top:    Val::Percent(33.333),
                width:  Val::Percent(100.0),
                height: Val::Px(1.0),
                ..default()
            },
            BackgroundColor(GRID_LINE_COLOR),
        ))
        .id();

    // Horizontal line at 67 %
    let h2 = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left:   Val::Percent(0.0),
                top:    Val::Percent(66.667),
                width:  Val::Percent(100.0),
                height: Val::Px(1.0),
                ..default()
            },
            BackgroundColor(GRID_LINE_COLOR),
        ))
        .id();

    // Vertical line at 33 %
    let v1 = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left:   Val::Percent(33.333),
                top:    Val::Percent(0.0),
                width:  Val::Px(1.0),
                height: Val::Percent(100.0),
                ..default()
            },
            BackgroundColor(GRID_LINE_COLOR),
        ))
        .id();

    // Vertical line at 67 %
    let v2 = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left:   Val::Percent(66.667),
                top:    Val::Percent(0.0),
                width:  Val::Px(1.0),
                height: Val::Percent(100.0),
                ..default()
            },
            BackgroundColor(GRID_LINE_COLOR),
        ))
        .id();

    // -----------------------------------------------------------------------
    // "PHOTO MODE" — top-left, 24 pt yellow
    // -----------------------------------------------------------------------
    let label_photo_mode = commands
        .spawn((
            Text::new("PHOTO MODE"),
            TextFont { font_size: 24.0, ..default() },
            TextColor(LABEL_PHOTO_MODE_COLOR),
            Node {
                position_type: PositionType::Absolute,
                left:   Val::Px(16.0),
                top:    Val::Px(16.0),
                ..default()
            },
        ))
        .id();

    // -----------------------------------------------------------------------
    // "ISO 200 | f/8 | 1/250s" — bottom-left, 14 pt white
    // -----------------------------------------------------------------------
    let label_exposure = commands
        .spawn((
            Text::new("ISO 200  |  f/8  |  1/250s"),
            TextFont { font_size: 14.0, ..default() },
            TextColor(LABEL_INFO_COLOR),
            Node {
                position_type: PositionType::Absolute,
                left:   Val::Px(16.0),
                bottom: Val::Px(16.0),
                ..default()
            },
        ))
        .id();

    // -----------------------------------------------------------------------
    // "Press F12 to capture" — bottom-right, 14 pt white
    // -----------------------------------------------------------------------
    let label_hint = commands
        .spawn((
            Text::new("Press F12 to capture"),
            TextFont { font_size: 14.0, ..default() },
            TextColor(LABEL_INFO_COLOR),
            Node {
                position_type: PositionType::Absolute,
                right:  Val::Px(16.0),
                bottom: Val::Px(16.0),
                ..default()
            },
        ))
        .id();

    // -----------------------------------------------------------------------
    // Wire children onto root
    // -----------------------------------------------------------------------
    commands.entity(root).add_children(&[
        h1,
        h2,
        v1,
        v2,
        label_photo_mode,
        label_exposure,
        label_hint,
    ]);
}

// ---------------------------------------------------------------------------
// Update: mirror PhotoMode.active → root Visibility
// ---------------------------------------------------------------------------

fn toggle_photo_hud_visibility(
    pm:       Res<PhotoMode>,
    mut roots: Query<&mut Visibility, With<PhotoHudRoot>>,
) {
    let target = if pm.active {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };

    for mut vis in &mut roots {
        if *vis != target {
            *vis = target;
        }
    }
}
