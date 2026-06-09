// Garage Build Screen -- Sprint 74 (polished).
//
// Full-screen overlay where the player configures their rig before driving.
// Opens automatically on first run (no saved build) or via Shift+G in-game.
//
// Left column: header band, category rows (Truck / Paint / Tire / Bumper /
//              Winch / Body-lift / Long-arm / Livery) with < > cycle arrows
//              and 24x24 colored icon nodes.
// Right column: spec sheet (darker card) + derived stats + BUILD & DRIVE
//               button (amber, pulsing) + RANDOM RIG button.
//
// Polish added in Sprint 74:
//   * Showroom turntable: chassis slowly yaw-rotates ~20 deg/s while open.
//   * Camera override: 3/4 low-angle showcase while garage is open,
//     restored when closed.
//   * Category icons: 24x24 colored Node per row (no font glyphs).
//   * Selection highlight: rows flash amber for 0.3 s on change.
//   * Derived stats in spec sheet (lift, tire diameter, approach, weight).
//   * BUILD & DRIVE: amber, pulsing brightness, hover brighten.
//   * RANDOM RIG button: randomizes all selections.
//   * Fade-in backdrop over ~0.2 s.
//   * Brief amber flash confirmation before BUILD & DRIVE closes.
//
// FONT CONSTRAINT: NO Unicode glyphs -- ASCII only in Text::new().
//
// Hotkey: Shift+G (G alone is taken by gauge.rs / trailers.rs).
// GlobalZIndex 950 -- above HUD (~100), below title screen (1000).
//
// Persistence: garage_build.json via platform_storage.
// First-run detection: absence of garage_build.json AND save_1.json.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::platform_storage;
use crate::variants::VehicleVariant;
use crate::vehicle_mods::{BumperKind, TireSize, VehicleModsState};
use crate::paint_shop::PaintShopState;
use crate::vehicle::{Chassis, RespawnRequest};

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

pub struct GarageBuildPlugin;

impl Plugin for GarageBuildPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GarageBuildUiState>()
            .add_systems(Startup, (load_or_detect_first_run, spawn_garage_ui).chain())
            .add_systems(
                Update,
                (
                    open_close_garage,
                    handle_category_buttons,
                    handle_random_rig,
                    apply_selections_to_resources,
                    update_garage_ui,
                    update_row_highlights,
                    animate_build_button,
                    handle_build_and_drive,
                    turntable_and_camera,
                    tick_backdrop_fade,
                    tick_build_flash,
                ),
            );
    }
}

// ---------------------------------------------------------------------------
// Persisted save format
// ---------------------------------------------------------------------------

const STORAGE_KEY: &str = "garage_build.json";

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GarageBuildSave {
    pub variant_idx: u8,   // 0..7 maps to VehicleVariant
    pub paint_idx:   u8,   // 0..7 maps to PALETTE in paint_shop
    pub tire_size:   TireSize,
    pub bumper:      BumperKind,
    pub winch:       bool,
    pub body_lift:   bool,
    pub long_arm:    bool,
    pub livery_idx:  u8,   // 0..3 maps to LIVERY_NAMES
}

impl Default for GarageBuildSave {
    fn default() -> Self {
        Self {
            variant_idx: 0,
            paint_idx:   0,
            tire_size:   TireSize::default(),
            bumper:      BumperKind::default(),
            winch:       false,
            body_lift:   false,
            long_arm:    false,
            livery_idx:  0,
        }
    }
}

// ---------------------------------------------------------------------------
// UI State resource
// ---------------------------------------------------------------------------

/// Number of truck silhouettes (VehicleVariant has 8 in total from variants.rs).
const VARIANT_COUNT: u8 = 8;
const PAINT_COUNT:   u8 = 8;
const LIVERY_COUNT:  u8 = 4;  // None / Mud / Camo / Racing

// Display names for each VehicleVariant index (mirrors variants.rs order).
const VARIANT_NAMES: [&str; 8] = [
    "Jeep TJ",
    "Ford Bronco",
    "Pickup",
    "Hummer",
    "Buggy",
    "Highland SK",
    "Dune Skipper",
    "Hauler SK",
];

// Display names for paint indices (mirrors PALETTE in paint_shop.rs).
const PAINT_NAMES: [&str; 8] = [
    "Red", "Blue", "Yellow", "Green",
    "Black", "White", "Orange", "Purple",
];

// Palette colors matching paint_shop.rs PALETTE (used for icon swatches).
const PAINT_COLORS: [Color; 8] = [
    Color::srgb(0.85, 0.15, 0.15),
    Color::srgb(0.15, 0.40, 0.85),
    Color::srgb(0.95, 0.85, 0.15),
    Color::srgb(0.20, 0.70, 0.30),
    Color::srgb(0.10, 0.10, 0.10),
    Color::srgb(0.95, 0.95, 0.92),
    Color::srgb(1.00, 0.55, 0.10),
    Color::srgb(0.60, 0.20, 0.85),
];

// Livery / sponsor preset names.
const LIVERY_NAMES: [&str; 4] = ["None", "Mud", "Camo", "Racing"];

/// Duration (seconds) a row highlight stays fully lit after a change.
const HIGHLIGHT_DURATION: f32 = 0.30;

/// Duration (seconds) of the BUILD & DRIVE flash confirmation.
const BUILD_FLASH_DURATION: f32 = 0.35;

/// Duration (seconds) of the backdrop fade-in.
const BACKDROP_FADE_DURATION: f32 = 0.20;

#[derive(Resource)]
pub struct GarageBuildUiState {
    pub open:        bool,
    pub first_run:   bool,

    // Working selections (not committed until BUILD & DRIVE).
    pub variant_idx: u8,
    pub paint_idx:   u8,
    pub tire_size:   TireSize,
    pub bumper:      BumperKind,
    pub winch:       bool,
    pub body_lift:   bool,
    pub long_arm:    bool,
    pub livery_idx:  u8,

    /// Track whether selections have been pushed to resources this frame so we
    /// avoid triggering RespawnRequest on every frame.
    dirty: bool,

    /// Per-category highlight timer (seconds remaining).
    row_highlight: [f32; 8],

    /// Turntable yaw accumulator (radians). Incremented while garage is open.
    turntable_yaw: f32,

    /// Cached camera transform before garage opened (to restore on close).
    saved_camera: Option<Transform>,

    /// Backdrop fade progress (0..=1).
    backdrop_alpha: f32,

    /// Timer for BUILD & DRIVE flash; negative = inactive.
    build_flash_timer: f32,

    /// Whether the build was confirmed and we should close after the flash.
    build_confirmed: bool,
}

impl Default for GarageBuildUiState {
    fn default() -> Self {
        Self {
            open:              false,
            first_run:         false,
            variant_idx:       0,
            paint_idx:         0,
            tire_size:         TireSize::default(),
            bumper:            BumperKind::default(),
            winch:             false,
            body_lift:         false,
            long_arm:          false,
            livery_idx:        0,
            dirty:             false,
            row_highlight:     [0.0; 8],
            turntable_yaw:     0.0,
            saved_camera:      None,
            backdrop_alpha:    0.0,
            build_flash_timer: -1.0,
            build_confirmed:   false,
        }
    }
}

impl GarageBuildUiState {
    fn from_save(s: &GarageBuildSave) -> Self {
        Self {
            variant_idx: s.variant_idx.min(VARIANT_COUNT - 1),
            paint_idx:   s.paint_idx.min(PAINT_COUNT - 1),
            tire_size:   s.tire_size,
            bumper:      s.bumper,
            winch:       s.winch,
            body_lift:   s.body_lift,
            long_arm:    s.long_arm,
            livery_idx:  s.livery_idx.min(LIVERY_COUNT - 1),
            ..Default::default()
        }
    }

    fn to_save(&self) -> GarageBuildSave {
        GarageBuildSave {
            variant_idx: self.variant_idx,
            paint_idx:   self.paint_idx,
            tire_size:   self.tire_size,
            bumper:      self.bumper,
            winch:       self.winch && self.bumper != BumperKind::Stock,
            body_lift:   self.body_lift,
            long_arm:    self.long_arm,
            livery_idx:  self.livery_idx,
        }
    }

    /// Compute derived stats for the spec sheet.
    /// Returns (lift_in, tire_dia_in, approach_pct, weight_lb).
    fn derived_stats(&self) -> (f32, f32, f32, f32) {
        use crate::vehicle_mods::{LONG_ARM_SPAWN_LIFT, BODY_LIFT_DELTA};

        // Lift height in inches.
        // long_arm  = 0.18 m ~ 7.1 in
        // body_lift = 0.08 m ~ 3.1 in
        // tire_delta stacks on top of both.
        let tire_delta_m = self.tire_size.radius() - TireSize::Stock.radius();
        let lift_m = if self.long_arm  { LONG_ARM_SPAWN_LIFT } else { 0.0 }
                   + if self.body_lift { BODY_LIFT_DELTA      } else { 0.0 }
                   + tire_delta_m;
        let lift_in = lift_m * 39.37_f32;

        // Tire diameter in inches: radius_m * 2 * 39.37
        let tire_dia_in = self.tire_size.radius() * 2.0 * 39.37_f32;

        // Approach angle improvement heuristic: 1.5 % per inch of lift.
        let approach_pct = lift_in * 1.5_f32;

        // Estimated weight in lb: base 3200 + bumpers + larger tires + long-arm + winch.
        let bumper_lb: f32 = match self.bumper {
            BumperKind::Stock          => 0.0,
            BumperKind::SteelFront     => 66.0,
            BumperKind::SteelFrontRear => 132.0,
        };
        let tire_lb: f32 = match self.tire_size {
            TireSize::Stock  => 0.0,
            TireSize::Large  => 80.0,   // 4 wheels x 20 lb
            TireSize::XLarge => 160.0,  // 4 wheels x 40 lb
        };
        let long_arm_lb = if self.long_arm { 55.0_f32 } else { 0.0 };
        let winch_lb    = if self.winch    { 44.0_f32 } else { 0.0 };
        let weight_lb   = 3200.0 + bumper_lb + tire_lb + long_arm_lb + winch_lb;

        (lift_in, tire_dia_in, approach_pct, weight_lb)
    }

    fn spec_text(&self) -> String {
        let tire_label = match self.tire_size {
            TireSize::Stock  => "33\" (Stock)",
            TireSize::Large  => "35\"",
            TireSize::XLarge => "37\"",
        };
        let bumper_label = match self.bumper {
            BumperKind::Stock          => "Stock",
            BumperKind::SteelFront     => "Steel Front",
            BumperKind::SteelFrontRear => "Steel Front+Rear",
        };
        let winch_label = if self.bumper == BumperKind::Stock {
            "N/A (need steel bumper)"
        } else if self.winch {
            "Fitted"
        } else {
            "None"
        };

        let (lift_in, tire_dia_in, approach_pct, weight_lb) = self.derived_stats();

        format!(
            "Truck      : {}\nPaint      : {}\nTires      : {}\nBumper     : {}\nWinch      : {}\nBody lift  : {}\nLong-arm   : {}\nLivery     : {}\n\n--- DERIVED STATS ---\nLift height: {:.1}\"\nTire dia   : {:.0}\"\nApproach+  : {:.0}%\nEst. weight: {:.0} lb",
            VARIANT_NAMES[self.variant_idx as usize],
            PAINT_NAMES[self.paint_idx as usize],
            tire_label,
            bumper_label,
            winch_label,
            if self.body_lift { "Yes (3\" spacers)" } else { "No" },
            if self.long_arm  { "Yes (+7\" travel)"  } else { "No" },
            LIVERY_NAMES[self.livery_idx as usize],
            lift_in,
            tire_dia_in,
            approach_pct,
            weight_lb,
        )
    }
}

// ---------------------------------------------------------------------------
// First-run detection + initial load
// ---------------------------------------------------------------------------

fn load_or_detect_first_run(mut ui: ResMut<GarageBuildUiState>) {
    if let Some(json) = platform_storage::read_string(STORAGE_KEY) {
        match serde_json::from_str::<GarageBuildSave>(&json) {
            Ok(save) => {
                info!("garage_build: loaded saved build from {}", STORAGE_KEY);
                *ui = GarageBuildUiState::from_save(&save);
                return;
            }
            Err(e) => {
                warn!("garage_build: could not parse {}: {}; using defaults", STORAGE_KEY, e);
            }
        }
    }

    let no_save = !platform_storage::exists("save_1.json");
    if no_save {
        info!("garage_build: first run detected -- will open garage after startup");
        ui.first_run = true;
    }
}

// ---------------------------------------------------------------------------
// Marker components
// ---------------------------------------------------------------------------

#[derive(Component)] struct GarageBuildRoot;
#[derive(Component)] struct GarageSpecText;
#[derive(Component)] struct GarageBuildDriveBtn;
#[derive(Component)] struct GarageRandomBtn;
#[derive(Component)] struct GarageFlashOverlay;

/// Tag on each cycle-left / cycle-right button.
#[derive(Component)]
struct GarageCycleBtn {
    category: u8,
    forward:  bool,
}

/// Tag on the value label in each category row.
#[derive(Component)]
struct GarageCategoryLabel { category: u8 }

/// Tag on the category row background node (for highlight animation).
#[derive(Component)]
struct GarageCategoryRow { category: u8 }

/// Tag on the paint icon swatch so we can recolor it live.
#[derive(Component)]
struct GaragePaintIcon;

// ---------------------------------------------------------------------------
// Colors
// ---------------------------------------------------------------------------

const PANEL_BG:    Color = Color::srgba(0.08, 0.09, 0.11, 0.98);
const PANEL_DARK:  Color = Color::srgba(0.05, 0.06, 0.08, 0.99);
const ACCENT:      Color = Color::srgb(1.0,  0.70, 0.20);
const TEXT_MAIN:   Color = Color::srgb(0.90, 0.90, 0.88);
const TEXT_DIM:    Color = Color::srgb(0.55, 0.55, 0.52);
const BTN_BG:      Color = Color::srgba(0.15, 0.16, 0.18, 1.0);
const AMBER_BTN:   Color = Color::srgba(0.85, 0.55, 0.05, 1.0);
const SPEC_BORDER: Color = Color::srgba(1.0, 0.70, 0.20, 0.60);

/// Category display names matching the 8 categories (0..7).
const CATEGORY_NAMES: [&str; 8] = [
    "Truck", "Paint", "Tire size", "Bumper",
    "Winch", "Body lift", "Long-arm", "Livery",
];

// ---------------------------------------------------------------------------
// UI spawn
// ---------------------------------------------------------------------------

fn spawn_garage_ui(mut commands: Commands) {
    // Outer full-screen dim (fades in via tick_backdrop_fade).
    let root = commands
        .spawn((
            GarageBuildRoot,
            Node {
                width:           Val::Percent(100.0),
                height:          Val::Percent(100.0),
                position_type:   PositionType::Absolute,
                flex_direction:  FlexDirection::Row,
                align_items:     AlignItems::Center,
                justify_content: JustifyContent::Center,
                column_gap:      Val::Px(24.0),
                display:         Display::None,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
            GlobalZIndex(950),
        ))
        .id();

    // Flash-confirmation overlay (absolute, covers full screen during flash).
    let flash = commands
        .spawn((
            GarageFlashOverlay,
            Node {
                width:         Val::Percent(100.0),
                height:        Val::Percent(100.0),
                position_type: PositionType::Absolute,
                ..default()
            },
            BackgroundColor(Color::srgba(1.0, 0.80, 0.10, 0.0)),
            GlobalZIndex(960),
        ))
        .id();
    commands.entity(root).add_child(flash);

    // ---- Left panel: header + category rows + RANDOM RIG --------------------
    let left = commands
        .spawn((
            Node {
                width:          Val::Px(400.0),
                flex_direction: FlexDirection::Column,
                padding:        UiRect::all(Val::Px(20.0)),
                row_gap:        Val::Px(8.0),
                ..default()
            },
            BackgroundColor(PANEL_BG),
        ))
        .id();

    // Header band: GARAGE title + subtitle + thick accent underline.
    let header_band = commands
        .spawn(Node {
            width:          Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            row_gap:        Val::Px(4.0),
            padding:        UiRect::bottom(Val::Px(8.0)),
            ..default()
        })
        .id();

    let title = commands
        .spawn((
            Text::new("GARAGE"),
            TextFont { font_size: 32.0, ..default() },
            TextColor(ACCENT),
        ))
        .id();

    let subtitle = commands
        .spawn((
            Text::new("Build your rig"),
            TextFont { font_size: 13.0, ..default() },
            TextColor(TEXT_DIM),
        ))
        .id();

    // 2 px amber underline beneath the title band.
    let underline = commands
        .spawn((
            Node {
                width:  Val::Percent(100.0),
                height: Val::Px(2.0),
                margin: UiRect::top(Val::Px(4.0)),
                ..default()
            },
            BackgroundColor(ACCENT),
        ))
        .id();

    commands.entity(header_band).add_children(&[title, subtitle, underline]);
    commands.entity(left).add_child(header_band);

    // 8 category rows.
    for cat in 0u8..8 {
        let row = spawn_category_row(&mut commands, cat);
        commands.entity(left).add_child(row);
    }

    // Thin separator + RANDOM RIG button.
    let random_sep = commands
        .spawn((
            Node {
                width:  Val::Percent(100.0),
                height: Val::Px(1.0),
                margin: UiRect::vertical(Val::Px(4.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.08)),
        ))
        .id();

    let random_btn = commands
        .spawn((
            GarageRandomBtn,
            Node {
                width:           Val::Percent(100.0),
                height:          Val::Px(36.0),
                align_items:     AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.20, 0.22, 0.26, 1.0)),
            Button,
        ))
        .id();

    let random_label = commands
        .spawn((
            Text::new("RANDOM RIG"),
            TextFont { font_size: 13.0, ..default() },
            TextColor(TEXT_DIM),
        ))
        .id();

    commands.entity(random_btn).add_child(random_label);
    commands.entity(left).add_children(&[random_sep, random_btn]);

    // ---- Right panel: spec sheet (darker card with amber border) ------------
    let right_border = commands
        .spawn((
            Node {
                width:          Val::Px(320.0),
                flex_direction: FlexDirection::Column,
                padding:        UiRect::all(Val::Px(3.0)),
                ..default()
            },
            BackgroundColor(SPEC_BORDER),
        ))
        .id();

    let right = commands
        .spawn((
            Node {
                width:           Val::Percent(100.0),
                flex_direction:  FlexDirection::Column,
                padding:         UiRect::all(Val::Px(16.0)),
                row_gap:         Val::Px(10.0),
                flex_grow:       1.0,
                justify_content: JustifyContent::SpaceBetween,
                ..default()
            },
            BackgroundColor(PANEL_DARK),
        ))
        .id();

    let spec_title = commands
        .spawn((
            Text::new("SPEC SHEET"),
            TextFont { font_size: 16.0, ..default() },
            TextColor(ACCENT),
        ))
        .id();

    let spec_sep = commands
        .spawn((
            Node {
                width:  Val::Percent(100.0),
                height: Val::Px(1.0),
                ..default()
            },
            BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.12)),
        ))
        .id();

    let spec_text = commands
        .spawn((
            GarageSpecText,
            Text::new(""),
            TextFont { font_size: 12.0, ..default() },
            TextColor(TEXT_MAIN),
        ))
        .id();

    let hint = commands
        .spawn((
            Text::new("Shift+G  close without saving"),
            TextFont { font_size: 11.0, ..default() },
            TextColor(TEXT_DIM),
        ))
        .id();

    // BUILD & DRIVE: big amber button -- primary action.
    let build_btn_root = commands
        .spawn((
            GarageBuildDriveBtn,
            Node {
                width:           Val::Percent(100.0),
                height:          Val::Px(60.0),
                align_items:     AlignItems::Center,
                justify_content: JustifyContent::Center,
                margin:          UiRect::top(Val::Px(6.0)),
                ..default()
            },
            BackgroundColor(AMBER_BTN),
            Button,
        ))
        .id();

    let build_btn_text = commands
        .spawn((
            Text::new("BUILD & DRIVE"),
            TextFont { font_size: 20.0, ..default() },
            // Near-black text on amber for legibility.
            TextColor(Color::srgb(0.06, 0.04, 0.01)),
        ))
        .id();

    commands.entity(build_btn_root).add_child(build_btn_text);
    commands.entity(right).add_children(&[spec_title, spec_sep, spec_text, hint, build_btn_root]);
    commands.entity(right_border).add_child(right);
    commands.entity(root).add_children(&[left, right_border]);
}

/// Build one category row: [24x24 icon | category label | < | value label | >]
fn spawn_category_row(commands: &mut Commands, cat: u8) -> Entity {
    let row = commands
        .spawn((
            GarageCategoryRow { category: cat },
            Node {
                width:           Val::Percent(100.0),
                height:          Val::Px(38.0),
                flex_direction:  FlexDirection::Row,
                align_items:     AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                column_gap:      Val::Px(6.0),
                padding:         UiRect::horizontal(Val::Px(4.0)),
                ..default()
            },
            BackgroundColor(BTN_BG),
        ))
        .id();

    // 24x24 category icon (pure Node shapes -- no font glyphs).
    let icon = spawn_category_icon(commands, cat);

    let cat_label = commands
        .spawn((
            Node { width: Val::Px(80.0), ..default() },
            Text::new(CATEGORY_NAMES[cat as usize]),
            TextFont { font_size: 13.0, ..default() },
            TextColor(TEXT_DIM),
        ))
        .id();

    let prev_btn = commands
        .spawn((
            GarageCycleBtn { category: cat, forward: false },
            Node {
                width:           Val::Px(28.0),
                height:          Val::Px(28.0),
                align_items:     AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.25, 0.25, 0.28, 1.0)),
            Button,
        ))
        .with_children(|p| {
            p.spawn((
                Text::new("<"),
                TextFont { font_size: 16.0, ..default() },
                TextColor(TEXT_MAIN),
            ));
        })
        .id();

    let val_label = commands
        .spawn((
            GarageCategoryLabel { category: cat },
            Node { flex_grow: 1.0, ..default() },
            Text::new(""),
            TextFont { font_size: 13.0, ..default() },
            TextColor(ACCENT),
        ))
        .id();

    let next_btn = commands
        .spawn((
            GarageCycleBtn { category: cat, forward: true },
            Node {
                width:           Val::Px(28.0),
                height:          Val::Px(28.0),
                align_items:     AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.25, 0.25, 0.28, 1.0)),
            Button,
        ))
        .with_children(|p| {
            p.spawn((
                Text::new(">"),
                TextFont { font_size: 16.0, ..default() },
                TextColor(TEXT_MAIN),
            ));
        })
        .id();

    commands.entity(row).add_children(&[icon, cat_label, prev_btn, val_label, next_btn]);
    row
}

/// Spawn a 24x24 colored icon Node for the given category.
/// All icons are built from colored Node rectangles -- NO font glyphs.
fn spawn_category_icon(commands: &mut Commands, cat: u8) -> Entity {
    match cat {
        0 => {
            // Truck: dark cab box atop a wider body box.
            let outer = commands.spawn((
                Node {
                    width:           Val::Px(24.0),
                    height:          Val::Px(24.0),
                    flex_direction:  FlexDirection::Column,
                    align_items:     AlignItems::Center,
                    justify_content: JustifyContent::FlexEnd,
                    ..default()
                },
                BackgroundColor(Color::NONE),
            )).id();
            let cab = commands.spawn((
                Node {
                    width:  Val::Px(12.0),
                    height: Val::Px(8.0),
                    margin: UiRect::bottom(Val::Px(1.0)),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.35, 0.45, 0.60)),
            )).id();
            let body = commands.spawn((
                Node { width: Val::Px(22.0), height: Val::Px(8.0), ..default() },
                BackgroundColor(Color::srgb(0.50, 0.55, 0.65)),
            )).id();
            commands.entity(outer).add_children(&[cab, body]);
            outer
        }
        1 => {
            // Paint: colored swatch -- starts at PAINT_COLORS[0].
            // update_garage_ui recolors this every frame via GaragePaintIcon.
            commands.spawn((
                GaragePaintIcon,
                Node { width: Val::Px(24.0), height: Val::Px(24.0), ..default() },
                BackgroundColor(PAINT_COLORS[0]),
            )).id()
        }
        2 => {
            // Tire: dark rounded square with lighter hub dot.
            let outer = commands.spawn((
                Node {
                    width:           Val::Px(24.0),
                    height:          Val::Px(24.0),
                    align_items:     AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                BackgroundColor(Color::srgb(0.18, 0.18, 0.18)),
                BorderRadius::all(Val::Px(12.0)),
            )).id();
            let hub = commands.spawn((
                Node { width: Val::Px(8.0), height: Val::Px(8.0), ..default() },
                BackgroundColor(Color::srgb(0.60, 0.60, 0.60)),
                BorderRadius::all(Val::Px(4.0)),
            )).id();
            commands.entity(outer).add_child(hub);
            outer
        }
        3 => {
            // Bumper: two horizontal bars (front and rear bumper abstraction).
            let outer = commands.spawn((
                Node {
                    width:           Val::Px(24.0),
                    height:          Val::Px(24.0),
                    flex_direction:  FlexDirection::Column,
                    align_items:     AlignItems::Center,
                    justify_content: JustifyContent::SpaceBetween,
                    padding:         UiRect::vertical(Val::Px(5.0)),
                    ..default()
                },
                BackgroundColor(Color::NONE),
            )).id();
            let bar1 = commands.spawn((
                Node { width: Val::Px(22.0), height: Val::Px(5.0), ..default() },
                BackgroundColor(Color::srgb(0.60, 0.62, 0.66)),
            )).id();
            let bar2 = commands.spawn((
                Node { width: Val::Px(22.0), height: Val::Px(5.0), ..default() },
                BackgroundColor(Color::srgb(0.45, 0.47, 0.50)),
            )).id();
            commands.entity(outer).add_children(&[bar1, bar2]);
            outer
        }
        4 => {
            // Winch: steel-grey square with a cable-spool horizontal line.
            let outer = commands.spawn((
                Node {
                    width:           Val::Px(24.0),
                    height:          Val::Px(24.0),
                    align_items:     AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                BackgroundColor(Color::srgb(0.45, 0.47, 0.52)),
            )).id();
            let spool = commands.spawn((
                Node { width: Val::Px(16.0), height: Val::Px(4.0), ..default() },
                BackgroundColor(Color::srgb(0.70, 0.72, 0.76)),
            )).id();
            commands.entity(outer).add_child(spool);
            outer
        }
        5 => {
            // Body lift: body panel separated from frame by an amber spacer.
            let outer = commands.spawn((
                Node {
                    width:           Val::Px(24.0),
                    height:          Val::Px(24.0),
                    flex_direction:  FlexDirection::Column,
                    align_items:     AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    row_gap:         Val::Px(3.0),
                    ..default()
                },
                BackgroundColor(Color::NONE),
            )).id();
            let top = commands.spawn((
                Node { width: Val::Px(20.0), height: Val::Px(6.0), ..default() },
                BackgroundColor(Color::srgb(0.45, 0.50, 0.60)),
            )).id();
            let spacer = commands.spawn((
                Node { width: Val::Px(8.0), height: Val::Px(3.0), ..default() },
                BackgroundColor(Color::srgb(0.70, 0.60, 0.30)),
            )).id();
            let bottom = commands.spawn((
                Node { width: Val::Px(22.0), height: Val::Px(5.0), ..default() },
                BackgroundColor(Color::srgb(0.35, 0.38, 0.42)),
            )).id();
            commands.entity(outer).add_children(&[top, spacer, bottom]);
            outer
        }
        6 => {
            // Long-arm: two horizontal bars of different widths (longer control arms).
            let outer = commands.spawn((
                Node {
                    width:           Val::Px(24.0),
                    height:          Val::Px(24.0),
                    flex_direction:  FlexDirection::Column,
                    align_items:     AlignItems::FlexStart,
                    justify_content: JustifyContent::Center,
                    row_gap:         Val::Px(4.0),
                    ..default()
                },
                BackgroundColor(Color::NONE),
            )).id();
            let arm1 = commands.spawn((
                Node { width: Val::Px(18.0), height: Val::Px(4.0), ..default() },
                BackgroundColor(Color::srgb(0.50, 0.55, 0.65)),
            )).id();
            let arm2 = commands.spawn((
                Node { width: Val::Px(22.0), height: Val::Px(4.0), ..default() },
                BackgroundColor(Color::srgb(0.50, 0.55, 0.65)),
            )).id();
            commands.entity(outer).add_children(&[arm1, arm2]);
            outer
        }
        _ => {
            // Livery (cat 7): horizontal racing stripes.
            let outer = commands.spawn((
                Node {
                    width:          Val::Px(24.0),
                    height:         Val::Px(24.0),
                    flex_direction: FlexDirection::Column,
                    ..default()
                },
                BackgroundColor(Color::NONE),
            )).id();
            let stripe_colors = [
                Color::srgb(0.80, 0.15, 0.15),
                Color::srgb(0.90, 0.90, 0.88),
                Color::srgb(0.20, 0.20, 0.80),
                Color::srgb(0.90, 0.90, 0.88),
                Color::srgb(0.80, 0.15, 0.15),
            ];
            for &sc in &stripe_colors {
                let stripe = commands.spawn((
                    Node {
                        width:    Val::Percent(100.0),
                        height:   Val::Px(4.0),
                        flex_grow: 1.0,
                        ..default()
                    },
                    BackgroundColor(sc),
                )).id();
                commands.entity(outer).add_child(stripe);
            }
            outer
        }
    }
}

// ---------------------------------------------------------------------------
// Open / close garage
// ---------------------------------------------------------------------------

fn open_close_garage(
    keys:      Res<ButtonInput<KeyCode>>,
    mut ui:    ResMut<GarageBuildUiState>,
    mut delay: Local<u8>,
) {
    if ui.first_run {
        *delay += 1;
        if *delay >= 2 {
            ui.first_run = false;
            ui.open = true;
            ui.backdrop_alpha = 0.0;
            info!("garage_build: opening for first run");
        }
        return;
    }

    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if shift && keys.just_pressed(KeyCode::KeyG) {
        ui.open = !ui.open;
        if ui.open {
            ui.backdrop_alpha = 0.0;
        }
        info!("garage_build: toggled (open={})", ui.open);
    }
}

// ---------------------------------------------------------------------------
// Cycle button interaction
// ---------------------------------------------------------------------------

fn handle_category_buttons(
    mut interactions: Query<
        (&Interaction, &GarageCycleBtn),
        Changed<Interaction>,
    >,
    mut ui: ResMut<GarageBuildUiState>,
) {
    for (interaction, btn) in &mut interactions {
        if *interaction != Interaction::Pressed {
            continue;
        }
        if !ui.open {
            continue;
        }
        let delta: i8 = if btn.forward { 1 } else { -1 };
        let changed = cycle_category(&mut ui, btn.category, delta);
        if changed {
            ui.dirty = true;
            ui.row_highlight[btn.category as usize] = HIGHLIGHT_DURATION;
        }
    }
}

// ---------------------------------------------------------------------------
// RANDOM RIG button
// ---------------------------------------------------------------------------

fn handle_random_rig(
    interactions: Query<&Interaction, (Changed<Interaction>, With<GarageRandomBtn>)>,
    mut ui:       ResMut<GarageBuildUiState>,
    time:         Res<Time>,
) {
    for interaction in &interactions {
        if *interaction != Interaction::Pressed {
            continue;
        }
        if !ui.open {
            continue;
        }

        // Simple LCG seeded from elapsed time -- no external crate needed.
        let seed = (time.elapsed_secs() * 100_000.0) as u64;
        let mut lcg = seed.wrapping_add(0xDEAD_BEEF_CAFE_BABEu64);
        let mut next = move || -> u32 {
            lcg = lcg
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            (lcg >> 33) as u32
        };

        ui.variant_idx = (next() % VARIANT_COUNT as u32) as u8;
        ui.paint_idx   = (next() % PAINT_COUNT   as u32) as u8;
        ui.tire_size   = match next() % 3 {
            0 => TireSize::Stock,
            1 => TireSize::Large,
            _ => TireSize::XLarge,
        };
        ui.bumper = match next() % 3 {
            0 => BumperKind::Stock,
            1 => BumperKind::SteelFront,
            _ => BumperKind::SteelFrontRear,
        };
        ui.winch     = ui.bumper != BumperKind::Stock && (next() % 2 == 0);
        ui.body_lift = next() % 2 == 0;
        ui.long_arm  = next() % 2 == 0;
        ui.livery_idx = (next() % LIVERY_COUNT as u32) as u8;

        ui.dirty = true;
        ui.row_highlight = [HIGHLIGHT_DURATION; 8];

        info!("garage_build: RANDOM RIG selected");
    }
}

/// Cycle a category by `delta` (+1 = next, -1 = prev). Returns true if changed.
fn cycle_category(ui: &mut GarageBuildUiState, cat: u8, delta: i8) -> bool {
    match cat {
        0 => {
            ui.variant_idx = wrap_add(ui.variant_idx, delta, VARIANT_COUNT);
            true
        }
        1 => {
            ui.paint_idx = wrap_add(ui.paint_idx, delta, PAINT_COUNT);
            true
        }
        2 => {
            ui.tire_size = if delta > 0 { ui.tire_size.next() } else { tire_prev(ui.tire_size) };
            true
        }
        3 => {
            ui.bumper = if delta > 0 { ui.bumper.next() } else { bumper_prev(ui.bumper) };
            if ui.bumper == BumperKind::Stock {
                ui.winch = false;
            }
            true
        }
        4 => {
            if ui.bumper != BumperKind::Stock {
                ui.winch = !ui.winch;
                true
            } else {
                false
            }
        }
        5 => {
            ui.body_lift = !ui.body_lift;
            true
        }
        6 => {
            ui.long_arm = !ui.long_arm;
            true
        }
        7 => {
            ui.livery_idx = wrap_add(ui.livery_idx, delta, LIVERY_COUNT);
            true
        }
        _ => false,
    }
}

fn wrap_add(val: u8, delta: i8, count: u8) -> u8 {
    let n = count as i16;
    (((val as i16) + (delta as i16) + n) % n) as u8
}

fn tire_prev(t: TireSize) -> TireSize {
    match t {
        TireSize::Stock  => TireSize::XLarge,
        TireSize::Large  => TireSize::Stock,
        TireSize::XLarge => TireSize::Large,
    }
}

fn bumper_prev(b: BumperKind) -> BumperKind {
    match b {
        BumperKind::Stock          => BumperKind::SteelFrontRear,
        BumperKind::SteelFront     => BumperKind::Stock,
        BumperKind::SteelFrontRear => BumperKind::SteelFront,
    }
}

// ---------------------------------------------------------------------------
// Apply selections -> resources (live preview)
// ---------------------------------------------------------------------------

fn apply_selections_to_resources(
    mut ui:      ResMut<GarageBuildUiState>,
    mut mods:    ResMut<VehicleModsState>,
    mut variant: ResMut<VehicleVariant>,
    mut paint:   ResMut<PaintShopState>,
    mut respawn: ResMut<RespawnRequest>,
) {
    if !ui.open || !ui.dirty {
        return;
    }
    ui.dirty = false;

    *variant          = idx_to_variant(ui.variant_idx);
    paint.current_idx = ui.paint_idx as u32;
    mods.tire_size    = ui.tire_size;
    mods.bumper       = ui.bumper;
    mods.winch        = ui.winch && ui.bumper != BumperKind::Stock;
    mods.body_lift    = ui.body_lift;
    mods.long_arm     = ui.long_arm;
    respawn.0         = true;
}

fn idx_to_variant(idx: u8) -> VehicleVariant {
    match idx {
        0 => VehicleVariant::JeepTJ,
        1 => VehicleVariant::FordBronco,
        2 => VehicleVariant::Pickup,
        3 => VehicleVariant::Hummer,
        4 => VehicleVariant::Buggy,
        5 => VehicleVariant::HighlandSK,
        6 => VehicleVariant::DuneSkipper,
        7 => VehicleVariant::HaulerSK,
        _ => VehicleVariant::JeepTJ,
    }
}

// ---------------------------------------------------------------------------
// UI update (visibility + labels + spec text + paint swatch icon)
// ---------------------------------------------------------------------------

fn update_garage_ui(
    ui:             Res<GarageBuildUiState>,
    mut root_q:     Query<&mut Node, With<GarageBuildRoot>>,
    mut label_q:    Query<(&mut Text, &GarageCategoryLabel)>,
    mut spec_q:     Query<&mut Text, (With<GarageSpecText>, Without<GarageCategoryLabel>)>,
    mut paint_icon: Query<&mut BackgroundColor, With<GaragePaintIcon>>,
) {
    // Show / hide root.
    for mut node in &mut root_q {
        node.display = if ui.open { Display::Flex } else { Display::None };
    }

    if !ui.open {
        return;
    }

    // Update category value labels.
    for (mut text, lbl) in &mut label_q {
        text.0 = category_value_text(&ui, lbl.category).to_string();
    }

    // Update spec sheet.
    for mut text in &mut spec_q {
        text.0 = ui.spec_text();
    }

    // Sync paint swatch icon color to current selection.
    for mut bg in &mut paint_icon {
        *bg = BackgroundColor(PAINT_COLORS[ui.paint_idx as usize % PAINT_COLORS.len()]);
    }
}

fn category_value_text(ui: &GarageBuildUiState, cat: u8) -> &'static str {
    match cat {
        0 => VARIANT_NAMES[ui.variant_idx as usize],
        1 => PAINT_NAMES[ui.paint_idx as usize],
        2 => match ui.tire_size {
            TireSize::Stock  => "33\" (Stock)",
            TireSize::Large  => "35\"",
            TireSize::XLarge => "37\"",
        },
        3 => match ui.bumper {
            BumperKind::Stock          => "Stock",
            BumperKind::SteelFront     => "Steel Front",
            BumperKind::SteelFrontRear => "Steel F+R",
        },
        4 => {
            if ui.bumper == BumperKind::Stock { "N/A" }
            else if ui.winch { "Fitted" }
            else { "None" }
        }
        5 => if ui.body_lift { "Yes" } else { "No" },
        6 => if ui.long_arm  { "Yes" } else { "No" },
        7 => LIVERY_NAMES[ui.livery_idx as usize],
        _ => "",
    }
}

// ---------------------------------------------------------------------------
// Row highlight animation (amber flash for ~0.3 s on value change)
// ---------------------------------------------------------------------------

fn update_row_highlights(
    mut ui:   ResMut<GarageBuildUiState>,
    mut rows: Query<(&GarageCategoryRow, &mut BackgroundColor)>,
    time:     Res<Time>,
) {
    if !ui.open {
        return;
    }

    let dt = time.delta_secs();

    for (row, mut bg) in &mut rows {
        let cat = row.category as usize;
        let t   = ui.row_highlight[cat];

        if t > 0.0 {
            // frac 1.0 = just changed (fully highlighted), 0.0 = back to normal.
            let frac = (t / HIGHLIGHT_DURATION).min(1.0);

            // Manual lerp between BTN_BG (0.15,0.16,0.18,1.0) and
            // ROW_HIGHLIGHT (1.0, 0.75, 0.15, 0.22).
            let r = 0.15 + (1.00 - 0.15) * frac;
            let g = 0.16 + (0.75 - 0.16) * frac;
            let b = 0.18 + (0.15 - 0.18) * frac;
            let a = 1.00 + (0.22 - 1.00) * frac;
            *bg = BackgroundColor(Color::srgba(r, g, b, a));

            ui.row_highlight[cat] = (t - dt).max(0.0);
        } else {
            *bg = BackgroundColor(BTN_BG);
        }
    }
}

// ---------------------------------------------------------------------------
// BUILD & DRIVE button animation (amber pulsing brightness + hover brighten)
// ---------------------------------------------------------------------------

fn animate_build_button(
    mut btn_q: Query<(&Interaction, &mut BackgroundColor), With<GarageBuildDriveBtn>>,
    time:      Res<Time>,
    ui:        Res<GarageBuildUiState>,
) {
    if !ui.open {
        return;
    }

    let Ok((interaction, mut bg)) = btn_q.single_mut() else { return };

    // Gentle pulsing brightness at 3.5 Hz.
    let pulse = (time.elapsed_secs() * 3.5).sin() * 0.5 + 0.5; // 0..1

    match *interaction {
        Interaction::Hovered | Interaction::Pressed => {
            let r = 0.95 + pulse * 0.05;
            let g = 0.68 + pulse * 0.05;
            *bg = BackgroundColor(Color::srgb(r, g, 0.10));
        }
        Interaction::None => {
            let r = 0.78 + pulse * 0.12;
            let g = 0.48 + pulse * 0.10;
            *bg = BackgroundColor(Color::srgb(r, g, 0.04));
        }
    }
}

// ---------------------------------------------------------------------------
// Backdrop fade-in over BACKDROP_FADE_DURATION seconds
// ---------------------------------------------------------------------------

fn tick_backdrop_fade(
    mut ui:   ResMut<GarageBuildUiState>,
    mut root: Query<&mut BackgroundColor, With<GarageBuildRoot>>,
    time:     Res<Time>,
) {
    if !ui.open {
        return;
    }

    let dt = time.delta_secs();
    ui.backdrop_alpha = (ui.backdrop_alpha + dt / BACKDROP_FADE_DURATION).min(1.0);
    let alpha = ui.backdrop_alpha * 0.60;

    for mut bg in &mut root {
        *bg = BackgroundColor(Color::srgba(0.0, 0.0, 0.0, alpha));
    }
}

// ---------------------------------------------------------------------------
// BUILD & DRIVE flash confirmation (amber flash before closing)
// ---------------------------------------------------------------------------

fn tick_build_flash(
    mut ui:      ResMut<GarageBuildUiState>,
    mut flash_q: Query<&mut BackgroundColor, With<GarageFlashOverlay>>,
    time:        Res<Time>,
) {
    let Ok(mut flash_bg) = flash_q.single_mut() else { return };

    if ui.build_flash_timer < 0.0 {
        *flash_bg = BackgroundColor(Color::srgba(1.0, 0.80, 0.10, 0.0));
        return;
    }

    let dt = time.delta_secs();
    ui.build_flash_timer -= dt;

    if ui.build_flash_timer <= 0.0 {
        *flash_bg = BackgroundColor(Color::srgba(1.0, 0.80, 0.10, 0.0));
        ui.build_flash_timer = -1.0;
        if ui.build_confirmed {
            ui.build_confirmed = false;
            ui.open = false;
            info!("garage_build: BUILD & DRIVE confirmed -- closing garage");
        }
    } else {
        // Fade the flash out as timer counts down.
        let frac = ui.build_flash_timer / BUILD_FLASH_DURATION;
        *flash_bg = BackgroundColor(Color::srgba(1.0, 0.80, 0.10, frac * 0.75));
    }
}

// ---------------------------------------------------------------------------
// Showroom turntable + camera override while garage is open
// ---------------------------------------------------------------------------

/// ~20 degrees per second turntable rotation (TAU / 18 radians per second).
const TURNTABLE_YAW_SPEED: f32 = std::f32::consts::TAU / 18.0;

/// Camera showcase parameters.
const SHOWCASE_DIST:   f32 = 11.0;  // metres from chassis centre
const SHOWCASE_HEIGHT: f32 =  3.5;  // metres above chassis

fn turntable_and_camera(
    mut ui:        ResMut<GarageBuildUiState>,
    vehicle_root:  Option<Res<crate::vehicle::VehicleRoot>>,
    mut chassis_q: Query<&mut Transform, With<Chassis>>,
    mut cam_q:     Query<&mut Transform, (With<Camera3d>, Without<Chassis>)>,
    time:          Res<Time>,
) {
    if !ui.open {
        // Garage closed -- restore the cached camera transform (if any).
        if let Some(saved) = ui.saved_camera.take() {
            if let Ok(mut cam) = cam_q.single_mut() {
                *cam = saved;
            }
        }
        return;
    }

    let dt = time.delta_secs();
    ui.turntable_yaw += TURNTABLE_YAW_SPEED * dt;

    // Spin the chassis around its Y axis (world-space yaw only).
    if let Some(vr) = vehicle_root.as_deref() {
        if let Ok(mut chassis_tf) = chassis_q.get_mut(vr.chassis) {
            chassis_tf.rotation = Quat::from_rotation_y(ui.turntable_yaw);
        }
    }

    // Camera: 3/4 low-angle showcase.
    let Ok(mut cam) = cam_q.single_mut() else { return };

    // Cache current camera exactly once on open.
    if ui.saved_camera.is_none() {
        ui.saved_camera = Some(*cam);
    }

    // Chassis world position (fallback to origin if vehicle not yet spawned).
    let chassis_pos = vehicle_root.as_deref()
        .and_then(|vr| chassis_q.get(vr.chassis).ok())
        .map(|tf| tf.translation)
        .unwrap_or(Vec3::ZERO);

    // Camera orbits 135 deg offset from turntable yaw (3/4 rear-left angle).
    let orbit_angle = ui.turntable_yaw + std::f32::consts::FRAC_PI_4 * 3.0;
    let cam_pos = chassis_pos + Vec3::new(
        orbit_angle.cos() * SHOWCASE_DIST,
        SHOWCASE_HEIGHT,
        orbit_angle.sin() * SHOWCASE_DIST,
    );

    cam.translation = cam_pos;
    cam.look_at(chassis_pos + Vec3::Y * 0.8, Vec3::Y);
}

// ---------------------------------------------------------------------------
// BUILD & DRIVE button
// ---------------------------------------------------------------------------

fn handle_build_and_drive(
    interactions: Query<&Interaction, (Changed<Interaction>, With<GarageBuildDriveBtn>)>,
    mut ui:        ResMut<GarageBuildUiState>,
    mut mods:      ResMut<VehicleModsState>,
    mut variant:   ResMut<VehicleVariant>,
    mut paint:     ResMut<PaintShopState>,
    mut respawn:   ResMut<RespawnRequest>,
) {
    for interaction in &interactions {
        if *interaction != Interaction::Pressed {
            continue;
        }
        if !ui.open {
            continue;
        }
        // Don't re-trigger while already flashing.
        if ui.build_flash_timer >= 0.0 {
            continue;
        }

        // Commit final state to all resources.
        *variant          = idx_to_variant(ui.variant_idx);
        paint.current_idx = ui.paint_idx as u32;
        mods.tire_size    = ui.tire_size;
        mods.bumper       = ui.bumper;
        mods.winch        = ui.winch && ui.bumper != BumperKind::Stock;
        mods.body_lift    = ui.body_lift;
        mods.long_arm     = ui.long_arm;
        respawn.0         = true;

        // Persist the build.
        let save = ui.to_save();
        match serde_json::to_string_pretty(&save) {
            Ok(json) => {
                if let Err(e) = platform_storage::write_string(STORAGE_KEY, &json) {
                    warn!("garage_build: save failed: {}", e);
                } else {
                    info!("garage_build: saved to {}", STORAGE_KEY);
                }
            }
            Err(e) => warn!("garage_build: serialize failed: {}", e),
        }

        // Kick off the amber flash; garage closes when it expires.
        ui.build_flash_timer = BUILD_FLASH_DURATION;
        ui.build_confirmed   = true;
        info!("garage_build: BUILD & DRIVE pressed -- flashing confirmation");
    }
}
