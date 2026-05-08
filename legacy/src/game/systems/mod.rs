use bevy::prelude::*;
use crate::game::{
    resources::{InputState, VehicleState, DebugInfo},
    constants::*,
    state::GameState,
};

pub mod loading;
pub mod menu;
pub mod game;
pub mod pause;

/// Initial setup system that runs on startup
pub fn setup(mut commands: Commands) {
    // Initialize any global resources or entities needed at startup
    info!("Initializing game systems...");

    // Setup basic scene
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });

    // Add light
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_4)),
        ..default()
    });
}

/// System to handle core input that affects game state
pub fn handle_input(
    keyboard: Res<Input<KeyCode>>,
    mut game_state: ResMut<GameState>,
) {
    // Handle pause/unpause
    if keyboard.just_pressed(KeyCode::Escape) {
        game_state.toggle_pause();
    }
}

/// System for handling input and updating input state
pub fn handle_input_old(
    keyboard: Res<Input<KeyCode>>,
    mut input_state: ResMut<InputState>,
) {
    // Throttle
    input_state.throttle = if keyboard.pressed(KeyCode::W) { 1.0 } else { 0.0 };
    
    // Brake
    input_state.brake = if keyboard.pressed(KeyCode::S) { 1.0 } else { 0.0 };
    
    // Steering
    input_state.steering = match (keyboard.pressed(KeyCode::A), keyboard.pressed(KeyCode::D)) {
        (true, false) => -1.0,  // Left
        (false, true) => 1.0,   // Right
        _ => 0.0,               // Neutral
    };
    
    // Handbrake
    input_state.handbrake = keyboard.pressed(KeyCode::Space);
}

/// System for updating vehicle physics
pub fn update_vehicle_physics(
    time: Res<Time>,
    input_state: Res<InputState>,
    mut vehicle_state: ResMut<VehicleState>,
) {
    let dt = time.delta_seconds();
    
    // Apply engine force
    let engine_force = input_state.throttle * MAX_ENGINE_FORCE;
    
    // Apply brake force
    let brake_force = input_state.brake * MAX_BRAKE_FORCE;
    
    // Update wheel speeds based on forces
    for wheel_speed in vehicle_state.wheel_speeds.iter_mut() {
        let drive_torque = engine_force * dt;
        let brake_torque = brake_force * dt;
        *wheel_speed += drive_torque - brake_torque;
        
        // Apply rolling resistance
        *wheel_speed *= (1.0 - ROLLING_RESISTANCE * dt).max(0.0);
    }
    
    // Update vehicle velocity
    let drag = -vehicle_state.velocity * DRAG_COEFFICIENT * vehicle_state.velocity.length();
    vehicle_state.velocity += drag * dt;
    
    // Apply steering
    let steering_angle = input_state.steering * MAX_STEERING_ANGLE;
    vehicle_state.angular_velocity.y = steering_angle * vehicle_state.velocity.length() / WHEELBASE;
}

/// System for updating game state
pub fn update_game_state(
    time: Res<Time>,
    mut game_state: ResMut<GameState>,
) {
    game_state.update(time.delta_seconds());
}

/// System for updating debug information
pub fn update_debug_info(
    time: Res<Time>,
    mut debug_info: ResMut<DebugInfo>,
) {
    let dt = time.delta_seconds();
    debug_info.frame_time = dt;
    debug_info.fps = 1.0 / dt;
    
    // Update custom metrics example
    debug_info.custom_metrics.insert("example_metric".to_string(), dt * 1000.0);
}

/// Plugin for registering all game systems
pub struct GameSystemsPlugin;

impl Plugin for GameSystemsPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(Update, (
                handle_input,
                update_vehicle_physics,
                update_game_state,
                update_debug_info,
            ));
    }
} 