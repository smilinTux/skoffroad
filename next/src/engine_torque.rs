// Engine torque: real RPM-based torque curve. Replaces the implicit constant
// drive force feel with a proper engine — peak torque around 2500 RPM, falls
// off above 4500. Drives engine_pro.rs's pitch via shared RPM resource.
//
// Sprint 34 — PRD v3 should-have S4
//
// Systems:
//   compute_rpm              — Update   — derives RPM from chassis speed
//   apply_torque_multiplier  — PhysicsSchedule (after NarrowPhase, before Solver)
//                              — nudges chassis drive force by (torque_mult - 1)
//   update_indicator         — Update   — refreshes bottom-right tachometer HUD
//
// Public API:
//   EngineTorquePlugin
//   EngineState (resource — exposes rpm for engine_pro.rs to read)

use bevy::prelude::*;
use avian3d::prelude::*;

use crate::vehicle::{Chassis, DriveInput, VehicleRoot};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Mirror of DRIVE_FORCE_PER_WHEEL in vehicle.rs — keep in sync.
const DRIVE_FORCE_PER_WHEEL_REF: f32 = 2600.0;

/// RPM at which the tachometer redline indicator turns red.
const REDLINE_RPM: f32 = 5500.0;

/// Maximum RPM on the scale.
const MAX_RPM: f32 = 6500.0;

/// Idle RPM (lower clamp).
const IDLE_RPM: f32 = 700.0;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

pub struct EngineTorquePlugin;

impl Plugin for EngineTorquePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<EngineState>()
           .add_systems(Startup, spawn_tachometer)
           .add_systems(
               Update,
               (compute_rpm, update_indicator)
                   .chain()
                   .run_if(resource_exists::<VehicleRoot>),
           )
           .add_systems(
               PhysicsSchedule,
               apply_torque_multiplier
                   .after(PhysicsStepSystems::NarrowPhase)
                   .before(PhysicsStepSystems::Solver)
                   .run_if(resource_exists::<VehicleRoot>),
           );
    }
}

#[derive(Resource, Default, Clone, Copy)]
pub struct EngineState {
    pub rpm: f32,
    pub torque_mult: f32,
    pub gear: u8,
}

// ---------------------------------------------------------------------------
// HUD components
// ---------------------------------------------------------------------------

#[derive(Component)]
struct TachRoot;

#[derive(Component)]
struct TachText;

#[derive(Component)]
struct TachBarFill;

// ---------------------------------------------------------------------------
// Torque curve helper
// ---------------------------------------------------------------------------

/// Returns a torque multiplier in [0.4 .. 1.0] for the given RPM:
///
///   700 .. 2500  linear ramp 0.4 → 1.0  (building torque)
///  2500 .. 4500  plateau at ~0.95        (peak band)
///  4500 .. 6500  linear falloff 1.0 → 0.5 (power falls off at high RPM)
fn torque_curve(rpm: f32) -> f32 {
    if rpm < 2500.0 {
        // Linear ramp: 0.4 at IDLE_RPM (700), 1.0 at 2500.
        let t = (rpm - IDLE_RPM) / (2500.0 - IDLE_RPM);
        0.4 + 0.6 * t.clamp(0.0, 1.0)
    } else if rpm < 4500.0 {
        // Plateau — slight dip to 0.95 across the peak band.
        0.95
    } else {
        // Linear falloff: 1.0 at 4500, 0.5 at 6500.
        let t = (rpm - 4500.0) / (6500.0 - 4500.0);
        1.0 - 0.5 * t.clamp(0.0, 1.0)
    }
}

// ---------------------------------------------------------------------------
// Startup: spawn tachometer HUD
// ---------------------------------------------------------------------------

fn spawn_tachometer(mut commands: Commands) {
    // Panel: bottom-right, above the speedometer gauge (gauge is at bottom 120 px,
    // height 100 px; 8 px gap → 120 + 100 + 8 = 228 px from bottom).
    // Panel height: text row (18 px) + 4 px bar + 8 px padding top/bottom = ~38 px.
    let panel = commands.spawn((
        TachRoot,
        Node {
            position_type:   PositionType::Absolute,
            right:           Val::Px(12.0),
            bottom:          Val::Px(228.0),
            width:           Val::Px(200.0),
            height:          Val::Px(38.0),
            flex_direction:  FlexDirection::Column,
            justify_content: JustifyContent::SpaceBetween,
            padding: UiRect {
                left:   Val::Px(8.0),
                right:  Val::Px(8.0),
                top:    Val::Px(6.0),
                bottom: Val::Px(6.0),
            },
            row_gap: Val::Px(4.0),
            display: Display::Flex,
            ..default()
        },
        BackgroundColor(Color::srgba(0.05, 0.05, 0.07, 0.80)),
    )).id();

    // RPM text label.
    let text_node = commands.spawn((
        TachText,
        Text::new("RPM: 1500"),
        TextFont { font_size: 13.0, ..default() },
        TextColor(Color::srgb(0.2, 0.85, 0.3)),
    )).id();

    // Bar background (full width, 4 px tall).
    let bar_bg = commands.spawn((
        Node {
            width:  Val::Percent(100.0),
            height: Val::Px(4.0),
            ..default()
        },
        BackgroundColor(Color::srgba(0.15, 0.15, 0.15, 1.0)),
    )).id();

    // Bar fill — width driven by RPM ratio.
    let bar_fill = commands.spawn((
        TachBarFill,
        Node {
            width:  Val::Percent(0.0),
            height: Val::Percent(100.0),
            ..default()
        },
        BackgroundColor(Color::srgb(0.2, 0.85, 0.3)),
    )).id();

    commands.entity(bar_bg).add_child(bar_fill);
    commands.entity(panel).add_children(&[text_node, bar_bg]);
}

// ---------------------------------------------------------------------------
// System: compute_rpm
// ---------------------------------------------------------------------------

/// Derive engine RPM from chassis ground speed.
///
/// Formula: rpm = speed_mps * 90 + 700  (idle at ~700, ~6300 at 100 m/s).
/// Clamped to [IDLE_RPM, MAX_RPM].  Also evaluates the torque curve and writes
/// both values into EngineState.
fn compute_rpm(
    vehicle:    Res<VehicleRoot>,
    chassis_q:  Query<&LinearVelocity, With<Chassis>>,
    mut state:  ResMut<EngineState>,
) {
    let Ok(lin_vel) = chassis_q.get(vehicle.chassis) else { return };

    let speed_mps = Vec3::new(lin_vel.x, lin_vel.y, lin_vel.z).length();
    let raw_rpm   = speed_mps * 90.0 + IDLE_RPM;
    let rpm       = raw_rpm.clamp(IDLE_RPM, MAX_RPM);

    state.rpm         = rpm;
    state.torque_mult = torque_curve(rpm);
    // Gear is informational — approximate 1-6 across the RPM range.
    state.gear = (((rpm - IDLE_RPM) / (MAX_RPM - IDLE_RPM) * 5.0).floor() as u8 + 1).min(6);
}

// ---------------------------------------------------------------------------
// System: apply_torque_multiplier  (PhysicsSchedule)
// ---------------------------------------------------------------------------

/// Apply an additional forward force proportional to (torque_mult - 1.0).
///
/// vehicle.rs already applies a flat DRIVE_FORCE_PER_WHEEL per grounded wheel.
/// This system adds (or subtracts) a corrective force so that the effective
/// drive force follows the torque curve:
///
///   extra_force = chassis_fwd * (torque_mult - 1.0) * DRIVE_FORCE_PER_WHEEL_REF * 4 * drive
///
/// At peak torque (torque_mult ≈ 0.95) the extra is −0.05 × base (tiny reduction).
/// Below 2500 RPM torque_mult < 1 → extra is negative (less force, sluggish low-end).
/// Above 4500 RPM torque_mult < 1 → extra is negative (power falls off at high RPM).
/// The combined feel: spunky mid-range, lazy high-end, weak from a standstill.
fn apply_torque_multiplier(
    state:      Res<EngineState>,
    input:      Res<DriveInput>,
    vehicle:    Res<VehicleRoot>,
    mut chassis_q: Query<(Forces, &Transform), With<Chassis>>,
) {
    if input.drive.abs() < 1e-3 { return; }

    let Ok((mut forces, transform)) = chassis_q.get_mut(vehicle.chassis) else { return };

    let chassis_fwd = (transform.rotation * Vec3::NEG_Z).normalize();
    let delta = (state.torque_mult - 1.0) * DRIVE_FORCE_PER_WHEEL_REF * 4.0 * input.drive;
    forces.apply_force(chassis_fwd * delta);
}

// ---------------------------------------------------------------------------
// System: update_indicator
// ---------------------------------------------------------------------------

/// Refresh the tachometer HUD text and bar each frame.
///
/// Text colour tier:
///   green  — peak band 2000..4500 RPM
///   yellow — outside peak band
///
/// Bar colour:
///   green  — below REDLINE_RPM (5500)
///   red    — above REDLINE_RPM
fn update_indicator(
    state:     Res<EngineState>,
    mut text_q: Query<(&mut Text, &mut TextColor), With<TachText>>,
    mut bar_q:  Query<(&mut Node, &mut BackgroundColor), With<TachBarFill>>,
) {
    let rpm = state.rpm;

    // Text colour: green in peak band, yellow otherwise.
    let text_color = if (2000.0..4500.0).contains(&rpm) {
        Color::srgb(0.2, 0.85, 0.3)
    } else {
        Color::srgb(0.95, 0.85, 0.2)
    };

    for (mut text, mut color) in &mut text_q {
        text.0  = format!("RPM: {:.0}", rpm);
        color.0 = text_color;
    }

    // Bar: rpm / MAX_RPM ratio, redline colour above REDLINE_RPM.
    let frac = (rpm / MAX_RPM).clamp(0.0, 1.0);
    let bar_color = if rpm >= REDLINE_RPM {
        Color::srgb(0.95, 0.2, 0.2)
    } else {
        Color::srgb(0.2, 0.85, 0.3)
    };

    for (mut node, mut bg) in &mut bar_q {
        node.width = Val::Percent(frac * 100.0);
        bg.0       = bar_color;
    }
}
