// Fuel system for SandK Offroad.
//
// Tank: 60 L. Burn rate driven by throttle input and vehicle speed:
//   BURN_BASE   = 0.05 L/s  (idle)
//   BURN_DRIVE  = 0.30 L/s  (added at full throttle)
//   BURN_SPEED  = 0.05 L/s  (added at 10+ m/s)
//
// At empty the chassis drive input is zeroed before apply_drive_input runs.
// R-key refills the tank (mirrors save.rs reset_chassis behaviour).
// Inside the repair zone (RepairActive.in_zone) the tank refills at 4 L/s.
//
// HUD: bottom-right panel, F9 to toggle.  Coloured bar + numeric readout.
// Empty banner: top-centre, auto-hides after 5 s or when fuel returns above 0.

use bevy::prelude::*;
use avian3d::prelude::LinearVelocity;

use crate::repair::RepairActive;
use crate::vehicle::{Chassis, DriveInput, VehicleRoot};

// ---- Burn constants ----------------------------------------------------------

const BURN_BASE:  f32 = 0.05; // L/s at idle
const BURN_DRIVE: f32 = 0.30; // L/s added at full throttle
const BURN_SPEED: f32 = 0.05; // L/s added at full speed (10 m/s)
const REFILL_ZONE_RATE: f32 = 4.0; // L/s inside repair zone

// ---- Public resource ---------------------------------------------------------

#[derive(Resource)]
pub struct Fuel {
    pub current_l: f32,
    pub capacity_l: f32,
}

impl Default for Fuel {
    fn default() -> Self {
        Self { current_l: 60.0, capacity_l: 60.0 }
    }
}

// ---- Private state -----------------------------------------------------------

#[derive(Resource, Default)]
struct FuelHudVisible(bool);

impl FuelHudVisible {
    fn default_visible() -> Self { Self(true) }
}

/// Tracks whether the empty-warning banner is currently shown and for how long.
#[derive(Resource, Default)]
struct EmptyWarning {
    showing: bool,
    timer_s: f32,
    /// True once we have already logged the warning this empty episode.
    logged: bool,
}

// ---- Components --------------------------------------------------------------

#[derive(Component)] struct FuelHudRoot;
#[derive(Component)] struct FuelBarFill;
#[derive(Component)] struct FuelNumericText;
#[derive(Component)] struct FuelEmptyBanner;

// ---- Plugin ------------------------------------------------------------------

pub struct FuelPlugin;

impl Plugin for FuelPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Fuel>()
           .insert_resource(FuelHudVisible::default_visible())
           .init_resource::<EmptyWarning>()
           .add_systems(Startup, spawn_fuel_hud)
           .add_systems(Update, (
               consume_fuel.before(crate::vehicle::apply_drive_input),
               handle_refill,
               update_fuel_hud,
               toggle_fuel_hud,
               update_empty_banner,
           ));
    }
}

// ---- Startup: build HUD ------------------------------------------------------

fn spawn_fuel_hud(mut commands: Commands) {
    // Outer panel: bottom-right, above the FPS counter (~12 px from bottom,
    // 64 px offset gives room for the FPS row beneath it).
    let panel = commands.spawn((
        FuelHudRoot,
        Node {
            position_type: PositionType::Absolute,
            right:  Val::Px(12.0),
            bottom: Val::Px(64.0),
            width:  Val::Px(200.0),
            height: Val::Px(48.0),
            flex_direction: FlexDirection::Column,
            padding: UiRect { left: Val::Px(8.0), right: Val::Px(8.0), top: Val::Px(5.0), bottom: Val::Px(5.0) },
            row_gap: Val::Px(4.0),
            display: Display::Flex,
            ..default()
        },
        BackgroundColor(Color::srgba(0.05, 0.05, 0.07, 0.80)),
    )).id();

    // Title + numeric on one row
    let top_row = commands.spawn(Node {
        flex_direction: FlexDirection::Row,
        justify_content: JustifyContent::SpaceBetween,
        width: Val::Percent(100.0),
        ..default()
    }).id();

    let title = commands.spawn((
        Text::new("FUEL"),
        TextFont { font_size: 11.0, ..default() },
        TextColor(Color::srgb(0.75, 0.75, 0.75)),
    )).id();

    let numeric = commands.spawn((
        FuelNumericText,
        Text::new("60.0 L / 60.0 L"),
        TextFont { font_size: 11.0, ..default() },
        TextColor(Color::srgb(0.75, 0.75, 0.75)),
    )).id();

    // Bar background
    let bar_bg = commands.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Px(8.0),
            ..default()
        },
        BackgroundColor(Color::srgba(0.15, 0.15, 0.15, 1.0)),
    )).id();

    // Bar fill (width set dynamically)
    let bar_fill = commands.spawn((
        FuelBarFill,
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..default()
        },
        BackgroundColor(Color::srgb(0.2, 0.85, 0.3)),
    )).id();

    commands.entity(bar_bg).add_child(bar_fill);
    commands.entity(top_row).add_children(&[title, numeric]);
    commands.entity(panel).add_children(&[top_row, bar_bg]);

    // --- Empty banner (top-centre, hidden by default) -------------------------
    const BANNER_W: f32 = 360.0;
    let banner = commands.spawn((
        FuelEmptyBanner,
        Node {
            position_type: PositionType::Absolute,
            left:   Val::Percent(50.0),
            top:    Val::Px(48.0),
            margin: UiRect { left: Val::Px(-(BANNER_W / 2.0)), ..default() },
            width:  Val::Px(BANNER_W),
            height: Val::Px(36.0),
            justify_content: JustifyContent::Center,
            align_items:     AlignItems::Center,
            display:         Display::None,
            ..default()
        },
        BackgroundColor(Color::srgba(0.55, 0.05, 0.05, 0.92)),
        Outline { width: Val::Px(1.0), offset: Val::Px(0.0), color: Color::srgb(0.95, 0.2, 0.2) },
    )).id();

    let banner_text = commands.spawn((
        Text::new("OUT OF FUEL -- press R to respawn"),
        TextFont { font_size: 14.0, ..default() },
        TextColor(Color::srgb(1.0, 0.55, 0.55)),
    )).id();

    commands.entity(banner).add_child(banner_text);
}

// ---- Update: consume fuel ----------------------------------------------------
// Runs before apply_drive_input so that zeroing drive takes effect this frame.

fn consume_fuel(
    time:     Res<Time>,
    vehicle:  Option<Res<VehicleRoot>>,
    chassis_q: Query<&LinearVelocity, With<Chassis>>,
    mut input: ResMut<DriveInput>,
    mut fuel:  ResMut<Fuel>,
    mut warn:  ResMut<EmptyWarning>,
) {
    if fuel.current_l <= 0.0 {
        // Tank already empty: disable drive.
        input.drive = 0.0;

        // Trigger the one-time empty warning.
        if !warn.logged {
            warn!("fuel: tank empty -- vehicle disabled");
            warn.logged  = true;
            warn.showing = true;
            warn.timer_s = 0.0;
        }
        return;
    }

    // Fuel is available: reset the empty-warning latch so it can fire again
    // on the next empty episode.
    warn.logged = false;

    // Compute speed-based normalised load.
    let speed_mps = if let Some(vehicle) = vehicle {
        if let Ok(lv) = chassis_q.get(vehicle.chassis) {
            Vec3::new(lv.x, lv.y, lv.z).length()
        } else {
            0.0
        }
    } else {
        0.0
    };

    let speed_norm = (speed_mps / 10.0).min(1.0);
    let burn = BURN_BASE + BURN_DRIVE * input.drive.abs() + BURN_SPEED * speed_norm;
    fuel.current_l = (fuel.current_l - burn * time.delta_secs()).max(0.0);
}

// ---- Update: handle refill (R-key + repair zone) ----------------------------

fn handle_refill(
    keys:   Res<ButtonInput<KeyCode>>,
    repair: Option<Res<RepairActive>>,
    time:   Res<Time>,
    mut fuel: ResMut<Fuel>,
    mut warn: ResMut<EmptyWarning>,
) {
    // R-key: instant full refill (mirrors save.rs reset_chassis).
    if keys.just_pressed(KeyCode::KeyR) {
        fuel.current_l = fuel.capacity_l;
        warn.showing   = false;
        warn.logged    = false;
        return;
    }

    // Repair zone: slow trickle refill.
    if let Some(repair) = repair {
        if repair.in_zone && fuel.current_l < fuel.capacity_l {
            fuel.current_l = (fuel.current_l + REFILL_ZONE_RATE * time.delta_secs())
                .min(fuel.capacity_l);
        }
    }
}

// ---- Update: update HUD bar and numeric -------------------------------------

fn update_fuel_hud(
    fuel:     Res<Fuel>,
    time:     Res<Time>,
    mut bar_q: Query<(&mut Node, &mut BackgroundColor), With<FuelBarFill>>,
    mut text_q: Query<&mut Text, With<FuelNumericText>>,
) {
    let frac = (fuel.current_l / fuel.capacity_l).clamp(0.0, 1.0);
    let pct  = frac * 100.0;

    // Colour: green >50%, yellow 20-50%, red <20%, flashing red <5%.
    let flash = pct < 5.0;
    let base_color = if pct > 50.0 {
        Color::srgb(0.2, 0.85, 0.3)
    } else if pct > 20.0 {
        Color::srgb(0.95, 0.85, 0.2)
    } else {
        Color::srgb(0.95, 0.2, 0.2)
    };

    let bar_color = if flash {
        // Flash at ~2 Hz using sine wave.
        let brightness = (time.elapsed_secs() * std::f32::consts::TAU * 2.0).sin() * 0.5 + 0.5;
        Color::srgba(0.95, 0.2 * brightness, 0.2 * brightness, 1.0)
    } else {
        base_color
    };

    for (mut node, mut bg) in &mut bar_q {
        node.width = Val::Percent(pct);
        bg.0 = bar_color;
    }

    for mut text in &mut text_q {
        text.0 = format!("{:.1} L / {:.1} L", fuel.current_l, fuel.capacity_l);
    }
}

// ---- Update: update empty-fuel banner ----------------------------------------

fn update_empty_banner(
    time:  Res<Time>,
    fuel:  Res<Fuel>,
    mut warn: ResMut<EmptyWarning>,
    mut banner_q: Query<&mut Node, With<FuelEmptyBanner>>,
) {
    // Auto-hide if fuel is restored above 0.
    if fuel.current_l > 0.0 && warn.showing {
        warn.showing = false;
    }

    // Advance timer while showing; hide after 5 s.
    if warn.showing {
        warn.timer_s += time.delta_secs();
        if warn.timer_s >= 5.0 {
            warn.showing = false;
        }
    }

    let display = if warn.showing { Display::Flex } else { Display::None };
    for mut node in &mut banner_q {
        node.display = display;
    }
}

// ---- Update: toggle fuel HUD (F9) -------------------------------------------

fn toggle_fuel_hud(
    keys:    Res<ButtonInput<KeyCode>>,
    mut vis: ResMut<FuelHudVisible>,
    mut root_q: Query<&mut Node, With<FuelHudRoot>>,
) {
    if keys.just_pressed(KeyCode::F9) {
        vis.0 = !vis.0;
        let display = if vis.0 { Display::Flex } else { Display::None };
        for mut node in &mut root_q {
            node.display = display;
        }
    }
}
