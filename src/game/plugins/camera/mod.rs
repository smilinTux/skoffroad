use bevy::prelude::*;
use crate::game::plugins::InputState;

/// Camera settings for controlling behavior
#[derive(Resource)]
pub struct CameraSettings {
    pub follow_distance: f32,
    pub follow_height: f32,
    pub follow_smoothness: f32,
    pub rotation_sensitivity: f32,
    pub zoom_sensitivity: f32,
    pub min_zoom: f32,
    pub max_zoom: f32,
}

impl Default for CameraSettings {
    fn default() -> Self {
        Self {
            follow_distance: 10.0,
            follow_height: 5.0,
            follow_smoothness: 0.1,
            rotation_sensitivity: 0.005,
            zoom_sensitivity: 0.5,
            min_zoom: 5.0,
            max_zoom: 20.0,
        }
    }
}

/// Component for marking the main game camera
#[derive(Component)]
pub struct GameCamera {
    pub target: Option<Entity>,
    pub orbit_angle: Vec2, // (yaw, pitch)
    pub current_zoom: f32,
}

impl Default for GameCamera {
    fn default() -> Self {
        Self {
            target: None,
            orbit_angle: Vec2::new(0.0, std::f32::consts::FRAC_PI_4),
            current_zoom: 10.0,
        }
    }
}

/// Plugin for managing camera systems
pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CameraSettings>()
            .add_systems(Startup, setup_camera)
            .add_systems(Update, (
                update_camera_position,
                update_camera_rotation,
                update_camera_zoom,
            ));
    }
}

/// Sets up the main game camera
fn setup_camera(mut commands: Commands) {
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(0.0, 5.0, 10.0)
                .looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        },
        GameCamera::default(),
    ));
}

/// Updates camera position based on target and settings
fn update_camera_position(
    mut camera_query: Query<(&mut Transform, &GameCamera), With<Camera3d>>,
    target_query: Query<&Transform, Without<Camera3d>>,
    settings: Res<CameraSettings>,
    time: Res<Time>,
) {
    for (mut camera_transform, game_camera) in camera_query.iter_mut() {
        if let Some(target_entity) = game_camera.target {
            if let Ok(target_transform) = target_query.get(target_entity) {
                let target_pos = target_transform.translation;
                
                // Calculate desired camera position
                let yaw = game_camera.orbit_angle.x;
                let pitch = game_camera.orbit_angle.y;
                let zoom = game_camera.current_zoom;
                
                let offset = Vec3::new(
                    zoom * yaw.cos() * pitch.cos(),
                    zoom * pitch.sin(),
                    zoom * yaw.sin() * pitch.cos(),
                );
                
                let desired_pos = target_pos + offset;
                
                // Smoothly interpolate to desired position
                camera_transform.translation = camera_transform.translation.lerp(
                    desired_pos,
                    settings.follow_smoothness * time.delta_seconds(),
                );
                
                // Look at target
                camera_transform.look_at(target_pos, Vec3::Y);
            }
        }
    }
}

/// Updates camera rotation based on input
fn update_camera_rotation(
    mut camera_query: Query<&mut GameCamera>,
    input: Res<InputState>,
    settings: Res<CameraSettings>,
) {
    for mut game_camera in camera_query.iter_mut() {
        let rotation_delta = input.camera_rotate * settings.rotation_sensitivity;
        game_camera.orbit_angle += rotation_delta;
        
        // Clamp pitch to prevent camera flipping
        game_camera.orbit_angle.y = game_camera.orbit_angle.y
            .clamp(-std::f32::consts::FRAC_PI_2, std::f32::consts::FRAC_PI_2);
    }
}

/// Updates camera zoom based on input
fn update_camera_zoom(
    mut camera_query: Query<&mut GameCamera>,
    input: Res<InputState>,
    settings: Res<CameraSettings>,
) {
    for mut game_camera in camera_query.iter_mut() {
        game_camera.current_zoom += input.camera_zoom * settings.zoom_sensitivity;
        game_camera.current_zoom = game_camera.current_zoom
            .clamp(settings.min_zoom, settings.max_zoom);
    }
} 