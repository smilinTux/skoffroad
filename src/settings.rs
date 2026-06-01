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
//   Graphics qual : Backslash (cycles Low -> Medium -> High -> Low)

use avian3d::prelude::{Physics, PhysicsTime};
use bevy::prelude::*;
use bevy_kira_audio::{Audio, AudioControl};
use bevy_kira_audio::prelude::Decibels;

use crate::asset_attribution::AttributionState;
use crate::buddy_recovery::RecoveryState as BuddyRecoveryState;
use crate::changelog::ChangelogState;
use crate::credits::CreditsState;
use crate::fast_travel_menu::FastTravelMenuState;
use crate::garage_build::GarageBuildUiState;
use crate::graphics_quality::GraphicsQuality;
use crate::map_select::MapSelectState;
use crate::mission_select::MissionSelectOpen;
use crate::vehicle::DriveInput;
use crate::vehicle_mods::ModsPanelState;
use crate::winch::WinchState;

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
enum SettingsRow {
    Volume,
    Sensitivity,
    DayLength,
    Quality,
}

// ---------------------------------------------------------------------------
// Colour constants (match HUD palette)
// ---------------------------------------------------------------------------

const OVERLAY_BG:    Color = Color::srgba(0.0, 0.0, 0.0, 0.55);
const PANEL_BG:      Color = Color::srgba(0.05, 0.05, 0.07, 0.92);
const COLOR_TITLE:   Color = Color::srgb(1.0, 0.9, 0.3);
const COLOR_LABEL:   Color = Color::srgb(0.70, 0.72, 0.75);
const COLOR_VALUE:   Color = Color::srgb(1.0,  1.0,  1.0);
const COLOR_KEY:     Color = Color::srgb(0.55, 0.85, 0.55);
const COLOR_HELP:    Color = Color::srgb(0.50, 0.50, 0.55);
const COLOR_DIVIDER: Color = Color::srgba(0.3, 0.3, 0.35, 0.6);
const COLOR_HINT:    Color = Color::srgb(0.45, 0.55, 0.65);

// ---------------------------------------------------------------------------
// Startup: build overlay tree (hidden by default)
// ---------------------------------------------------------------------------

fn spawn_overlay(mut commands: Commands) {
    // Persistent bottom navigation hint — always visible in-game.
    // Gives new players a discoverable entry point for menus.
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            bottom:        Val::Px(6.0),
            left:          Val::Px(0.0),
            width:         Val::Percent(100.0),
            align_items:   AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
        GlobalZIndex(10),
    )).with_children(|p| {
        p.spawn((
            Text::new("Esc = pause/options   ? = controls   Shift+Tab = missions   Shift+G = garage"),
            TextFont { font_size: 11.0, ..default() },
            TextColor(Color::srgba(0.55, 0.65, 0.75, 0.70)),
        ));
    });

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
            GlobalZIndex(500),
        ))
        .id();

    // Centred panel — wider to fit labeled rows
    let panel = commands
        .spawn((
            Node {
                width:           Val::Px(480.0),
                flex_direction:  FlexDirection::Column,
                padding:         UiRect::all(Val::Px(28.0)),
                row_gap:         Val::Px(0.0),
                ..default()
            },
            BackgroundColor(PANEL_BG),
            BorderColor::all(Color::srgba(0.4, 0.4, 0.5, 0.5)),
        ))
        .id();

    // Title row
    let title = commands.spawn((
        Text::new("OPTIONS  /  PAUSED"),
        TextFont { font_size: 26.0, ..default() },
        TextColor(COLOR_TITLE),
        Node { margin: UiRect::bottom(Val::Px(6.0)), ..default() },
    )).id();

    let subtitle = commands.spawn((
        Text::new("Press Esc to resume"),
        TextFont { font_size: 13.0, ..default() },
        TextColor(COLOR_HINT),
        Node { margin: UiRect::bottom(Val::Px(16.0)), ..default() },
    )).id();

    // Section divider
    let div1 = divider(&mut commands);

    // Column header row: SETTING / VALUE / KEYS
    let hdr = row_header(&mut commands);

    let div2 = divider(&mut commands);

    // Settings rows — each is a 3-column row: label | bar+value | key hint
    let vol_row = setting_row(&mut commands, SettingsRow::Volume,
        "Volume", "- / =");
    let sens_row = setting_row(&mut commands, SettingsRow::Sensitivity,
        "Mouse Sensitivity", ", / .");
    let day_row = setting_row(&mut commands, SettingsRow::DayLength,
        "Day Length", "; / '");
    let qual_row = setting_row(&mut commands, SettingsRow::Quality,
        "Graphics Quality", "\\ to cycle");

    let div3 = divider(&mut commands);

    // Global hint footer
    let help = commands.spawn((
        Text::new("Esc = resume game    ? = full keybind help    Shift+G = garage"),
        TextFont { font_size: 12.0, ..default() },
        TextColor(COLOR_HELP),
        Node { margin: UiRect::top(Val::Px(10.0)), ..default() },
    )).id();

    commands.entity(panel).add_children(&[
        title, subtitle,
        div1, hdr, div2,
        vol_row, sens_row, day_row, qual_row,
        div3, help,
    ]);
    commands.entity(root).add_children(&[panel]);
}

/// Build a thin horizontal rule acting as a divider.
fn divider(commands: &mut Commands) -> Entity {
    commands.spawn((
        Node {
            width:  Val::Percent(100.0),
            height: Val::Px(1.0),
            margin: UiRect::vertical(Val::Px(6.0)),
            ..default()
        },
        BackgroundColor(COLOR_DIVIDER),
    )).id()
}

/// Build the column header row ("SETTING | VALUE | KEYS").
fn row_header(commands: &mut Commands) -> Entity {
    let row = commands.spawn(Node {
        flex_direction: FlexDirection::Row,
        width:          Val::Percent(100.0),
        column_gap:     Val::Px(8.0),
        padding:        UiRect::vertical(Val::Px(2.0)),
        ..default()
    }).id();

    let lbl = commands.spawn((
        Text::new("SETTING"),
        TextFont { font_size: 11.0, ..default() },
        TextColor(Color::srgb(0.50, 0.52, 0.56)),
        Node { width: Val::Px(150.0), flex_shrink: 0.0, ..default() },
    )).id();
    let val = commands.spawn((
        Text::new("VALUE"),
        TextFont { font_size: 11.0, ..default() },
        TextColor(Color::srgb(0.50, 0.52, 0.56)),
        Node { flex_grow: 1.0, ..default() },
    )).id();
    let key = commands.spawn((
        Text::new("KEY"),
        TextFont { font_size: 11.0, ..default() },
        TextColor(Color::srgb(0.50, 0.52, 0.56)),
        Node { width: Val::Px(120.0), flex_shrink: 0.0, ..default() },
    )).id();

    commands.entity(row).add_children(&[lbl, val, key]);
    row
}

/// Build a labeled setting row.  The SettingsRow marker is placed on the
/// VALUE cell so `update_overlay` can find it and update the text.
fn setting_row(
    commands: &mut Commands,
    marker:   SettingsRow,
    label:    &str,
    key_hint: &str,
) -> Entity {
    let row = commands.spawn(Node {
        flex_direction: FlexDirection::Row,
        width:          Val::Percent(100.0),
        align_items:    AlignItems::Center,
        column_gap:     Val::Px(8.0),
        padding:        UiRect::vertical(Val::Px(5.0)),
        ..default()
    }).id();

    let lbl = commands.spawn((
        Text::new(label),
        TextFont { font_size: 15.0, ..default() },
        TextColor(COLOR_LABEL),
        Node { width: Val::Px(150.0), flex_shrink: 0.0, ..default() },
    )).id();

    // Value cell — marker here so update_overlay can look it up.
    let val = commands.spawn((
        marker,
        Text::new(""),
        TextFont { font_size: 15.0, ..default() },
        TextColor(COLOR_VALUE),
        Node { flex_grow: 1.0, ..default() },
    )).id();

    let key = commands.spawn((
        Text::new(key_hint),
        TextFont { font_size: 13.0, ..default() },
        TextColor(COLOR_KEY),
        Node { width: Val::Px(120.0), flex_shrink: 0.0, ..default() },
    )).id();

    commands.entity(row).add_children(&[lbl, val, key]);
    row
}

// ---------------------------------------------------------------------------
// Toggle pause on Esc
// ---------------------------------------------------------------------------

fn toggle_pause(
    keys:         Res<ButtonInput<KeyCode>>,
    mut cfg:      ResMut<SettingsState>,
    garage:       Option<Res<GarageBuildUiState>>,
    ms_open:      Option<Res<MissionSelectOpen>>,
    ft_menu:      Option<Res<FastTravelMenuState>>,
    map_sel:      Option<Res<MapSelectState>>,
    mods_panel:   Option<Res<ModsPanelState>>,
    changelog:    Option<Res<ChangelogState>>,
    credits:      Option<Res<CreditsState>>,
    attribution:  Option<Res<AttributionState>>,
    winch:        Option<Res<WinchState>>,
    recovery:     Option<Res<BuddyRecoveryState>>,
) {
    if !keys.just_pressed(KeyCode::Escape) {
        return;
    }
    // Don't toggle the pause/settings overlay if another system already consumed Esc.
    // Each modal/action handles its own Esc; we just avoid double-firing here.
    let garage_open      = garage.map(|g| g.open).unwrap_or(false);
    let missions_open    = ms_open.map(|m| m.0).unwrap_or(false);
    let ft_open          = ft_menu.map(|f| f.open).unwrap_or(false);
    let map_open         = map_sel.map(|m| m.open).unwrap_or(false);
    let mods_open        = mods_panel.map(|m| m.open).unwrap_or(false);
    let changelog_open   = changelog.map(|c| c.open).unwrap_or(false);
    let credits_open     = credits.map(|c| c.open).unwrap_or(false);
    let attr_open        = attribution.map(|a| a.open).unwrap_or(false);
    let winch_active     = winch.map(|w| w.anchor_pos.is_some()).unwrap_or(false);
    let recovery_active  = recovery.map(|r| r.active.is_some()).unwrap_or(false);
    if garage_open || missions_open || ft_open || map_open || mods_open
        || changelog_open || credits_open || attr_open
        || winch_active || recovery_active
    {
        return;
    }
    cfg.paused = !cfg.paused;
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
    mut rows:  Query<(&SettingsRow, &mut Text)>,
) {
    // Toggle overlay visibility
    for mut node in &mut roots {
        node.display = if cfg.paused { Display::Flex } else { Display::None };
    }

    if !cfg.paused {
        return;
    }

    for (row, mut text) in &mut rows {
        match row {
            SettingsRow::Volume => {
                text.0 = format!(
                    "[{}]  {:.0}%",
                    bar14(cfg.master_volume),
                    cfg.master_volume * 100.0,
                );
            }
            SettingsRow::Sensitivity => {
                text.0 = format!(
                    "[{}]  {:.1}x",
                    bar14((cfg.mouse_sensitivity - 0.1) / 2.9),
                    cfg.mouse_sensitivity,
                );
            }
            SettingsRow::DayLength => {
                let mins = (cfg.day_length_s / 60.0) as u32;
                let secs = (cfg.day_length_s % 60.0) as u32;
                text.0 = format!(
                    "[{}]  {}m{}s",
                    bar14((cfg.day_length_s - 30.0) / 570.0),
                    mins, secs,
                );
            }
            SettingsRow::Quality => {
                let (label, t) = match *quality {
                    GraphicsQuality::Low    => ("Low    (older HW / legacy)", 0.0_f32),
                    GraphicsQuality::Medium => ("Medium (PBR + tonemap)",     0.5_f32),
                    GraphicsQuality::High   => ("High   (PBR + SSAO)",        1.0_f32),
                };
                text.0 = format!("[{}]  {}", bar14(t), label);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// ASCII bar helper (14 chars wide, e.g. "#########-----")
// ---------------------------------------------------------------------------

fn bar14(t: f32) -> String {
    let filled = (t.clamp(0.0, 1.0) * 14.0).round() as usize;
    "#".repeat(filled) + &"-".repeat(14 - filled)
}
