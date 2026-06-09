// Garage Build Screen — Sprint 74 (polish pass).
//
// Full-screen overlay where the player configures their rig before driving.
// Opens automatically on first run (no saved build) or via Shift+G in-game.
//
// Left column: category rows (Truck / Paint / Tire / Bumper / Winch /
//              Body-lift / Long-arm / Livery) with < > ASCII cycle buttons,
//              each row has a 24x24 colored icon square and a change-pulse
//              animation on the row background.
// Right column: spec sheet (dark card, amber border, derived stats) +
//               BUILD & DRIVE button (amber) + RANDOM RIG button.
//
// Turntable: while garage is open, chassis yaw rotates ~20 deg/s and
//            camera is placed at a flattering low 3/4 angle.
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
use crate::vehicle::{RespawnRequest, Chassis, VehicleRoot};

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

pub struct GarageBuildPlugin;

impl Plugin for GarageBuildPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GarageBuildUiState>()
            .init_resource::<GarageCameraState>()
            .add_systems(Startup, (load_or_detect_first_run, spawn_garage_ui).chain())
            .add_systems(
                Update,
                (
                    open_close_garage,
                    garage_turntable,
                    handle_category_buttons,
                    handle_random_rig,
                    apply_selections_to_resources,
                    tick_row_pulses,
                    update_garage_ui,
                    handle_build_and_drive,
                    animate_backdrop,
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

/// Paint swatch colors matching PALETTE in paint_shop.rs exactly.
const PAINT_SWATCHES: [Color; 8] = [
    Color::srgb(0.85, 0.15, 0.15), // Red
    Color::srgb(0.15, 0.40, 0.85), // Blue
    Color::srgb(0.95, 0.85, 0.15), // Yellow
    Color::srgb(0.20, 0.70, 0.30), // Green
    Color::srgb(0.10, 0.10, 0.10), // Black
    Color::srgb(0.95, 0.95, 0.92), // White
    Color::srgb(1.00, 0.55, 0.10), // Orange
    Color::srgb(0.60, 0.20, 0.85), // Purple
];

/// 24x24 icon square colors per category (cat 0..7).
/// Paint (cat=1) is dynamic — its icon is updated each frame via GarageIconSwatch.
const ICON_COLORS: [Color; 8] = [
    Color::srgb(0.55, 0.45, 0.30), // Truck    — tan/sandy
    Color::srgb(0.85, 0.15, 0.15), // Paint    — default red (updated dynamically)
    Color::srgb(0.22, 0.22, 0.22), // Tire     — dark rubber
    Color::srgb(0.58, 0.58, 0.62), // Bumper   — steel grey
    Color::srgb(0.70, 0.70, 0.72), // Winch    — light steel
    Color::srgb(0.30, 0.55, 0.30), // Body lift — forest green
    Color::srgb(0.42, 0.30, 0.18), // Long-arm  — rust/brown
    Color::srgb(0.20, 0.40, 0.65), // Livery   — racing blue
];

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

    /// Backdrop fade-in: 0.0 = closed/faded out, 1.0 = fully visible.
    pub backdrop_alpha: f32,

    /// Which row (0..7) just had a change — drives row pulse timer.
    pub pulse_row: Option<u8>,
}

impl Default for GarageBuildUiState {
    fn default() -> Self {
        Self {
            open:           false,
            first_run:      false,
            variant_idx:    0,
            paint_idx:      0,
            tire_size:      TireSize::default(),
            bumper:         BumperKind::default(),
            winch:          false,
            body_lift:      false,
            long_arm:       false,
            livery_idx:     0,
            dirty:          false,
            backdrop_alpha: 0.0,
            pulse_row:      None,
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

    /// Derived stats for spec sheet.
    fn lift_height_inches(&self) -> f32 {
        use crate::vehicle_mods::{LONG_ARM_SPAWN_LIFT, BODY_LIFT_DELTA, TireSize as TS};
        let tire_delta = self.tire_size.radius() - TS::Stock.radius();
        let long_arm   = if self.long_arm { LONG_ARM_SPAWN_LIFT } else { 0.0 };
        let body       = if self.body_lift { BODY_LIFT_DELTA } else { 0.0 };
        // metres to inches: 1 m = 39.3701"
        (long_arm + body + tire_delta) * 39.3701
    }

    fn tire_diameter_inches(&self) -> f32 {
        // radius in metres * 2 * 39.3701
        self.tire_size.radius() * 2.0 * 39.3701
    }

    fn est_weight_lbs(&self) -> f32 {
        // Base weight varies by variant (rough estimates, fun flavour).
        let base: f32 = match self.variant_idx {
            0 => 3400.0, // Jeep TJ
            1 => 3700.0, // Bronco
            2 => 4100.0, // Pickup
            3 => 6000.0, // Hummer
            4 => 2200.0, // Buggy
            5 => 3600.0, // Highland SK
            6 => 2800.0, // Dune Skipper
            7 => 5500.0, // Hauler SK
            _ => 3500.0,
        };
        let tire_penalty = match self.tire_size {
            TireSize::Stock  => 0.0,
            TireSize::Large  => 60.0,
            TireSize::XLarge => 130.0,
        };
        let bumper_penalty = match self.bumper {
            BumperKind::Stock          => 0.0,
            BumperKind::SteelFront     => 66.0,
            BumperKind::SteelFrontRear => 132.0,
        };
        let winch_penalty  = if self.winch     { 85.0  } else { 0.0 };
        let lift_penalty   = if self.long_arm  { 45.0  } else { 0.0 };
        let body_penalty   = if self.body_lift { 8.0   } else { 0.0 };
        base + tire_penalty + bumper_penalty + winch_penalty + lift_penalty + body_penalty
    }
}

// ---------------------------------------------------------------------------
// Garage camera state — save/restore for turntable
// ---------------------------------------------------------------------------

#[derive(Resource, Default)]
pub struct GarageCameraState {
    /// Cached camera transform before the garage opened.
    pub saved_transform: Option<Transform>,
    /// Accumulated turntable yaw (radians) applied to the chassis.
    pub turntable_yaw:   f32,
    /// Whether the garage was open last frame (edge detection).
    pub was_open:        bool,
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
#[derive(Component)] struct GarageRandomRigBtn;
#[derive(Component)] struct GarageBackdrop;

/// Tag on each cycle-left / cycle-right button.
/// `category` is a small integer (0 = Truck … 7 = Livery) and `forward` = true for >.
#[derive(Component)]
struct GarageCycleBtn {
    category: u8,
    forward:  bool,
}

/// Tag on the value label in each category row so we can update it.
#[derive(Component)]
struct GarageCategoryLabel { category: u8 }

/// Tag on the category row background node so we can pulse it.
#[derive(Component)]
struct GarageCategoryRow {
    category: u8,
    /// Pulse timer in seconds (0.0 = no pulse; counts down from 0.3).
    pulse_t:  f32,
}

/// Tag on the 24x24 icon swatch in the Paint row — updated to current paint color.
#[derive(Component)]
struct GarageIconSwatch;

// ---------------------------------------------------------------------------
// Colors
// ---------------------------------------------------------------------------

const PANEL_BG:      Color = Color::srgba(0.08, 0.09, 0.11, 0.98);
const SPEC_CARD_BG:  Color = Color::srgba(0.05, 0.06, 0.08, 0.98);
const ACCENT:        Color = Color::srgb(1.0,  0.70, 0.20);
const TEXT_MAIN:     Color = Color::srgb(0.90, 0.90, 0.88);
const TEXT_DIM:      Color = Color::srgb(0.55, 0.55, 0.52);
const BTN_BG:        Color = Color::srgba(0.15, 0.16, 0.18, 1.0);
const ROW_BASE_BG:   Color = Color::srgba(0.13, 0.14, 0.17, 1.0);
const ROW_PULSE_BG:  Color = Color::srgba(0.40, 0.32, 0.08, 1.0);
const BUILD_BTN_BG:  Color = Color::srgb(0.88, 0.60, 0.08);
const BUILD_BTN_HL:  Color = Color::srgb(1.00, 0.75, 0.15);
const RANDOM_BTN_BG: Color = Color::srgba(0.18, 0.28, 0.18, 1.0);
const AMBER_BORDER:  Color = Color::srgb(0.85, 0.55, 0.05);

/// Category display names matching the 8 categories (0..7).
const CATEGORY_NAMES: [&str; 8] = [
    "Truck", "Paint", "Tire size", "Bumper",
    "Winch", "Body lift", "Long-arm", "Livery",
];

// ---------------------------------------------------------------------------
// UI spawn
// ---------------------------------------------------------------------------

fn spawn_garage_ui(mut commands: Commands) {
    // Full-screen dim / backdrop — we animate its alpha for fade-in.
    let root = commands
        .spawn((
            GarageBuildRoot,
            GarageBackdrop,
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

    // ---- Left panel: category list ------------------------------------------
    let left = commands
        .spawn((
            Node {
                width:          Val::Px(420.0),
                flex_direction: FlexDirection::Column,
                padding:        UiRect::all(Val::Px(20.0)),
                row_gap:        Val::Px(8.0),
                ..default()
            },
            BackgroundColor(PANEL_BG),
        ))
        .id();

    // Header area: title + subtitle
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

    let sep = commands
        .spawn((
            Node {
                width:  Val::Percent(100.0),
                height: Val::Px(2.0),
                margin: UiRect::vertical(Val::Px(6.0)),
                ..default()
            },
            BackgroundColor(AMBER_BORDER),
        ))
        .id();

    commands.entity(left).add_children(&[title, subtitle, sep]);

    // Category rows
    for cat in 0u8..8 {
        let row = spawn_category_row(&mut commands, cat);
        commands.entity(left).add_child(row);
    }

    // ---- Right panel: spec sheet + buttons ----------------------------------
    let right = commands
        .spawn((
            Node {
                width:           Val::Px(320.0),
                flex_direction:  FlexDirection::Column,
                row_gap:         Val::Px(14.0),
                ..default()
            },
        ))
        .id();

    // Spec sheet card
    let spec_card = commands
        .spawn((
            Node {
                flex_direction:  FlexDirection::Column,
                padding:         UiRect::all(Val::Px(16.0)),
                row_gap:         Val::Px(6.0),
                border:          UiRect::all(Val::Px(2.0)),
                ..default()
            },
            BackgroundColor(SPEC_CARD_BG),
            BorderColor(AMBER_BORDER),
        ))
        .id();

    let spec_title = commands
        .spawn((
            Text::new("SPEC SHEET"),
            TextFont { font_size: 15.0, ..default() },
            TextColor(ACCENT),
        ))
        .id();

    let spec_sep = commands
        .spawn((
            Node {
                width:  Val::Percent(100.0),
                height: Val::Px(1.0),
                margin: UiRect::vertical(Val::Px(2.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.10)),
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

    commands.entity(spec_card).add_children(&[spec_title, spec_sep, spec_text]);

    // Hint text
    let hint = commands
        .spawn((
            Text::new("Shift+G  close without saving"),
            TextFont { font_size: 11.0, ..default() },
            TextColor(TEXT_DIM),
        ))
        .id();

    // BUILD & DRIVE button (large amber)
    let build_btn = commands
        .spawn((
            GarageBuildDriveBtn,
            Node {
                width:           Val::Percent(100.0),
                height:          Val::Px(60.0),
                align_items:     AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(BUILD_BTN_BG),
            Button,
        ))
        .with_children(|p| {
            p.spawn((
                Text::new("BUILD & DRIVE"),
                TextFont { font_size: 20.0, ..default() },
                TextColor(Color::srgb(0.08, 0.05, 0.02)),
            ));
        })
        .id();

    // RANDOM RIG button
    let random_btn = commands
        .spawn((
            GarageRandomRigBtn,
            Node {
                width:           Val::Percent(100.0),
                height:          Val::Px(42.0),
                align_items:     AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(RANDOM_BTN_BG),
            Button,
        ))
        .with_children(|p| {
            p.spawn((
                Text::new("RANDOM RIG"),
                TextFont { font_size: 15.0, ..default() },
                TextColor(TEXT_MAIN),
            ));
        })
        .id();

    commands.entity(right).add_children(&[spec_card, hint, build_btn, random_btn]);
    commands.entity(root).add_children(&[left, right]);
}

/// Build one category row: [icon | label | < | value | >]
fn spawn_category_row(commands: &mut Commands, cat: u8) -> Entity {
    let row = commands
        .spawn((
            GarageCategoryRow { category: cat, pulse_t: 0.0 },
            Node {
                width:           Val::Percent(100.0),
                height:          Val::Px(38.0),
                flex_direction:  FlexDirection::Row,
                align_items:     AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                column_gap:      Val::Px(6.0),
                padding:         UiRect::horizontal(Val::Px(6.0)),
                ..default()
            },
            BackgroundColor(ROW_BASE_BG),
        ))
        .id();

    // 24x24 colored icon square.
    let icon_color = ICON_COLORS[cat as usize];
    let mut icon_spawn = commands.spawn((
        Node {
            width:  Val::Px(24.0),
            height: Val::Px(24.0),
            flex_shrink: 0.0,
            ..default()
        },
        BackgroundColor(icon_color),
    ));
    // Tag Paint row icon so we can update its color dynamically.
    if cat == 1 {
        icon_spawn.insert(GarageIconSwatch);
    }
    let icon = icon_spawn.id();

    let cat_label = commands
        .spawn((
            Node { width: Val::Px(78.0), flex_shrink: 0.0, ..default() },
            Text::new(CATEGORY_NAMES[cat as usize]),
            TextFont { font_size: 12.0, ..default() },
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
                flex_shrink:     0.0,
                ..default()
            },
            BackgroundColor(BTN_BG),
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
            TextFont { font_size: 12.0, ..default() },
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
                flex_shrink:     0.0,
                ..default()
            },
            BackgroundColor(BTN_BG),
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
// Turntable + camera positioning
// ---------------------------------------------------------------------------

const TURNTABLE_DEG_PER_SEC: f32 = 20.0;

/// Flattering low 3/4-angle turntable camera offset from chassis center.
/// 8m out, 3.5m up, slightly behind the right shoulder — looks showroom.
const TURNTABLE_OFFSET: Vec3 = Vec3::new(6.0, 3.5, 7.0);

fn garage_turntable(
    ui:        Res<GarageBuildUiState>,
    mut gc:    ResMut<GarageCameraState>,
    vehicle:   Option<Res<VehicleRoot>>,
    chassis_q: Query<&Transform, With<Chassis>>,
    mut cam_q: Query<&mut Transform, (With<Camera3d>, Without<Chassis>)>,
    time:      Res<Time>,
) {
    let just_opened  = ui.open  && !gc.was_open;
    let just_closed  = !ui.open && gc.was_open;
    gc.was_open = ui.open;

    let Ok(mut cam) = cam_q.single_mut() else { return };

    if just_opened {
        // Cache current camera transform to restore later.
        gc.saved_transform = Some(*cam);
        gc.turntable_yaw   = 0.0;
    }

    if just_closed {
        // Restore camera to pre-garage position.
        if let Some(saved) = gc.saved_transform.take() {
            *cam = saved;
        }
        return;
    }

    if !ui.open {
        return;
    }

    // Accumulate turntable yaw.
    gc.turntable_yaw += TURNTABLE_DEG_PER_SEC.to_radians() * time.delta_secs();

    // Get chassis world position (or fall back to origin).
    let chassis_pos = if let Some(vehicle) = vehicle {
        if let Ok(chassis_tf) = chassis_q.get(vehicle.chassis) {
            chassis_tf.translation
        } else {
            Vec3::ZERO
        }
    } else {
        Vec3::ZERO
    };

    // Rotate the offset around the chassis Y axis.
    let rot      = Quat::from_rotation_y(gc.turntable_yaw);
    let cam_pos  = chassis_pos + rot * TURNTABLE_OFFSET;
    let look_at  = chassis_pos + Vec3::Y * 1.2;

    *cam = Transform::from_translation(cam_pos).looking_at(look_at, Vec3::Y);
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
            ui.dirty     = true;
            ui.pulse_row = Some(btn.category);
        }
    }
}

// ---------------------------------------------------------------------------
// Random Rig button
// ---------------------------------------------------------------------------

fn handle_random_rig(
    interactions: Query<&Interaction, (Changed<Interaction>, With<GarageRandomRigBtn>)>,
    mut ui:        ResMut<GarageBuildUiState>,
    time:          Res<Time>,
) {
    for interaction in &interactions {
        if *interaction != Interaction::Pressed {
            continue;
        }
        if !ui.open {
            continue;
        }

        // Use elapsed time as a cheap entropy source (no rand crate needed).
        let seed = (time.elapsed_secs() * 1_000_000.0) as u64;
        let r = |n: u64, s: u64| -> u8 { ((seed.wrapping_mul(6364136223846793005).wrapping_add(s)) % n) as u8 };

        ui.variant_idx = r(VARIANT_COUNT as u64, 1);
        ui.paint_idx   = r(PAINT_COUNT as u64,   2);
        ui.tire_size   = match r(3, 3) {
            0 => TireSize::Stock,
            1 => TireSize::Large,
            _ => TireSize::XLarge,
        };
        ui.bumper = match r(3, 4) {
            0 => BumperKind::Stock,
            1 => BumperKind::SteelFront,
            _ => BumperKind::SteelFrontRear,
        };
        ui.winch      = ui.bumper != BumperKind::Stock && r(2, 5) == 1;
        ui.body_lift  = r(2, 6) == 1;
        ui.long_arm   = r(2, 7) == 1;
        ui.livery_idx = r(LIVERY_COUNT as u64, 8);

        ui.dirty     = true;
        ui.pulse_row = None; // all rows changed, skip single-row pulse
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
// Row pulse animation
// ---------------------------------------------------------------------------

const PULSE_DURATION: f32 = 0.3;

fn tick_row_pulses(
    ui:      Res<GarageBuildUiState>,
    time:    Res<Time>,
    mut rows: Query<(&mut GarageCategoryRow, &mut BackgroundColor)>,
) {
    let dt = time.delta_secs();

    for (mut row, mut bg) in &mut rows {
        // Start a new pulse if this row just changed.
        if let Some(pcat) = ui.pulse_row {
            if pcat == row.category && row.pulse_t <= 0.0 {
                row.pulse_t = PULSE_DURATION;
            }
        }

        if row.pulse_t > 0.0 {
            row.pulse_t = (row.pulse_t - dt).max(0.0);
            // Lerp from bright pulse color back to base.
            let t = row.pulse_t / PULSE_DURATION;
            let r_base = ROW_BASE_BG.to_srgba();
            let r_pulse = ROW_PULSE_BG.to_srgba();
            let blended = Srgba::new(
                r_base.red   + (r_pulse.red   - r_base.red)   * t,
                r_base.green + (r_pulse.green - r_base.green) * t,
                r_base.blue  + (r_pulse.blue  - r_base.blue)  * t,
                1.0,
            );
            *bg = BackgroundColor(Color::srgba(blended.red, blended.green, blended.blue, blended.alpha));
        } else {
            *bg = BackgroundColor(ROW_BASE_BG);
        }
    }
}

// ---------------------------------------------------------------------------
// Backdrop fade-in
// ---------------------------------------------------------------------------

const BACKDROP_FADE_SPEED: f32 = 5.0; // ~0.2s to reach alpha 0.60

fn animate_backdrop(
    mut ui:     ResMut<GarageBuildUiState>,
    time:       Res<Time>,
    mut root_q: Query<(&mut Node, &mut BackgroundColor), With<GarageBackdrop>>,
) {
    for (mut node, mut bg) in &mut root_q {
        if ui.open {
            node.display = Display::Flex;
            // Fade in.
            let target = 0.60_f32;
            ui.backdrop_alpha = (ui.backdrop_alpha + time.delta_secs() * BACKDROP_FADE_SPEED).min(target);
            *bg = BackgroundColor(Color::srgba(0.0, 0.0, 0.0, ui.backdrop_alpha));
        } else {
            // Fade out.
            ui.backdrop_alpha = (ui.backdrop_alpha - time.delta_secs() * BACKDROP_FADE_SPEED * 2.0).max(0.0);
            *bg = BackgroundColor(Color::srgba(0.0, 0.0, 0.0, ui.backdrop_alpha));
            if ui.backdrop_alpha <= 0.0 {
                node.display = Display::None;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// UI update (visibility + labels + spec text + build btn hover + icon swatch)
// ---------------------------------------------------------------------------

fn update_garage_ui(
    ui:             Res<GarageBuildUiState>,
    mut root_q:     Query<&mut Node, (With<GarageBuildRoot>, Without<GarageBackdrop>)>,
    mut label_q:    Query<(&mut Text, &mut TextColor, &GarageCategoryLabel)>,
    mut spec_q:     Query<&mut Text, (With<GarageSpecText>, Without<GarageCategoryLabel>)>,
    mut build_q:    Query<(&Interaction, &mut BackgroundColor), With<GarageBuildDriveBtn>>,
    mut random_q:   Query<(&Interaction, &mut BackgroundColor), (With<GarageRandomRigBtn>, Without<GarageBuildDriveBtn>)>,
    mut swatch_q:   Query<&mut BackgroundColor, (With<GarageIconSwatch>, Without<GarageBuildDriveBtn>, Without<GarageRandomRigBtn>, Without<GarageCategoryRow>)>,
) {
    // Show / hide main layout nodes inside root (root display controlled by animate_backdrop).
    for mut node in &mut root_q {
        node.display = if ui.open { Display::Flex } else { Display::None };
    }

    if !ui.open {
        return;
    }

    // Update category value labels.
    for (mut text, mut color, lbl) in &mut label_q {
        text.0 = category_value_text(&ui, lbl.category).to_string();
        // Active value in bright amber.
        color.0 = ACCENT;
    }

    // Update spec sheet.
    for mut text in &mut spec_q {
        text.0 = spec_text(&ui);
    }

    // Hover effect on BUILD & DRIVE button.
    for (interaction, mut bg) in &mut build_q {
        *bg = match *interaction {
            Interaction::Hovered  => BackgroundColor(BUILD_BTN_HL),
            Interaction::Pressed  => BackgroundColor(BUILD_BTN_HL),
            Interaction::None     => BackgroundColor(BUILD_BTN_BG),
        };
    }

    // Hover effect on RANDOM RIG button.
    for (interaction, mut bg) in &mut random_q {
        *bg = match *interaction {
            Interaction::Hovered  => BackgroundColor(Color::srgba(0.25, 0.40, 0.25, 1.0)),
            Interaction::Pressed  => BackgroundColor(Color::srgba(0.25, 0.40, 0.25, 1.0)),
            Interaction::None     => BackgroundColor(RANDOM_BTN_BG),
        };
    }

    // Update paint icon swatch color to current paint selection.
    for mut bg in &mut swatch_q {
        *bg = BackgroundColor(PAINT_SWATCHES[ui.paint_idx as usize]);
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

fn spec_text(ui: &GarageBuildUiState) -> String {
    let tire_label = match ui.tire_size {
        TireSize::Stock  => "33\" (Stock)",
        TireSize::Large  => "35\"",
        TireSize::XLarge => "37\"",
    };
    let bumper_label = match ui.bumper {
        BumperKind::Stock          => "Stock",
        BumperKind::SteelFront     => "Steel Front",
        BumperKind::SteelFrontRear => "Steel Front+Rear",
    };
    let winch_label = if ui.bumper == BumperKind::Stock {
        "N/A (need steel bumper)"
    } else if ui.winch {
        "Fitted"
    } else {
        "None"
    };
    let lift_in  = ui.lift_height_inches();
    let tire_dia = ui.tire_diameter_inches();
    let weight   = ui.est_weight_lbs();

    format!(
        "Truck      : {}\nPaint      : {}\nTires      : {}\nBumper     : {}\nWinch      : {}\nBody lift  : {}\nLong-arm   : {}\nLivery     : {}\n---\nLift ht    : {:.1}\"\nTire dia   : {:.1}\"\nEst. weight: {:.0} lbs",
        VARIANT_NAMES[ui.variant_idx as usize],
        PAINT_NAMES[ui.paint_idx as usize],
        tire_label,
        bumper_label,
        winch_label,
        if ui.body_lift { "Yes (3\" spacers)" } else { "No" },
        if ui.long_arm  { "Yes (+7\" travel)"  } else { "No" },
        LIVERY_NAMES[ui.livery_idx as usize],
        lift_in,
        tire_dia,
        weight,
    )
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
    mut gc:        ResMut<GarageCameraState>,
    mut cam_q:     Query<&mut Transform, (With<Camera3d>, Without<Chassis>)>,
) {
    for interaction in &interactions {
        if *interaction != Interaction::Pressed {
            continue;
        }
        if !ui.open {
            continue;
        }

        // Write final state to all resources.
        *variant          = idx_to_variant(ui.variant_idx);
        paint.current_idx = ui.paint_idx as u32;
        mods.tire_size    = ui.tire_size;
        mods.bumper       = ui.bumper;
        mods.winch        = ui.winch && ui.bumper != BumperKind::Stock;
        mods.body_lift    = ui.body_lift;
        mods.long_arm     = ui.long_arm;
        respawn.0         = true;

        // Restore camera.
        if let Some(saved) = gc.saved_transform.take() {
            if let Ok(mut cam) = cam_q.single_mut() {
                *cam = saved;
            }
        }
        gc.was_open = false;

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
        info!("garage_build: BUILD & DRIVE -- closing garage");
    }
}
