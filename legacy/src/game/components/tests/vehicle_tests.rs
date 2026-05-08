use bevy::prelude::*;
use bevy::math::Vec3;
use crate::game::components::{Vehicle, Suspension, Wheel};

// Helper function to create a test vehicle with default settings
fn create_test_vehicle() -> Vehicle {
    Vehicle {
        mass: 1500.0,
        max_speed: 150.0,
        acceleration: 10.0,
        brake_force: 20.0,
        steering_angle: 0.0,
        max_steering_angle: 45.0,
        steering_speed: 2.0,
        ..Default::default()
    }
}

#[test]
fn test_vehicle_default() {
    let vehicle = Vehicle::default();
    assert!(vehicle.mass > 0.0);
    assert!(vehicle.max_speed > 0.0);
    assert!(vehicle.acceleration > 0.0);
    assert!(vehicle.brake_force > 0.0);
    assert_eq!(vehicle.steering_angle, 0.0);
    assert!(vehicle.max_steering_angle > 0.0);
    assert!(vehicle.steering_speed > 0.0);
}

#[test]
fn test_suspension_default() {
    let suspension = Suspension::default();
    assert!(suspension.spring_stiffness > 0.0);
    assert!(suspension.damping > 0.0);
    assert!(suspension.rest_length > 0.0);
    assert!(suspension.max_compression > 0.0);
    assert!(suspension.max_extension > suspension.rest_length);
}

#[test]
fn test_wheel_default() {
    let wheel = Wheel::default();
    assert!(wheel.radius > 0.0);
    assert!(wheel.width > 0.0);
    assert_eq!(wheel.position, Vec3::ZERO);
    assert!(!wheel.is_front);
    assert_eq!(wheel.rotation_angle, 0.0);
}

#[test]
fn test_wheel_factory_methods() {
    let front_left = Wheel::front_left(0.3);
    assert!(front_left.is_front);
    assert!(front_left.position.x < 0.0); // Left side
    assert!(front_left.position.z < 0.0); // Front

    let rear_right = Wheel::rear_right(0.3);
    assert!(!rear_right.is_front);
    assert!(rear_right.position.x > 0.0); // Right side
    assert!(rear_right.position.z > 0.0); // Rear
}

#[test]
fn test_wheel_custom_configuration() {
    let custom_wheel = Wheel {
        radius: 0.4,
        width: 0.2,
        position: Vec3::new(1.0, -0.5, 2.0),
        is_front: true,
        rotation_angle: 45.0,
        ..Default::default()
    };

    assert_eq!(custom_wheel.radius, 0.4);
    assert_eq!(custom_wheel.width, 0.2);
    assert_eq!(custom_wheel.position, Vec3::new(1.0, -0.5, 2.0));
    assert!(custom_wheel.is_front);
    assert_eq!(custom_wheel.rotation_angle, 45.0);
}

#[test]
fn test_vehicle_movement() {
    let mut vehicle = create_test_vehicle();
    let initial_speed = vehicle.current_speed;
    
    // Test acceleration
    vehicle.accelerate(1.0, 0.016); // 16ms frame time
    assert!(vehicle.current_speed > initial_speed);
    assert!(vehicle.current_speed <= vehicle.max_speed);

    // Test braking
    let speed_before_brake = vehicle.current_speed;
    vehicle.brake(1.0, 0.016);
    assert!(vehicle.current_speed < speed_before_brake);

    // Test steering
    let initial_angle = vehicle.steering_angle;
    vehicle.steer(1.0, 0.016); // Turn right
    assert!(vehicle.steering_angle > initial_angle);
    assert!(vehicle.steering_angle <= vehicle.max_steering_angle);
}

#[test]
fn test_suspension_physics() {
    let mut suspension = Suspension::default();
    let ground_height = -1.0;
    let vehicle_height = 0.0;
    
    // Test compression force calculation
    let force = suspension.calculate_force(vehicle_height, ground_height);
    assert!(force >= 0.0); // Force should push upward
    
    // Test suspension compression limits
    let max_compression_force = suspension.calculate_force(
        ground_height + suspension.max_compression,
        ground_height
    );
    assert!(max_compression_force > 0.0);
    
    // Test suspension extension limits
    let max_extension_force = suspension.calculate_force(
        ground_height + suspension.max_extension,
        ground_height
    );
    assert!(max_extension_force <= 0.0);
}

#[test]
fn test_vehicle_suspension_integration() {
    let mut vehicle = create_test_vehicle();
    let mut suspension = Suspension::default();
    
    // Test suspension response to vehicle movement
    vehicle.accelerate(1.0, 0.016);
    let force = suspension.calculate_force(
        vehicle.transform.translation.y,
        vehicle.transform.translation.y - suspension.rest_length
    );
    
    assert!(force != 0.0); // Suspension should respond to vehicle movement
    assert!(suspension.current_compression >= 0.0);
    assert!(suspension.current_compression <= suspension.max_compression);
} 