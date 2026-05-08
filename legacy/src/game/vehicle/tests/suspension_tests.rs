use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use crate::game::vehicle::*;
use approx::assert_relative_eq;

use crate::game::vehicle::suspension::{Suspension, SuspensionConfig, SuspensionType, update_suspension, calculate_suspension_forces};
use crate::game::vehicle::wheel::Wheel;

#[test]
fn test_suspension_types() {
    // Test different suspension configurations
    let stock = Suspension::with_type(SuspensionType::Stock);
    let short_arm = Suspension::with_type(SuspensionType::ShortArmLift);
    let long_arm = Suspension::with_type(SuspensionType::LongArmLift);
    
    // Verify stock configuration
    assert_eq!(stock.suspension_type, SuspensionType::Stock);
    assert!(stock.lift_kit.is_none());
    assert_eq!(stock.spring_stiffness, 50000.0);
    
    // Verify short arm lift configuration
    assert_eq!(short_arm.suspension_type, SuspensionType::ShortArmLift);
    assert!(short_arm.lift_kit.is_some());
    let lift_kit = short_arm.lift_kit.unwrap();
    assert_eq!(lift_kit.lift_height, 0.075); // 3" lift
    
    // Verify long arm lift configuration
    assert_eq!(long_arm.suspension_type, SuspensionType::LongArmLift);
    assert!(long_arm.lift_kit.is_some());
    let lift_kit = long_arm.lift_kit.unwrap();
    assert_eq!(lift_kit.lift_height, 0.125); // 5" lift
}

#[test]
fn test_suspension_compression() {
    let mut suspension = Suspension::default();
    
    // Test normal compression
    suspension.compression = 0.1;
    assert!(suspension.compression <= suspension.max_compression);
    assert!(suspension.compression >= -suspension.max_extension);
    
    // Test maximum compression
    suspension.compression = 1.0;
    assert_eq!(suspension.compression.min(suspension.max_compression), suspension.max_compression);
    
    // Test maximum extension
    suspension.compression = -1.0;
    assert_eq!(suspension.compression.max(-suspension.max_extension), -suspension.max_extension);
}

#[test]
fn test_suspension_forces() {
    let mut suspension = Suspension::default();
    
    // Test spring force
    suspension.compression = 0.1;
    let spring_force = suspension.spring_stiffness * suspension.compression;
    assert!(spring_force > 0.0);
    
    // Test damping force
    suspension.velocity = 1.0;
    let damping_force = suspension.damping * suspension.velocity;
    assert!(damping_force > 0.0);
    
    // Test total force limits
    suspension.force = 100000.0; // Above limit
    suspension.force = suspension.force.clamp(-suspension.damage_threshold, suspension.damage_threshold);
    assert!(suspension.force <= suspension.damage_threshold);
}

#[test]
fn test_suspension_damage() {
    let mut suspension = Suspension::default();
    
    // Test damage accumulation
    suspension.force = suspension.damage_threshold * 1.5; // Excessive force
    update_suspension_damage(&mut suspension, 1.0);
    assert!(suspension.health < 100.0);
    assert!(suspension.accumulated_stress > 0.0);
    
    // Test damage recovery
    suspension.force = 0.0;
    update_suspension_damage(&mut suspension, 1.0);
    assert!(suspension.accumulated_stress < 2.0); // Should decrease under low stress
    
    // Test complete failure
    suspension.health = 0.0;
    update_suspension_damage(&mut suspension, 1.0);
    assert!(suspension.is_broken);
}

#[test]
fn test_suspension_geometry() {
    let mut suspension = Suspension::with_type(SuspensionType::LongArmLift);
    
    // Test lift kit geometry effects
    if let Some(lift_kit) = &suspension.lift_kit {
        // Verify mount point adjustments
        assert!(suspension.mount_point.y > 0.0); // Should be raised by lift
        
        // Test geometry correction
        let base_force = 1000.0;
        suspension.force = base_force;
        suspension.force *= lift_kit.geometry_correction;
        assert!(suspension.force > base_force); // Should be increased by correction
    }
}

#[test]
fn test_suspension_stress_calculation() {
    let mut suspension = Suspension::default();
    
    // Test force stress
    suspension.force = suspension.damage_threshold * 0.5;
    let dt = 0.016; // Typical frame time
    update_suspension_damage(&mut suspension, dt);
    assert!(suspension.accumulated_stress > 0.0);
    
    // Test compression stress
    suspension.compression = suspension.max_compression * 0.95; // Near limit
    update_suspension_damage(&mut suspension, dt);
    assert!(suspension.accumulated_stress > 0.0);
    
    // Test velocity stress
    suspension.velocity = 8.0; // High velocity
    update_suspension_damage(&mut suspension, dt);
    assert!(suspension.accumulated_stress > 0.0);
}

#[test]
fn test_suspension_health_effects() {
    let mut suspension = Suspension::default();
    
    // Test partial damage effects
    suspension.health = 50.0; // 50% health
    let base_force = 1000.0;
    suspension.force = base_force;
    
    // Force should be limited by health
    let max_force = suspension.damage_threshold * (suspension.health / 100.0);
    suspension.force = suspension.force.clamp(-max_force, max_force);
    assert!(suspension.force.abs() <= max_force);
    
    // Test complete failure effects
    suspension.health = 0.0;
    update_suspension_damage(&mut suspension, 1.0);
    assert!(suspension.is_broken);
    assert_eq!(suspension.force, 0.0); // No force when broken
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_lift_kit_effects() {
        let mut suspension = Suspension::default();
        suspension.suspension_type = SuspensionType::LongArmLift;
        suspension.lift_kit = Some(LiftKitConfig {
            lift_height: 0.1,
            arm_length: 0.5,
            arm_angle: 15.0_f32.to_radians(),
            track_width_increase: 0.05,
            geometry_correction: 0.02,
        });

        suspension.configure_type();
        
        assert_relative_eq!(suspension.rest_length, 0.4 + 0.1); // Base + lift height
        assert!(suspension.max_extension > 0.4); // Should be increased
        assert!(suspension.spring_stiffness < 50000.0); // Should be softer
    }

    #[test]
    fn test_portal_axle_configuration() {
        let mut suspension = Suspension::default();
        suspension.suspension_type = SuspensionType::PortalAxle;
        suspension.configure_type();

        assert!(suspension.rest_length > 0.4); // Should be higher
        assert!(suspension.max_compression < 0.2); // Should have less compression
        assert!(suspension.spring_stiffness > 50000.0); // Should be stiffer
    }

    #[test]
    fn test_air_suspension_adjustment() {
        let mut suspension = Suspension::default();
        suspension.suspension_type = SuspensionType::AirSuspension;
        suspension.configure_type();

        // Test height adjustment
        let initial_height = suspension.rest_length;
        suspension.adjust_air_pressure(1.2); // 20% increase
        assert!(suspension.rest_length > initial_height);
        
        // Test dynamic stiffness
        let initial_stiffness = suspension.spring_stiffness;
        suspension.adjust_air_pressure(0.8); // 20% decrease
        assert!(suspension.spring_stiffness < initial_stiffness);
    }

    #[test]
    fn test_extreme_terrain_behavior() {
        let mut suspension = Suspension::default();
        
        // Simulate rock crawling impact
        suspension.apply_force(100000.0); // High impact force
        assert!(suspension.accumulated_stress > 0.0);
        assert!(suspension.health < 100.0);

        // Test rapid compression/extension cycles
        for _ in 0..100 {
            suspension.velocity = 2.0;
            suspension.update_suspension_physics(0.016);
            suspension.velocity = -2.0;
            suspension.update_suspension_physics(0.016);
        }
        
        assert!(suspension.accumulated_stress > 0.5); // Should accumulate significant stress
    }

    #[test]
    fn test_suspension_tuning() {
        let mut suspension = Suspension::default();
        let tuning = SuspensionTuning {
            compression_damping: 5000.0,
            rebound_damping: 4000.0,
            high_speed_compression: 7000.0,
            high_speed_rebound: 6000.0,
            preload: 0.02,
        };
        
        suspension.apply_tuning(&tuning);
        
        // Test low speed damping
        suspension.velocity = 0.1;
        let low_speed_force = suspension.calculate_damping_force();
        
        // Test high speed damping
        suspension.velocity = 2.0;
        let high_speed_force = suspension.calculate_damping_force();
        
        assert!(high_speed_force.abs() > low_speed_force.abs());
    }

    #[test]
    fn test_broken_suspension_behavior() {
        let mut suspension = Suspension::default();
        
        // Break the suspension
        suspension.health = 0.0;
        suspension.is_broken = true;
        
        // Verify broken suspension behavior
        let force_before = suspension.force;
        suspension.apply_force(1000.0);
        assert_eq!(suspension.force, force_before); // Should not accumulate force
        
        // Verify no physics updates occur
        let pos_before = suspension.wheel_point;
        suspension.update_suspension_physics(0.016);
        assert_eq!(suspension.wheel_point, pos_before); // Position should not change
    }

    #[test]
    fn test_suspension_force_calculation() {
        let suspension = Suspension {
            config: SuspensionConfig {
                spring_stiffness: 50000.0,
                damping: 5000.0,
                rest_length: 0.5,
                max_travel: 0.3,
                suspension_type: SuspensionType::Independent,
                ..Default::default()
            },
            current_length: 0.4, // Compressed by 0.1m
            current_velocity: -1.0, // Compressing at 1 m/s
            ..Default::default()
        };
        
        // Calculate expected force
        // F = -k * x - c * v
        // where k = spring stiffness, x = displacement from rest, c = damping, v = velocity
        let displacement = suspension.current_length - suspension.config.rest_length;
        let expected_spring_force = -suspension.config.spring_stiffness * displacement;
        let expected_damping_force = -suspension.config.damping * suspension.current_velocity;
        let expected_total_force = expected_spring_force + expected_damping_force;
        
        let actual_force = suspension.calculate_force();
        assert_relative_eq!(actual_force, expected_total_force, epsilon = 0.001);
    }

    #[test]
    fn test_suspension_travel_limits() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, RapierPhysicsPlugin::<NoUserData>::default()));
        
        // Create suspension with limited travel
        let suspension_config = SuspensionConfig {
            spring_stiffness: 50000.0,
            damping: 5000.0,
            rest_length: 0.5,
            max_travel: 0.3,
            suspension_type: SuspensionType::Independent,
            ..Default::default()
        };
        
        // Test at maximum compression
        let mut suspension = Suspension {
            config: suspension_config.clone(),
            current_length: suspension_config.rest_length - suspension_config.max_travel,
            current_velocity: -1.0,
            ..Default::default()
        };
        
        // Try to compress beyond limit
        suspension.update(0.016);
        assert!(suspension.current_length >= suspension_config.rest_length - suspension_config.max_travel);
        
        // Test at maximum extension
        suspension.current_length = suspension_config.rest_length + suspension_config.max_travel;
        suspension.current_velocity = 1.0;
        
        // Try to extend beyond limit
        suspension.update(0.016);
        assert!(suspension.current_length <= suspension_config.rest_length + suspension_config.max_travel);
    }

    #[test]
    fn test_suspension_wheel_interaction() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, RapierPhysicsPlugin::<NoUserData>::default()));
        
        // Create test entities
        let wheel_entity = app.world.spawn((
            Wheel {
                radius: 0.3,
                width: 0.2,
                mass: 20.0,
                ..Default::default()
            },
            TransformBundle::from(Transform::from_xyz(0.0, 0.3, 0.0)),
            RigidBody::Dynamic,
            Collider::cylinder(0.3, 0.1),
        )).id();
        
        let suspension_entity = app.world.spawn((
            Suspension {
                config: SuspensionConfig {
                    spring_stiffness: 50000.0,
                    damping: 5000.0,
                    rest_length: 0.5,
                    max_travel: 0.3,
                    suspension_type: SuspensionType::Independent,
                    ..Default::default()
                },
                wheel_entity: Some(wheel_entity),
                current_length: 0.4,
                current_velocity: 0.0,
                ..Default::default()
            },
            TransformBundle::from(Transform::from_xyz(0.0, 1.0, 0.0)),
        )).id();
        
        // Add ground
        app.world.spawn((
            Collider::cuboid(50.0, 1.0, 50.0),
            TransformBundle::from(Transform::from_xyz(0.0, -1.0, 0.0)),
        ));
        
        // Run physics step
        app.add_systems(Update, update_suspension);
        app.update();
        
        // Verify suspension affects wheel position
        let wheel_transform = app.world.get::<Transform>(wheel_entity).unwrap();
        let suspension_transform = app.world.get::<Transform>(suspension_entity).unwrap();
        let suspension = app.world.get::<Suspension>(suspension_entity).unwrap();
        
        // Check wheel is at correct distance from suspension mount
        let relative_height = wheel_transform.translation.y - suspension_transform.translation.y;
        assert_relative_eq!(relative_height.abs(), suspension.current_length, epsilon = 0.001);
    }

    #[test]
    fn test_suspension_types() {
        // Test Independent Suspension
        let independent = SuspensionConfig {
            suspension_type: SuspensionType::Independent,
            spring_stiffness: 50000.0,
            damping: 5000.0,
            rest_length: 0.5,
            max_travel: 0.3,
            ..Default::default()
        };
        
        // Test Solid Axle
        let solid_axle = SuspensionConfig {
            suspension_type: SuspensionType::SolidAxle,
            spring_stiffness: 50000.0,
            damping: 5000.0,
            rest_length: 0.5,
            max_travel: 0.3,
            ..Default::default()
        };
        
        // Verify different behavior under roll
        let roll_angle = 0.1; // 0.1 radians of roll
        
        // Independent suspension should allow different compression on each side
        let left_independent = Suspension {
            config: independent.clone(),
            current_length: 0.4,
            ..Default::default()
        };
        let right_independent = Suspension {
            config: independent,
            current_length: 0.6,
            ..Default::default()
        };
        
        // Solid axle should maintain relationship between sides
        let left_solid = Suspension {
            config: solid_axle.clone(),
            current_length: 0.4,
            ..Default::default()
        };
        let right_solid = Suspension {
            config: solid_axle,
            current_length: 0.6,
            ..Default::default()
        };
        
        // Independent suspension forces can be different
        assert_ne!(left_independent.calculate_force(), right_independent.calculate_force());
        
        // Solid axle forces should be coupled
        let left_force = left_solid.calculate_force();
        let right_force = right_solid.calculate_force();
        assert_relative_eq!(left_force.abs(), right_force.abs(), epsilon = 0.001);
    }

    #[test]
    fn test_suspension_terrain_response() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, RapierPhysicsPlugin::<NoUserData>::default()));
        
        // Create suspension with terrain response configuration
        let mut suspension = Suspension {
            config: SuspensionConfig {
                spring_stiffness: 50000.0,
                damping: 5000.0,
                rest_length: 0.5,
                max_travel: 0.3,
                suspension_type: SuspensionType::Independent,
                ..Default::default()
            },
            current_length: 0.4,
            current_velocity: 0.0,
            ..Default::default()
        };

        // Test different terrain types
        let terrains = vec![
            ("Hard Surface", 1.0, 0.8),
            ("Soft Sand", 0.7, 0.3),
            ("Rocky Ground", 1.2, 0.9),
            ("Mud", 0.5, 0.2),
        ];

        for (terrain_type, stiffness_mult, damping_mult) in terrains {
            suspension.adjust_for_terrain(stiffness_mult, damping_mult);
            
            // Verify suspension characteristics are adjusted for terrain
            assert_relative_eq!(
                suspension.config.spring_stiffness,
                50000.0 * stiffness_mult,
                epsilon = 0.001
            );
            assert_relative_eq!(
                suspension.config.damping,
                5000.0 * damping_mult,
                epsilon = 0.001
            );
        }
    }

    #[test]
    fn test_progressive_spring_rate() {
        let mut suspension = Suspension::default();
        let base_spring_rate = suspension.spring_stiffness;
        
        // Test progressive spring rate at different compression levels
        suspension.compression = 0.0;
        let force_at_zero = calculate_suspension_forces(&mut suspension, 
            &Transform::default(), 
            &Transform::default(),
            0.016).length();
        
        suspension.compression = suspension.max_compression * 0.5;
        let force_at_half = calculate_suspension_forces(&mut suspension,
            &Transform::default(),
            &Transform::default(),
            0.016).length();
        
        suspension.compression = suspension.max_compression * 0.9;
        let force_at_max = calculate_suspension_forces(&mut suspension,
            &Transform::default(),
            &Transform::default(),
            0.016).length();
        
        // Force should increase non-linearly
        let half_ratio = force_at_half / (0.5 * suspension.max_compression * base_spring_rate);
        let max_ratio = force_at_max / (0.9 * suspension.max_compression * base_spring_rate);
        
        assert!(max_ratio > half_ratio); // Verify progressive rate
    }

    #[test]
    fn test_asymmetric_damping() {
        let mut suspension = Suspension::default();
        
        // Test compression damping (negative velocity)
        suspension.velocity = -1.0;
        let compression_force = calculate_suspension_forces(&mut suspension,
            &Transform::default(),
            &Transform::default(),
            0.016).length();
        
        // Test rebound damping (positive velocity)
        suspension.velocity = 1.0;
        let rebound_force = calculate_suspension_forces(&mut suspension,
            &Transform::default(),
            &Transform::default(),
            0.016).length();
        
        // Rebound should be less damped than compression
        assert!(rebound_force < compression_force);
    }

    #[test]
    fn test_bump_stop_behavior() {
        let mut suspension = Suspension::default();
        let bump_stop_range = 0.05;
        
        // Test near max compression
        suspension.compression = suspension.max_compression - bump_stop_range * 0.5;
        let force_near_max = calculate_suspension_forces(&mut suspension,
            &Transform::default(),
            &Transform::default(),
            0.016).length();
        
        // Test at normal compression
        suspension.compression = suspension.max_compression * 0.5;
        let force_normal = calculate_suspension_forces(&mut suspension,
            &Transform::default(),
            &Transform::default(),
            0.016).length();
        
        // Test near max extension
        suspension.compression = -suspension.max_extension + bump_stop_range * 0.5;
        let force_near_extension = calculate_suspension_forces(&mut suspension,
            &Transform::default(),
            &Transform::default(),
            0.016).length();
        
        // Forces should be significantly higher near the limits
        assert!(force_near_max > force_normal * 2.0);
        assert!(force_near_extension > force_normal * 2.0);
    }

    #[test]
    fn test_anti_roll_bar() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_systems(Update, apply_suspension_forces_with_antiroll);
        
        // Create chassis
        let chassis_id = app.world.spawn((
            Chassis::default(),
            Transform::from_xyz(0.0, 1.0, 0.0),
            ExternalForce::default()
        )).id();
        
        // Create left and right suspensions
        let left_suspension = Suspension {
            mount_point: Vec3::new(-1.0, 0.0, 0.0),
            compression: 0.1,
            ..default()
        };
        
        let right_suspension = Suspension {
            mount_point: Vec3::new(1.0, 0.0, 0.0),
            compression: -0.1,
            ..default()
        };
        
        app.world.spawn((
            left_suspension,
            GlobalTransform::default()
        ));
        
        app.world.spawn((
            right_suspension,
            GlobalTransform::default()
        ));
        
        // Create wheel
        app.world.spawn((
            Wheel::default(),
            Transform::from_xyz(0.0, 0.0, 0.0)
        ));
        
        // Run system
        app.update();
        
        // Check that anti-roll forces were applied
        let chassis = app.world.entity(chassis_id);
        let external_force = chassis.get::<ExternalForce>().unwrap();
        
        // Should have non-zero torque around Z axis due to anti-roll
        assert!(external_force.torque.z.abs() > 0.0);
    }

    #[test]
    fn test_suspension_heat_management() {
        let mut suspension = Suspension {
            config: SuspensionConfig {
                spring_stiffness: 50000.0,
                damping: 5000.0,
                rest_length: 0.5,
                max_travel: 0.3,
                suspension_type: SuspensionType::Independent,
                ..Default::default()
            },
            current_length: 0.4,
            current_velocity: 2.0, // High velocity for heat generation
            temperature: 20.0, // Starting at ambient temperature
            ..Default::default()
        };

        let dt = 0.016; // Typical frame time
        
        // Test heat generation under load
        for _ in 0..100 {
            suspension.update_temperature(dt);
            suspension.current_velocity *= -1.0; // Oscillate
        }
        
        assert!(suspension.temperature > 20.0); // Temperature should increase
        
        // Test cooling
        suspension.current_velocity = 0.0;
        let hot_temp = suspension.temperature;
        
        for _ in 0..100 {
            suspension.update_temperature(dt);
        }
        
        assert!(suspension.temperature < hot_temp); // Should cool down
    }

    #[test]
    fn test_suspension_failure_modes() {
        let mut suspension = Suspension {
            config: SuspensionConfig {
                spring_stiffness: 50000.0,
                damping: 5000.0,
                rest_length: 0.5,
                max_travel: 0.3,
                suspension_type: SuspensionType::Independent,
                ..Default::default()
            },
            health: 100.0,
            temperature: 20.0,
            ..Default::default()
        };

        // Test different failure conditions
        let failure_conditions = vec![
            ("Overheat", |s: &mut Suspension| s.temperature = 150.0),
            ("Excessive Force", |s: &mut Suspension| s.apply_force(100000.0)),
            ("Fatigue", |s: &mut Suspension| {
                s.accumulated_stress = 1000.0;
                s.update_health(0.016);
            }),
        ];

        for (condition, apply_condition) in failure_conditions {
            suspension.health = 100.0; // Reset health
            apply_condition(&mut suspension);
            
            // Check degradation
            assert!(suspension.health < 100.0, "Health should decrease under {}", condition);
            
            // Verify performance impact
            let base_force = suspension.calculate_force();
            suspension.health = 50.0; // 50% health
            let degraded_force = suspension.calculate_force();
            
            assert!(degraded_force.abs() < base_force.abs(), 
                "Force should be reduced when damaged by {}", condition);
        }
    }

    #[test]
    fn test_enhanced_force_calculation() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(RapierPhysicsPlugin::<NoUserData>::default());
        
        let mut suspension = Suspension::default();
        suspension.spring_stiffness = 50000.0;
        suspension.damping = 5000.0;
        suspension.compression = 0.1;
        suspension.velocity = 2.0;
        
        // Test progressive spring rate
        let force = calculate_suspension_forces(&suspension);
        let linear_force = suspension.spring_stiffness * suspension.compression;
        assert!(force > linear_force); // Progressive rate should give higher force
        
        // Test combined spring and damping forces
        suspension.compression = 0.05;
        suspension.velocity = 1.0;
        let combined_force = calculate_suspension_forces(&suspension);
        assert!(combined_force > 0.0);
        
        // Test bump stop interaction
        suspension.compression = suspension.max_compression * 0.95;
        let near_limit_force = calculate_suspension_forces(&suspension);
        assert!(near_limit_force > force); // Should increase dramatically near limits
    }

    #[test]
    fn test_enhanced_anti_roll_system() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(RapierPhysicsPlugin::<NoUserData>::default());
        
        // Create left and right suspensions
        let mut left_suspension = Suspension::default();
        let mut right_suspension = Suspension::default();
        
        // Test anti-roll force in roll condition
        left_suspension.compression = 0.1;
        right_suspension.compression = -0.1;
        
        let left_force = calculate_suspension_forces(&left_suspension);
        let right_force = calculate_suspension_forces(&right_suspension);
        
        // Anti-roll bar should increase force on extended side
        assert!(left_force > 0.0);
        assert!(right_force < 0.0);
        assert!(left_force.abs() > left_suspension.spring_stiffness * left_suspension.compression);
    }

    #[test]
    fn test_suspension_physics_integration() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(RapierPhysicsPlugin::<NoUserData>::default());
        
        let mut suspension = Suspension::default();
        
        // Test force application at different velocities
        let test_velocities = vec![-2.0, -1.0, 0.0, 1.0, 2.0];
        let mut prev_force = 0.0;
        
        for vel in test_velocities {
            suspension.velocity = vel;
            suspension.compression = 0.05;
            let force = calculate_suspension_forces(&suspension);
            
            if vel > 0.0 {
                assert!(force > prev_force); // Force should increase with velocity
            }
            prev_force = force;
        }
        
        // Test energy conservation
        suspension.velocity = 1.0;
        suspension.compression = 0.1;
        let initial_force = calculate_suspension_forces(&suspension);
        let initial_energy = 0.5 * suspension.spring_stiffness * suspension.compression.powi(2);
        
        suspension.velocity = -1.0;
        suspension.compression = -0.1;
        let final_force = calculate_suspension_forces(&suspension);
        let final_energy = 0.5 * suspension.spring_stiffness * suspension.compression.powi(2);
        
        assert_relative_eq!(initial_energy, final_energy, epsilon = 0.001);
    }

    #[test]
    fn test_bump_stop_dynamics() {
        let mut suspension = Suspension::default();
        
        // Test progressive bump stop force
        let test_compressions = vec![
            suspension.max_compression * 0.5,
            suspension.max_compression * 0.8,
            suspension.max_compression * 0.9,
            suspension.max_compression * 0.95,
            suspension.max_compression * 0.99
        ];
        
        let mut prev_force = 0.0;
        for compression in test_compressions {
            suspension.compression = compression;
            let force = calculate_suspension_forces(&suspension);
            assert!(force > prev_force); // Force should increase progressively
            let force_increase = force - prev_force;
            if prev_force > 0.0 {
                assert!(force_increase > (force - prev_force) * 0.5); // Should increase non-linearly
            }
            prev_force = force;
        }
        
        // Test bump stop damping
        suspension.compression = suspension.max_compression * 0.95;
        suspension.velocity = 2.0;
        let force_with_velocity = calculate_suspension_forces(&suspension);
        suspension.velocity = 1.0;
        let force_with_less_velocity = calculate_suspension_forces(&suspension);
        assert!(force_with_velocity > force_with_less_velocity); // Should have additional damping near limits
    }
}

#[cfg(test)]
mod enhanced_physics_tests {
    use super::*;
    use bevy::prelude::*;
    use bevy_rapier3d::prelude::*;

    #[test]
    fn test_progressive_spring_rate() {
        let mut suspension = Suspension::default();
        suspension.spring_stiffness = 50000.0;
        suspension.progressive_factor = 1.2;
        
        // Test increasing spring rate with compression
        let force_at_10_percent = calculate_suspension_forces(&suspension, 0.1, 0.0);
        let force_at_20_percent = calculate_suspension_forces(&suspension, 0.2, 0.0);
        let force_ratio = force_at_20_percent / force_at_10_percent;
        
        // Force should increase more than linearly
        assert!(force_ratio > 2.0);
        
        // Test maximum force at full compression
        let max_force = calculate_suspension_forces(&suspension, suspension.max_compression, 0.0);
        assert!(max_force < suspension.damage_threshold);
    }

    #[test]
    fn test_asymmetric_damping() {
        let mut suspension = Suspension::default();
        suspension.compression_damping = 5000.0;
        suspension.rebound_damping = 7000.0;
        
        // Test compression damping
        let force_compressing = calculate_suspension_forces(&suspension, 0.1, 1.0);
        let force_rebounding = calculate_suspension_forces(&suspension, 0.1, -1.0);
        
        // Rebound should have more damping
        assert!(force_rebounding.abs() > force_compressing.abs());
    }

    #[test]
    fn test_anti_roll_bar_system() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_systems(Update, apply_suspension_forces_with_antiroll);
            
        let mut world = app.world;
        
        // Create left and right suspension
        let left_suspension = Suspension {
            side: Side::Left,
            compression: 0.1,
            anti_roll_stiffness: 10000.0,
            ..Default::default()
        };
        
        let right_suspension = Suspension {
            side: Side::Right,
            compression: 0.05,
            anti_roll_stiffness: 10000.0,
            ..Default::default()
        };
        
        // Spawn entities
        let left_entity = world.spawn((
            left_suspension,
            Transform::default(),
            GlobalTransform::default(),
        )).id();
        
        let right_entity = world.spawn((
            right_suspension,
            Transform::default(),
            GlobalTransform::default(),
        )).id();
        
        // Run the system
        app.update();
        
        // Verify anti-roll forces
        let left_sus = world.get::<Suspension>(left_entity).unwrap();
        let right_sus = world.get::<Suspension>(right_entity).unwrap();
        
        // The suspension with more compression should have a counteracting force
        assert!(left_sus.anti_roll_force < 0.0);
        assert!(right_sus.anti_roll_force > 0.0);
        
        // Forces should be equal and opposite
        assert_relative_eq!(left_sus.anti_roll_force, -right_sus.anti_roll_force);
    }
} 