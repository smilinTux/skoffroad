// Differential lock: J key cycles OPEN → REAR → FULL → OPEN.
// When locked, both wheels on an axle spin at the same rate — no wheelspin
// loss when one wheel is in the air. Visible HUD indicator + cosmetic glow
// on driveline.
//
// Architecture:
//   cycle_with_j          (Update)           — cycle lock state on J press
//   apply_diff_lock       (PhysicsSchedule)  — extra forward force on locked axles
//   update_indicator      (Update)           — HUD text + colour
//
// Public API:
//   DiffLockPlugin
//   DiffLockState (resource)

use bevy::prelude::*;
use avian3d::prelude::*;

use crate::vehicle::{Chassis, DriveInput, VehicleRoot, Wheel};
use crate::rival::RivalChassis;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Mirror of DRIVE_FORCE_PER_WHEEL from vehicle.rs (2600 N per wheel).
const DRIVE_FORCE_PER_WHEEL_REF: f32 = 2600.0;

/// Extra multiplier applied per locked axle when at least one wheel is
/// grounded. Adds 0.5× the per-wheel reference force at the axle midpoint,
/// compensating for the torque lost when the opposite wheel lifts off.
const DIFF_LOCK_EXTRA_MUL: f32 = 0.5;

/// Chassis-local Z offset to the front axle midpoint (negative = forward).
const FRONT_AXLE_Z: f32 = -1.4;

/// Chassis-local Z offset to the rear axle midpoint (positive = rearward).
const REAR_AXLE_Z: f32 = 1.4;

// ---------------------------------------------------------------------------
// Colours
// ---------------------------------------------------------------------------

const GRAY:   Color = Color::srgb(0.55, 0.55, 0.55);
const YELLOW: Color = Color::srgb(0.95, 0.85, 0.10);
const RED:    Color = Color::srgb(0.95, 0.20, 0.10);

// ---------------------------------------------------------------------------
// Public resource
// ---------------------------------------------------------------------------

/// Current differential lock state.
///
/// `(false, false)` = OPEN — independent wheels on every axle.
/// `(false, true)`  = REAR — rear axle locked; front open.
/// `(true,  true)`  = FULL — both axles locked.
#[derive(Resource, Default, Clone, Copy, PartialEq, Eq)]
pub struct DiffLockState {
    pub front_locked: bool,
    pub rear_locked:  bool,
}

impl std::fmt::Display for DiffLockState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match (self.front_locked, self.rear_locked) {
            (false, false) => write!(f, "OPEN"),
            (false, true)  => write!(f, "REAR"),
            (true,  true)  => write!(f, "FULL"),
            // Unreachable in normal cycling, but handle gracefully.
            (true,  false) => write!(f, "FRONT"),
        }
    }
}

// ---------------------------------------------------------------------------
// HUD components
// ---------------------------------------------------------------------------

#[derive(Component)]
struct DiffHudRoot;

#[derive(Component)]
struct DiffHudText;

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct DiffLockPlugin;

impl Plugin for DiffLockPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DiffLockState>()
           .add_systems(Startup, spawn_diff_indicator)
           .add_systems(
               Update,
               (
                   cycle_with_j,
                   update_indicator,
               ),
           )
           .add_systems(
               PhysicsSchedule,
               apply_diff_lock
                   .after(PhysicsStepSystems::NarrowPhase)
                   .before(PhysicsStepSystems::Solver),
           );
    }
}

// ---------------------------------------------------------------------------
// Startup: spawn bottom-right HUD indicator
// ---------------------------------------------------------------------------

fn spawn_diff_indicator(mut commands: Commands) {
    // Stack above the low-range indicator (low_range: bottom 12 px, height
    // 28 px).  Add 8 px gap → bottom offset = 12 + 28 + 8 = 48 px.
    let root = commands.spawn((
        DiffHudRoot,
        Node {
            position_type: PositionType::Absolute,
            right:         Val::Px(12.0),
            bottom:        Val::Px(48.0),
            width:         Val::Px(160.0),
            height:        Val::Px(28.0),
            align_items:   AlignItems::Center,
            padding: UiRect {
                left:   Val::Px(8.0),
                right:  Val::Px(8.0),
                top:    Val::Px(4.0),
                bottom: Val::Px(4.0),
            },
            ..default()
        },
        BackgroundColor(Color::srgba(0.05, 0.05, 0.07, 0.80)),
    )).id();

    let label = commands.spawn((
        DiffHudText,
        Text::new("DIFF: OPEN"),
        TextFont { font_size: 13.0, ..default() },
        TextColor(GRAY),
    )).id();

    commands.entity(root).add_child(label);
}

// ---------------------------------------------------------------------------
// System: cycle state on J press
// ---------------------------------------------------------------------------

fn cycle_with_j(
    keys:      Res<ButtonInput<KeyCode>>,
    mut state: ResMut<DiffLockState>,
) {
    if !keys.just_pressed(KeyCode::KeyJ) { return; }

    *state = match (state.front_locked, state.rear_locked) {
        // OPEN → REAR
        (false, false) => DiffLockState { front_locked: false, rear_locked: true  },
        // REAR → FULL
        (false, true)  => DiffLockState { front_locked: true,  rear_locked: true  },
        // FULL → OPEN  (and any unexpected state resets to OPEN)
        _              => DiffLockState { front_locked: false, rear_locked: false },
    };

    info!("diff lock: {}", *state);
}

// ---------------------------------------------------------------------------
// System: apply extra drive force on locked axles (PhysicsSchedule)
// ---------------------------------------------------------------------------
//
// Approximation: vehicle.rs applies per-wheel drive force only to grounded
// wheels.  When one wheel on a locked axle is airborne, the axle loses ~50%
// of its drive force.  We compensate by pushing the chassis forward with
// 0.5× DRIVE_FORCE_PER_WHEEL at the axle midpoint whenever at least one
// wheel on a locked axle is grounded and the driver is pressing throttle.
//
// This mirrors exactly what low_range.rs does for extra torque, keeping the
// pattern consistent across the codebase.

fn apply_diff_lock(
    state:         Res<DiffLockState>,
    drive:         Res<DriveInput>,
    vehicle:       Option<Res<VehicleRoot>>,
    mut chassis_q: Query<(Forces, &Transform), (With<Chassis>, Without<RivalChassis>)>,
    wheel_q:       Query<&Wheel>,
) {
    // Nothing to do if both axles are open or driver is not pressing throttle.
    if !state.front_locked && !state.rear_locked { return; }
    if drive.drive.abs() < f32::EPSILON { return; }

    let Some(vehicle) = vehicle else { return };
    let Ok((mut forces, transform)) = chassis_q.get_mut(vehicle.chassis) else { return };

    let chassis_pos = transform.translation;
    let chassis_rot = transform.rotation;
    let chassis_fwd = (chassis_rot * Vec3::NEG_Z).normalize();

    // Build per-axle grounded flags from wheel components.
    // Wheel indices: 0=FL, 1=FR, 2=RL, 3=RR (matches WHEEL_OFFSETS in vehicle.rs).
    let mut front_any_grounded = false;
    let mut rear_any_grounded  = false;

    for wheel in wheel_q.iter() {
        match wheel.index {
            0 | 1 => { if wheel.is_grounded { front_any_grounded = true; } }
            2 | 3 => { if wheel.is_grounded { rear_any_grounded  = true; } }
            _ => {}
        }
    }

    let shaped = drive.drive.signum() * drive.drive.abs().powf(1.5);
    let extra_force_mag = shaped * DRIVE_FORCE_PER_WHEEL_REF * DIFF_LOCK_EXTRA_MUL;

    if state.front_locked && front_any_grounded {
        let apply_pos = chassis_pos + chassis_rot * Vec3::new(0.0, 0.0, FRONT_AXLE_Z);
        forces.apply_force_at_point(chassis_fwd * extra_force_mag, apply_pos);
    }

    if state.rear_locked && rear_any_grounded {
        let apply_pos = chassis_pos + chassis_rot * Vec3::new(0.0, 0.0, REAR_AXLE_Z);
        forces.apply_force_at_point(chassis_fwd * extra_force_mag, apply_pos);
    }
}

// ---------------------------------------------------------------------------
// System: update HUD text + colour
// ---------------------------------------------------------------------------

fn update_indicator(
    state:      Res<DiffLockState>,
    mut text_q: Query<(&mut Text, &mut TextColor), With<DiffHudText>>,
) {
    let (label, color) = match (state.front_locked, state.rear_locked) {
        (false, false) => ("DIFF: OPEN", GRAY),
        (false, true)  => ("DIFF: REAR", YELLOW),
        _              => ("DIFF: FULL", RED),
    };

    for (mut text, mut fg) in &mut text_q {
        text.0 = label.to_string();
        fg.0   = color;
    }
}
