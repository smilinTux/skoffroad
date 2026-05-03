// Mouse-controlled chase camera that orbits around the vehicle chassis.
// Bevy 0.18: events are now "messages"; use MessageReader, not EventReader.

use bevy::{input::mouse::MouseMotion, prelude::*};
use crate::vehicle::VehicleRoot;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(CameraState::default())
           .add_systems(Startup, spawn_camera)
           .add_systems(Update, mouse_look)
           .add_systems(Update, chase_camera.after(mouse_look));
    }
}

#[derive(Resource)]
struct CameraState {
    yaw:   f32,
    pitch: f32,
    dist:  f32,
}

impl Default for CameraState {
    fn default() -> Self {
        Self { yaw: 0.0, pitch: -0.4, dist: 14.0 }
    }
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 10.0, 20.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

fn mouse_look(
    mut state:   ResMut<CameraState>,
    mut motion:  MessageReader<MouseMotion>,
    buttons:     Res<ButtonInput<MouseButton>>,
) {
    // Orbit with right-mouse-button held.
    if !buttons.pressed(MouseButton::Right) {
        motion.clear();
        return;
    }
    const SENS: f32 = 0.005;
    for ev in motion.read() {
        state.yaw   -= ev.delta.x * SENS;
        state.pitch -= ev.delta.y * SENS;
        state.pitch  = state.pitch.clamp(-1.4, -0.05);
    }
}

fn chase_camera(
    state:     Res<CameraState>,
    vehicle:   Option<Res<VehicleRoot>>,
    chassis_q: Query<&Transform, With<crate::vehicle::Chassis>>,
    mut cam_q: Query<&mut Transform, (With<Camera3d>, Without<crate::vehicle::Chassis>)>,
) {
    let Some(vehicle) = vehicle else { return };
    let Ok(chassis) = chassis_q.get(vehicle.chassis) else { return };
    // Bevy 0.18: single_mut() replaces get_single_mut().
    let Ok(mut cam) = cam_q.single_mut() else { return };

    let offset = Quat::from_rotation_y(state.yaw)
        * Quat::from_rotation_x(state.pitch)
        * Vec3::new(0.0, 0.0, state.dist);

    let target = chassis.translation + Vec3::Y * 1.5;
    cam.translation = target + offset;
    cam.look_at(target, Vec3::Y);
}
