// Drive mode: `4` key cycles 2WD (rear-wheel drive only) ↔ 4WD (all wheels
// driven). 4WD is the default behavior; 2WD cancels the drive force on the
// two front wheels by applying an equal counter-force on the chassis.
//
// Architecture:
//   cycle_with_4          (Update)          — flip four_wheel_drive on Digit4
//   apply_2wd_drag        (PhysicsSchedule) — backward counter-force for front 2 wheels
//   update_indicator      (Update)          — HUD text: cyan=4WD, yellow=2WD
//
// Public API:
//   DriveModePlugin
//   DriveModeState (resource)

use bevy::prelude::*;
use avian3d::prelude::*;

use crate::vehicle::{Chassis, DriveInput, VehicleRoot};
use crate::rival::RivalChassis;

// ---- Constants ---------------------------------------------------------------

/// Mirror of DRIVE_FORCE_PER_WHEEL from vehicle.rs (2600 N per wheel).
const DRIVE_FORCE_PER_WHEEL_REF: f32 = 2600.0;

// ---- Colours -----------------------------------------------------------------

const CYAN:   Color = Color::srgb(0.0, 0.95, 0.95);
const YELLOW: Color = Color::srgb(1.0, 0.90, 0.0);

// ---- Public resource ---------------------------------------------------------

#[derive(Resource, Clone, Copy, PartialEq, Eq)]
pub struct DriveModeState {
    pub four_wheel_drive: bool,
}

impl Default for DriveModeState {
    fn default() -> Self {
        Self { four_wheel_drive: true }
    }
}

// ---- HUD components ----------------------------------------------------------

#[derive(Component)]
struct DriveModeHudRoot;

#[derive(Component)]
struct DriveModeHudText;

// ---- Plugin ------------------------------------------------------------------

pub struct DriveModePlugin;

impl Plugin for DriveModePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DriveModeState>()
           .add_systems(Startup, spawn_drive_indicator)
           .add_systems(
               Update,
               (
                   cycle_with_4,
                   update_indicator,
               ),
           )
           .add_systems(
               PhysicsSchedule,
               apply_2wd_drag
                   .after(PhysicsStepSystems::NarrowPhase)
                   .before(PhysicsStepSystems::Solver),
           );
    }
}

// ---- Startup: spawn bottom-right HUD indicator --------------------------------

fn spawn_drive_indicator(mut commands: Commands) {
    // Positioned just above the low_range indicator (bottom 12 + 28 height + 4 gap = 44).
    let root = commands.spawn((
        DriveModeHudRoot,
        Node {
            position_type: PositionType::Absolute,
            right:         Val::Px(12.0),
            bottom:        Val::Px(44.0),
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
        DriveModeHudText,
        Text::new("DRIVE: 4WD"),
        TextFont { font_size: 14.0, ..default() },
        TextColor(CYAN),
    )).id();

    commands.entity(root).add_child(label);
}

// ---- System: cycle on Digit4 press -------------------------------------------

fn cycle_with_4(
    keys:      Res<ButtonInput<KeyCode>>,
    mut state: ResMut<DriveModeState>,
) {
    if keys.just_pressed(KeyCode::Digit4) {
        state.four_wheel_drive = !state.four_wheel_drive;
        info!("drive mode: {}", if state.four_wheel_drive { "4WD" } else { "2WD" });
    }
}

// ---- System: counter-force for front 2 wheels when in 2WD (PhysicsSchedule) --

fn apply_2wd_drag(
    state:         Res<DriveModeState>,
    drive:         Res<DriveInput>,
    vehicle:       Option<Res<VehicleRoot>>,
    mut chassis_q: Query<(Forces, &Transform), (With<Chassis>, Without<RivalChassis>)>,
) {
    // Only relevant in 2WD and when the player is applying forward throttle.
    if state.four_wheel_drive { return; }
    if drive.drive <= 0.0     { return; }

    let Some(vehicle) = vehicle else { return };
    let Ok((mut forces, transform)) = chassis_q.get_mut(vehicle.chassis) else { return };

    // vehicle.rs applies DRIVE_FORCE_PER_WHEEL to all 4 wheels when driven.
    // In 2WD we want to cancel the contribution of the front 2 wheels (5200 N).
    // We apply a backward body-centre force equal to: drive * 2 * 2600 N.
    let chassis_fwd   = (transform.rotation * Vec3::NEG_Z).normalize();
    let shaped_drive  = drive.drive.abs().powf(1.5); // same throttle curve as vehicle.rs
    let counter_force = chassis_fwd * -(shaped_drive * DRIVE_FORCE_PER_WHEEL_REF * 2.0);
    forces.apply_force(counter_force);
}

// ---- System: update HUD text and colour --------------------------------------

fn update_indicator(
    state:      Res<DriveModeState>,
    mut text_q: Query<(&mut Text, &mut TextColor), With<DriveModeHudText>>,
) {
    let (label, color) = if state.four_wheel_drive {
        ("DRIVE: 4WD", CYAN)
    } else {
        ("DRIVE: 2WD", YELLOW)
    };

    for (mut text, mut fg) in &mut text_q {
        text.0 = label.to_string();
        fg.0   = color;
    }
}
