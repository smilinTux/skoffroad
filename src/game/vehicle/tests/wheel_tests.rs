use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use approx::assert_relative_eq;
use crate::game::vehicle::{
    wheel::{Wheel, WheelBundle, update_wheel_physics, update_wheel_contact},
    tire::{TireModel, TireState},
};

#[test]
fn test_wheel_bundle_creation() {
    let bundle = WheelBundle::default();
    
    assert_eq!(bundle.wheel.radius, 0.4);
    assert_eq!(bundle.wheel.width, 0.3);
    assert_eq!(bundle.wheel.mass, 25.0);
    assert_eq!(bundle.wheel.inertia, 2.5);
    
    // Verify collider dimensions
    if let Collider::Cylinder(height, radius) = bundle.collider {
        assert_relative_eq!(height, 0.15); // half width
        assert_relative_eq!(radius, 0.4);  // wheel radius
    } else {
        panic!("Expected cylinder collider");
    }
}

#[test]
fn test_wheel_slip_calculations() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_systems(Update, update_wheel_physics);
    
    // Create a wheel with ground contact
    let wheel_entity = app.world.spawn(WheelBundle {
        wheel: Wheel {
            ground_contact: true,
            angular_velocity: 10.0, // rad/s
            ..default()
        },
        velocity: Velocity {
            linvel: Vec3::new(0.0, 0.0, 5.0), // m/s forward
            ..default()
        },
        transform: Transform::from_xyz(0.0, 0.4, 0.0),
        ..default()
    }).id();
    
    // Run one physics step
    app.update();
    
    // Check slip calculations
    let wheel = app.world.get::<Wheel>(wheel_entity).unwrap();
    let wheel_speed = wheel.angular_velocity * wheel.radius;
    let expected_slip_ratio = (wheel_speed - 5.0) / 5.0;
    
    assert_relative_eq!(wheel.slip_ratio, expected_slip_ratio, epsilon = 0.001);
}

#[test]
fn test_tire_forces() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_systems(Update, update_wheel_physics);
    
    // Create a wheel with slip
    let wheel_entity = app.world.spawn(WheelBundle {
        wheel: Wheel {
            ground_contact: true,
            normal_force: 5000.0, // 5kN normal force
            slip_angle: 0.1,      // ~5.7 degrees slip angle
            slip_ratio: 0.05,     // 5% slip ratio
            ..default()
        },
        transform: Transform::from_xyz(0.0, 0.4, 0.0),
        ..default()
    }).id();
    
    // Run physics step
    app.update();
    
    // Check that forces were applied
    let external_force = app.world.get::<ExternalForce>(wheel_entity).unwrap();
    assert!(external_force.force.length() > 0.0, "No tire forces generated");
    assert!(external_force.torque.length() > 0.0, "No wheel torque generated");
}

#[test]
fn test_ground_contact_detection() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugin(RapierPhysicsPlugin::<NoUserData>::default())
        .add_systems(Update, update_wheel_contact);
    
    // Create ground plane
    app.world.spawn(Collider::cuboid(50.0, 0.1, 50.0))
        .insert(Transform::from_xyz(0.0, 0.0, 0.0));
    
    // Create wheel above ground
    let wheel_entity = app.world.spawn(WheelBundle {
        wheel: Wheel {
            radius: 0.4,
            ..default()
        },
        transform: Transform::from_xyz(0.0, 0.35, 0.0), // Slight compression
        ..default()
    }).id();
    
    // Run physics step
    app.update();
    
    // Check ground contact and normal force
    let wheel = app.world.get::<Wheel>(wheel_entity).unwrap();
    assert!(wheel.ground_contact, "Ground contact not detected");
    assert!(wheel.normal_force > 0.0, "No normal force generated");
}

#[test]
fn test_wheel_torque_application() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_systems(Update, update_wheel_physics);
    
    // Create wheel with drive torque
    let wheel_entity = app.world.spawn(WheelBundle {
        wheel: Wheel {
            ground_contact: true,
            drive_torque: 1000.0, // 1000 Nm drive torque
            inertia: 2.5,
            ..default()
        },
        ..default()
    }).id();
    
    // Run physics for 0.1 seconds
    app.world.resource_mut::<Time>().update();
    app.update();
    
    // Check angular velocity increase
    let wheel = app.world.get::<Wheel>(wheel_entity).unwrap();
    let expected_angular_accel = 1000.0 / 2.5; // torque / inertia
    let expected_angular_vel = expected_angular_accel * 0.1; // for 0.1 seconds
    
    assert_relative_eq!(
        wheel.angular_velocity,
        expected_angular_vel,
        epsilon = 0.1
    );
}

#[test]
fn test_brake_torque() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_systems(Update, update_wheel_physics);
    
    // Create spinning wheel with brake torque
    let wheel_entity = app.world.spawn(WheelBundle {
        wheel: Wheel {
            ground_contact: true,
            angular_velocity: 50.0,  // Initial angular velocity
            brake_torque: 500.0,     // Brake torque
            inertia: 2.5,
            ..default()
        },
        ..default()
    }).id();
    
    // Run physics for 0.1 seconds
    app.world.resource_mut::<Time>().update();
    app.update();
    
    // Check angular velocity decrease
    let wheel = app.world.get::<Wheel>(wheel_entity).unwrap();
    let expected_angular_accel = -500.0 / 2.5; // -brake_torque / inertia
    let expected_angular_vel = 50.0 + expected_angular_accel * 0.1;
    
    assert_relative_eq!(
        wheel.angular_velocity,
        expected_angular_vel,
        epsilon = 0.1
    );
    assert!(wheel.angular_velocity < 50.0, "Brake did not slow wheel");
} 