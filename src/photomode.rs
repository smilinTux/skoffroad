// Photo mode — press P to freeze the world and grab a clean screenshot.
//
// v0.4 limitation: other HUD panels (speed overlay, mini-map, compass, etc.)
// remain visible. Players can dismiss them individually with their own keys:
//   H  — hide HUD      M  — hide mini-map
//   C  — hide compass  F3 — hide dev inspector
// A per-panel "hide-in-photo-mode" pass is tracked for v0.5.

use bevy::{prelude::*, window::{CursorOptions, PrimaryWindow}};
use crate::settings::SettingsState;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

pub struct PhotoModePlugin;

impl Plugin for PhotoModePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PhotoMode>()
            .add_systems(Startup, spawn_photo_banner)
            .add_systems(Update, (toggle_photo_mode, update_photo_banner));
    }
}

/// Global photo-mode state — read by any system that wants to know.
#[derive(Resource, Default)]
pub struct PhotoMode {
    pub active:         bool,
    /// True for exactly one frame after activation — useful for sound stings.
    pub just_activated: bool,
}

// ---------------------------------------------------------------------------
// UI components
// ---------------------------------------------------------------------------

#[derive(Component)]
struct PhotoBannerRoot;

// ---------------------------------------------------------------------------
// Colours
// ---------------------------------------------------------------------------

const BANNER_BG:   Color = Color::srgba(0.05, 0.05, 0.07, 0.85);
const BANNER_TEXT: Color = Color::WHITE;

// ---------------------------------------------------------------------------
// Startup: build banner (hidden by default)
// ---------------------------------------------------------------------------

fn spawn_photo_banner(mut commands: Commands) {
    let root = commands.spawn((
        PhotoBannerRoot,
        Node {
            position_type:   PositionType::Absolute,
            // Centre horizontally: place left edge at 50 %, pull back by half
            // the banner width (200 px) via a negative left margin.
            left:            Val::Percent(50.0),
            top:             Val::Px(8.0),
            width:           Val::Px(400.0),
            padding:         UiRect {
                left:   Val::Px(16.0),
                right:  Val::Px(16.0),
                top:    Val::Px(6.0),
                bottom: Val::Px(6.0),
            },
            margin:          UiRect {
                left: Val::Px(-200.0),
                ..default()
            },
            justify_content: JustifyContent::Center,
            align_items:     AlignItems::Center,
            display:         Display::None, // hidden until photo mode activates
            ..default()
        },
        BackgroundColor(BANNER_BG),
        Outline {
            width: Val::Px(1.0),
            color: Color::WHITE,
            offset: Val::Px(0.0),
        },
        // Sits above other panels (z-index 10).
        ZIndex(10),
    )).id();

    let label = commands.spawn((
        Text::new("PHOTO MODE  —  press P to exit"),
        TextFont { font_size: 14.0, ..default() },
        TextColor(BANNER_TEXT),
    )).id();

    commands.entity(root).add_child(label);
}

// ---------------------------------------------------------------------------
// Toggle on KeyP
// ---------------------------------------------------------------------------

fn toggle_photo_mode(
    keys:         Res<ButtonInput<KeyCode>>,
    mut pm:       ResMut<PhotoMode>,
    mut cfg:      ResMut<SettingsState>,
    mut cursor_q: Query<&mut CursorOptions, With<PrimaryWindow>>,
) {
    // Clear one-frame flag from last activation.
    pm.just_activated = false;

    if !keys.just_pressed(KeyCode::KeyP) {
        return;
    }

    pm.active = !pm.active;

    if pm.active {
        pm.just_activated = true;
        cfg.paused = true;

        // Hide the OS cursor so it doesn't appear in screenshots.
        if let Ok(mut cursor) = cursor_q.single_mut() {
            cursor.visible = false;
        }
    } else {
        cfg.paused = false;

        if let Ok(mut cursor) = cursor_q.single_mut() {
            cursor.visible = true;
        }
    }
}

// ---------------------------------------------------------------------------
// Show / hide banner to match photo-mode state
// ---------------------------------------------------------------------------

fn update_photo_banner(
    pm:       Res<PhotoMode>,
    mut roots: Query<&mut Node, With<PhotoBannerRoot>>,
) {
    for mut node in &mut roots {
        node.display = if pm.active { Display::Flex } else { Display::None };
    }
}
