// Wind system: slowly-varying wind force on the chassis + HUD indicator.
//
// Wind direction rotates in a smooth circle (0.05 rad/s), speed oscillates
// between 1.0 and 6.5 m/s. Force is applied in PhysicsSchedule so it
// integrates cleanly with Avian's solver.

use bevy::prelude::*;
use avian3d::prelude::*;

use crate::vehicle::{Chassis, VehicleRoot};

// ---- Constants ----------------------------------------------------------------

// N per (m/s). At 5 m/s: 250 N lateral. At 6.5 m/s: 325 N.
// Rolling resistance + lateral grip (8000 N/(m/s)) at rest easily overcomes
// this, so a stationary vehicle won't drift noticeably. At highway speed the
// lateral path will bow and the driver must counter-steer.
const WIND_FORCE_COEFF: f32 = 50.0;

// ---- Resources ----------------------------------------------------------------

#[derive(Resource)]
pub struct WindState {
    /// Current wind direction in world space (XZ plane, unit length).
    pub direction: Vec3,
    /// Current wind speed in m/s.
    pub speed_mps: f32,
}

impl Default for WindState {
    fn default() -> Self {
        Self { direction: Vec3::X, speed_mps: 3.0 }
    }
}

/// Controls visibility of the wind HUD panel. Default: visible.
#[derive(Resource)]
pub struct WindHudVisible(pub bool);

impl Default for WindHudVisible {
    fn default() -> Self {
        Self(true)
    }
}

// ---- Components ---------------------------------------------------------------

#[derive(Component)]
struct WindHudRoot;

#[derive(Component)]
enum WindHudText {
    Direction,
    Speed,
}

// ---- Plugin -------------------------------------------------------------------

pub struct WindPlugin;

impl Plugin for WindPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WindState>()
           .init_resource::<WindHudVisible>()
           .add_systems(Startup, spawn_wind_hud)
           .add_systems(Update, (
               update_wind_state,
               update_wind_hud,
               toggle_wind_hud,
           ))
           .add_systems(PhysicsSchedule,
               apply_wind_force
                   .after(PhysicsStepSystems::NarrowPhase)
                   .before(PhysicsStepSystems::Solver),
           );
    }
}

// ---- Wind state update --------------------------------------------------------

fn update_wind_state(time: Res<Time>, mut wind: ResMut<WindState>) {
    let t = time.elapsed_secs();
    // Smooth circular drift: direction completes one full revolution in ~125 s.
    let angle = t * 0.05;
    wind.direction = Vec3::new(angle.cos(), 0.0, angle.sin());
    // Speed oscillates in [1.0, 6.5] m/s — abs keeps it always positive.
    wind.speed_mps = 2.5 + 4.0 * (t * 0.07).sin().abs();
}

// ---- Force application --------------------------------------------------------

fn apply_wind_force(
    wind: Res<WindState>,
    vehicle: Option<Res<VehicleRoot>>,
    mut chassis_q: Query<Forces, With<Chassis>>,
) {
    let Some(vehicle) = vehicle else { return };
    let Ok(mut forces) = chassis_q.get_mut(vehicle.chassis) else { return };

    let f_wind = wind.direction * wind.speed_mps * WIND_FORCE_COEFF;
    forces.apply_force(f_wind);
}

// ---- HUD: spawn ---------------------------------------------------------------

fn spawn_wind_hud(mut commands: Commands) {
    // Panel sits below the top-right stats panel (which ends at top+140+12 = 164 px).
    let root = commands.spawn((
        WindHudRoot,
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(12.0),
            top: Val::Px(168.0),
            width: Val::Px(160.0),
            flex_direction: FlexDirection::Column,
            padding: UiRect::all(Val::Px(8.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.05, 0.05, 0.07, 0.75)),
    )).id();

    let dir_text = commands.spawn((
        WindHudText::Direction,
        Text::new("WIND: E"),
        TextFont { font_size: 13.0, ..default() },
        TextColor(Color::srgb(0.85, 0.85, 0.85)),
    )).id();

    let spd_text = commands.spawn((
        WindHudText::Speed,
        Text::new("     3.0 m/s"),
        TextFont { font_size: 13.0, ..default() },
        TextColor(Color::srgb(0.3, 0.95, 0.3)),
    )).id();

    commands.entity(root).add_children(&[dir_text, spd_text]);
}

// ---- HUD: update --------------------------------------------------------------

fn update_wind_hud(
    wind: Res<WindState>,
    hud_vis: Res<WindHudVisible>,
    mut texts: Query<(&WindHudText, &mut Text, &mut TextColor)>,
    mut root_q: Query<&mut Node, With<WindHudRoot>>,
) {
    // Show/hide driven by WindHudVisible.
    let disp = if hud_vis.0 { Display::Flex } else { Display::None };
    for mut node in &mut root_q {
        node.display = disp;
    }

    if !hud_vis.0 { return; }

    let dir_label = compass_label(wind.direction);

    // Colour: green at calm (1 m/s) → red at strong (6.5 m/s). Range is 1–6.5.
    let t = ((wind.speed_mps - 1.0) / 5.5).clamp(0.0, 1.0);
    let spd_color = Color::srgb(t, 1.0 - t * 0.7, 0.0);

    for (label, mut text, mut color) in &mut texts {
        match label {
            WindHudText::Direction => {
                text.0 = format!("WIND: {}", dir_label);
            }
            WindHudText::Speed => {
                text.0 = format!("  {:.1} m/s", wind.speed_mps);
                color.0 = spd_color;
            }
        }
    }
}

// ---- HUD: toggle --------------------------------------------------------------

fn toggle_wind_hud(
    keys: Res<ButtonInput<KeyCode>>,
    mut hud_vis: ResMut<WindHudVisible>,
    mut root_q: Query<&mut Node, With<WindHudRoot>>,
) {
    if keys.just_pressed(KeyCode::KeyZ) {
        hud_vis.0 = !hud_vis.0;
        let disp = if hud_vis.0 { Display::Flex } else { Display::None };
        for mut node in &mut root_q {
            node.display = disp;
        }
    }
}

// ---- Helpers ------------------------------------------------------------------

/// Map a world-space XZ direction to the nearest compass point label.
/// +X = east, -Z = north (Bevy convention).
fn compass_label(dir: Vec3) -> &'static str {
    // atan2(x, -z): 0 = north, π/2 = east, π = south, 3π/2 = west.
    let deg = dir.x.atan2(-dir.z).to_degrees().rem_euclid(360.0);
    match deg as u32 {
        338..=360 | 0..=22  => "N",
        23..=67              => "NE",
        68..=112             => "E",
        113..=157            => "SE",
        158..=202            => "S",
        203..=247            => "SW",
        248..=292            => "W",
        _                    => "NW",
    }
}
