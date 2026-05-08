// Camera modes: V key cycles through 5 camera perspectives.
//
//   Chase       (default — current behavior, behind+above chassis)
//   WheelFL     (mounted just outside front-left tire, looking inward+forward)
//   WheelFR     (front-right mirror)
//   FirstPerson (interior driver POV)
//   FreeOrbit   (orbit around chassis at fixed 8 m radius / 4 m height)
//
// Public API:
//   CameraModesPlugin
//   CameraMode (enum)
//   CameraModesState (resource)
//
// Sprint 31 — last-writer-wins: apply_camera_mode runs in Update after
// camera.rs's update_camera and overrides the Camera3d Transform for all
// non-Chase modes.

use bevy::prelude::*;
use crate::camera::CameraSet;
use crate::vehicle::{Chassis, VehicleRoot};

// ---- Plugin -----------------------------------------------------------------

pub struct CameraModesPlugin;

impl Plugin for CameraModesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CameraModesState>()
            .add_systems(Startup, spawn_cam_indicator)
            .add_systems(
                Update,
                (cycle_with_v, apply_camera_mode, update_indicator)
                    .chain()
                    .after(CameraSet),
            );
    }
}

// ---- Public types -----------------------------------------------------------

#[derive(Resource, Default, Clone, Copy)]
pub struct CameraModesState {
    pub mode: CameraMode,
}

#[derive(Default, Clone, Copy, PartialEq, Eq, Debug)]
pub enum CameraMode {
    #[default]
    Chase,
    WheelFL,
    WheelFR,
    FirstPerson,
    FreeOrbit,
}

// ---- Components -------------------------------------------------------------

/// Marker on the camera-mode indicator text node.
#[derive(Component)]
struct CamModeIndicator;

// ---- Startup ----------------------------------------------------------------

fn spawn_cam_indicator(mut commands: Commands) {
    // Small semi-transparent badge at top-left below the main HUD panel.
    // Positioned at left:12, top:220 to avoid overlapping the main HUD.
    let container = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(12.0),
                top: Val::Px(220.0),
                padding: UiRect::axes(Val::Px(8.0), Val::Px(4.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.05, 0.05, 0.07, 0.70)),
        ))
        .id();

    let label = commands
        .spawn((
            CamModeIndicator,
            Text::new("CAM: Chase"),
            TextFont {
                font_size: 12.0,
                ..default()
            },
            TextColor(Color::srgb(0.9, 0.9, 0.75)),
        ))
        .id();

    commands.entity(container).add_child(label);
}

// ---- System: cycle on V key -------------------------------------------------

fn cycle_with_v(
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<CameraModesState>,
) {
    if keys.just_pressed(KeyCode::KeyV) {
        state.mode = match state.mode {
            CameraMode::Chase       => CameraMode::WheelFL,
            CameraMode::WheelFL     => CameraMode::WheelFR,
            CameraMode::WheelFR     => CameraMode::FirstPerson,
            CameraMode::FirstPerson => CameraMode::FreeOrbit,
            CameraMode::FreeOrbit   => CameraMode::Chase,
        };
        info!("camera mode: {:?}", state.mode);
    }
}

// ---- System: apply camera transform -----------------------------------------

fn apply_camera_mode(
    state:      Res<CameraModesState>,
    vehicle:    Option<Res<VehicleRoot>>,
    chassis_q:  Query<&Transform, With<Chassis>>,
    mut cam_q:  Query<&mut Transform, (With<Camera3d>, Without<Chassis>)>,
    time:       Res<Time>,
    // Smoothing cache for WheelFL / WheelFR (motion-sickness mitigation).
    mut prev_cam: Local<Option<Transform>>,
) {
    // Chase: leave camera.rs's output untouched.
    if state.mode == CameraMode::Chase {
        *prev_cam = None;
        return;
    }

    let Some(vehicle) = vehicle else { return };
    let Ok(chassis)   = chassis_q.get(vehicle.chassis) else { return };
    let Ok(mut cam)   = cam_q.single_mut() else { return };

    let chassis_pos = chassis.translation;
    let chassis_rot = chassis.rotation;

    // Chassis basis vectors.
    let chassis_right = chassis_rot * Vec3::X;       // +X = right
    let chassis_fwd   = chassis_rot * Vec3::NEG_Z;   // -Z = forward in Bevy convention
    let chassis_up    = chassis_rot * Vec3::Y;

    let new_tf = match state.mode {
        CameraMode::Chase => unreachable!(), // handled above

        // ---- Wheel-FL -------------------------------------------------------
        // Camera sits outside the FL wheel, slightly in front and above,
        // looking back-and-down at the wheel + ground ahead so the player
        // sees the tire turn, articulate, and kick mud forward.
        // FL wheel local pos = (-1.1, -0.35, -1.4); chassis_fwd is -Z so
        // local_z = -1.4 means +1.4 along chassis_fwd (forward of chassis).
        CameraMode::WheelFL => {
            let cam_pos = chassis_pos
                + chassis_right * (-2.2)
                + chassis_fwd   * 0.6
                + chassis_up    * 0.2;
            let look_target = chassis_pos
                + chassis_right * (-1.1)
                + chassis_fwd   * 1.4
                + chassis_up    * (-0.4);
            Transform::from_translation(cam_pos).looking_at(look_target, Vec3::Y)
        }

        // ---- Wheel-FR -------------------------------------------------------
        CameraMode::WheelFR => {
            let cam_pos = chassis_pos
                + chassis_right * 2.2
                + chassis_fwd   * 0.6
                + chassis_up    * 0.2;
            let look_target = chassis_pos
                + chassis_right * 1.1
                + chassis_fwd   * 1.4
                + chassis_up    * (-0.4);
            Transform::from_translation(cam_pos).looking_at(look_target, Vec3::Y)
        }

        // ---- First-person (driver POV) --------------------------------------
        CameraMode::FirstPerson => {
            let cam_pos = chassis_pos
                + chassis_up  * 0.6
                + chassis_fwd * (-0.5);
            Transform::from_translation(cam_pos)
                .looking_at(cam_pos + chassis_fwd * 5.0, chassis_up)
        }

        // ---- Free orbit (slow auto-rotation) --------------------------------
        CameraMode::FreeOrbit => {
            let angle   = time.elapsed_secs() * 0.3;
            let cam_pos = chassis_pos
                + Vec3::new(angle.cos() * 8.0, 4.0, angle.sin() * 8.0);
            let look_target = chassis_pos + Vec3::Y * 1.0;
            Transform::from_translation(cam_pos).looking_at(look_target, Vec3::Y)
        }
    };

    // Wheel cams: lerp position to dampen wheel-bump jitter (M1 comfort).
    let final_tf = match state.mode {
        CameraMode::WheelFL | CameraMode::WheelFR => {
            let smoothed_pos = if let Some(prev) = *prev_cam {
                prev.translation.lerp(new_tf.translation, 0.4)
            } else {
                new_tf.translation
            };
            Transform::from_translation(smoothed_pos)
                .looking_at(
                    // Re-derive look target from the (possibly lerped) position
                    // by mirroring the orientation of new_tf.
                    smoothed_pos + new_tf.forward() * 5.0,
                    Vec3::Y,
                )
        }
        _ => new_tf,
    };

    *prev_cam = Some(final_tf);
    *cam = final_tf;
}

// ---- System: update HUD indicator text --------------------------------------

fn update_indicator(
    state:    Res<CameraModesState>,
    mut text_q: Query<&mut Text, With<CamModeIndicator>>,
) {
    let Ok(mut text) = text_q.single_mut() else { return };
    let label = match state.mode {
        CameraMode::Chase       => "CAM: Chase",
        CameraMode::WheelFL     => "CAM: Wheel FL",
        CameraMode::WheelFR     => "CAM: Wheel FR",
        CameraMode::FirstPerson => "CAM: First Person",
        CameraMode::FreeOrbit   => "CAM: Free Orbit",
    };
    text.0 = label.to_string();
}
