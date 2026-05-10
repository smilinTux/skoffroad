// Vehicle modification system — Sprint 48.
//
// Lets the player toggle a small set of accessories that affect both the
// visual appearance and (for long-arm / tire-size) the physics.
//
// KEY BINDINGS (active only when the mods panel is open — M to toggle):
//   M   — open / close the mods panel
//   1   — toggle long-arm suspension kit
//   2   — cycle tire size  (Stock → 35" → 37" → Stock)
//   3   — cycle bumper     (Stock → SteelFront → SteelFrontRear → Stock)
//   4   — toggle winch     (requires BumperKind >= SteelFront)
//   Esc — close the panel
//
// IMPORTANT: mods take effect on the NEXT chassis respawn (R key).
//
// Persistence: saves to "vehicle_mods.json" via platform_storage
// (native → ~/.skoffroad/vehicle_mods.json, WASM → localStorage).

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use crate::platform_storage;

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

/// Tire-size preset.  Physical wheel radius in metres:
///   Stock (33") = 0.35 m   35" = 0.40 m   37" = 0.45 m
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum TireSize {
    #[default]
    Stock,  // 33" — matches original WHEEL_RADIUS constant
    Large,  // 35"
    XLarge, // 37"
}

impl TireSize {
    /// Physical radius in metres used for wheel mesh and raycast.
    pub fn radius(self) -> f32 {
        match self {
            TireSize::Stock  => 0.35,
            TireSize::Large  => 0.40,
            TireSize::XLarge => 0.45,
        }
    }
    /// Human-readable label.
    pub fn label(self) -> &'static str {
        match self {
            TireSize::Stock  => "33\" (Stock)",
            TireSize::Large  => "35\"",
            TireSize::XLarge => "37\"",
        }
    }
    /// Cycle through variants in order.
    pub fn next(self) -> Self {
        match self {
            TireSize::Stock  => TireSize::Large,
            TireSize::Large  => TireSize::XLarge,
            TireSize::XLarge => TireSize::Stock,
        }
    }
}

/// Bumper kit preset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default)]
pub enum BumperKind {
    #[default]
    Stock,         // original thin plastic-look bumpers
    SteelFront,    // front replaced with chunky steel + D-rings
    SteelFrontRear, // both bumpers replaced
}

impl BumperKind {
    pub fn label(self) -> &'static str {
        match self {
            BumperKind::Stock         => "Stock",
            BumperKind::SteelFront    => "Steel Front",
            BumperKind::SteelFrontRear => "Steel Front+Rear",
        }
    }
    /// Cycle through variants in order.
    pub fn next(self) -> Self {
        match self {
            BumperKind::Stock         => BumperKind::SteelFront,
            BumperKind::SteelFront    => BumperKind::SteelFrontRear,
            BumperKind::SteelFrontRear => BumperKind::Stock,
        }
    }
}

/// The full mods state — persisted to disk and read by `spawn_vehicle`.
///
/// All defaults must produce behaviour identical to the pre-Sprint-48 vehicle
/// so that `cargo test --test drive_test` keeps passing without modification.
#[derive(Resource, Serialize, Deserialize, Clone, Debug)]
pub struct VehicleModsState {
    /// Long-arm suspension kit: increases suspension travel and chassis height.
    pub long_arm:  bool,
    /// Tire size preset (affects wheel radius and raycast length).
    pub tire_size: TireSize,
    /// Bumper kit preset.
    pub bumper:    BumperKind,
    /// Winch visual (only available when bumper != Stock).
    pub winch:     bool,
}

impl Default for VehicleModsState {
    fn default() -> Self {
        Self {
            long_arm:  false,
            tire_size: TireSize::Stock,
            bumper:    BumperKind::Stock,
            winch:     false,
        }
    }
}

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct VehicleModsPlugin;

impl Plugin for VehicleModsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<VehicleModsState>()
           .init_resource::<ModsPanelState>()
           .add_systems(Startup, (load_mods_state, spawn_mods_panel).chain())
           .add_systems(Update, (
               toggle_mods_panel,
               handle_mods_keys,
               update_mods_panel_view,
               save_mods_on_change,
           ));
    }
}

// ---------------------------------------------------------------------------
// Persistence
// ---------------------------------------------------------------------------

const STORAGE_KEY: &str = "vehicle_mods.json";

fn load_mods_state(mut state: ResMut<VehicleModsState>) {
    match platform_storage::read_string(STORAGE_KEY) {
        None => {
            info!("vehicle_mods: no saved state, using defaults");
        }
        Some(json) => match serde_json::from_str::<VehicleModsState>(&json) {
            Ok(loaded) => {
                *state = loaded;
                info!("vehicle_mods: loaded from {}", STORAGE_KEY);
            }
            Err(e) => {
                warn!("vehicle_mods: could not parse {}: {}; using defaults", STORAGE_KEY, e);
            }
        },
    }
}

fn save_mods_on_change(state: Res<VehicleModsState>) {
    if !state.is_changed() {
        return;
    }
    match serde_json::to_string_pretty(state.as_ref()) {
        Ok(json) => {
            if let Err(e) = platform_storage::write_string(STORAGE_KEY, &json) {
                warn!("vehicle_mods: save failed: {}", e);
            }
        }
        Err(e) => warn!("vehicle_mods: serialize failed: {}", e),
    }
}

// ---------------------------------------------------------------------------
// UI state + panel components
// ---------------------------------------------------------------------------

#[derive(Resource, Default)]
pub struct ModsPanelState {
    pub open: bool,
}

#[derive(Component)]
struct ModsPanelRoot;

#[derive(Component)]
struct ModsStatusText;

// Color constants for the panel.
const PANEL_BG:    Color = Color::srgba(0.04, 0.04, 0.06, 0.92);
const COLOR_TITLE: Color = Color::srgb(1.0, 0.75, 0.2);
const COLOR_BODY:  Color = Color::srgb(0.85, 0.85, 0.85);
const COLOR_HINT:  Color = Color::srgb(0.55, 0.55, 0.55);

fn spawn_mods_panel(mut commands: Commands) {
    // Full-screen overlay (hidden until M is pressed).
    let root = commands.spawn((
        ModsPanelRoot,
        Node {
            width:           Val::Percent(100.0),
            height:          Val::Percent(100.0),
            position_type:   PositionType::Absolute,
            align_items:     AlignItems::Center,
            justify_content: JustifyContent::Center,
            display:         Display::None,
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.45)),
    )).id();

    // Centred modal panel.
    let panel = commands.spawn((
        Node {
            width:          Val::Px(420.0),
            flex_direction: FlexDirection::Column,
            padding:        UiRect::all(Val::Px(20.0)),
            row_gap:        Val::Px(8.0),
            ..default()
        },
        BackgroundColor(PANEL_BG),
    )).id();

    // Title.
    let title = commands.spawn((
        Text::new("VEHICLE MODS"),
        TextFont { font_size: 24.0, ..default() },
        TextColor(COLOR_TITLE),
    )).id();

    // Status block (updated every frame the panel is open).
    let status = commands.spawn((
        ModsStatusText,
        Text::new(""),
        TextFont { font_size: 15.0, ..default() },
        TextColor(COLOR_BODY),
    )).id();

    // Footer hint.
    let footer = commands.spawn((
        Text::new("1 long-arm   2 tire size   3 bumper   4 winch   Esc close\nChanges apply instantly (chassis respawns)"),
        TextFont { font_size: 12.0, ..default() },
        TextColor(COLOR_HINT),
    )).id();

    commands.entity(panel).add_children(&[title, status, footer]);
    commands.entity(root).add_child(panel);
}

// ---------------------------------------------------------------------------
// Input handling
// ---------------------------------------------------------------------------

fn toggle_mods_panel(
    keys:      Res<ButtonInput<KeyCode>>,
    mut panel: ResMut<ModsPanelState>,
) {
    // Shift+M: M alone is taken by minimap (toggle) and medals (panel).
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if shift && keys.just_pressed(KeyCode::KeyM) {
        panel.open = !panel.open;
    }
    if keys.just_pressed(KeyCode::Escape) {
        panel.open = false;
    }
}

fn handle_mods_keys(
    keys:        Res<ButtonInput<KeyCode>>,
    panel:       Res<ModsPanelState>,
    mut state:   ResMut<VehicleModsState>,
    mut respawn: ResMut<crate::vehicle::RespawnRequest>,
) {
    if !panel.open {
        return;
    }

    let mut changed = false;

    // 1 — toggle long-arm kit.
    if keys.just_pressed(KeyCode::Digit1) {
        state.long_arm = !state.long_arm;
        changed = true;
    }

    // 2 — cycle tire size.
    if keys.just_pressed(KeyCode::Digit2) {
        state.tire_size = state.tire_size.next();
        changed = true;
    }

    // 3 — cycle bumper.
    if keys.just_pressed(KeyCode::Digit3) {
        state.bumper = state.bumper.next();
        // If bumper is now Stock, force winch off (winch requires a steel bumper).
        if state.bumper == BumperKind::Stock {
            state.winch = false;
        }
        changed = true;
    }

    // 4 — toggle winch (requires steel bumper).
    if keys.just_pressed(KeyCode::Digit4) {
        if state.bumper != BumperKind::Stock {
            state.winch = !state.winch;
            changed = true;
        }
        // If bumper is Stock, silently ignore (or show a subtle hint via panel text).
    }

    // Trigger an immediate chassis respawn so the visual changes are visible
    // without the player having to remember to press R.
    if changed {
        respawn.0 = true;
    }
}

// ---------------------------------------------------------------------------
// Panel view update
// ---------------------------------------------------------------------------

fn update_mods_panel_view(
    panel_state: Res<ModsPanelState>,
    mods:        Res<VehicleModsState>,
    mut roots:   Query<&mut Node, With<ModsPanelRoot>>,
    mut texts:   Query<&mut Text, With<ModsStatusText>>,
) {
    // Show / hide.
    for mut node in &mut roots {
        node.display = if panel_state.open { Display::Flex } else { Display::None };
    }

    if !panel_state.open {
        return;
    }

    // Build status string.
    let winch_str = if mods.bumper == BumperKind::Stock {
        "n/a (need steel bumper)"
    } else if mods.winch {
        "ON"
    } else {
        "OFF"
    };

    let status = format!(
        "[1] Long-arm kit : {}\n[2] Tire size    : {}\n[3] Bumper kit   : {}\n[4] Winch        : {}",
        if mods.long_arm { "ON" } else { "OFF" },
        mods.tire_size.label(),
        mods.bumper.label(),
        winch_str,
    );

    for mut text in &mut texts {
        text.0 = status.clone();
    }
}

// ---------------------------------------------------------------------------
// Physics / mesh constants exposed to vehicle.rs
// ---------------------------------------------------------------------------

/// Base suspension length (stock). Increased when long-arm kit is active.
pub const BASE_SUSPENSION_LEN: f32 = 0.60;

/// Extra suspension length added by the long-arm kit.
pub const LONG_ARM_SUSP_DELTA: f32 = 0.25; // 0.60 → 0.85 m

/// Chassis spawn-Y lift added by the long-arm kit.
pub const LONG_ARM_SPAWN_LIFT: f32 = 0.20;

impl VehicleModsState {
    /// Effective suspension length for physics.
    /// Bigger tires extend the rest length by the tire delta so the chassis
    /// rides higher (otherwise the larger wheel mesh just clips through the fender).
    pub fn suspension_len(&self) -> f32 {
        let tire_delta = self.tire_size.radius() - TireSize::Stock.radius();
        let long_arm   = if self.long_arm { LONG_ARM_SUSP_DELTA } else { 0.0 };
        BASE_SUSPENSION_LEN + long_arm + tire_delta
    }

    /// Additional chassis Y offset at spawn so the chassis lands at the right
    /// rest height (long-arm + tire-size both add to this).
    pub fn spawn_y_lift(&self) -> f32 {
        let tire_delta = self.tire_size.radius() - TireSize::Stock.radius();
        let long_arm   = if self.long_arm { LONG_ARM_SPAWN_LIFT } else { 0.0 };
        long_arm + tire_delta
    }

    /// Wheel mesh radius.
    pub fn wheel_radius(&self) -> f32 {
        self.tire_size.radius()
    }

    /// Additional mass added by steel bumper pieces (+30 kg per steel piece).
    pub fn extra_mass(&self) -> f32 {
        match self.bumper {
            BumperKind::Stock         => 0.0,
            BumperKind::SteelFront    => 30.0,
            BumperKind::SteelFrontRear => 60.0,
        }
    }
}
