// Speedometer gauge for skoffroad.
//
// Design choice: horizontal progress-bar gauge with a needle tick.
// A circular dial gauge requires drawing arcs which Bevy's native UI does not
// support.  Instead we render a clean numeric + bar widget that fits the flat
// HUD aesthetic without external crates.
//
// Sprint 86: added a needle tick marker at the right edge of the fill bar so
// the speedo reads as a real moving indicator.  The needle is a thin 4x18 px
// Node absolutely positioned inside the bar-background container at
// `left: Val::Percent(frac * 100.0)`, updated every frame from the chassis
// LinearVelocity magnitude.  No per-frame allocation — only Val mutation.
//
// Layout (200x118 px, bottom-right above the fuel gauge):
//   Row 1: large bold speed value (48 pt) + "mph" label (14 pt)
//   Row 2: progress bar (10 px high) with needle tick (18 px tall, 4 px wide)
//
// Toggle: G key.  Default: visible.

use bevy::prelude::*;
use avian3d::prelude::LinearVelocity;

use crate::vehicle::{Chassis, VehicleRoot};

// ---- Constants ---------------------------------------------------------------

const MAX_MPH: f32 = 60.0;

// ---- Plugin ------------------------------------------------------------------

pub struct GaugePlugin;

impl Plugin for GaugePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(GaugeVisible(true))
           .add_systems(Startup, spawn_gauge)
           .add_systems(
               Update,
               (update_gauge, toggle_gauge)
                   .run_if(resource_exists::<VehicleRoot>),
           );
    }
}

// ---- Resources & components --------------------------------------------------

#[derive(Resource)]
struct GaugeVisible(bool);

#[derive(Component)]
struct GaugeRoot;

#[derive(Component)]
struct GaugeSpeedText;

#[derive(Component)]
struct GaugeBarFill;

/// Needle tick: a thin vertical bar that rides the right edge of the fill bar.
#[derive(Component)]
struct GaugeNeedle;

// ---- Startup: build HUD panel ------------------------------------------------

fn spawn_gauge(mut commands: Commands) {
    // Outer panel: bottom-right, above fuel gauge (fuel sits at bottom 64 px,
    // height 48 px; leave 8 px gap -> 64 + 48 + 8 = 120 px from bottom).
    let panel = commands.spawn((
        GaugeRoot,
        Node {
            position_type: PositionType::Absolute,
            right:           Val::Px(12.0),
            bottom:          Val::Px(120.0),
            width:           Val::Px(200.0),
            height:          Val::Px(118.0),
            flex_direction:  FlexDirection::Column,
            justify_content: JustifyContent::SpaceBetween,
            padding:         UiRect { left: Val::Px(8.0), right: Val::Px(8.0),
                                      top: Val::Px(8.0),  bottom: Val::Px(8.0) },
            row_gap:         Val::Px(6.0),
            display:         Display::Flex,
            ..default()
        },
        BackgroundColor(Color::srgba(0.05, 0.05, 0.07, 0.80)),
    )).id();

    // -- Top row: numeric value + "mph" label ----------------------------------
    let top_row = commands.spawn(Node {
        flex_direction:  FlexDirection::Row,
        align_items:     AlignItems::FlexEnd,
        column_gap:      Val::Px(4.0),
        ..default()
    }).id();

    let speed_text = commands.spawn((
        GaugeSpeedText,
        Text::new("0"),
        TextFont { font_size: 48.0, ..default() },
        TextColor(Color::WHITE),
    )).id();

    let mph_label = commands.spawn((
        Text::new("mph"),
        TextFont { font_size: 14.0, ..default() },
        TextColor(Color::srgb(0.75, 0.75, 0.75)),
    )).id();

    commands.entity(top_row).add_children(&[speed_text, mph_label]);

    // -- Bottom section: bar background container + fill + needle --------------
    //
    // The bar_bg uses position_type Relative (default) so its children with
    // PositionType::Absolute are anchored to it.  The needle is a sibling of
    // the fill bar, also a child of bar_bg, so it overlays the bar cleanly.

    let bar_bg = commands.spawn((
        Node {
            width:         Val::Percent(100.0),
            height:        Val::Px(18.0), // tall enough to show the needle
            position_type: PositionType::Relative,
            overflow:      Overflow::clip(),
            ..default()
        },
        BackgroundColor(Color::srgba(0.15, 0.15, 0.15, 1.0)),
    )).id();

    let bar_fill = commands.spawn((
        GaugeBarFill,
        Node {
            position_type: PositionType::Absolute,
            left:   Val::Px(0.0),
            top:    Val::Px(0.0),
            width:  Val::Percent(0.0), // driven by update_gauge
            height: Val::Percent(100.0),
            ..default()
        },
        BackgroundColor(Color::srgb(0.2, 0.85, 0.3)),
    )).id();

    // Needle tick: a 4-px wide, full-height bright bar at the fill edge.
    // Positioned absolutely at left = frac * bar_width, updated every frame.
    let needle = commands.spawn((
        GaugeNeedle,
        Node {
            position_type: PositionType::Absolute,
            left:   Val::Percent(0.0), // driven by update_gauge
            top:    Val::Px(0.0),
            width:  Val::Px(4.0),
            height: Val::Percent(100.0),
            ..default()
        },
        BackgroundColor(Color::WHITE),
    )).id();

    commands.entity(bar_bg).add_children(&[bar_fill, needle]);
    commands.entity(panel).add_children(&[top_row, bar_bg]);
}

// ---- Update: read chassis velocity and refresh widgets -----------------------

fn update_gauge(
    vehicle:    Res<VehicleRoot>,
    chassis_q:  Query<&LinearVelocity, With<Chassis>>,
    mut text_q:   Query<&mut Text,               With<GaugeSpeedText>>,
    mut bar_q:    Query<(&mut Node, &mut BackgroundColor), With<GaugeBarFill>>,
    mut needle_q: Query<&mut Node,               (With<GaugeNeedle>, Without<GaugeBarFill>)>,
) {
    let Ok(lin_vel) = chassis_q.get(vehicle.chassis) else { return };

    let speed_mps = Vec3::new(lin_vel.x, lin_vel.y, lin_vel.z).length();
    let speed_mph = speed_mps * 2.237_f32;
    let frac      = (speed_mph / MAX_MPH).clamp(0.0, 1.0);

    // Colour buckets: green (<40%), yellow (40-75%), red (>75%).
    let bar_color = if frac < 0.40 {
        Color::srgb(0.2, 0.85, 0.3)
    } else if frac < 0.75 {
        Color::srgb(0.95, 0.85, 0.2)
    } else {
        Color::srgb(0.95, 0.2, 0.2)
    };

    for mut text in &mut text_q {
        text.0 = format!("{:.0}", speed_mph);
    }

    for (mut node, mut bg) in &mut bar_q {
        node.width = Val::Percent(frac * 100.0);
        bg.0       = bar_color;
    }

    // Drive the needle to the right edge of the fill bar.
    // Clamp so the needle doesn't extend past the right edge of the container
    // at 100% (the 4-px needle width means we stop at 96%).
    let needle_pct = (frac * 100.0).clamp(0.0, 96.0);
    for mut node in &mut needle_q {
        node.left = Val::Percent(needle_pct);
    }
}

// ---- Toggle: G key -----------------------------------------------------------

fn toggle_gauge(
    keys:    Res<ButtonInput<KeyCode>>,
    mut vis: ResMut<GaugeVisible>,
    mut root_q: Query<&mut Node, With<GaugeRoot>>,
) {
    if keys.just_pressed(KeyCode::KeyG) {
        vis.0 = !vis.0;
        let display = if vis.0 { Display::Flex } else { Display::None };
        for mut node in &mut root_q {
            node.display = display;
        }
    }
}
