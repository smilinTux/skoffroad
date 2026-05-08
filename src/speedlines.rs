// Screen-edge vignette that darkens at high speed — cheap "speed lines" effect.
//
// Four full-edge UI Nodes (top, bottom, left, right) each 80 px wide/tall.
// Their BackgroundColor alpha ramps from 0.0 at <=5 m/s up to 0.45 at >=25 m/s.
// Toggle: X key. Default: enabled.

use bevy::prelude::*;
use avian3d::prelude::LinearVelocity;

use crate::vehicle::{Chassis, VehicleRoot};

// ---- Speed thresholds --------------------------------------------------------

const SPEED_LOW:  f32 = 5.0;   // m/s — below this: fully transparent
const SPEED_HIGH: f32 = 25.0;  // m/s — above this: max opacity
const ALPHA_MAX:  f32 = 0.45;

// ---- Plugin ------------------------------------------------------------------

pub struct SpeedLinesPlugin;

impl Plugin for SpeedLinesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SpeedLinesEnabled>()
           .add_systems(Startup, spawn_speed_lines)
           .add_systems(Update, (
               update_speed_lines.run_if(resource_exists::<crate::vehicle::VehicleRoot>),
               toggle_speed_lines,
           ));
    }
}

// ---- Resources & components --------------------------------------------------

#[derive(Resource)]
pub struct SpeedLinesEnabled(pub bool);

impl Default for SpeedLinesEnabled {
    fn default() -> Self { Self(true) }
}

#[derive(Component)]
struct SpeedLinePanel;

// ---- Startup: spawn four edge panels ----------------------------------------

fn spawn_speed_lines(mut commands: Commands) {
    let transparent = BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0));

    // Top edge
    commands.spawn((
        SpeedLinePanel,
        Node {
            position_type: PositionType::Absolute,
            top:    Val::Px(0.0),
            left:   Val::Px(0.0),
            width:  Val::Percent(100.0),
            height: Val::Px(80.0),
            ..default()
        },
        transparent,
        ZIndex(10),
    ));

    // Bottom edge
    commands.spawn((
        SpeedLinePanel,
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(0.0),
            left:   Val::Px(0.0),
            width:  Val::Percent(100.0),
            height: Val::Px(80.0),
            ..default()
        },
        transparent,
        ZIndex(10),
    ));

    // Left edge
    commands.spawn((
        SpeedLinePanel,
        Node {
            position_type: PositionType::Absolute,
            top:    Val::Px(0.0),
            left:   Val::Px(0.0),
            width:  Val::Px(80.0),
            height: Val::Percent(100.0),
            ..default()
        },
        transparent,
        ZIndex(10),
    ));

    // Right edge
    commands.spawn((
        SpeedLinePanel,
        Node {
            position_type: PositionType::Absolute,
            top:   Val::Px(0.0),
            right: Val::Px(0.0),
            width:  Val::Px(80.0),
            height: Val::Percent(100.0),
            ..default()
        },
        transparent,
        ZIndex(10),
    ));
}

// ---- Update: set alpha from chassis speed ------------------------------------

fn update_speed_lines(
    enabled: Res<SpeedLinesEnabled>,
    vehicle: Res<VehicleRoot>,
    chassis_q: Query<&LinearVelocity, With<Chassis>>,
    mut panel_q: Query<&mut BackgroundColor, With<SpeedLinePanel>>,
) {
    let alpha = if !enabled.0 {
        0.0
    } else {
        let Ok(lin_vel) = chassis_q.get(vehicle.chassis) else { return };
        let speed = Vec3::new(lin_vel.x, lin_vel.y, lin_vel.z).length();

        if speed <= SPEED_LOW {
            0.0
        } else if speed >= SPEED_HIGH {
            ALPHA_MAX
        } else {
            let t = (speed - SPEED_LOW) / (SPEED_HIGH - SPEED_LOW);
            t * ALPHA_MAX
        }
    };

    for mut bg in &mut panel_q {
        bg.0 = Color::srgba(0.0, 0.0, 0.0, alpha);
    }
}

// ---- Toggle: X key -----------------------------------------------------------

fn toggle_speed_lines(
    keys:    Res<ButtonInput<KeyCode>>,
    mut enabled: ResMut<SpeedLinesEnabled>,
) {
    if keys.just_pressed(KeyCode::KeyX) {
        enabled.0 = !enabled.0;
    }
}
