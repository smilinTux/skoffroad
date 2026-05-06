// Driver assists: traction control + auto-righting toggle. Y key cycles
// through Off → Traction → AutoRight → Both → Off.
//
// Public API:
//   AssistsPlugin
//   AssistsState (resource)

use bevy::prelude::*;
use avian3d::prelude::*;

use crate::vehicle::{Chassis, VehicleRoot};

// ---- Public plugin -----------------------------------------------------------

pub struct AssistsPlugin;

impl Plugin for AssistsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AssistsState>()
           .add_systems(Startup, spawn_assists_hud)
           .add_systems(Update, (
               cycle_with_y,
               apply_traction_control.run_if(resource_exists::<VehicleRoot>),
               apply_auto_right.run_if(resource_exists::<VehicleRoot>),
               update_indicator,
           ));
    }
}

// ---- Public resource ---------------------------------------------------------

#[derive(Resource, Default, Clone, Copy)]
pub struct AssistsState {
    pub traction_control: bool,
    pub auto_right: bool,
}

// ---- HUD components ----------------------------------------------------------

#[derive(Component)]
struct AssistsIndicator;

// ---- Startup: spawn HUD indicator top-right ----------------------------------

fn spawn_assists_hud(mut commands: Commands) {
    // Positioned top-right at top: 250px, right: 14px — below other top-right elements.
    let panel = commands.spawn((
        AssistsIndicator,
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(250.0),
            right: Val::Px(14.0),
            padding: UiRect::all(Val::Px(6.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.05, 0.05, 0.07, 0.75)),
    )).id();

    let text = commands.spawn((
        Text::new("ASSISTS: OFF"),
        TextFont {
            font_size: 13.0,
            ..default()
        },
        TextColor(Color::srgb(0.85, 0.85, 0.85)),
    )).id();

    commands.entity(panel).add_child(text);
}

// ---- Update: Y key cycles through modes -------------------------------------

fn cycle_with_y(
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<AssistsState>,
) {
    if !keys.just_pressed(KeyCode::KeyY) {
        return;
    }

    // Cycle: (false,false) → (true,false) → (false,true) → (true,true) → (false,false)
    let (tc, ar) = (state.traction_control, state.auto_right);
    let next = match (tc, ar) {
        (false, false) => (true, false),
        (true, false)  => (false, true),
        (false, true)  => (true, true),
        (true, true)   => (false, false),
    };
    state.traction_control = next.0;
    state.auto_right = next.1;

    let mode_label = match (next.0, next.1) {
        (true, false)  => "TC",
        (false, true)  => "AR",
        (true, true)   => "TC+AR",
        (false, false) => "OFF",
    };
    info!("Assists mode changed to: {}", mode_label);
}

// ---- Update: traction control damping ----------------------------------------

fn apply_traction_control(
    state: Res<AssistsState>,
    vehicle: Res<VehicleRoot>,
    mut chassis_q: Query<&mut AngularVelocity, With<Chassis>>,
) {
    if !state.traction_control { return; }

    let Ok(mut ang_vel) = chassis_q.get_mut(vehicle.chassis) else { return };

    // Cap lateral (Y-axis world) angular velocity: if |ang_y| > 4.0 rad/s,
    // scale down by 0.7 each frame to kill donuts/spinouts.
    if ang_vel.0.y.abs() > 4.0 {
        ang_vel.0.y *= 0.7;
    }
}

// ---- Update: auto-right torque -----------------------------------------------

fn apply_auto_right(
    state: Res<AssistsState>,
    vehicle: Res<VehicleRoot>,
    mut chassis_q: Query<(Forces, &Transform), With<Chassis>>,
) {
    if !state.auto_right { return; }

    let Ok((mut forces, transform)) = chassis_q.get_mut(vehicle.chassis) else { return };

    // Compute chassis up vector in world space.
    let chassis_rot = transform.rotation;
    let chassis_up  = (chassis_rot * Vec3::Y).normalize();

    // Only act when chassis is on its side or upside-down (up.y < 0.5, ~> 60° tilt).
    if chassis_up.y >= 0.5 { return; }

    // Only act when angular velocity is near zero (chassis is not already rolling).
    let ang_vel = forces.angular_velocity();
    if ang_vel.length() >= 1.0 { return; }

    // Compute a torque that rolls the chassis upright.
    // The axis of rotation is the cross product of the chassis_up and world_up (Vec3::Y).
    // This gives a torque vector perpendicular to both, rotating chassis_up toward Vec3::Y.
    let torque_axis = chassis_up.cross(Vec3::Y);

    // If cross product is near-zero (already perfectly upside-down), nudge along X.
    let torque_axis = if torque_axis.length_squared() < 1e-4 {
        Vec3::X
    } else {
        torque_axis.normalize()
    };

    forces.apply_torque(torque_axis * 5000.0);
}

// ---- Update: HUD text update --------------------------------------------------

fn update_indicator(
    state: Res<AssistsState>,
    panel_q: Query<&Children, With<AssistsIndicator>>,
    mut text_q: Query<(&mut Text, &mut TextColor)>,
) {
    let Ok(children) = panel_q.single() else { return };

    let tc = state.traction_control;
    let ar = state.auto_right;

    // Format label: "ASSISTS: TC+AR", "ASSISTS: TC", "ASSISTS: AR", "ASSISTS: OFF"
    let label = match (tc, ar) {
        (true, true)   => "ASSISTS: TC+AR".to_string(),
        (true, false)  => "ASSISTS: TC".to_string(),
        (false, true)  => "ASSISTS: AR".to_string(),
        (false, false) => "ASSISTS: OFF".to_string(),
    };

    // Color: green (TC), yellow (AR), cyan (both), default grey (OFF)
    let color = match (tc, ar) {
        (true, true)   => Color::srgb(0.0, 1.0, 1.0),   // cyan
        (true, false)  => Color::srgb(0.3, 0.95, 0.3),  // green
        (false, true)  => Color::srgb(1.0, 0.9, 0.0),   // yellow
        (false, false) => Color::srgb(0.85, 0.85, 0.85), // grey
    };

    for child in children.iter() {
        if let Ok((mut text, mut text_color)) = text_q.get_mut(child) {
            text.0 = label.clone();
            text_color.0 = color;
        }
    }
}
