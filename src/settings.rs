// Pause overlay and runtime settings for skoffroad.
//
// Esc toggles SettingsState::paused.  When paused:
//   - Avian physics halts via Time::<Physics>::pause() / unpause().
//   - A full-screen dark overlay with a centred panel appears.
//   - In-game drive input is zeroed so the vehicle doesn't drift further.
//   - TimeOfDay::paused is LEFT ALONE — it has its own T-key toggle.
//
// Adjustment keys (chosen to avoid W/A/S/D, arrows, Space, T, [, ], H, Q, E, V):
//   Volume        : Minus / Equal
//   Mouse sens    : Comma / Period
//   Day length    : Semicolon / Quote

use avian3d::prelude::{Physics, PhysicsTime};
use bevy::prelude::*;
use bevy_kira_audio::{Audio, AudioControl};
use bevy_kira_audio::prelude::Decibels;

use crate::graphics_quality::GraphicsQuality;
use crate::vehicle::DriveInput;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

pub struct SettingsPlugin;

impl Plugin for SettingsPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(SettingsState::default())
            .add_systems(Startup, spawn_overlay)
            .add_systems(
                Update,
                (
                    toggle_pause,
                    adjust_settings,
                    apply_physics_pause,
                    zero_drive_when_paused,
                    apply_master_volume,
                    update_overlay,
                )
                    .chain(),
            );
    }
}

/// Runtime-configurable settings, readable by other systems.
#[derive(Resource)]
pub struct SettingsState {
    pub paused:            bool,
    pub master_volume:     f32,   // 0.0 ..= 1.0
    pub mouse_sensitivity: f32,   // 0.1 ..= 3.0
    pub day_length_s:      f32,   // 30.0 ..= 600.0  (writes back to TimeOfDay)
}

impl Default for SettingsState {
    fn default() -> Self {
        Self {
            paused:            false,
            master_volume:     0.7,
            mouse_sensitivity: 1.0,
            day_length_s:      120.0,
        }
    }
}

// ---------------------------------------------------------------------------
// UI component markers
// ---------------------------------------------------------------------------

#[derive(Component)]
struct OverlayRoot;

#[derive(Component)]
enum SettingsText {
    Paused,
    Volume,
    Sensitivity,
    DayLength,
    Quality,
    Help,
}

// ---------------------------------------------------------------------------
// Colour constants (match HUD palette)
// ---------------------------------------------------------------------------

const OVERLAY_BG:  Color = Color::srgba(0.0, 0.0, 0.0, 0.55);
const PANEL_BG:    Color = Color::srgba(0.05, 0.05, 0.07, 0.88);
const COLOR_TITLE: Color = Color::srgb(1.0, 0.9, 0.3);
const COLOR_BODY:  Color = Color::srgb(0.85, 0.85, 0.85);
const COLOR_HELP:  Color = Color::srgb(0.55, 0.55, 0.55);

// ---------------------------------------------------------------------------
// Startup: build overlay tree (hidden by default)
// ---------------------------------------------------------------------------

fn spawn_overlay(mut commands: Commands) {
    // Full-screen dim layer — children are the centred panel
    let root = commands
        .spawn((
            OverlayRoot,
            Node {
                width:           Val::Percent(100.0),
                height:          Val::Percent(100.0),
                position_type:   PositionType::Absolute,
                align_items:     AlignItems::Center,
                justify_content: JustifyContent::Center,
                display:         Display::None, // hidden until Esc
                ..default()
            },
            BackgroundColor(OVERLAY_BG),
        ))
        .id();

    // Centred white panel
    let panel = commands
        .spawn((
            Node {
                width:           Val::Px(360.0),
                flex_direction:  FlexDirection::Column,
                padding:         UiRect::all(Val::Px(24.0)),
                row_gap:         Val::Px(12.0),
                ..default()
            },
            BackgroundColor(PANEL_BG),
        ))
        .id();

    let title = commands.spawn((
        SettingsText::Paused,
        Text::new("PAUSED"),
        TextFont { font_size: 40.0, ..default() },
        TextColor(COLOR_TITLE),
    )).id();

    let vol = commands.spawn((
        SettingsText::Volume,
        Text::new(""),
        TextFont { font_size: 16.0, ..default() },
        TextColor(COLOR_BODY),
    )).id();

    let sens = commands.spawn((
        SettingsText::Sensitivity,
        Text::new(""),
        TextFont { font_size: 16.0, ..default() },
        TextColor(COLOR_BODY),
    )).id();

    let day = commands.spawn((
        SettingsText::DayLength,
        Text::new(""),
        TextFont { font_size: 16.0, ..default() },
        TextColor(COLOR_BODY),
    )).id();

    let quality = commands.spawn((
        SettingsText::Quality,
        Text::new(""),
        TextFont { font_size: 16.0, ..default() },
        TextColor(COLOR_BODY),
    )).id();

    let help = commands.spawn((
        SettingsText::Help,
        Text::new(
            "[ - / = ] volume    [ , / . ] sensitivity\n\
             [ ; / ' ] day len   [ \\ ] quality   [ Esc ] resume",
        ),
        TextFont { font_size: 13.0, ..default() },
        TextColor(COLOR_HELP),
    )).id();

    commands.entity(panel).add_children(&[title, vol, sens, day, quality, help]);
    commands.entity(root).add_children(&[panel]);
}

// ---------------------------------------------------------------------------
// Toggle pause on Esc
// ---------------------------------------------------------------------------

fn toggle_pause(
    keys:     Res<ButtonInput<KeyCode>>,
    mut cfg:  ResMut<SettingsState>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        cfg.paused = !cfg.paused;
    }
}

// ---------------------------------------------------------------------------
// Keyboard adjustment of settings values while paused
// ---------------------------------------------------------------------------

fn adjust_settings(
    keys:        Res<ButtonInput<KeyCode>>,
    mut cfg:     ResMut<SettingsState>,
    mut tod:     ResMut<crate::sky::TimeOfDay>,
    mut quality: ResMut<GraphicsQuality>,
) {
    if !cfg.paused {
        return;
    }

    // Backslash cycles GraphicsQuality. The change persists via config.rs and
    // takes full effect on the next launch (some post-FX components only
    // attach in PostStartup); the wetness, splat-blend and bloom components
    // already react live.
    if keys.just_pressed(KeyCode::Backslash) {
        *quality = quality.cycle_next();
        info!("settings: graphics quality -> {}", quality.as_str());
    }

    // Volume: Minus / Equal
    if keys.just_pressed(KeyCode::Minus) {
        cfg.master_volume = (cfg.master_volume - 0.05).max(0.0);
    }
    if keys.just_pressed(KeyCode::Equal) {
        cfg.master_volume = (cfg.master_volume + 0.05).min(1.0);
    }

    // Mouse sensitivity: Comma / Period
    if keys.just_pressed(KeyCode::Comma) {
        cfg.mouse_sensitivity = (cfg.mouse_sensitivity - 0.1).max(0.1);
    }
    if keys.just_pressed(KeyCode::Period) {
        cfg.mouse_sensitivity = (cfg.mouse_sensitivity + 0.1).min(3.0);
    }

    // Day length: Semicolon / Quote
    if keys.just_pressed(KeyCode::Semicolon) {
        cfg.day_length_s = (cfg.day_length_s - 10.0).max(30.0);
    }
    if keys.just_pressed(KeyCode::Quote) {
        cfg.day_length_s = (cfg.day_length_s + 10.0).min(600.0);
    }

    // Write day_length_s back to the sky resource immediately.
    tod.day_length_s = cfg.day_length_s;
}

// ---------------------------------------------------------------------------
// Pause / resume Avian physics via Time::<Physics>
// ---------------------------------------------------------------------------

fn apply_physics_pause(
    cfg:       Res<SettingsState>,
    mut phys:  ResMut<Time<Physics>>,
) {
    if !cfg.is_changed() {
        return;
    }
    if cfg.paused {
        phys.pause();
    } else {
        phys.unpause();
    }
}

// ---------------------------------------------------------------------------
// Zero drive input while paused so the vehicle doesn't accelerate
// ---------------------------------------------------------------------------

fn zero_drive_when_paused(
    cfg:        Res<SettingsState>,
    mut drive:  ResMut<DriveInput>,
) {
    if cfg.paused {
        drive.drive  = 0.0;
        drive.steer  = 0.0;
        drive.brake  = false;
    }
}

// ---------------------------------------------------------------------------
// Apply master_volume to the kira Audio channel
// ---------------------------------------------------------------------------

fn apply_master_volume(
    cfg:   Res<SettingsState>,
    audio: Option<Res<Audio>>,
) {
    // Only act when the value actually changes; avoids spamming kira each frame.
    if !cfg.is_changed() {
        return;
    }
    let Some(audio) = audio else { return };

    // Convert linear 0..1 to decibels.  Silence floor at -60 dB.
    let db = 20.0 * cfg.master_volume.max(1e-6_f32).log10();
    let db = db.max(-60.0);
    audio.set_volume(Decibels(db));
}

// ---------------------------------------------------------------------------
// Show/hide overlay and refresh text content
// ---------------------------------------------------------------------------

fn update_overlay(
    cfg:       Res<SettingsState>,
    quality:   Res<GraphicsQuality>,
    mut roots: Query<&mut Node, With<OverlayRoot>>,
    mut texts: Query<(&SettingsText, &mut Text)>,
) {
    // Toggle overlay visibility
    for mut node in &mut roots {
        node.display = if cfg.paused { Display::Flex } else { Display::None };
    }

    if !cfg.paused {
        return;
    }

    for (label, mut text) in &mut texts {
        match label {
            SettingsText::Paused => { /* static */ }
            SettingsText::Volume => {
                text.0 = format!(
                    "Volume:       {}  {:.0}%",
                    bar12(cfg.master_volume),
                    cfg.master_volume * 100.0,
                );
            }
            SettingsText::Sensitivity => {
                text.0 = format!(
                    "Mouse sens:   {}  {:.1}",
                    bar12((cfg.mouse_sensitivity - 0.1) / 2.9),
                    cfg.mouse_sensitivity,
                );
            }
            SettingsText::DayLength => {
                text.0 = format!(
                    "Day length:   {}  {:.0}s",
                    bar12((cfg.day_length_s - 30.0) / 570.0),
                    cfg.day_length_s,
                );
            }
            SettingsText::Quality => {
                let q_idx = match *quality {
                    GraphicsQuality::Low => 0,
                    GraphicsQuality::Medium => 1,
                    GraphicsQuality::High => 2,
                };
                let bar_t = q_idx as f32 / 2.0;
                let label = match *quality {
                    GraphicsQuality::Low => "LOW    (legacy / older HW)",
                    GraphicsQuality::Medium => "MEDIUM (PBR + tonemap)",
                    GraphicsQuality::High => "HIGH   (PBR + SSAO + grading)",
                };
                text.0 = format!("Quality:      {}  {}", bar12(bar_t), label);
            }
            SettingsText::Help => { /* static */ }
        }
    }
}

// ---------------------------------------------------------------------------
// Bar helper (12 chars wide)
// ---------------------------------------------------------------------------

fn bar12(t: f32) -> String {
    let filled = (t.clamp(0.0, 1.0) * 12.0).round() as usize;
    "\u{2588}".repeat(filled) + &"\u{2591}".repeat(12 - filled)
}
