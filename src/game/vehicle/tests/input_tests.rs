use bevy::prelude::*;
use bevy::input::Input;
use bevy::input::keyboard::KeyCode;
use bevy::input::gamepad::{Gamepad, GamepadButton, GamepadButtonType, GamepadAxis, GamepadAxisType, Gamepads};

use crate::game::vehicle::{
    Vehicle,
    input::{VehicleInput, VehicleInputConfig, handle_vehicle_input, apply_vehicle_input},
    wheel::Wheel,
    drivetrain::{Drivetrain, GearState},
};

#[test]
fn test_vehicle_input_defaults() {
    let input = VehicleInput::default();
    assert_eq!(input.throttle, 0.0);
    assert_eq!(input.brake, 0.0);
    assert_eq!(input.steering, 0.0);
    assert!(!input.handbrake);
    assert!(!input.gear_up);
    assert!(!input.gear_down);
    assert!(!input.differential_lock);
}

#[test]
fn test_keyboard_input_handling() {
    // Create a minimal Bevy app for testing
    let mut app = App::new();
    
    // Add required resources
    app.init_resource::<VehicleInputConfig>()
       .init_resource::<Time>();

    // Add keyboard input resource
    let mut keyboard = Input::<KeyCode>::default();
    app.insert_resource(keyboard.clone());

    // Create test entity with vehicle components
    let vehicle_entity = app.world.spawn((
        VehicleInput::default(),
        Vehicle::default(),
    )).id();

    // Run input system once to initialize
    app.add_systems(Update, handle_vehicle_input);
    app.update();

    // Test throttle input
    {
        let mut keyboard = app.world.resource_mut::<Input<KeyCode>>();
        keyboard.press(KeyCode::W);
        
        // Run system for 0.5 seconds
        let mut time = app.world.resource_mut::<Time>();
        time.update();
        time.set_delta(0.5);
        
        app.update();

        let input = app.world.entity(vehicle_entity).get::<VehicleInput>().unwrap();
        assert!(input.throttle > 0.0, "Throttle should increase when W is pressed");
    }

    // Test brake input
    {
        let mut keyboard = app.world.resource_mut::<Input<KeyCode>>();
        keyboard.clear();
        keyboard.press(KeyCode::S);
        
        app.update();

        let input = app.world.entity(vehicle_entity).get::<VehicleInput>().unwrap();
        assert!(input.brake > 0.0, "Brake should increase when S is pressed");
    }

    // Test steering input
    {
        let mut keyboard = app.world.resource_mut::<Input<KeyCode>>();
        keyboard.clear();
        keyboard.press(KeyCode::D);
        
        app.update();

        let input = app.world.entity(vehicle_entity).get::<VehicleInput>().unwrap();
        assert!(input.steering > 0.0, "Steering should be positive when D is pressed");
    }

    // Test handbrake toggle
    {
        let mut keyboard = app.world.resource_mut::<Input<KeyCode>>();
        keyboard.clear();
        keyboard.press(KeyCode::Space);
        keyboard.update();  // Simulate frame update
        
        app.update();

        let input = app.world.entity(vehicle_entity).get::<VehicleInput>().unwrap();
        assert!(input.handbrake, "Handbrake should toggle on when Space is pressed");
    }
}

#[test]
fn test_gamepad_input_handling() {
    let mut app = App::new();
    
    // Add required resources
    app.init_resource::<VehicleInputConfig>()
       .init_resource::<Time>()
       .init_resource::<Gamepads>()
       .init_resource::<Input<GamepadButton>>()
       .init_resource::<Axis<GamepadAxis>>();

    // Create test gamepad
    let gamepad = Gamepad::new(0);
    app.world.resource_mut::<Gamepads>().register(gamepad);

    // Create test entity
    let vehicle_entity = app.world.spawn((
        VehicleInput::default(),
        Vehicle::default(),
    )).id();

    // Add input system
    app.add_systems(Update, handle_vehicle_input);

    // Test throttle axis
    {
        let mut axis = app.world.resource_mut::<Axis<GamepadAxis>>();
        let throttle_axis = GamepadAxis {
            gamepad,
            axis_type: GamepadAxisType::RightZ,
        };
        axis.set(throttle_axis, 0.75);
        
        app.update();

        let input = app.world.entity(vehicle_entity).get::<VehicleInput>().unwrap();
        assert!(input.throttle > 0.7, "Throttle should match gamepad axis value");
    }

    // Test steering axis
    {
        let mut axis = app.world.resource_mut::<Axis<GamepadAxis>>();
        let steering_axis = GamepadAxis {
            gamepad,
            axis_type: GamepadAxisType::LeftStickX,
        };
        axis.set(steering_axis, -0.5);
        
        app.update();

        let input = app.world.entity(vehicle_entity).get::<VehicleInput>().unwrap();
        assert!(input.steering < 0.0, "Steering should be negative for left input");
    }
}

#[test]
fn test_input_application() {
    let mut app = App::new();
    
    // Add required resources
    app.init_resource::<Time>();

    // Create test wheels
    let wheel_entities: Vec<Entity> = (0..4).map(|_| {
        app.world.spawn(Wheel::default()).id()
    }).collect();

    // Create test vehicle with input
    let mut input = VehicleInput::default();
    input.throttle = 0.5;
    input.steering = 0.25;
    
    let vehicle_entity = app.world.spawn((
        input,
        Vehicle::default(),
        Drivetrain::default(),
    )).id();

    // Add application system
    app.add_systems(Update, apply_vehicle_input);
    app.update();

    // Check wheel torques were applied
    for wheel_entity in wheel_entities {
        let wheel = app.world.entity(wheel_entity).get::<Wheel>().unwrap();
        if wheel_entity.index() < 2 {
            assert!(wheel.steering_angle != 0.0, "Front wheels should have steering angle applied");
        }
    }

    // Test gear shifting
    {
        let mut vehicle_input = app.world.entity_mut(vehicle_entity);
        let mut input = vehicle_input.get_mut::<VehicleInput>().unwrap();
        let mut drivetrain = vehicle_input.get_mut::<Drivetrain>().unwrap();
        
        // Set up for gear shift
        drivetrain.gear_state = GearState::Engaged;
        input.gear_up = true;
        
        app.update();

        let drivetrain = app.world.entity(vehicle_entity).get::<Drivetrain>().unwrap();
        assert!(drivetrain.current_gear > 1, "Gear should increase after shift up");
    }
}

#[test]
fn test_input_smoothing() {
    let mut app = App::new();
    
    // Add required resources
    app.init_resource::<VehicleInputConfig>()
       .init_resource::<Time>();

    // Create test entity
    let vehicle_entity = app.world.spawn((
        VehicleInput::default(),
        Vehicle::default(),
    )).id();

    // Add input system
    app.add_systems(Update, handle_vehicle_input);

    // Test steering smoothing
    {
        let mut keyboard = app.world.resource_mut::<Input<KeyCode>>();
        keyboard.press(KeyCode::D);
        
        // Run system multiple times with time steps
        let mut time = app.world.resource_mut::<Time>();
        time.update();
        time.set_delta(0.016); // ~60 FPS
        
        let mut prev_steering = 0.0;
        for _ in 0..5 {
            app.update();
            let input = app.world.entity(vehicle_entity).get::<VehicleInput>().unwrap();
            assert!(input.steering > prev_steering, "Steering should increase smoothly");
            prev_steering = input.steering;
        }

        // Test return to center
        keyboard.clear();
        
        for _ in 0..5 {
            app.update();
            let input = app.world.entity(vehicle_entity).get::<VehicleInput>().unwrap();
            assert!(input.steering < prev_steering, "Steering should return to center");
            prev_steering = input.steering;
        }
    }
} 