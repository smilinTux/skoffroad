// Intro cinematic: a 5-second scripted camera flythrough across the map
// when the app starts. Plays once, can be skipped with any key.
//
// Public API:
//   IntroVideoPlugin
//   IntroVideoState (resource)

use bevy::prelude::*;
use crate::vehicle::{Chassis, VehicleRoot};

pub struct IntroVideoPlugin;

impl Plugin for IntroVideoPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<IntroVideoState>()
            .add_systems(Update, tick_intro)
            .add_systems(Update, apply_camera_path)
            .add_systems(Update, skip_with_any_key);
    }
}

#[derive(Resource, Default)]
pub struct IntroVideoState {
    pub playing: bool,
    pub elapsed_s: f32,
}

/// Watches app time; once >= 2 s fires the intro cinematic exactly once.
fn tick_intro(
    time: Res<Time>,
    mut state: ResMut<IntroVideoState>,
    mut started: Local<bool>,
) {
    if *started {
        return;
    }
    if time.elapsed_secs() >= 2.0 {
        *started = true;
        state.playing = true;
        state.elapsed_s = 0.0;
        info!("intro: cinematic playing");
    }
}

/// Drives the camera along a scripted orbit while `state.playing` is true.
/// Runs every Update frame; because it writes `cam.translation` and
/// `cam.look_at` after camera.rs's `update_camera` (last-writer wins in the
/// same schedule), it overrides the normal chase camera for the duration.
fn apply_camera_path(
    mut state: ResMut<IntroVideoState>,
    vehicle: Option<Res<VehicleRoot>>,
    chassis_q: Query<&Transform, With<Chassis>>,
    mut cam_q: Query<&mut Transform, (With<Camera3d>, Without<Chassis>)>,
    time: Res<Time>,
) {
    if !state.playing {
        return;
    }

    let Some(vehicle) = vehicle else { return };
    let Ok(chassis_tf) = chassis_q.get(vehicle.chassis) else { return };
    let Ok(mut cam_tf) = cam_q.single_mut() else { return };

    let dt = time.delta_secs();
    state.elapsed_s += dt;

    let t = state.elapsed_s;
    let chassis_pos = chassis_tf.translation;

    // One-and-a-fifth revolutions over 5 seconds.
    let angle = t / 5.0 * std::f32::consts::TAU * 1.2;
    let height = 4.0 + (t * 0.7).sin() * 2.0;

    let cam_pos = chassis_pos + Vec3::new(angle.cos() * 12.0, height, angle.sin() * 12.0);
    let look_target = chassis_pos + Vec3::new(0.0, 1.0, 0.0);

    *cam_tf = Transform::from_translation(cam_pos).looking_at(look_target, Vec3::Y);

    if state.elapsed_s >= 5.0 {
        state.playing = false;
        info!("intro: complete");
    }
}

/// Lets the player skip the intro with any keyboard key.
fn skip_with_any_key(
    mut state: ResMut<IntroVideoState>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    if !state.playing {
        return;
    }
    if keys.get_just_pressed().next().is_some() {
        state.playing = false;
        info!("intro: skipped");
    }
}
