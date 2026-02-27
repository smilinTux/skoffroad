use bevy::prelude::*;
use bevy::math::Vec3;
use serde::{Deserialize, Serialize};

/// Component for core vehicle properties and behavior
#[derive(Component, Debug)]
pub struct Vehicle {
    // Movement properties
    pub speed: f32,
    pub acceleration: f32,
    pub max_speed: f32,
    pub turn_speed: f32,
    
    // Ground detection
    pub ground_check_ray: f32,
    pub is_grounded: bool,
    
    // Physics properties
    pub mass: f32,
    pub center_of_mass_offset: Vec3,
    pub config: VehicleConfig,
    pub velocity: Vec3,
    pub angular_velocity: Vec3,
    pub engine_force: f32,
    pub brake_force: f32,
    pub steering_angle: f32,
}

impl Default for Vehicle {
    fn default() -> Self {
        Self {
            speed: 0.0,
            acceleration: 15.0,
            max_speed: 30.0,
            turn_speed: 2.0,
            ground_check_ray: 0.5,
            is_grounded: false,
            mass: 1500.0, // kg
            center_of_mass_offset: Vec3::new(0.0, -0.5, 0.0), // Slightly lowered center of mass
            config: VehicleConfig {
                name: "Default Vehicle".to_string(),
                model_path: "models/vehicles/default.glb".to_string(),
                mass: 1000.0,
                dimensions: Vec3Config {
                    x: 4.0,
                    y: 1.5,
                    z: 2.0,
                },
                wheel_radius: 0.3,
                wheel_positions: vec![
                    Vec3Config { x: 1.5, y: -0.5, z: 0.8 },
                    Vec3Config { x: 1.5, y: -0.5, z: -0.8 },
                    Vec3Config { x: -1.5, y: -0.5, z: 0.8 },
                    Vec3Config { x: -1.5, y: -0.5, z: -0.8 },
                ],
                suspension_stiffness: 40.0,
                suspension_damping: 4.0,
                max_engine_force: 2000.0,
                max_brake_force: 800.0,
                max_steering_angle: 0.4,
            },
            velocity: Vec3::ZERO,
            angular_velocity: Vec3::ZERO,
            engine_force: 0.0,
            brake_force: 0.0,
            steering_angle: 0.0,
        }
    }
}

/// Component for vehicle suspension system
#[derive(Component, Debug)]
pub struct Suspension {
    // Spring properties
    pub spring_strength: f32,
    pub damping: f32,
    pub rest_length: f32,
    pub min_length: f32,
    pub max_length: f32,
    pub max_force: f32,
    
    // Wheel configuration
    pub wheel_positions: Vec<Vec3>,
    pub wheel_radius: f32,
    
    // Runtime state
    pub previous_lengths: Vec<f32>,
    pub wheel_entities: Vec<Entity>,
}

impl Default for Suspension {
    fn default() -> Self {
        Self {
            spring_strength: 50000.0,
            damping: 4000.0,
            rest_length: 0.5,
            min_length: 0.2,
            max_length: 0.8,
            max_force: 100000.0,
            wheel_positions: vec![
                Vec3::new(-0.8, 0.0, 1.0),  // Front left
                Vec3::new(0.8, 0.0, 1.0),   // Front right
                Vec3::new(-0.8, 0.0, -1.0), // Rear left
                Vec3::new(0.8, 0.0, -1.0),  // Rear right
            ],
            wheel_radius: 0.4,
            previous_lengths: vec![0.5; 4],
            wheel_entities: Vec::new(),
        }
    }
}

/// Component for individual wheel properties
#[derive(Component, Debug)]
pub struct Wheel {
    pub index: usize,
    pub steering_angle: f32,
    pub angular_velocity: f32,
    pub torque: f32,
    pub can_steer: bool,
    pub can_drive: bool,
}

impl Default for Wheel {
    fn default() -> Self {
        Self {
            index: 0,
            steering_angle: 0.0,
            angular_velocity: 0.0,
            torque: 0.0,
            can_steer: false,
            can_drive: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vec3Config {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl From<Vec3Config> for Vec3 {
    fn from(config: Vec3Config) -> Self {
        Vec3::new(config.x, config.y, config.z)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VehicleConfig {
    pub name: String,
    pub model_path: String,
    pub mass: f32,
    pub dimensions: Vec3Config,
    pub wheel_radius: f32,
    pub wheel_positions: Vec<Vec3Config>,
    pub suspension_stiffness: f32,
    pub suspension_damping: f32,
    pub max_engine_force: f32,
    pub max_brake_force: f32,
    pub max_steering_angle: f32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_vehicle_config_deserialization() {
        let json_data = json!({
            "name": "Test Vehicle",
            "model_path": "models/test.glb",
            "mass": 1500.0,
            "dimensions": {
                "x": 4.5,
                "y": 1.8,
                "z": 2.2
            },
            "wheel_radius": 0.4,
            "wheel_positions": [
                {"x": 1.8, "y": -0.5, "z": 0.9},
                {"x": 1.8, "y": -0.5, "z": -0.9},
                {"x": -1.8, "y": -0.5, "z": 0.9},
                {"x": -1.8, "y": -0.5, "z": -0.9}
            ],
            "suspension_stiffness": 45.0,
            "suspension_damping": 4.5,
            "max_engine_force": 2500.0,
            "max_brake_force": 900.0,
            "max_steering_angle": 0.45
        });

        let config: VehicleConfig = serde_json::from_value(json_data).unwrap();
        assert_eq!(config.name, "Test Vehicle");
        assert_eq!(config.mass, 1500.0);
        assert_eq!(config.dimensions.x, 4.5);
        assert_eq!(config.wheel_positions.len(), 4);
        assert_eq!(config.wheel_positions[0].x, 1.8);
    }

    #[test]
    fn test_vehicle_default() {
        let vehicle = Vehicle::default();
        assert_eq!(vehicle.config.name, "Default Vehicle");
        assert_eq!(vehicle.velocity, Vec3::ZERO);
        assert_eq!(vehicle.steering_angle, 0.0);
    }

    #[test]
    fn test_vec3_config_conversion() {
        let config = Vec3Config {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        };
        let vec3: Vec3 = config.into();
        assert_eq!(vec3.x, 1.0);
        assert_eq!(vec3.y, 2.0);
        assert_eq!(vec3.z, 3.0);
    }
} 