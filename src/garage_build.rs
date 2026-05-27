// Garage Build Screen — Sprint 73.
//
// Full-screen overlay where the player configures their rig before driving.
// Opens automatically on first run (no saved build) or via Shift+G in-game.
//
// Left column: category rows (Truck / Paint / Tire / Bumper / Winch /
//              Body-lift / Long-arm / Livery) with ‹ › cycle arrows.
// Right column: spec sheet + BUILD & DRIVE button.
//
// Each category change writes directly to the relevant resource
// (VehicleModsState, PaintShopState, VehicleVariant) and fires
// RespawnRequest so the truck rebuilds — the world behind the overlay
// is the live preview.
//
// Hotkey: Shift+G (G alone is taken by gauge.rs / trailers.rs).
// GlobalZIndex 950 — above HUD (~100), below title screen (1000).
//
// Persistence: garage_build.json via platform_storage.
// First-run detection: absence of garage_build.json AND save_1.json.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::platform_storage;
use crate::variants::VehicleVariant;
use crate::vehicle_mods::{BumperKind, TireSize, VehicleModsState};
use crate::paint_shop::PaintShopState;
use crate::vehicle::RespawnRequest;

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
                    apply_selections_to_resources,
                    update_garage_ui,
                    handle_build_and_drive,
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
    pub livery_idx:  u8,   // 0..5 maps to LIVERY_NAMES
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

// Livery / sponsor preset names.
const LIVERY_NAMES: [&str; 4] = ["None", "Mud", "Camo", "Racing"];

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
}

impl Default for GarageBuildUiState {
    fn default() -> Self {
        Self {
            open:        false,
            first_run:   false,
            variant_idx: 0,
            paint_idx:   0,
            tire_size:   TireSize::default(),
            bumper:      BumperKind::default(),
            winch:       false,
            body_lift:   false,
            long_arm:    false,
            livery_idx:  0,
            dirty:       false,
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
        format!(
            "Truck      : {}\nPaint      : {}\nTires      : {}\nBumper     : {}\nWinch      : {}\nBody lift  : {}\nLong-arm   : {}\nLivery     : {}",
            VARIANT_NAMES[self.variant_idx as usize],
            PAINT_NAMES[self.paint_idx as usize],
            tire_label,
            bumper_label,
            winch_label,
            if self.body_lift { "Yes (3\" spacers)" } else { "No" },
            if self.long_arm  { "Yes (+7\" travel)"  } else { "No" },
            LIVERY_NAMES[self.livery_idx as usize],
        )
    }
}

// ---------------------------------------------------------------------------
// First-run detection + initial load
// ---------------------------------------------------------------------------

fn load_or_detect_first_run(mut ui: ResMut<GarageBuildUiState>) {
    // Attempt to read a previously saved build.
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

    // No saved build — check whether there is also no save file (genuine first run).
    let no_save = !platform_storage::exists("save_1.json");
    if no_save {
        info!("garage_build: first run detected — will open garage after startup");
        ui.first_run = true;
        // Open will be triggered in open_close_garage after a small delay.
    }
}

// ---------------------------------------------------------------------------
// Marker components
// ---------------------------------------------------------------------------

#[derive(Component)] struct GarageBuildRoot;
#[derive(Component)] struct GarageSpecText;
#[derive(Component)] struct GarageBuildDriveBtn;

/// Tag on each cycle-left / cycle-right button.
/// `category` is a small integer (0 = Truck … 7 = Livery) and `forward` = true for ›.
#[derive(Component)]
struct GarageCycleBtn {
    category: u8,
    forward:  bool,
}

/// Tag on the value label in each category row so we can update it.
#[derive(Component)]
struct GarageCategoryLabel { category: u8 }

// ---------------------------------------------------------------------------
// UI spawn
// ---------------------------------------------------------------------------

const PANEL_BG:  Color = Color::srgba(0.08, 0.09, 0.11, 0.98);
const ACCENT:    Color = Color::srgb(1.0,  0.70, 0.20);
const TEXT_MAIN: Color = Color::srgb(0.90, 0.90, 0.88);
const TEXT_DIM:  Color = Color::srgb(0.55, 0.55, 0.52);
const BTN_BG:    Color = Color::srgba(0.15, 0.16, 0.18, 1.0);
const BTN_HL:    Color = Color::srgba(0.22, 0.50, 0.90, 1.0);

/// Category display names matching the 8 categories (0..7).
const CATEGORY_NAMES: [&str; 8] = [
    "Truck", "Paint", "Tire size", "Bumper",
    "Winch", "Body lift", "Long-arm", "Livery",
];

fn spawn_garage_ui(mut commands: Commands) {
    // Outer full-screen dim.
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
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.60)),
            GlobalZIndex(950),
        ))
        .id();

    // ---- Left panel: category list ------------------------------------------
    let left = commands
        .spawn((
            Node {
                width:          Val::Px(380.0),
                flex_direction: FlexDirection::Column,
                padding:        UiRect::all(Val::Px(20.0)),
                row_gap:        Val::Px(10.0),
                ..default()
            },
            BackgroundColor(PANEL_BG),
        ))
        .id();

    // Title
    let title = commands
        .spawn((
            Text::new("GARAGE"),
            TextFont { font_size: 30.0, ..default() },
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

    let sep = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Px(1.0),
                margin: UiRect::vertical(Val::Px(4.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.08)),
        ))
        .id();

    commands.entity(left).add_children(&[title, subtitle, sep]);

    // Category rows
    for cat in 0u8..8 {
        let row = spawn_category_row(&mut commands, cat);
        commands.entity(left).add_child(row);
    }

    // ---- Right panel: spec sheet + build button -----------------------------
    let right = commands
        .spawn((
            Node {
                width:          Val::Px(310.0),
                flex_direction: FlexDirection::Column,
                padding:        UiRect::all(Val::Px(20.0)),
                row_gap:        Val::Px(12.0),
                justify_content: JustifyContent::SpaceBetween,
                ..default()
            },
            BackgroundColor(PANEL_BG),
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
                width: Val::Percent(100.0),
                height: Val::Px(1.0),
                ..default()
            },
            BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.08)),
        ))
        .id();

    let spec_text = commands
        .spawn((
            GarageSpecText,
            Text::new(""),
            TextFont { font_size: 13.0, ..default() },
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

    // BUILD & DRIVE button
    let build_btn_root = commands
        .spawn((
            GarageBuildDriveBtn,
            Node {
                width:           Val::Percent(100.0),
                height:          Val::Px(52.0),
                align_items:     AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(BTN_HL),
            Button,
        ))
        .id();

    let build_btn_text = commands
        .spawn((
            Text::new("BUILD & DRIVE"),
            TextFont { font_size: 18.0, ..default() },
            TextColor(Color::WHITE),
        ))
        .id();

    commands.entity(build_btn_root).add_child(build_btn_text);
    commands.entity(right).add_children(&[spec_title, spec_sep, spec_text, hint, build_btn_root]);

    commands.entity(root).add_children(&[left, right]);
}

/// Build one category row: [label | ‹ | value | ›]
fn spawn_category_row(commands: &mut Commands, cat: u8) -> Entity {
    let row = commands
        .spawn((
            Node {
                width:           Val::Percent(100.0),
                height:          Val::Px(36.0),
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

    let cat_label = commands
        .spawn((
            Node { width: Val::Px(90.0), ..default() },
            Text::new(CATEGORY_NAMES[cat as usize]),
            TextFont { font_size: 13.0, ..default() },
            TextColor(TEXT_DIM),
        ))
        .id();

    let prev_btn = commands
        .spawn((
            GarageCycleBtn { category: cat, forward: false },
            Node {
                width: Val::Px(28.0), height: Val::Px(28.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.25, 0.25, 0.28, 1.0)),
            Button,
        ))
        .with_children(|p| {
            p.spawn((
                Text::new("‹"),
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
                width: Val::Px(28.0), height: Val::Px(28.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.25, 0.25, 0.28, 1.0)),
            Button,
        ))
        .with_children(|p| {
            p.spawn((
                Text::new("›"),
                TextFont { font_size: 16.0, ..default() },
                TextColor(TEXT_MAIN),
            ));
        })
        .id();

    commands.entity(row).add_children(&[cat_label, prev_btn, val_label, next_btn]);
    row
}

// ---------------------------------------------------------------------------
// Open / close garage
// ---------------------------------------------------------------------------

fn open_close_garage(
    keys:     Res<ButtonInput<KeyCode>>,
    mut ui:   ResMut<GarageBuildUiState>,
    mut delay: Local<u8>,
) {
    // First-run: open after a short startup delay (2 frames).
    if ui.first_run {
        *delay += 1;
        if *delay >= 2 {
            ui.first_run = false;
            ui.open = true;
            info!("garage_build: opening for first run");
        }
        return;
    }

    // Shift+G toggles the garage mid-game.
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if shift && keys.just_pressed(KeyCode::KeyG) {
        ui.open = !ui.open;
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
        }
    }
}

/// Cycle a category by `delta` (+1 = next, -1 = prev). Returns true if changed.
fn cycle_category(ui: &mut GarageBuildUiState, cat: u8, delta: i8) -> bool {
    match cat {
        0 => { // Truck
            ui.variant_idx = wrap_add(ui.variant_idx, delta, VARIANT_COUNT);
            true
        }
        1 => { // Paint
            ui.paint_idx = wrap_add(ui.paint_idx, delta, PAINT_COUNT);
            true
        }
        2 => { // Tire size
            ui.tire_size = if delta > 0 { ui.tire_size.next() } else { tire_prev(ui.tire_size) };
            true
        }
        3 => { // Bumper
            ui.bumper = if delta > 0 { ui.bumper.next() } else { bumper_prev(ui.bumper) };
            // If bumper is now Stock, force winch off.
            if ui.bumper == BumperKind::Stock {
                ui.winch = false;
            }
            true
        }
        4 => { // Winch — only toggle if steel bumper
            if ui.bumper != BumperKind::Stock {
                ui.winch = !ui.winch;
                true
            } else {
                false
            }
        }
        5 => { // Body lift
            ui.body_lift = !ui.body_lift;
            true
        }
        6 => { // Long-arm
            ui.long_arm = !ui.long_arm;
            true
        }
        7 => { // Livery
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
// Apply selections → resources (live preview)
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

    // Vehicle variant.
    *variant = idx_to_variant(ui.variant_idx);

    // Paint (PaintShopState stores index; apply_paint_on_change in paint_shop.rs watches this).
    paint.current_idx = ui.paint_idx as u32;

    // Mods.
    mods.tire_size = ui.tire_size;
    mods.bumper    = ui.bumper;
    mods.winch     = ui.winch && ui.bumper != BumperKind::Stock;
    mods.body_lift = ui.body_lift;
    mods.long_arm  = ui.long_arm;

    // Trigger chassis respawn so the new config is visible.
    respawn.0 = true;
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
// UI update (visibility + labels + spec text)
// ---------------------------------------------------------------------------

fn update_garage_ui(
    ui:             Res<GarageBuildUiState>,
    mut root_q:     Query<&mut Node, With<GarageBuildRoot>>,
    mut label_q:    Query<(&mut Text, &GarageCategoryLabel)>,
    mut spec_q:     Query<&mut Text, (With<GarageSpecText>, Without<GarageCategoryLabel>)>,
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

        // Write final state to all resources.
        *variant         = idx_to_variant(ui.variant_idx);
        paint.current_idx = ui.paint_idx as u32;
        mods.tire_size   = ui.tire_size;
        mods.bumper      = ui.bumper;
        mods.winch       = ui.winch && ui.bumper != BumperKind::Stock;
        mods.body_lift   = ui.body_lift;
        mods.long_arm    = ui.long_arm;
        respawn.0        = true;

        // Save build.
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

        ui.open = false;
        info!("garage_build: BUILD & DRIVE — closing garage");
    }
}
