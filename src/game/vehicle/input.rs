use bevy::prelude::*;
use bevy::input::Input;
use bevy::input::keyboard::KeyCode;
use bevy::input::gamepad::{Gamepad, GamepadButton, GamepadButtonType, GamepadAxis, GamepadAxisType};

use crate::game::vehicle::{
    Vehicle,
    wheel::Wheel,
    drivetrain::{Drivetrain, GearState},
};

/// Component for storing vehicle input state
#[derive(Component, Default)]
pub struct VehicleInput {
    pub throttle: f32,      // 0.0 to 1.0
    pub brake: f32,         // 0.0 to 1.0
    pub steering: f32,      // -1.0 to 1.0
    pub handbrake: bool,    // Handbrake engaged
    pub gear_up: bool,      // Request gear shift up
    pub gear_down: bool,    // Request gear shift down
    pub differential_lock: bool, // Differential lock engaged
}

/// Resource for storing input configuration
#[derive(Resource)]
pub struct VehicleInputConfig {
    pub keyboard_config: KeyboardConfig,
    pub gamepad_config: GamepadConfig,
    pub steering_sensitivity: f32,
    pub steering_speed: f32,      // How quickly steering responds
    pub steering_return_speed: f32, // How quickly steering returns to center
    pub throttle_sensitivity: f32,
    pub brake_sensitivity: f32,
}

impl Default for VehicleInputConfig {
    fn default() -> Self {
        Self {
            keyboard_config: KeyboardConfig::default(),
            gamepad_config: GamepadConfig::default(),
            steering_sensitivity: 1.0,
            steering_speed: 5.0,
            steering_return_speed: 3.0,
            throttle_sensitivity: 1.0,
            brake_sensitivity: 1.0,
        }
    }
}

#[derive(Default)]
pub struct KeyboardConfig {
    pub throttle: KeyCode,
    pub brake: KeyCode,
    pub steer_left: KeyCode,
    pub steer_right: KeyCode,
    pub handbrake: KeyCode,
    pub gear_up: KeyCode,
    pub gear_down: KeyCode,
    pub diff_lock: KeyCode,
}

impl Default for KeyboardConfig {
    fn default() -> Self {
        Self {
            throttle: KeyCode::W,
            brake: KeyCode::S,
            steer_left: KeyCode::A,
            steer_right: KeyCode::D,
            handbrake: KeyCode::Space,
            gear_up: KeyCode::E,
            gear_down: KeyCode::Q,
            diff_lock: KeyCode::L,
        }
    }
}

#[derive(Default)]
pub struct GamepadConfig {
    pub throttle_axis: GamepadAxisType,
    pub brake_axis: GamepadAxisType,
    pub steering_axis: GamepadAxisType,
    pub handbrake: GamepadButtonType,
    pub gear_up: GamepadButtonType,
    pub gear_down: GamepadButtonType,
    pub diff_lock: GamepadButtonType,
}

impl Default for GamepadConfig {
    fn default() -> Self {
        Self {
            throttle_axis: GamepadAxisType::RightZ,    // Right trigger
            brake_axis: GamepadAxisType::LeftZ,        // Left trigger
            steering_axis: GamepadAxisType::LeftStickX,
            handbrake: GamepadButtonType::South,       // A/X button
            gear_up: GamepadButtonType::RightTrigger2, // Right bumper
            gear_down: GamepadButtonType::LeftTrigger2,// Left bumper
            diff_lock: GamepadButtonType::West,        // X/Square button
        }
    }
}

/// System to handle keyboard and gamepad input for vehicles
pub fn handle_vehicle_input(
    mut vehicle_query: Query<(&mut VehicleInput, &Vehicle)>,
    keyboard: Res<Input<KeyCode>>,
    gamepads: Res<Gamepads>,
    gamepad_input: Res<Input<GamepadButton>>,
    gamepad_axis: Res<Axis<GamepadAxis>>,
    config: Res<VehicleInputConfig>,
    time: Res<Time>,
) {
    let dt = time.delta_seconds();

    for (mut input, _vehicle) in vehicle_query.iter_mut() {
        // Reset one-shot inputs
        input.gear_up = false;
        input.gear_down = false;

        // Handle keyboard input
        let kb = &config.keyboard_config;
        
        // Throttle and brake (keyboard)
        if keyboard.pressed(kb.throttle) {
            input.throttle = (input.throttle + config.throttle_sensitivity * dt).min(1.0);
        } else {
            input.throttle = (input.throttle - config.throttle_sensitivity * dt).max(0.0);
        }

        if keyboard.pressed(kb.brake) {
            input.brake = (input.brake + config.brake_sensitivity * dt).min(1.0);
        } else {
            input.brake = (input.brake - config.brake_sensitivity * dt).max(0.0);
        }

        // Steering (keyboard)
        let mut steering_target = 0.0;
        if keyboard.pressed(kb.steer_left) { steering_target -= 1.0; }
        if keyboard.pressed(kb.steer_right) { steering_target += 1.0; }

        // Gear shifts (keyboard)
        if keyboard.just_pressed(kb.gear_up) { input.gear_up = true; }
        if keyboard.just_pressed(kb.gear_down) { input.gear_down = true; }

        // Toggles (keyboard)
        if keyboard.just_pressed(kb.handbrake) { input.handbrake = !input.handbrake; }
        if keyboard.just_pressed(kb.diff_lock) { input.differential_lock = !input.differential_lock; }

        // Handle gamepad input (if available)
        if let Some(gamepad) = gamepads.iter().next() {
            let gp = &config.gamepad_config;

            // Throttle and brake (gamepad)
            let throttle_axis = GamepadAxis { gamepad, axis_type: gp.throttle_axis };
            let brake_axis = GamepadAxis { gamepad, axis_type: gp.brake_axis };
            
            if let Some(throttle_value) = gamepad_axis.get(throttle_axis) {
                input.throttle = throttle_value.max(0.0);
            }
            if let Some(brake_value) = gamepad_axis.get(brake_axis) {
                input.brake = brake_value.max(0.0);
            }

            // Steering (gamepad)
            let steering_axis = GamepadAxis { gamepad, axis_type: gp.steering_axis };
            if let Some(axis_value) = gamepad_axis.get(steering_axis) {
                steering_target = axis_value;
            }

            // Gear shifts (gamepad)
            let gear_up_btn = GamepadButton { gamepad, button_type: gp.gear_up };
            let gear_down_btn = GamepadButton { gamepad, button_type: gp.gear_down };
            if gamepad_input.just_pressed(gear_up_btn) { input.gear_up = true; }
            if gamepad_input.just_pressed(gear_down_btn) { input.gear_down = true; }

            // Toggles (gamepad)
            let handbrake_btn = GamepadButton { gamepad, button_type: gp.handbrake };
            let diff_lock_btn = GamepadButton { gamepad, button_type: gp.diff_lock };
            if gamepad_input.just_pressed(handbrake_btn) { input.handbrake = !input.handbrake; }
            if gamepad_input.just_pressed(diff_lock_btn) { input.differential_lock = !input.differential_lock; }
        }

        // Apply steering with smoothing
        let steering_delta = if steering_target != 0.0 {
            (steering_target - input.steering) * config.steering_speed
        } else {
            -input.steering * config.steering_return_speed
        };
        input.steering = (input.steering + steering_delta * dt).clamp(-1.0, 1.0);
    }
}

/// System to apply vehicle inputs to the physics simulation
pub fn apply_vehicle_input(
    mut vehicle_query: Query<(&VehicleInput, &mut Vehicle, &mut Drivetrain)>,
    mut wheel_query: Query<&mut Wheel>,
    time: Res<Time>,
) {
    let dt = time.delta_seconds();

    for (input, mut vehicle, mut drivetrain) in vehicle_query.iter_mut() {
        // Handle gear changes
        if input.gear_up && drivetrain.gear_state == GearState::Engaged {
            drivetrain.shift_up();
        }
        if input.gear_down && drivetrain.gear_state == GearState::Engaged {
            drivetrain.shift_down();
        }

        // Calculate engine torque based on throttle input
        let engine_torque = if drivetrain.gear_state == GearState::Engaged {
            drivetrain.calculate_engine_torque(input.throttle)
        } else {
            0.0
        };

        // Distribute torque to wheels based on differential settings
        let wheel_torques = drivetrain.distribute_torque(
            engine_torque,
            input.differential_lock,
        );

        // Apply steering angle to wheels
        let (max_angle, _) = vehicle.get_ackermann_angles(input.steering);

        // Update wheel properties
        for (wheel_index, mut wheel) in wheel_query.iter_mut().enumerate() {
            // Apply drive torque
            wheel.drive_torque = wheel_torques[wheel_index];

            // Apply brake torque
            let brake_torque = if input.handbrake && (wheel_index == 2 || wheel_index == 3) {
                // Handbrake affects rear wheels only
                vehicle.handbrake_torque
            } else {
                input.brake * vehicle.brake_torque
            };
            wheel.brake_torque = brake_torque;

            // Apply steering to front wheels
            if wheel_index < 2 {
                // Calculate individual wheel steering angle using Ackermann geometry
                let steering_sign = if wheel_index == 0 { 1.0 } else { -1.0 };
                wheel.steering_angle = max_angle * steering_sign;
            }
        }
    }
}

/// Plugin to register vehicle input systems
pub struct VehicleInputPlugin;

impl Plugin for VehicleInputPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<VehicleInputConfig>()
           .add_systems(Update, (
               handle_vehicle_input,
               apply_vehicle_input.after(handle_vehicle_input),
           ));
    }
} 