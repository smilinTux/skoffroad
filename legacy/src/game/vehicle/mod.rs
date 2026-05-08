use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use serde::{Deserialize, Serialize};
use crate::game::constants::*;
use crate::game::vehicle::suspension::{
    Suspension, SuspensionType, update_advanced_suspension_physics,
};
use rand::Rng;
use glam::FloatExt;

mod chassis;
mod wheel;
mod suspension;

pub use chassis::*;
pub use wheel::*;
pub use suspension::*;

/// Configuration for a vehicle, including all physical properties and component relationships
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VehicleConfig {
    pub name: String,
    pub mass: f32,
    pub dimensions: Vec3,
    pub wheel_radius: f32,
    pub wheelbase: f32,
    pub track_width: f32,
    pub center_of_mass: Vec3,
    pub max_steering_angle: f32,
    pub suspension_config: SuspensionConfig,
    pub drivetrain_config: DrivetrainConfig,
}

impl Default for VehicleConfig {
    fn default() -> Self {
        Self {
            name: "Jeep TJ".to_string(),
            mass: 1500.0,
            dimensions: Vec3::new(JEEP_WIDTH, JEEP_HEIGHT, JEEP_LENGTH),
            wheel_radius: WHEEL_RADIUS,
            wheelbase: WHEELBASE,
            track_width: TRACK_WIDTH,
            center_of_mass: Vec3::new(0.0, -0.2, 0.0), // Slightly below center for better stability
            max_steering_angle: MAX_STEERING_ANGLE,
            suspension_config: SuspensionConfig::default(),
            drivetrain_config: DrivetrainConfig::default(),
        }
    }
}

/// Configuration for the vehicle's suspension system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuspensionConfig {
    pub spring_strength: f32,
    pub damping: f32,
    pub rest_length: f32,
    pub min_length: f32,
    pub max_length: f32,
    pub max_force: f32,
}

impl Default for SuspensionConfig {
    fn default() -> Self {
        Self {
            spring_strength: 50000.0,
            damping: 4000.0,
            rest_length: 0.5,
            min_length: 0.2,
            max_length: 0.8,
            max_force: 50000.0,
        }
    }
}

/// Configuration for the vehicle's drivetrain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DrivetrainConfig {
    pub max_engine_torque: f32,
    pub max_brake_torque: f32,
    pub gear_ratios: Vec<f32>,
    pub final_drive_ratio: f32,
    pub drive_type: DriveType,
}

impl Default for DrivetrainConfig {
    fn default() -> Self {
        Self {
            max_engine_torque: 400.0,
            max_brake_torque: 1000.0,
            gear_ratios: vec![-2.72, 0.0, 3.59, 2.19, 1.41, 1.00, 0.83],
            final_drive_ratio: 3.73,
            drive_type: DriveType::FourWD,
        }
    }
}

/// Available drive types for vehicles
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum DriveType {
    RearWD,
    FrontWD,
    FourWD,
}

/// Main component for vehicles, containing all state and references to child components
#[derive(Component)]
pub struct Vehicle {
    pub config: VehicleConfig,
    pub wheel_entities: [Entity; 4],
    pub suspension_states: [SuspensionState; 4],
    pub steering_angle: f32,
    pub throttle: f32,
    pub brake: f32,
    pub handbrake: bool,
    pub current_gear: i32,
    pub engine_rpm: f32,
    pub vehicle_speed: f32,
}

impl Default for Vehicle {
    fn default() -> Self {
        Self {
            config: VehicleConfig::default(),
            wheel_entities: [Entity::PLACEHOLDER; 4],
            suspension_states: [SuspensionState::default(); 4],
            steering_angle: 0.0,
            throttle: 0.0,
            brake: 0.0,
            handbrake: false,
            current_gear: 1,
            engine_rpm: 0.0,
            vehicle_speed: 0.0,
        }
    }
}

/// Current state of a suspension unit
#[derive(Debug, Clone, Copy)]
pub struct SuspensionState {
    pub compression: f32,
    pub velocity: f32,
    pub force: Vec3,
    pub ground_contact: bool,
    pub ground_normal: Vec3,
    pub ground_point: Vec3,
}

impl Default for SuspensionState {
    fn default() -> Self {
        Self {
            compression: 0.0,
            velocity: 0.0,
            force: Vec3::ZERO,
            ground_contact: false,
            ground_normal: -Vec3::Y,
            ground_point: Vec3::ZERO,
        }
    }
}

/// Bundle for spawning a complete vehicle with all necessary components
#[derive(Bundle)]
pub struct VehicleBundle {
    pub vehicle: Vehicle,
    pub rigid_body: RigidBody,
    pub collider: Collider,
    pub mass_properties: ColliderMassProperties,
    pub friction: Friction,
    pub restitution: Restitution,
    pub damping: Damping,
    pub external_force: ExternalForce,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub name: Name,
}

impl Default for VehicleBundle {
    fn default() -> Self {
        let config = VehicleConfig::default();
        Self {
            vehicle: Vehicle::default(),
            rigid_body: RigidBody::Dynamic,
            collider: Collider::cuboid(
                config.dimensions.x / 2.0,
                config.dimensions.y / 2.0,
                config.dimensions.z / 2.0,
            ),
            mass_properties: ColliderMassProperties::Mass(config.mass),
            friction: Friction::coefficient(0.5),
            restitution: Restitution::coefficient(0.2),
            damping: Damping {
                linear_damping: 0.2,
                angular_damping: 0.2,
            },
            external_force: ExternalForce::default(),
            transform: Transform::from_xyz(0.0, 5.0, 0.0),
            global_transform: GlobalTransform::default(),
            name: Name::new("Vehicle"),
        }
    }
}

/// Plugin that handles all vehicle-related systems
pub struct VehiclePlugin;

impl Plugin for VehiclePlugin {
    fn build(&self, app: &mut App) {
        app
            .register_type::<Vehicle>()
            .register_type::<Chassis>()
            .add_systems(
                Update,
                (
                    handle_chassis_collisions,
                    update_suspension,
                    update_advanced_suspension_physics,
                    update_vehicle_state,
                ).chain()
            );
    }
}

/// System to update vehicle state based on physics simulation
fn update_vehicle_state(
    mut vehicle_query: Query<(
        &mut Vehicle,
        &Transform,
        &Velocity,
        &mut ExternalForce,
    )>,
    chassis_query: Query<&Chassis>,
    mut wheel_query: Query<(&mut Wheel, &Transform)>,
    time: Res<Time>,
) {
    for (mut vehicle, transform, velocity, mut external_force) in vehicle_query.iter_mut() {
        // Update vehicle speed
        vehicle.vehicle_speed = velocity.linvel.length();

        // Calculate engine RPM based on wheel rotation and gear ratio
        if let Some(wheel_speed) = wheel_query
            .iter()
            .filter(|(wheel, _)| wheel.is_powered)
            .map(|(wheel, transform)| wheel.angular_velocity)
            .next()
        {
            let gear_ratio = vehicle.config.drivetrain_config.gear_ratios[vehicle.current_gear as usize];
            let final_drive = vehicle.config.drivetrain_config.final_drive_ratio;
            vehicle.engine_rpm = wheel_speed.abs() * gear_ratio * final_drive * 60.0 / (2.0 * std::f32::consts::PI);
        }

        // Apply engine torque to powered wheels
        if vehicle.throttle > 0.0 && vehicle.current_gear > 0 {
            let gear_ratio = vehicle.config.drivetrain_config.gear_ratios[vehicle.current_gear as usize];
            let final_drive = vehicle.config.drivetrain_config.final_drive_ratio;
            let total_ratio = gear_ratio * final_drive;
            
            let engine_torque = vehicle.config.drivetrain_config.max_engine_torque * vehicle.throttle;
            let wheel_torque = engine_torque * total_ratio;

            // Distribute torque based on drive type
            match vehicle.config.drivetrain_config.drive_type {
                DriveType::RearWD => {
                    // Apply torque to rear wheels only
                    for wheel_entity in &vehicle.wheel_entities[2..] {
                        if let Ok((mut wheel, _)) = wheel_query.get_mut(*wheel_entity) {
                            wheel.apply_torque(wheel_torque / 2.0);
                        }
                    }
                }
                DriveType::FrontWD => {
                    // Apply torque to front wheels only
                    for wheel_entity in &vehicle.wheel_entities[..2] {
                        if let Ok((mut wheel, _)) = wheel_query.get_mut(*wheel_entity) {
                            wheel.apply_torque(wheel_torque / 2.0);
                        }
                    }
                }
                DriveType::FourWD => {
                    // Apply torque to all wheels
                    for wheel_entity in &vehicle.wheel_entities {
                        if let Ok((mut wheel, _)) = wheel_query.get_mut(*wheel_entity) {
                            wheel.apply_torque(wheel_torque / 4.0);
                        }
                    }
                }
            }
        }

        // Apply braking force
        if vehicle.brake > 0.0 {
            let brake_torque = vehicle.config.drivetrain_config.max_brake_torque * vehicle.brake;
            for wheel_entity in &vehicle.wheel_entities {
                if let Ok((mut wheel, _)) = wheel_query.get_mut(*wheel_entity) {
                    wheel.apply_brake_torque(brake_torque);
                }
            }
        }

        // Update steering
        let max_angle = vehicle.config.max_steering_angle;
        for (i, wheel_entity) in vehicle.wheel_entities[..2].iter().enumerate() {
            if let Ok((mut wheel, _)) = wheel_query.get_mut(*wheel_entity) {
                let side_factor = if i == 0 { 1.1 } else { 0.9 }; // Ackermann steering
                wheel.steering_angle = vehicle.steering_angle * max_angle * side_factor;
            }
        }
    }
}

pub fn update_vehicle_physics(
    mut vehicle_query: Query<(&mut Vehicle, &Transform, &Velocity, &mut ExternalForce)>,
    mut wheel_query: Query<(&mut Wheel, &Transform)>,
    time: Res<Time>,
) {
    let dt = time.delta_seconds();

    for (mut vehicle, transform, velocity, mut external_force) in vehicle_query.iter_mut() {
        // Calculate wheel forces
        for (i, wheel_entity) in vehicle.wheel_entities.iter().enumerate() {
            if let Ok((mut wheel, wheel_transform)) = wheel_query.get_mut(*wheel_entity) {
                // Calculate wheel torque based on engine and drivetrain
                let gear_ratio = vehicle.config.drivetrain_config.gear_ratios.get(vehicle.current_gear as usize).copied().unwrap_or(1.0);
                let final_drive = vehicle.config.drivetrain_config.final_drive_ratio;
                let engine_torque = vehicle.config.drivetrain_config.max_engine_torque * vehicle.throttle;
                let wheel_torque = engine_torque * gear_ratio * final_drive * dt;
                
                // Distribute torque to all wheels equally (simplified)
                wheel.apply_torque(wheel_torque / 4.0);

                // Apply braking
                if vehicle.brake > 0.0 {
                    let brake_torque = vehicle.config.drivetrain_config.max_brake_torque * vehicle.brake * dt;
                    wheel.apply_brake_torque(brake_torque);
                }

                // Update steering angle for front wheels
                if i < 2 {
                    let side_factor = if i == 0 { 1.1 } else { 0.9 }; // Ackermann steering
                    wheel.steering_angle = vehicle.steering_angle * vehicle.config.max_steering_angle * side_factor;
                }
            }
        }

        // Update vehicle state
        // Optionally update vehicle state here if needed
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WheelType {
    Front,
    Rear,
    All,
} 