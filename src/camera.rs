// Chase / cockpit camera for skoffroad.
// Bevy 0.18: events are "messages"; use MessageReader, not EventReader.

use bevy::{input::mouse::MouseMotion, prelude::*};
use avian3d::prelude::LinearVelocity;
use crate::vehicle::{Chassis, VehicleRoot};

pub struct CameraPlugin;

/// Ordering anchor: chase-cam writes Camera3d Transform here. Other plugins
/// (camera_modes.rs) chain `.after(CameraSet::Update)` so they overwrite the
/// transform last. Without this, scheduler ambiguity leaves wheel/FP/orbit
/// modes stuck in chase view.
#[derive(SystemSet, Debug, Hash, Eq, PartialEq, Clone)]
pub struct CameraSet;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        // V-key mode toggle removed — camera_modes.rs is the source of truth
        // for camera selection (5-mode cycle), and runs after this plugin.
        app.insert_resource(CameraState::default())
           .add_systems(Startup, spawn_camera)
           .add_systems(Update, mouse_look)
           .add_systems(Update, update_camera.in_set(CameraSet).after(mouse_look));
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CameraMode { Chase, Cockpit }

#[derive(Resource)]
struct CameraState {
    yaw:          f32,
    pitch:        f32,
    dist:         f32,
    mode:         CameraMode,
    // Smoothed values kept across frames for exponential damping.
    smooth_pos:   Vec3,
    smooth_look:  Vec3,
    smooth_fov:   f32,
}

impl Default for CameraState {
    fn default() -> Self {
        Self {
            yaw:         0.0,
            pitch:       -0.4,
            dist:        14.0,
            mode:        CameraMode::Chase,
            smooth_pos:  Vec3::new(0.0, 10.0, 20.0),
            smooth_look: Vec3::ZERO,
            smooth_fov:  60_f32.to_radians(),
        }
    }
}

// Higher = faster settle; 8.0 ≈ half-second-ish lag.
const DAMPING: f32 = 8.0;

fn spawn_camera(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Projection::Perspective(PerspectiveProjection {
            fov: 60_f32.to_radians(),
            ..default()
        }),
        Transform::from_xyz(0.0, 10.0, 20.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

fn mouse_look(
    mut state:  ResMut<CameraState>,
    mut motion: MessageReader<MouseMotion>,
    buttons:    Res<ButtonInput<MouseButton>>,
    keys:       Res<ButtonInput<KeyCode>>,
) {
    // In cockpit mode consume and discard mouse so delta doesn't accumulate.
    if state.mode == CameraMode::Cockpit {
        motion.clear();
        return;
    }

    const SENS: f32 = 0.005;
    const KEY_RATE: f32 = 0.03;

    // Right-mouse-button orbit.
    if buttons.pressed(MouseButton::Right) {
        for ev in motion.read() {
            state.yaw   -= ev.delta.x * SENS;
            state.pitch -= ev.delta.y * SENS;
        }
    } else {
        motion.clear();
    }

    // Q/E keyboard fallback for orbit.
    if keys.pressed(KeyCode::KeyQ) { state.yaw += KEY_RATE; }
    if keys.pressed(KeyCode::KeyE) { state.yaw -= KEY_RATE; }

    state.pitch = state.pitch.clamp(-1.4, 0.0);
}

fn update_camera(
    mut state:    ResMut<CameraState>,
    vehicle:      Option<Res<VehicleRoot>>,
    chassis_q:    Query<(&Transform, &LinearVelocity), With<Chassis>>,
    mut cam_q:    Query<
        (&mut Transform, &mut Projection),
        (With<Camera3d>, Without<Chassis>),
    >,
    time:         Res<Time>,
) {
    let Some(vehicle) = vehicle else { return };
    let Ok((chassis, lin_vel)) = chassis_q.get(vehicle.chassis) else { return };
    let Ok((mut cam, mut proj)) = cam_q.single_mut() else { return };

    let dt = time.delta_secs();
    // Frame-rate-independent exponential smoothing factor.
    let alpha = 1.0 - (-DAMPING * dt).exp();

    let chassis_fwd = (chassis.rotation * Vec3::NEG_Z).normalize();

    match state.mode {
        CameraMode::Chase => {
            // World-space offset based on yaw + pitch orbit.
            let orbit = Quat::from_rotation_y(state.yaw)
                * Quat::from_rotation_x(state.pitch)
                * Vec3::new(0.0, 0.0, state.dist);

            let target_pos  = chassis.translation + orbit;
            // Look-ahead: keeps road ahead in frame rather than staring at chassis center.
            let target_look = chassis.translation
                + chassis_fwd * 3.0
                + Vec3::Y * 1.5;

            state.smooth_pos  = state.smooth_pos.lerp(target_pos, alpha);
            state.smooth_look = state.smooth_look.lerp(target_look, alpha);

            cam.translation = state.smooth_pos;
            cam.look_at(state.smooth_look, Vec3::Y);
        }

        CameraMode::Cockpit => {
            // Driver's seat: slightly above chassis center, back from front bumper.
            let seat_local = Vec3::new(0.0, 0.5, -0.6);
            let target_pos  = chassis.translation + chassis.rotation * seat_local;
            // Look far along chassis forward; smooth_pos skips lag in cockpit.
            let target_look = chassis.translation + chassis_fwd * 30.0;

            // Cockpit position matches chassis directly — no lag wanted here.
            cam.translation = target_pos;
            cam.look_at(target_look, Vec3::Y);

            // Keep smooth_pos in sync so switching back to chase doesn't teleport.
            state.smooth_pos  = target_pos;
            state.smooth_look = target_look;
        }
    }

    // Dynamic FOV based on speed (chase only; cockpit uses fixed 70°).
    let speed = Vec3::new(lin_vel.x, lin_vel.y, lin_vel.z).length();
    let target_fov = match state.mode {
        CameraMode::Chase => {
            let t = (speed / 20.0).clamp(0.0, 1.0);
            (60.0_f32 + t * 18.0).to_radians() // 60° → 78°
        }
        CameraMode::Cockpit => 70_f32.to_radians(),
    };
    state.smooth_fov = state.smooth_fov + (target_fov - state.smooth_fov) * alpha;

    if let Projection::Perspective(ref mut persp) = *proj {
        persp.fov = state.smooth_fov;
    }
}
