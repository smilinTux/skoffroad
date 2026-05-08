// compass_hud.rs — Sprint 26
//
// Top-center compass strip showing N / NE / E / SE / S / SW / W / NW marks.
// The strip slides horizontally based on chassis yaw so the cardinal direction
// directly forward is centered.  Distinct from compass.rs (waypoint pointer).
//
// Layout:
//   Panel : 320 × 28 px, horizontally centred, top: 60 px (below course panel)
//   Strip : 1024 px wide child node, left offset driven each frame by yaw
//   Pointer: 2 px wide triangle indicator at panel center

use bevy::prelude::*;
use std::f32::consts::TAU;

use crate::vehicle::{Chassis, VehicleRoot};

// ---------------------------------------------------------------------------
// Public plugin
// ---------------------------------------------------------------------------

pub struct CompassHudPlugin;

impl Plugin for CompassHudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_compass_hud)
            .add_systems(
                Update,
                update_compass_strip.run_if(resource_exists::<VehicleRoot>),
            );
    }
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const PANEL_W: f32  = 320.0;
const PANEL_H: f32  = 28.0;
const STRIP_W: f32  = 1024.0;

// Background colour for the panel
const PANEL_BG: Color = Color::srgba(0.05, 0.05, 0.07, 0.75);

// Label colours
const CARDINAL_COLOR: Color    = Color::srgb(1.0, 1.0, 1.0);         // white, 14 pt
const SUBCARDINAL_COLOR: Color = Color::srgb(0.7, 0.7, 0.7);         // grey,  11 pt
const POINTER_COLOR: Color     = Color::srgb(0.95, 0.85, 0.15);       // gold triangle indicator

// Direction definitions: (label, fractional position in [0,1), is_cardinal)
const DIRS: &[(&str, f32, bool)] = &[
    ("N",  0.000, true),
    ("NE", 0.125, false),
    ("E",  0.250, true),
    ("SE", 0.375, false),
    ("S",  0.500, true),
    ("SW", 0.625, false),
    ("W",  0.750, true),
    ("NW", 0.875, false),
];

// ---------------------------------------------------------------------------
// Components
// ---------------------------------------------------------------------------

#[derive(Component)]
struct CompassHudPanel;

#[derive(Component)]
struct CompassHudStrip;

// ---------------------------------------------------------------------------
// Startup: spawn the panel, inner strip, labels, and pointer
// ---------------------------------------------------------------------------

fn spawn_compass_hud(mut commands: Commands) {
    // --- Root panel (clipping container) ---
    let panel = commands
        .spawn((
            CompassHudPanel,
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(60.0),
                // Horizontally centered via auto margins
                left: Val::Auto,
                right: Val::Auto,
                width: Val::Px(PANEL_W),
                height: Val::Px(PANEL_H),
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(PANEL_BG),
            ZIndex(25),
        ))
        .id();

    // --- Strip (wider than panel; we slide it left/right each frame) ---
    let strip = commands
        .spawn((
            CompassHudStrip,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                width: Val::Px(STRIP_W),
                height: Val::Px(PANEL_H),
                ..default()
            },
        ))
        .id();

    // Populate labels inside the strip
    for &(label, frac, is_cardinal) in DIRS {
        let x_center = frac * STRIP_W;
        // Offset slightly left so text is centered on tick position
        let text_offset = if is_cardinal { -5.0 } else { -7.0 };
        let font_size = if is_cardinal { 14.0 } else { 11.0 };
        let color = if is_cardinal { CARDINAL_COLOR } else { SUBCARDINAL_COLOR };

        let lbl = commands
            .spawn((
                Text::new(label),
                TextFont { font_size, ..default() },
                TextColor(color),
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(x_center + text_offset),
                    top: Val::Px(4.0),
                    ..default()
                },
            ))
            .id();
        commands.entity(strip).add_child(lbl);

        // Small tick mark below label
        let tick = commands
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(x_center - 0.5),
                    top: Val::Px(PANEL_H - 6.0),
                    width: Val::Px(1.0),
                    height: Val::Px(5.0),
                    ..default()
                },
                BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.30)),
            ))
            .id();
        commands.entity(strip).add_child(tick);
    }

    commands.entity(panel).add_child(strip);

    // --- Center pointer triangle (a thin colored bar at mid-panel) ---
    // Represented as a 3 px wide × 8 px tall rectangle pinned at center-top
    let pointer = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(PANEL_W / 2.0 - 1.5),
                top: Val::Px(PANEL_H - 8.0),
                width: Val::Px(3.0),
                height: Val::Px(8.0),
                ..default()
            },
            BackgroundColor(POINTER_COLOR),
            ZIndex(26),
        ))
        .id();

    commands.entity(panel).add_child(pointer);
}

// ---------------------------------------------------------------------------
// Update: slide the strip to reflect chassis yaw
// ---------------------------------------------------------------------------

fn update_compass_strip(
    vehicle: Res<VehicleRoot>,
    chassis_q: Query<&Transform, With<Chassis>>,
    mut strip_q: Query<&mut Node, With<CompassHudStrip>>,
) {
    let Ok(tf) = chassis_q.get(vehicle.chassis) else { return };

    // Forward direction in world space.  For a default Bevy/Avian spawn
    // facing -Z, forward() gives (0, 0, -1) at yaw=0.
    // We define yaw=0 as "facing North (-Z)".  Rotating CW increases yaw,
    // so East (+X) = π/2, South (+Z) = π, West (-X) = 3π/2.
    let fwd = tf.forward();
    // atan2(x, -z):  x=sin(yaw), -z=cos(yaw)  →  yaw in (-π, π]
    let yaw = fwd.x.atan2(-fwd.z);

    // Normalise to [0, 1) where 0 = N, 0.25 = E, 0.5 = S, 0.75 = W
    let yaw_norm = (yaw / TAU).rem_euclid(1.0);

    // Strip left offset: place yaw_norm's position under PANEL_W / 2
    //   strip_left + yaw_norm * STRIP_W  ==  PANEL_W / 2
    //   strip_left = PANEL_W / 2  -  yaw_norm * STRIP_W
    let strip_left = PANEL_W / 2.0 - yaw_norm * STRIP_W;

    for mut node in &mut strip_q {
        node.left = Val::Px(strip_left);
    }
}
