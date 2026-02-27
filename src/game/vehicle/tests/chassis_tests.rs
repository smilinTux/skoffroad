use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use approx::assert_relative_eq;

use crate::game::vehicle::chassis::{Chassis, ChassisBundle, handle_chassis_collisions, ChassisCollisionConfig, ColliderPart};

#[test]
fn test_inertia_tensor_calculation() {
    let chassis = Chassis {
        mass: 1000.0,
        dimensions: Vec3::new(2.0, 1.0, 4.0),
        ..Default::default()
    };
    
    // For a rectangular prism, verify the diagonal elements are correct
    // Ixx = m/12 * (h² + d²)
    // Iyy = m/12 * (w² + d²)
    // Izz = m/12 * (w² + h²)
    let expected_ixx = chassis.mass / 12.0 * (chassis.dimensions.y.powi(2) + chassis.dimensions.z.powi(2));
    let expected_iyy = chassis.mass / 12.0 * (chassis.dimensions.x.powi(2) + chassis.dimensions.z.powi(2));
    let expected_izz = chassis.mass / 12.0 * (chassis.dimensions.x.powi(2) + chassis.dimensions.y.powi(2));
    
    assert_relative_eq!(chassis.inertia_tensor.x.x, expected_ixx, epsilon = 0.001);
    assert_relative_eq!(chassis.inertia_tensor.y.y, expected_iyy, epsilon = 0.001);
    assert_relative_eq!(chassis.inertia_tensor.z.z, expected_izz, epsilon = 0.001);
}

#[test]
fn test_collision_detection() {
    let mut app = App::new();
    
    // Add necessary plugins
    app.add_plugins((
        MinimalPlugins,
        RapierPhysicsPlugin::<NoUserData>::default(),
    ));
    
    // Spawn test terrain
    app.world.spawn((
        Collider::cuboid(50.0, 1.0, 50.0),
        TransformBundle::from(Transform::from_xyz(0.0, -1.0, 0.0)),
    ));
    
    // Spawn test chassis
    app.world.spawn(ChassisBundle {
        chassis: Chassis {
            mass: 1000.0,
            dimensions: Vec3::new(2.0, 1.0, 4.0),
            ..Default::default()
        },
        transform: TransformBundle::from(Transform::from_xyz(0.0, 1.0, 0.0)),
        ..Default::default()
    });
    
    // Run physics step
    app.update();
    
    // Get chassis entity and check for collisions
    let chassis_query = app.world.query_filtered::<Entity, With<Chassis>>();
    let chassis_entity = chassis_query.iter(&app.world).next().unwrap();
    
    let mut collision_count = 0;
    app.world.resource_scope(|world, rapier_context: Mut<RapierContext>| {
        rapier_context.intersections_with(chassis_entity, |_entity| {
            collision_count += 1;
            true
        });
    });
    
    // Should detect collision with ground
    assert!(collision_count > 0);
}

#[test]
fn test_chassis_bundle_creation() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    
    let chassis_bundle = ChassisBundle {
        chassis: Chassis {
            mass: 1000.0,
            dimensions: Vec3::new(2.0, 1.0, 4.0),
            ..Default::default()
        },
        transform: TransformBundle::from(Transform::from_xyz(0.0, 1.0, 0.0)),
        ..Default::default()
    };
    
    let chassis_entity = app.world.spawn(chassis_bundle).id();
    
    // Verify components were added correctly
    let chassis = app.world.get::<Chassis>(chassis_entity).unwrap();
    let collider = app.world.get::<Collider>(chassis_entity).unwrap();
    let rigid_body = app.world.get::<RigidBody>(chassis_entity).unwrap();
    
    assert_eq!(chassis.mass, 1000.0);
    assert_eq!(chassis.dimensions, Vec3::new(2.0, 1.0, 4.0));
    assert!(matches!(rigid_body, RigidBody::Dynamic));
}

#[test]
fn test_compound_collider_creation() {
    let chassis = Chassis::default();
    let bundle = ChassisBundle::default();
    
    // Verify collider was created with all parts
    if let Collider::Compound(shapes) = bundle.collider {
        assert_eq!(shapes.len(), 6); // Main body + 2 bumpers + 2 side skirts + roof
        
        // Check main body dimensions
        let (_, _, main_body) = &shapes[0];
        if let Collider::Cuboid(half_extents) = main_body {
            assert_relative_eq!(half_extents.x, chassis.dimensions.x / 2.0);
            assert_relative_eq!(half_extents.y, chassis.dimensions.y / 2.0);
            assert_relative_eq!(half_extents.z, chassis.dimensions.z / 2.0);
        } else {
            panic!("Main body collider should be a cuboid");
        }
    } else {
        panic!("Bundle collider should be a compound shape");
    }
}

#[test]
fn test_collision_response() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(RapierPhysicsPlugin::<NoUserData>::default());
    
    // Create ground plane
    app.world.spawn((
        Collider::cuboid(50.0, 0.1, 50.0),
        TransformBundle::from(Transform::from_xyz(0.0, 0.0, 0.0)),
    ));
    
    // Spawn chassis slightly above ground
    let chassis_entity = app.world.spawn(ChassisBundle {
        transform: Transform::from_xyz(0.0, 1.0, 0.0),
        ..default()
    }).id();
    
    // Run physics for a few frames
    for _ in 0..10 {
        app.update();
    }
    
    // Check that chassis has made contact with ground
    let chassis = app.world.get::<Chassis>(chassis_entity).unwrap();
    assert!(!chassis.ground_contacts.is_empty());
    assert!(chassis.ground_clearance < 1.0);
}

#[test]
fn test_collision_config() {
    let config = ChassisCollisionConfig::default();
    
    // Test front bumper configuration
    assert_relative_eq!(config.front_bumper.friction, 0.6);
    assert_relative_eq!(config.front_bumper.restitution, 0.3);
    assert!(config.front_bumper.offset.y < 0.0); // Should be below center
    
    // Test side skirts configuration
    assert_eq!(config.side_skirts.len(), 2);
    assert_relative_eq!(config.side_skirts[0].friction, 0.5);
    assert_relative_eq!(config.side_skirts[0].dimensions.x, 0.2); // Thin side skirts
    
    // Test roof configuration
    assert!(config.roof.dimensions.x < config.body_dimensions.x); // Roof should be narrower than body
    assert_relative_eq!(config.roof.friction, 0.4); // Lower friction for roof
}

#[test]
fn test_contact_force_calculation() {
    use crate::game::vehicle::chassis::calculate_contact_force;
    
    let config = ChassisCollisionConfig::default();
    let point = Vec3::new(0.0, 0.0, 0.0);
    let normal = Vec3::Y;
    let penetration = 0.1;
    
    let force = calculate_contact_force(&config, point, normal, penetration);
    
    // Force should be upward
    assert!(force.y > 0.0);
    assert_relative_eq!(force.x, 0.0);
    assert_relative_eq!(force.z, 0.0);
    
    // Force should increase with penetration
    let deeper_force = calculate_contact_force(&config, point, normal, penetration * 2.0);
    assert!(deeper_force.y > force.y);
}

#[test]
fn test_high_speed_collision() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(RapierPhysicsPlugin::<NoUserData>::default());
    
    // Create wall
    app.world.spawn((
        Collider::cuboid(0.1, 2.0, 5.0),
        TransformBundle::from(Transform::from_xyz(5.0, 2.0, 0.0)),
    ));
    
    // Spawn chassis with high velocity
    let chassis_entity = app.world.spawn(ChassisBundle {
        transform: Transform::from_xyz(0.0, 2.0, 0.0),
        velocity: Velocity {
            linvel: Vec3::new(50.0, 0.0, 0.0), // High speed towards wall
            angvel: Vec3::ZERO,
        },
        ..default()
    }).id();
    
    // Run physics
    for _ in 0..10 {
        app.update();
    }
    
    // Check that chassis didn't tunnel through the wall
    let transform = app.world.get::<Transform>(chassis_entity).unwrap();
    assert!(transform.translation.x < 5.0);
} 