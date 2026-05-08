use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Component, Debug, Clone, Serialize, Deserialize)]
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

#[derive(Component, Debug, Clone, Serialize, Deserialize)]
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

impl Default for VehicleConfig {
    fn default() -> Self {
        Self {
            name: "Default Vehicle".to_string(),
            model_path: "models/default.glb".to_string(),
            mass: 1500.0,
            dimensions: Vec3Config {
                x: 4.0,
                y: 1.8,
                z: 2.0,
            },
            wheel_radius: 0.4,
            wheel_positions: vec![
                Vec3Config { x: 1.3, y: -0.5, z: 0.8 },
                Vec3Config { x: 1.3, y: -0.5, z: -0.8 },
                Vec3Config { x: -1.3, y: -0.5, z: 0.8 },
                Vec3Config { x: -1.3, y: -0.5, z: -0.8 },
            ],
            suspension_stiffness: 45.0,
            suspension_damping: 4.5,
            max_engine_force: 2500.0,
            max_brake_force: 800.0,
            max_steering_angle: 0.4,
        }
    }
}

#[derive(Component, Debug)]
pub struct Vehicle {
    pub config: VehicleConfig,
    pub current_speed: f32,
    pub current_steering: f32,
    pub engine_force: f32,
    pub brake_force: f32,
}

impl Default for Vehicle {
    fn default() -> Self {
        Self {
            config: VehicleConfig::default(),
            current_speed: 0.0,
            current_steering: 0.0,
            engine_force: 0.0,
            brake_force: 0.0,
        }
    }
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
            "mass": 2000.0,
            "dimensions": {
                "x": 4.5,
                "y": 2.0,
                "z": 2.2
            },
            "wheel_radius": 0.5,
            "wheel_positions": [
                {"x": 1.5, "y": -0.6, "z": 1.0},
                {"x": 1.5, "y": -0.6, "z": -1.0},
                {"x": -1.5, "y": -0.6, "z": 1.0},
                {"x": -1.5, "y": -0.6, "z": -1.0}
            ],
            "suspension_stiffness": 55.0,
            "suspension_damping": 5.5,
            "max_engine_force": 3500.0,
            "max_brake_force": 1200.0,
            "max_steering_angle": 0.6
        });

        let config: VehicleConfig = serde_json::from_value(json_data).unwrap();
        
        assert_eq!(config.name, "Test Vehicle");
        assert_eq!(config.mass, 2000.0);
        assert_eq!(config.dimensions.x, 4.5);
        assert_eq!(config.wheel_positions.len(), 4);
        assert_eq!(config.wheel_positions[0].x, 1.5);
    }

    #[test]
    fn test_default_vehicle() {
        let vehicle = Vehicle::default();
        assert_eq!(vehicle.current_speed, 0.0);
        assert_eq!(vehicle.current_steering, 0.0);
        assert_eq!(vehicle.config.name, "Default Vehicle");
    }

    #[test]
    fn test_vec3_conversion() {
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