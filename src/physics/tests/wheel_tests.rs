use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use crate::physics::wheel::{Wheel, WheelForces, update_wheel_physics};
use crate::physics::suspension::Suspension;
use bevy::math::Vec3;
use crate::physics::{TerrainProperties, PhysicsTerrainType};
use crate::physics::tire_temperature::{TireTemperature, TireTemperatureState, TireWearState};

#[test]
fn test_wheel_default_values() {
    let wheel = Wheel::default();
    assert_eq!(wheel.radius, 0.4);
    assert_eq!(wheel.width, 0.3);
    assert_eq!(wheel.mass, 25.0);
    assert_eq!(wheel.mount_point, Vec3::ZERO);
    assert_eq!(wheel.position, Vec3::ZERO);
    assert_eq!(wheel.angular_velocity, 0.0);
    assert_eq!(wheel.steering_angle, 0.0);
    assert_eq!(wheel.drive_torque, 0.0);
    assert_eq!(wheel.brake_torque, 0.0);
    assert_eq!(wheel.rolling_resistance, 0.015);
    assert_eq!(wheel.grip_coefficient, 1.0);
}

#[test]
fn test_wheel_forces_default() {
    let forces = WheelForces::default();
    assert_eq!(forces.normal_force, Vec3::ZERO);
    assert_eq!(forces.lateral_force, Vec3::ZERO);
    assert_eq!(forces.longitudinal_force, Vec3::ZERO);
    assert_eq!(forces.ground_contact_point, None);
    assert_eq!(forces.slip_ratio, 0.0);
    assert_eq!(forces.slip_angle, 0.0);
    assert!(forces.terrain_properties.is_some());
}

fn setup_test_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(RapierPhysicsPlugin::<NoUserData>::default())
        .add_systems(Update, update_wheel_physics);
    app
}

#[test]
fn test_wheel_no_ground_contact() {
    let mut app = setup_test_app();
    
    // Add test entities
    let wheel_entity = app.world.spawn((
        Wheel::default(),
        WheelForces::default(),
        Suspension {
            spring_stiffness: 50000.0,
            damping: 5000.0,
            rest_length: 0.5,
            ..Default::default()
        },
        Transform::from_xyz(0.0, 1.0, 0.0),
        GlobalTransform::default(),
    )).id();
    
    // Run systems
    app.update();
    
    // Check results
    let forces = app.world.get::<WheelForces>(wheel_entity).unwrap();
    assert_eq!(forces.ground_contact_point, None);
    assert_eq!(forces.normal_force, Vec3::ZERO);
    assert_eq!(forces.lateral_force, Vec3::ZERO);
    assert_eq!(forces.longitudinal_force, Vec3::ZERO);
}

#[test]
fn test_wheel_drive_torque() {
    let mut app = setup_test_app();
    
    // Add ground plane
    app.world.spawn(Collider::cuboid(50.0, 0.1, 50.0));
    
    // Add test wheel with drive torque
    let wheel_entity = app.world.spawn((
        Wheel {
            drive_torque: 1000.0,
            ..Default::default()
        },
        WheelForces::default(),
        Suspension {
            spring_stiffness: 50000.0,
            damping: 5000.0,
            rest_length: 0.5,
            ..Default::default()
        },
        Transform::from_xyz(0.0, 0.5, 0.0),
        GlobalTransform::default(),
    )).id();
    
    // Run systems
    app.update();
    
    // Check results
    let wheel = app.world.get::<Wheel>(wheel_entity).unwrap();
    let forces = app.world.get::<WheelForces>(wheel_entity).unwrap();
    
    assert!(wheel.angular_velocity > 0.0, "Wheel should rotate with positive drive torque");
    assert!(forces.longitudinal_force.length() > 0.0, "Should generate forward force");
    assert!(forces.ground_contact_point.is_some(), "Should detect ground contact");
}

#[test]
fn test_wheel_braking() {
    let mut app = setup_test_app();
    
    // Add ground plane
    app.world.spawn(Collider::cuboid(50.0, 0.1, 50.0));
    
    // Add test wheel with initial angular velocity and brake torque
    let wheel_entity = app.world.spawn((
        Wheel {
            angular_velocity: 50.0,
            brake_torque: 1000.0,
            ..Default::default()
        },
        WheelForces::default(),
        Suspension {
            spring_stiffness: 50000.0,
            damping: 5000.0,
            rest_length: 0.5,
            ..Default::default()
        },
        Transform::from_xyz(0.0, 0.5, 0.0),
        GlobalTransform::default(),
    )).id();
    
    // Run systems
    app.update();
    
    // Check results
    let wheel = app.world.get::<Wheel>(wheel_entity).unwrap();
    assert!(wheel.angular_velocity < 50.0, "Wheel should slow down when braking");
}

#[test]
fn test_suspension_compression() {
    let mut app = setup_test_app();
    
    // Add ground plane
    app.world.spawn(Collider::cuboid(50.0, 0.1, 50.0));
    
    // Add test wheel
    let wheel_entity = app.world.spawn((
        Wheel::default(),
        WheelForces::default(),
        Suspension {
            spring_stiffness: 50000.0,
            damping: 5000.0,
            rest_length: 0.5,
            ..Default::default()
        },
        Transform::from_xyz(0.0, 0.3, 0.0), // Positioned to cause suspension compression
        GlobalTransform::default(),
    )).id();
    
    // Run systems
    app.update();
    
    // Check results
    let forces = app.world.get::<WheelForces>(wheel_entity).unwrap();
    assert!(forces.normal_force.length() > 0.0, "Should generate upward force under compression");
    assert!(forces.ground_contact_point.is_some(), "Should detect ground contact");
}

#[test]
fn test_wheel_slip_calculation() {
    let mut wheel = Wheel::default();
    wheel.angular_velocity = 10.0; // rad/s
    
    let vehicle_velocity = Vec3::new(5.0, 0.0, 0.0); // 5 m/s forward
    let contact_point = Vec3::ZERO;
    let forward = Vec3::X;
    let right = Vec3::Z;
    
    let slip = wheel.calculate_slip(vehicle_velocity, contact_point, forward, right);
    
    // Expected slip ratio: (rω - v) / |v| = (0.4 * 10 - 5) / 5 = -0.2
    let expected_slip_ratio = -0.2;
    assert!((slip.longitudinal - expected_slip_ratio).abs() < 0.001);
}

#[test]
fn test_terrain_friction_effects() {
    let mut wheel = Wheel::default();
    let mut forces = WheelForces::default();
    
    // Test different terrain types
    let terrains = vec![
        TerrainProperties::new(PhysicsTerrainType::Asphalt),
        TerrainProperties::new(PhysicsTerrainType::Mud),
        TerrainProperties::new(PhysicsTerrainType::Ice),
    ];
    
    let vehicle_velocity = Vec3::new(10.0, 0.0, 0.0);
    forces.normal_force = 1000.0;
    forces.contact_point = Some(Vec3::ZERO);
    
    let mut prev_force = f32::MAX;
    
    for terrain in terrains {
        forces.terrain_properties = Some(terrain);
        wheel.update_forces(&mut forces, vehicle_velocity, 0.016);
        
        // Force should be lower for lower friction surfaces
        assert!(forces.longitudinal_force.abs() < prev_force);
        prev_force = forces.longitudinal_force.abs();
    }
}

#[test]
fn test_wheel_torque_application() {
    let mut wheel = Wheel::default();
    let mut forces = WheelForces::default();
    
    wheel.drive_torque = 1000.0; // Apply drive torque
    forces.contact_point = Some(Vec3::ZERO);
    forces.normal_force = 1000.0;
    
    let initial_angular_velocity = wheel.angular_velocity;
    wheel.update_forces(&mut forces, Vec3::ZERO, 0.016);
    
    // Angular velocity should increase with positive drive torque
    assert!(wheel.angular_velocity > initial_angular_velocity);
    
    // Apply brake torque
    wheel.drive_torque = 0.0;
    wheel.brake_torque = 500.0;
    let velocity_before_brake = wheel.angular_velocity;
    wheel.update_forces(&mut forces, Vec3::ZERO, 0.016);
    
    // Angular velocity should decrease with brake torque
    assert!(wheel.angular_velocity < velocity_before_brake);
}

#[test]
fn test_tire_temperature_effects() {
    let mut app = setup_test_app();
    
    // Add ground plane
    app.world.spawn(Collider::cuboid(50.0, 0.1, 50.0));
    
    // Add test wheel with tire temperature
    let wheel_entity = app.world.spawn((
        Wheel {
            drive_torque: 1000.0,
            ..Default::default()
        },
        WheelForces::default(),
        TireTemperature::default(),
        Transform::from_xyz(0.0, 0.5, 0.0),
        GlobalTransform::default(),
    )).id();
    
    // Run multiple updates to heat up the tire
    for _ in 0..100 {
        app.update();
    }
    
    // Check results
    let tire_temp = app.world.get::<TireTemperature>(wheel_entity).unwrap();
    let forces = app.world.get::<WheelForces>(wheel_entity).unwrap();
    
    assert!(tire_temp.surface_temp > tire_temp.core_temp, "Surface should heat up faster than core");
    assert!(tire_temp.surface_temp > 20.0, "Tire should heat up from slip");
    assert!(forces.slip_power > 0.0, "Should generate heat from slip");
}

#[test]
fn test_tire_temperature_states() {
    let mut tire_temp = TireTemperature::default();
    
    // Test cold state
    assert_eq!(tire_temp.get_temperature_state(), TireTemperatureState::Cold);
    
    // Heat up to optimal
    tire_temp.surface_temp = 90.0;
    tire_temp.core_temp = 70.0;
    assert_eq!(tire_temp.get_temperature_state(), TireTemperatureState::Optimal);
    
    // Overheat
    tire_temp.surface_temp = 130.0;
    tire_temp.core_temp = 120.0;
    assert_eq!(tire_temp.get_temperature_state(), TireTemperatureState::Overheated);
}

#[test]
fn test_tire_grip_with_temperature() {
    let mut tire_temp = TireTemperature::default();
    let cold_grip = tire_temp.get_grip_multiplier();
    
    // Heat up to optimal temperature
    tire_temp.surface_temp = 80.0;
    tire_temp.core_temp = 80.0;
    let optimal_grip = tire_temp.get_grip_multiplier();
    
    // Overheat
    tire_temp.surface_temp = 130.0;
    tire_temp.core_temp = 130.0;
    let overheated_grip = tire_temp.get_grip_multiplier();
    
    assert!(optimal_grip > cold_grip, "Optimal temperature should provide better grip than cold");
    assert!(optimal_grip > overheated_grip, "Optimal temperature should provide better grip than overheated");
}

#[test]
fn test_tire_wear_accumulation() {
    let mut tire_temp = TireTemperature::default();
    let initial_wear = tire_temp.wear;
    
    // Simulate high temperature and slip power
    tire_temp.surface_temp = 130.0; // Overheating
    tire_temp.core_temp = 125.0;
    tire_temp.update(100.0, 1000.0, 1.0, 20.0); // High slip power and load
    
    assert!(tire_temp.wear > initial_wear, "Tire should accumulate wear under stress");
    assert_eq!(tire_temp.get_wear_state(), TireWearState::Good); // Still good as wear just started
    
    // Simulate extended high stress
    for _ in 0..1000 {
        tire_temp.update(100.0, 1000.0, 0.1, 20.0);
    }
    
    assert!(tire_temp.wear > 0.3, "Tire should show significant wear after extended stress");
    assert!(tire_temp.wear <= 1.0, "Tire wear should be clamped to 1.0");
}

#[test]
fn test_continuous_grip_curve() {
    let mut tire_temp = TireTemperature::default();
    let mut prev_grip = tire_temp.get_grip_multiplier();
    
    // Test grip changes are continuous
    for temp in (20..140).step_by(5) {
        tire_temp.surface_temp = temp as f32;
        tire_temp.core_temp = temp as f32;
        let current_grip = tire_temp.get_grip_multiplier();
        
        // Grip changes should be relatively small between steps
        assert!((current_grip - prev_grip).abs() < 0.1, 
                "Grip changes should be continuous, change was too large at temp {}", temp);
        prev_grip = current_grip;
    }
}

#[test]
fn test_temperature_history() {
    let mut tire_temp = TireTemperature::default();
    let initial_avg = tire_temp.get_average_temperature();
    
    // Heat up the tire
    tire_temp.surface_temp = 90.0;
    tire_temp.core_temp = 80.0;
    
    // Update multiple times to fill temperature history
    for _ in 0..15 {
        tire_temp.update(50.0, 1000.0, 0.1, 20.0);
    }
    
    let hot_avg = tire_temp.get_average_temperature();
    assert!(hot_avg > initial_avg, "Average temperature should increase after heating");
    
    // Cool down
    for _ in 0..30 {
        tire_temp.update(0.0, 0.0, 0.1, 20.0);
    }
    
    let final_avg = tire_temp.get_average_temperature();
    assert!(final_avg < hot_avg, "Average temperature should decrease after cooling");
}

#[test]
fn test_wear_effect_on_grip() {
    let mut tire_temp = TireTemperature::default();
    tire_temp.surface_temp = 80.0; // Optimal temperature
    tire_temp.core_temp = 80.0;
    
    let initial_grip = tire_temp.get_grip_multiplier();
    assert!(initial_grip > 0.99, "Should have maximum grip when new and at optimal temperature");
    
    // Simulate wear
    tire_temp.wear = 0.5; // 50% worn
    let worn_grip = tire_temp.get_grip_multiplier();
    assert!(worn_grip < initial_grip, "Grip should decrease with wear");
    assert!(worn_grip > 0.7, "Grip shouldn't decrease too much at 50% wear");
    
    tire_temp.wear = 1.0; // Fully worn
    let fully_worn_grip = tire_temp.get_grip_multiplier();
    assert!(fully_worn_grip < worn_grip, "Grip should be lowest when fully worn");
    assert!(fully_worn_grip > 0.6, "Even fully worn tires should maintain some grip");
} 