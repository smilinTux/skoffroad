// Boost / nitrous system for SandK Offroad.
//
// Hold Left Shift to apply a forward thrust force on the chassis.  The boost
// meter holds 3 s of charge, drains at 1 s/s while active, and recharges at
// 0.5 s/s when released (10 s full recharge from empty).
//
// Force: 6000 N forward — more than doubles the base 2800 N drive force for
// a dramatic kick without making the vehicle uncontrollable.
//
// HUD: small panel bottom-right (above the speedometer gauge).
//   Title: BOOST  |  numeric: "2.4 s / 3.0 s"
//   Horizontal bar: blue (full) → green (half) → red (low/active).

use bevy::prelude::*;
use avian3d::prelude::*;

use crate::vehicle::{Chassis, VehicleRoot};

// ---- Constant ----------------------------------------------------------------

const BOOST_FORCE: f32 = 6000.0;

// ---- Public resource ---------------------------------------------------------

#[derive(Resource)]
pub struct BoostState {
    /// Available boost charge in seconds.
    pub charge_s: f32,
    /// Maximum charge capacity in seconds.
    pub max_s: f32,
    /// True while Left Shift is held and charge remains.
    pub active: bool,
}

impl Default for BoostState {
    fn default() -> Self {
        Self { charge_s: 3.0, max_s: 3.0, active: false }
    }
}

// ---- HUD components ----------------------------------------------------------

#[derive(Component)] struct BoostHudRoot;
#[derive(Component)] struct BoostBarFill;
#[derive(Component)] struct BoostNumericText;

// ---- Plugin ------------------------------------------------------------------

pub struct BoostPlugin;

impl Plugin for BoostPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BoostState>()
           .add_systems(Startup, spawn_boost_hud)
           .add_systems(Update, (
               update_boost_state
                   .run_if(resource_exists::<VehicleRoot>),
               update_boost_hud,
           ))
           .add_systems(
               PhysicsSchedule,
               apply_boost_force
                   .after(PhysicsStepSystems::NarrowPhase)
                   .before(PhysicsStepSystems::Solver),
           );
    }
}

// ---- Startup: build HUD panel ------------------------------------------------

fn spawn_boost_hud(mut commands: Commands) {
    // Panel: bottom-right, above the speedometer gauge (at ~230 px from bottom).
    let panel = commands.spawn((
        BoostHudRoot,
        Node {
            position_type: PositionType::Absolute,
            right:          Val::Px(12.0),
            bottom:         Val::Px(230.0),
            width:          Val::Px(200.0),
            height:         Val::Px(36.0),
            flex_direction: FlexDirection::Column,
            padding: UiRect {
                left:   Val::Px(8.0),
                right:  Val::Px(8.0),
                top:    Val::Px(4.0),
                bottom: Val::Px(4.0),
            },
            row_gap: Val::Px(4.0),
            display: Display::Flex,
            ..default()
        },
        BackgroundColor(Color::srgba(0.05, 0.05, 0.07, 0.80)),
    )).id();

    // Top row: title left, numeric right.
    let top_row = commands.spawn(Node {
        flex_direction:  FlexDirection::Row,
        justify_content: JustifyContent::SpaceBetween,
        width:           Val::Percent(100.0),
        ..default()
    }).id();

    let title = commands.spawn((
        Text::new("BOOST"),
        TextFont { font_size: 11.0, ..default() },
        TextColor(Color::srgb(0.75, 0.75, 0.75)),
    )).id();

    let numeric = commands.spawn((
        BoostNumericText,
        Text::new("3.0 s / 3.0 s"),
        TextFont { font_size: 11.0, ..default() },
        TextColor(Color::srgb(0.75, 0.75, 0.75)),
    )).id();

    commands.entity(top_row).add_children(&[title, numeric]);

    // Bar background.
    let bar_bg = commands.spawn((
        Node {
            width:  Val::Percent(100.0),
            height: Val::Px(6.0),
            ..default()
        },
        BackgroundColor(Color::srgba(0.15, 0.15, 0.15, 1.0)),
    )).id();

    // Bar fill (width driven by update_boost_hud).
    let bar_fill = commands.spawn((
        BoostBarFill,
        Node {
            width:  Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..default()
        },
        BackgroundColor(Color::srgb(0.2, 0.5, 1.0)),
    )).id();

    commands.entity(bar_bg).add_child(bar_fill);
    commands.entity(panel).add_children(&[top_row, bar_bg]);
}

// ---- Update: manage charge and active flag -----------------------------------

fn update_boost_state(
    keys:    Res<ButtonInput<KeyCode>>,
    time:    Res<Time>,
    mut boost: ResMut<BoostState>,
) {
    let dt = time.delta_secs();

    if keys.pressed(KeyCode::ShiftLeft) && boost.charge_s > 0.0 {
        boost.active    = true;
        boost.charge_s  = (boost.charge_s - dt).max(0.0);
    } else {
        boost.active    = false;
        boost.charge_s  = (boost.charge_s + 0.5 * dt).min(boost.max_s);
    }
}

// ---- PhysicsSchedule: apply thrust force when active ------------------------

fn apply_boost_force(
    boost:      Res<BoostState>,
    vehicle:    Option<Res<VehicleRoot>>,
    mut chassis_q: Query<(Forces, &Transform), With<Chassis>>,
) {
    if !boost.active { return; }

    let Some(vehicle) = vehicle else { return };
    let Ok((mut forces, transform)) = chassis_q.get_mut(vehicle.chassis) else { return };

    // Chassis forward: Bevy convention — local -Z is forward.
    let forward = (transform.rotation * Vec3::NEG_Z).normalize();
    forces.apply_force(forward * BOOST_FORCE);
}

// ---- Update: refresh HUD bar and numeric ------------------------------------

fn update_boost_hud(
    boost:     Res<BoostState>,
    mut bar_q: Query<(&mut Node, &mut BackgroundColor), With<BoostBarFill>>,
    mut text_q: Query<&mut Text, With<BoostNumericText>>,
) {
    let frac = (boost.charge_s / boost.max_s).clamp(0.0, 1.0);

    // Colour ramp: blue (full) → green (half-charged) → red (low/active).
    let bar_color = if boost.active || frac < 0.15 {
        Color::srgb(0.95, 0.15, 0.15)
    } else if frac > 0.60 {
        Color::srgb(0.2, 0.5, 1.0)
    } else {
        Color::srgb(0.2, 0.85, 0.3)
    };

    for (mut node, mut bg) in &mut bar_q {
        node.width = Val::Percent(frac * 100.0);
        bg.0       = bar_color;
    }

    for mut text in &mut text_q {
        text.0 = format!("{:.1} s / {:.1} s", boost.charge_s, boost.max_s);
    }
}
