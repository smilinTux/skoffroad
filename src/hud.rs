// HUD overlay for skoffroad — built with Bevy 0.18 built-in UI.
//
// Layout:
//   Top-left   : semi-transparent panel — speed, throttle/steer/brake, tilt
//   Top-right  : semi-transparent panel — session stats (distance, max speed/tilt, clock, time)
//   Bottom-right: FPS counter
//
// Toggle: press H to show/hide all HUD nodes.

use bevy::{
    diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin},
    prelude::*,
};
use avian3d::prelude::LinearVelocity;

use crate::sky::TimeOfDay;
use crate::vehicle::{Chassis, DriveInput, VehicleRoot};

// ---- Public plugin ----------------------------------------------------------

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        // Register FPS diagnostics if not already present.
        if !app.is_plugin_added::<FrameTimeDiagnosticsPlugin>() {
            app.add_plugins(FrameTimeDiagnosticsPlugin::default());
        }

        app.init_resource::<HudVisible>()
            .init_resource::<SessionStats>()
            .add_systems(Startup, spawn_hud)
            .add_systems(
                Update,
                (update_session_stats, update_hud, toggle_hud)
                    .run_if(resource_exists::<VehicleRoot>),
            );
    }
}

// ---- Resources & components -------------------------------------------------

/// Tracks whether the HUD is currently visible.
#[derive(Resource)]
pub struct HudVisible(pub bool);

impl Default for HudVisible {
    fn default() -> Self {
        Self(true)
    }
}

/// Running gameplay statistics accumulated since session start.
#[derive(Resource, Default)]
pub struct SessionStats {
    pub distance_m:   f32,
    pub max_speed_mps: f32,
    pub max_tilt_deg:  f32,
    pub last_pos:      Option<Vec3>,
    pub elapsed_s:     f32,
}

/// Marker on the root UI node — used for toggle.
#[derive(Component)]
struct HudRoot;

/// Marker on each text leaf so update systems can find exactly the right node.
#[derive(Component)]
enum HudText {
    Speed,
    Throttle,
    Steer,
    Brake,
    Tilt,
    Fps,
    Distance,
    MaxSpeed,
    MaxTilt,
    Clock,
    SessionTime,
}

// ---- Color constants ---------------------------------------------------------

const BG: Color = Color::srgba(0.05, 0.05, 0.07, 0.75);
const COLOR_SPEED: Color = Color::WHITE;
const COLOR_SUB: Color = Color::srgb(0.85, 0.85, 0.85);
const COLOR_BRAKE_ON: Color = Color::srgb(0.95, 0.35, 0.25);

fn fps_color(fps: f32) -> Color {
    if fps >= 50.0 {
        Color::srgb(0.3, 0.95, 0.3)
    } else if fps >= 30.0 {
        Color::srgb(0.95, 0.85, 0.2)
    } else {
        Color::srgb(0.95, 0.2, 0.2)
    }
}

// ---- Startup: spawn UI tree -------------------------------------------------

fn spawn_hud(mut commands: Commands) {
    // ---- Full-screen transparent container (no background, passes clicks through) ----
    let root = commands
        .spawn((
            HudRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                ..default()
            },
        ))
        .id();

    // ---- Top-left info panel ------------------------------------------------
    let tl_panel = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(12.0),
                top: Val::Px(12.0),
                width: Val::Px(280.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(12.0)),
                ..default()
            },
            BackgroundColor(BG),
        ))
        .id();

    // Speed text (large)
    let speed_text = commands
        .spawn((
            HudText::Speed,
            Text::new("0 mph"),
            TextFont {
                font_size: 36.0,
                ..default()
            },
            TextColor(COLOR_SPEED),
        ))
        .id();

    // Throttle line
    let throttle_text = commands
        .spawn((
            HudText::Throttle,
            Text::new("THROTTLE: ..........  +0.00"),
            TextFont {
                font_size: 14.0,
                ..default()
            },
            TextColor(COLOR_SUB),
        ))
        .id();

    // Steer line
    let steer_text = commands
        .spawn((
            HudText::Steer,
            Text::new("STEER:    ..........  +0.00"),
            TextFont {
                font_size: 14.0,
                ..default()
            },
            TextColor(COLOR_SUB),
        ))
        .id();

    // Brake line
    let brake_text = commands
        .spawn((
            HudText::Brake,
            Text::new("BRAKE:    [off]"),
            TextFont {
                font_size: 14.0,
                ..default()
            },
            TextColor(COLOR_SUB),
        ))
        .id();

    // Tilt line
    let tilt_text = commands
        .spawn((
            HudText::Tilt,
            Text::new("TILT:      0.0 deg"),
            TextFont {
                font_size: 14.0,
                ..default()
            },
            TextColor(COLOR_SUB),
        ))
        .id();

    // Wire top-left panel children
    commands
        .entity(tl_panel)
        .add_children(&[speed_text, throttle_text, steer_text, brake_text, tilt_text]);

    // ---- Top-right stats panel ----------------------------------------------
    let tr_panel = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(12.0),
                top: Val::Px(12.0),
                width: Val::Px(280.0),
                height: Val::Px(140.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(12.0)),
                ..default()
            },
            BackgroundColor(BG),
        ))
        .id();

    let dist_text = commands
        .spawn((
            HudText::Distance,
            Text::new("DIST:  0.00 m"),
            TextFont { font_size: 14.0, ..default() },
            TextColor(COLOR_SUB),
        ))
        .id();

    let maxspd_text = commands
        .spawn((
            HudText::MaxSpeed,
            Text::new("MAX SPD:  0.0 mph"),
            TextFont { font_size: 14.0, ..default() },
            TextColor(COLOR_SUB),
        ))
        .id();

    let maxtilt_text = commands
        .spawn((
            HudText::MaxTilt,
            Text::new("MAX TILT:  0.0 deg"),
            TextFont { font_size: 14.0, ..default() },
            TextColor(COLOR_SUB),
        ))
        .id();

    let clock_text = commands
        .spawn((
            HudText::Clock,
            Text::new("TIME:  00:00"),
            TextFont { font_size: 14.0, ..default() },
            TextColor(COLOR_SUB),
        ))
        .id();

    let session_text = commands
        .spawn((
            HudText::SessionTime,
            Text::new("SESSION:  00:00"),
            TextFont { font_size: 14.0, ..default() },
            TextColor(COLOR_SUB),
        ))
        .id();

    commands
        .entity(tr_panel)
        .add_children(&[dist_text, maxspd_text, maxtilt_text, clock_text, session_text]);

    // ---- Bottom-right FPS panel ---------------------------------------------
    let br_panel = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(12.0),
                bottom: Val::Px(12.0),
                width: Val::Px(120.0),
                height: Val::Px(40.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(BG),
        ))
        .id();

    let fps_text = commands
        .spawn((
            HudText::Fps,
            Text::new("FPS: --"),
            TextFont {
                font_size: 14.0,
                ..default()
            },
            TextColor(COLOR_SUB),
        ))
        .id();

    commands.entity(br_panel).add_children(&[fps_text]);

    // Attach panels to root — all three are children so H-toggle hides them all.
    commands
        .entity(root)
        .add_children(&[tl_panel, tr_panel, br_panel]);
}

// ---- Session-stats update system --------------------------------------------

fn update_session_stats(
    vehicle: Res<VehicleRoot>,
    chassis_q: Query<(&Transform, &LinearVelocity), With<Chassis>>,
    mut stats: ResMut<SessionStats>,
    time: Res<Time>,
) {
    let Ok((transform, lin_vel)) = chassis_q.get(vehicle.chassis) else {
        return;
    };

    let dt = time.delta_secs();
    stats.elapsed_s += dt;

    let speed_mps = lin_vel.0.length();
    if speed_mps > stats.max_speed_mps {
        stats.max_speed_mps = speed_mps;
    }

    let chassis_up = transform.up();
    let dot = chassis_up.dot(Vec3::Y).clamp(-1.0, 1.0);
    let tilt_deg = dot.acos().to_degrees();
    if tilt_deg > stats.max_tilt_deg {
        stats.max_tilt_deg = tilt_deg;
    }

    // Accumulate horizontal-plane distance; gate at 5 m per tick to skip teleports.
    let pos_xz = Vec2::new(transform.translation.x, transform.translation.z);
    if let Some(prev) = stats.last_pos {
        let prev_xz = Vec2::new(prev.x, prev.z);
        let delta = pos_xz.distance(prev_xz);
        if delta <= 5.0 {
            stats.distance_m += delta;
        }
    }
    stats.last_pos = Some(transform.translation);
}

// ---- Update system ----------------------------------------------------------

fn update_hud(
    vehicle: Res<VehicleRoot>,
    input: Res<DriveInput>,
    chassis_q: Query<(&Transform, &LinearVelocity), With<Chassis>>,
    diagnostics: Res<DiagnosticsStore>,
    stats: Res<SessionStats>,
    tod: Res<TimeOfDay>,
    mut texts: Query<(&HudText, &mut Text, &mut TextColor)>,
) {
    let Ok((transform, lin_vel)) = chassis_q.get(vehicle.chassis) else {
        return;
    };

    // Speed in mph  (m/s × 2.237)
    let speed_mps = lin_vel.0.length();
    let speed_mph = speed_mps * 2.237;

    // Tilt: angle between chassis up-vector and world-up (Y)
    let chassis_up = transform.up();
    let dot = chassis_up.dot(Vec3::Y).clamp(-1.0, 1.0);
    let tilt_deg = dot.acos().to_degrees();

    // FPS
    let fps_val = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|d| d.smoothed())
        .unwrap_or(0.0) as f32;

    // Time-of-day clock: t in [0, 1) → HH:MM 24-hour
    let total_minutes = (tod.t * 24.0 * 60.0) as u32;
    let tod_h = total_minutes / 60;
    let tod_m = total_minutes % 60;
    let clock_str = if tod.paused {
        format!("TIME:  {:02}:{:02} (paused)", tod_h, tod_m)
    } else {
        format!("TIME:  {:02}:{:02}", tod_h, tod_m)
    };

    // Session time MM:SS
    let elapsed_total_s = stats.elapsed_s as u32;
    let sess_m = elapsed_total_s / 60;
    let sess_s = elapsed_total_s % 60;

    // Distance formatting: show km when >= 1000 m
    let dist_str = if stats.distance_m >= 1000.0 {
        format!("DIST:  {:.3} km", stats.distance_m / 1000.0)
    } else {
        format!("DIST:  {:.1} m", stats.distance_m)
    };

    for (label, mut text, mut color) in &mut texts {
        match label {
            HudText::Speed => {
                text.0 = format!("{:.0} mph", speed_mph);
            }
            HudText::Throttle => {
                text.0 = format!("THROTTLE: {}  {:+.2}", bar10(input.drive), input.drive);
            }
            HudText::Steer => {
                text.0 = format!("STEER:    {}  {:+.2}", centered_bar10(input.steer), input.steer);
            }
            HudText::Brake => {
                if input.brake {
                    text.0 = "BRAKE:    [ON]".to_string();
                    color.0 = COLOR_BRAKE_ON;
                } else {
                    text.0 = "BRAKE:    [off]".to_string();
                    color.0 = COLOR_SUB;
                }
            }
            HudText::Tilt => {
                text.0 = format!("TILT:     {:.1} deg", tilt_deg);
            }
            HudText::Fps => {
                text.0 = format!("FPS: {:.1}", fps_val);
                color.0 = fps_color(fps_val);
            }
            HudText::Distance => {
                text.0 = dist_str.clone();
            }
            HudText::MaxSpeed => {
                text.0 = format!("MAX SPD:  {:.1} mph", stats.max_speed_mps * 2.237);
            }
            HudText::MaxTilt => {
                text.0 = format!("MAX TILT: {:.1} deg", stats.max_tilt_deg);
            }
            HudText::Clock => {
                text.0 = clock_str.clone();
            }
            HudText::SessionTime => {
                text.0 = format!("SESSION:  {:02}:{:02}", sess_m, sess_s);
            }
        }
    }
}

// ---- Toggle system ----------------------------------------------------------

fn toggle_hud(
    keys: Res<ButtonInput<KeyCode>>,
    mut visible: ResMut<HudVisible>,
    mut root_q: Query<&mut Node, With<HudRoot>>,
) {
    // Shift+H: H alone is used by the tutorial step advancer, so the HUD
    // toggle is gated by Shift to avoid double-firing during onboarding.
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if shift && keys.just_pressed(KeyCode::KeyH) {
        visible.0 = !visible.0;
        for mut node in &mut root_q {
            node.display = if visible.0 {
                Display::Flex
            } else {
                Display::None
            };
        }
    }
}

// ---- Bar helpers ------------------------------------------------------------

/// 10-char bar for a value in [-1, 1] treated as fill from left.
/// Positive: fills from left. Negative: fills from left for absolute value.
fn bar10(val: f32) -> String {
    let filled = (val.abs() * 10.0).round() as usize;
    let filled = filled.min(10);
    "\u{2588}".repeat(filled) + &"\u{2591}".repeat(10 - filled)
}

/// 10-char bar centred at position 5. Positive fills right of centre, negative fills left.
fn centered_bar10(val: f32) -> String {
    let centre = 5usize;
    let offset = (val.abs() * 5.0).round() as usize;
    let offset = offset.min(5);

    let mut chars = ['\u{2591}'; 10];

    if val >= 0.0 {
        // fill positions [centre .. centre+offset)
        for i in centre..(centre + offset).min(10) {
            chars[i] = '\u{2588}';
        }
    } else {
        // fill positions [(centre-offset) .. centre)
        let start = centre.saturating_sub(offset);
        for i in start..centre {
            chars[i] = '\u{2588}';
        }
    }

    chars.iter().collect()
}
