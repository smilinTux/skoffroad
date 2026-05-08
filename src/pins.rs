// Mini-map overlay pins: banner arches, waypoint, and course target.
//
// Spawns absolutely-positioned UI nodes over the same screen area as the
// minimap panel (bottom-left corner). Positions are computed by mirroring
// minimap.rs WORLD_EXTENT / MAP_PX constants and the same XZ→pixel formula.
//
// This file must NOT modify minimap.rs or any other source file.

use bevy::prelude::*;

use crate::compass::Waypoint;
use crate::course::CourseState;

// ---- Plugin -----------------------------------------------------------------

pub struct PinsPlugin;

impl Plugin for PinsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_banner_pins)
           .add_systems(Update, (update_waypoint_pin, update_course_pin));
    }
}

// ---- Constants (mirror minimap.rs) -----------------------------------------

const MAP_PX: f32       = 200.0;   // rendered size in screen pixels
const WORLD_EXTENT: f32 = 200.0;   // terrain spans [-100, +100] on X and Z
const MAP_LEFT: f32     = 12.0;    // minimap panel left edge offset
const MAP_BOTTOM: f32   = 12.0;    // minimap panel bottom edge offset
const PIN_SIZE: f32     = 10.0;    // banner / waypoint pin diameter in pixels
const TARGET_PIN: f32   = 14.0;    // course-target pin diameter in pixels

// ---- Banner definitions -----------------------------------------------------

#[derive(Component, Clone, Copy)]
enum BannerKind {
    Start,
    Checkpoint,
    Finish,
}

/// XZ positions matching banners.rs ARCHES order.
const BANNERS: [([f32; 2], BannerKind); 4] = [
    ([ 5.0,  -5.0], BannerKind::Start),
    ([40.0,  30.0], BannerKind::Checkpoint),
    ([-40.0, 50.0], BannerKind::Checkpoint),
    ([60.0, -40.0], BannerKind::Finish),
];

fn banner_color(kind: BannerKind) -> Color {
    match kind {
        BannerKind::Start      => Color::srgb(0.2, 1.0, 0.2),
        BannerKind::Checkpoint => Color::srgb(1.0, 0.9, 0.2),
        BannerKind::Finish     => Color::srgb(1.0, 0.2, 0.2),
    }
}

// ---- Dynamic pin markers ----------------------------------------------------

/// The waypoint pin (magenta).
#[derive(Component)]
struct WaypointPin;

/// The course-target pin (cyan).
#[derive(Component)]
struct CoursePin;

// ---- Coordinate conversion --------------------------------------------------

/// Convert world XZ to absolute screen-space left/bottom pixel offsets so that
/// a UI node with PositionType::Absolute lands on top of the minimap panel.
///
/// The minimap panel sits at left=12, bottom=12 and is MAP_PX×MAP_PX pixels.
/// World X in [-100, 100] → left within the panel [0, MAP_PX].
/// World Z in [-100, 100] → bottom within the panel [MAP_PX, 0]
///   (world +Z maps to the top of the image, i.e. lower bottom value — same
///    orientation used by minimap.rs update_minimap which uses `top = pz`).
fn world_xz_to_map_px(x: f32, z: f32) -> (f32, f32) {
    let nx = (x / WORLD_EXTENT + 0.5) * MAP_PX;
    let nz = (z / WORLD_EXTENT + 0.5) * MAP_PX;
    // In minimap.rs: node.top = pz  →  screen-top grows with world-Z.
    // Translating to bottom anchor: bottom = MAP_PX - nz.
    let left   = MAP_LEFT   + nx - PIN_SIZE * 0.5;
    let bottom = MAP_BOTTOM + (MAP_PX - nz) - PIN_SIZE * 0.5;
    (left, bottom)
}

/// Same as world_xz_to_map_px but for a pin of custom size.
fn world_xz_to_map_px_sized(x: f32, z: f32, size: f32) -> (f32, f32) {
    let nx = (x / WORLD_EXTENT + 0.5) * MAP_PX;
    let nz = (z / WORLD_EXTENT + 0.5) * MAP_PX;
    let left   = MAP_LEFT   + nx - size * 0.5;
    let bottom = MAP_BOTTOM + (MAP_PX - nz) - size * 0.5;
    (left, bottom)
}

// ---- Startup: spawn banner pins ---------------------------------------------

fn spawn_banner_pins(mut commands: Commands) {
    for ([x, z], kind) in &BANNERS {
        let (left, bottom) = world_xz_to_map_px(*x, *z);
        commands.spawn((
            *kind,
            Node {
                position_type: PositionType::Absolute,
                left:   Val::Px(left),
                bottom: Val::Px(bottom),
                width:  Val::Px(PIN_SIZE),
                height: Val::Px(PIN_SIZE),
                ..default()
            },
            BackgroundColor(banner_color(*kind)),
            ZIndex(15),
        ));
    }

    // Waypoint pin — hidden until Waypoint resource exists.
    commands.spawn((
        WaypointPin,
        Node {
            position_type: PositionType::Absolute,
            left:    Val::Px(0.0),
            bottom:  Val::Px(0.0),
            width:   Val::Px(PIN_SIZE),
            height:  Val::Px(PIN_SIZE),
            display: Display::None,
            ..default()
        },
        BackgroundColor(Color::srgb(1.0, 0.2, 1.0)),
        ZIndex(16),
    ));

    // Course-target pin — hidden until CourseState has a target.
    commands.spawn((
        CoursePin,
        Node {
            position_type: PositionType::Absolute,
            left:    Val::Px(0.0),
            bottom:  Val::Px(0.0),
            width:   Val::Px(TARGET_PIN),
            height:  Val::Px(TARGET_PIN),
            display: Display::None,
            ..default()
        },
        BackgroundColor(Color::srgb(0.2, 1.0, 1.0)),
        ZIndex(17),
    ));
}

// ---- Per-frame updates ------------------------------------------------------

fn update_waypoint_pin(
    waypoint: Option<Res<Waypoint>>,
    mut pin_q: Query<&mut Node, With<WaypointPin>>,
) {
    let Ok(mut node) = pin_q.single_mut() else { return };

    match waypoint {
        Some(wpt) => {
            let (left, bottom) =
                world_xz_to_map_px(wpt.position.x, wpt.position.z);
            node.display = Display::Flex;
            node.left    = Val::Px(left);
            node.bottom  = Val::Px(bottom);
        }
        None => {
            node.display = Display::None;
        }
    }
}

fn update_course_pin(
    course: Option<Res<CourseState>>,
    mut pin_q: Query<&mut Node, With<CoursePin>>,
) {
    let Ok(mut node) = pin_q.single_mut() else { return };

    let target = course.and_then(|cs| cs.current_target);

    match target {
        Some(pos) => {
            let (left, bottom) =
                world_xz_to_map_px_sized(pos.x, pos.z, TARGET_PIN);
            node.display = Display::Flex;
            node.left    = Val::Px(left);
            node.bottom  = Val::Px(bottom);
        }
        None => {
            node.display = Display::None;
        }
    }
}
