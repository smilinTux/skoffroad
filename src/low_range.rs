// Low-range gearing: L key cycles HIGH/LOW. LOW = 3× torque, 1/3× max speed.
// Critical for crawling steep grades or rock garden obstacles.
//
// Architecture:
//   toggle_with_l         (Update)         — flip low_engaged on L key press
//   apply_low_range_forces (PhysicsSchedule) — extra forward force + speed-cap drag
//   update_indicator      (Update)         — HUD text colour: cyan=HIGH, orange=LOW
//
// Public API:
//   LowRangePlugin
//   LowRangeState (resource)

use bevy::prelude::*;
use avian3d::prelude::*;

use crate::vehicle::{Chassis, DriveInput, VehicleRoot};
use crate::rival::RivalChassis;

// ---- Constants ---------------------------------------------------------------

/// Mirror of DRIVE_FORCE_PER_WHEEL from vehicle.rs (2600 N per wheel).
const DRIVE_FORCE_PER_WHEEL_REF: f32 = 2600.0;

/// Number of driven wheels (4WD).
const WHEEL_COUNT: f32 = 4.0;

/// Extra multiplier in LOW — vehicle.rs already provides 1×, we add 2× more
/// so total torque reaches 3× normal. Applied as a body-centre forward push.
const LOW_TORQUE_EXTRA: f32 = 2.0;

/// Maximum chassis speed (m/s) in low range before drag is applied.
const LOW_MAX_SPEED_MS: f32 = 8.0;

/// Drag coefficient (N per m/s of excess speed) applied when over the cap.
const LOW_DRAG_COEFF: f32 = 1500.0;

// ---- Colours -----------------------------------------------------------------

const CYAN:   Color = Color::srgb(0.0, 0.95, 0.95);
const ORANGE: Color = Color::srgb(1.0, 0.55, 0.05);

// ---- Public resource ---------------------------------------------------------

#[derive(Resource, Default, Clone, Copy, PartialEq, Eq)]
pub struct LowRangeState {
    pub low_engaged: bool,
}

// ---- HUD components ----------------------------------------------------------

#[derive(Component)]
struct GearHudRoot;

#[derive(Component)]
struct GearHudText;

// ---- Plugin ------------------------------------------------------------------

pub struct LowRangePlugin;

impl Plugin for LowRangePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LowRangeState>()
           .add_systems(Startup, spawn_gear_indicator)
           .add_systems(
               Update,
               (
                   toggle_with_l,
                   update_indicator,
               ),
           )
           .add_systems(
               PhysicsSchedule,
               apply_low_range_forces
                   .after(PhysicsStepSystems::NarrowPhase)
                   .before(PhysicsStepSystems::Solver),
           );
    }
}

// ---- Startup: spawn bottom-right HUD indicator --------------------------------

fn spawn_gear_indicator(mut commands: Commands) {
    let root = commands.spawn((
        GearHudRoot,
        Node {
            position_type: PositionType::Absolute,
            right:         Val::Px(12.0),
            bottom:        Val::Px(12.0),
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
        GearHudText,
        Text::new("GEAR: HIGH"),
        TextFont { font_size: 14.0, ..default() },
        TextColor(CYAN),
    )).id();

    commands.entity(root).add_child(label);
}

// ---- System: toggle on L key press -------------------------------------------

fn toggle_with_l(
    keys:      Res<ButtonInput<KeyCode>>,
    mut state: ResMut<LowRangeState>,
) {
    if keys.just_pressed(KeyCode::KeyL) {
        state.low_engaged = !state.low_engaged;
        info!("gear: {}", if state.low_engaged { "LOW" } else { "HIGH" });
    }
}

// ---- System: extra force + speed cap (PhysicsSchedule) ----------------------

fn apply_low_range_forces(
    state:         Res<LowRangeState>,
    drive:         Res<DriveInput>,
    vehicle:       Option<Res<VehicleRoot>>,
    mut chassis_q: Query<(Forces, &Transform), (With<Chassis>, Without<RivalChassis>)>,
) {
    if !state.low_engaged { return; }

    let Some(vehicle) = vehicle else { return };
    let Ok((mut forces, transform)) = chassis_q.get_mut(vehicle.chassis) else { return };

    let forward  = (transform.rotation * Vec3::NEG_Z).normalize();
    let velocity = forces.linear_velocity();
    let speed    = velocity.length();

    // -- Extra forward thrust (adds 2× base force = 3× total with vehicle.rs) --
    if drive.drive.abs() > 0.0 {
        let shaped      = drive.drive.signum() * drive.drive.abs().powf(1.5);
        let extra_force = shaped * DRIVE_FORCE_PER_WHEEL_REF * WHEEL_COUNT * LOW_TORQUE_EXTRA;
        forces.apply_force(forward * extra_force);
    }

    // -- Velocity-cap drag: resist speeds above LOW_MAX_SPEED_MS --------------
    let excess = (speed - LOW_MAX_SPEED_MS).max(0.0);
    if excess > 0.0 {
        let drag_dir   = if speed > 0.0 { -velocity / speed } else { Vec3::ZERO };
        let drag_force = drag_dir * excess * LOW_DRAG_COEFF;
        forces.apply_force(drag_force);
    }
}

// ---- System: update HUD text and colour --------------------------------------

fn update_indicator(
    state:      Res<LowRangeState>,
    mut text_q: Query<(&mut Text, &mut TextColor), With<GearHudText>>,
) {
    let (label, color) = if state.low_engaged {
        ("GEAR: LOW", ORANGE)
    } else {
        ("GEAR: HIGH", CYAN)
    };

    for (mut text, mut fg) in &mut text_q {
        text.0 = label.to_string();
        fg.0   = color;
    }
}
