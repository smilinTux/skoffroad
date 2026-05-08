// Wheelie counter for skoffroad.
//
// Detects when the chassis is balanced on its rear wheels (front wheels off
// ground while moving forward) and tracks duration / maximum wheelie length.
//
// HUD layout:
//   Active banner : top-centre at 130 px   "WHEELIE! 1.34 s"  (amber)
//   Post popup    : same slot, 3 s after end "WHEELIE: 2.12 s (best 3.45)" (green / amber)

use bevy::prelude::*;
use avian3d::prelude::LinearVelocity;

use crate::vehicle::{Chassis, VehicleRoot, Wheel};

// ---- Constants ---------------------------------------------------------------

const MIN_WHEELIE_SPEED: f32  = 1.0;  // m/s longitudinal speed required
const MIN_WHEELIE_DURATION: f32 = 0.5; // s — shorter ones are filtered as noise
const POPUP_DURATION: f32     = 3.0;  // s the post-wheelie popup stays on screen
const BANNER_W: f32           = 320.0;
const BANNER_TOP: f32         = 130.0;

// Colors
const AMBER: Color  = Color::srgb(1.0, 0.70, 0.0);
const GREEN: Color  = Color::srgb(0.2, 0.95, 0.35);

// ---- Public resource ---------------------------------------------------------

/// Running wheelie statistics accumulated since session start.
#[derive(Resource, Default)]
pub struct WheelieStats {
    /// Whether a wheelie is currently in progress.
    pub in_wheelie: bool,
    /// Seconds the current wheelie has lasted (resets when wheelie ends).
    pub current_wheelie_s: f32,
    /// Cumulative wheelie time across the session.
    pub total_wheelie_s: f32,
    /// Longest single wheelie this session.
    pub longest_wheelie_s: f32,
    /// Number of wheelies that lasted > 0.5 s.
    pub wheelie_count: u32,
}

// ---- Private state -----------------------------------------------------------

/// Tracks the post-wheelie popup state.
#[derive(Resource, Default)]
struct PopupState {
    showing: bool,
    timer_s: f32,
    duration_s: f32,
    best_s: f32,
    is_new_best: bool,
}

// ---- Components --------------------------------------------------------------

#[derive(Component)] struct WheelieHudRoot;
#[derive(Component)] struct WheelieHudText;

// ---- Plugin ------------------------------------------------------------------

pub struct WheelieCounterPlugin;

impl Plugin for WheelieCounterPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WheelieStats>()
           .init_resource::<PopupState>()
           .add_systems(Startup, spawn_wheelie_hud)
           .add_systems(Update, (
               detect_wheelie.run_if(resource_exists::<crate::vehicle::VehicleRoot>),
               update_wheelie_hud,
           ));
    }
}

// ---- Startup: spawn HUD ------------------------------------------------------

fn spawn_wheelie_hud(mut commands: Commands) {
    let root = commands.spawn((
        WheelieHudRoot,
        Node {
            position_type:   PositionType::Absolute,
            left:            Val::Percent(50.0),
            top:             Val::Px(BANNER_TOP),
            margin:          UiRect { left: Val::Px(-(BANNER_W / 2.0)), ..default() },
            width:           Val::Px(BANNER_W),
            height:          Val::Px(38.0),
            justify_content: JustifyContent::Center,
            align_items:     AlignItems::Center,
            display:         Display::None,
            ..default()
        },
        BackgroundColor(Color::srgba(0.10, 0.06, 0.0, 0.88)),
        Outline {
            width:  Val::Px(1.5),
            offset: Val::Px(0.0),
            color:  AMBER,
        },
    )).id();

    let label = commands.spawn((
        WheelieHudText,
        Text::new("WHEELIE! 0.00 s"),
        TextFont { font_size: 18.0, ..default() },
        TextColor(AMBER),
    )).id();

    commands.entity(root).add_child(label);
}

// ---- Detection ---------------------------------------------------------------

fn detect_wheelie(
    vehicle:    Res<VehicleRoot>,
    chassis_q:  Query<&LinearVelocity, With<Chassis>>,
    wheel_q:    Query<&Wheel>,
    time:       Res<Time>,
    mut stats:  ResMut<WheelieStats>,
    mut popup:  ResMut<PopupState>,
) {
    // Gather ground contact for each wheel by index.
    let mut grounded = [false; 4];
    for wheel in wheel_q.iter() {
        if wheel.index < 4 {
            grounded[wheel.index] = wheel.is_grounded;
        }
    }

    let front_grounded = grounded[0] || grounded[1];
    let rear_grounded  = grounded[2] || grounded[3];

    // Longitudinal speed from chassis linear velocity (world-space magnitude is
    // fine here — we only care that the vehicle is moving, not reversing).
    let speed_mps = if let Ok(lv) = chassis_q.get(vehicle.chassis) {
        Vec3::new(lv.x, lv.y, lv.z).length()
    } else {
        0.0
    };

    let wheelie_condition = !front_grounded && rear_grounded && speed_mps > MIN_WHEELIE_SPEED;

    let dt = time.delta_secs();

    if wheelie_condition {
        if !stats.in_wheelie {
            // Wheelie just started.
            stats.in_wheelie = true;
            stats.current_wheelie_s = 0.0;
        }
        stats.current_wheelie_s += dt;
        stats.total_wheelie_s   += dt;
    } else if stats.in_wheelie {
        // Wheelie just ended.
        let duration = stats.current_wheelie_s;
        stats.in_wheelie        = false;

        if duration >= MIN_WHEELIE_DURATION {
            let is_new_best = duration > stats.longest_wheelie_s;
            if is_new_best {
                stats.longest_wheelie_s = duration;
            }
            stats.wheelie_count += 1;

            // Trigger popup.
            popup.showing    = true;
            popup.timer_s    = 0.0;
            popup.duration_s = duration;
            popup.best_s     = stats.longest_wheelie_s;
            popup.is_new_best = is_new_best;
        }

        stats.current_wheelie_s = 0.0;
    }

    // Advance popup timer.
    if popup.showing {
        popup.timer_s += dt;
        if popup.timer_s >= POPUP_DURATION {
            popup.showing = false;
        }
    }
}

// ---- HUD update --------------------------------------------------------------

fn update_wheelie_hud(
    stats:     Res<WheelieStats>,
    popup:     Res<PopupState>,
    mut root_q: Query<(&mut Node, &mut BackgroundColor, &mut Outline), With<WheelieHudRoot>>,
    mut text_q: Query<(&mut Text, &mut TextColor), With<WheelieHudText>>,
) {
    let (display, text_str, fg_color, outline_color) = if stats.in_wheelie {
        let s = format!("WHEELIE! {:.2} s", stats.current_wheelie_s);
        (Display::Flex, s, AMBER, AMBER)
    } else if popup.showing {
        let s = format!(
            "WHEELIE: {:.2} s  (best {:.2})",
            popup.duration_s, popup.best_s
        );
        let color = if popup.is_new_best { GREEN } else { AMBER };
        (Display::Flex, s, color, color)
    } else {
        (Display::None, String::new(), AMBER, AMBER)
    };

    for (mut node, mut bg, mut outline) in &mut root_q {
        node.display = display;
        // Tint background for popup vs active.
        if stats.in_wheelie {
            bg.0 = Color::srgba(0.10, 0.06, 0.0, 0.88);
        } else if popup.showing && popup.is_new_best {
            bg.0 = Color::srgba(0.0, 0.12, 0.04, 0.88);
        } else {
            bg.0 = Color::srgba(0.10, 0.06, 0.0, 0.88);
        }
        outline.color = outline_color;
    }

    for (mut text, mut color) in &mut text_q {
        if !text_str.is_empty() {
            text.0 = text_str.clone();
        }
        color.0 = fg_color;
    }
}
