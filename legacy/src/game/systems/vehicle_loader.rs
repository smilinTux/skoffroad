use bevy::prelude::*;
use std::fs;
use std::path::Path;

use crate::game::components::vehicle_config::{Vehicle, VehicleConfig};

#[derive(Debug)]
pub enum VehicleLoadError {
    FileNotFound(String),
    InvalidJson(String),
}

/// Loads a vehicle configuration from a JSON file
pub fn load_vehicle_config(path: impl AsRef<Path>) -> Result<VehicleConfig, VehicleLoadError> {
    let path = path.as_ref();
    
    // Read the file contents
    let contents = fs::read_to_string(path)
        .map_err(|e| VehicleLoadError::FileNotFound(format!("Failed to read {}: {}", path.display(), e)))?;

    // Parse JSON into VehicleConfig
    serde_json::from_str(&contents)
        .map_err(|e| VehicleLoadError::InvalidJson(format!("Invalid JSON in {}: {}", path.display(), e)))
}

/// Spawns a vehicle entity with the given configuration
pub fn spawn_vehicle(
    commands: &mut Commands,
    config: VehicleConfig,
    transform: Transform,
) -> Entity {
    commands
        .spawn((
            Vehicle {
                config,
                ..Default::default()
            },
            transform,
            Name::new("Vehicle"),
        ))
        .id()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_load_valid_config() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test_vehicle.json");
        
        let json_content = r#"{
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
        }"#;

        let mut file = File::create(&file_path).unwrap();
        file.write_all(json_content.as_bytes()).unwrap();

        let config = load_vehicle_config(&file_path).unwrap();
        assert_eq!(config.name, "Test Vehicle");
        assert_eq!(config.mass, 2000.0);
    }

    #[test]
    fn test_load_invalid_json() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("invalid.json");
        
        let invalid_json = r#"{ invalid json }"#;
        let mut file = File::create(&file_path).unwrap();
        file.write_all(invalid_json.as_bytes()).unwrap();

        match load_vehicle_config(&file_path) {
            Err(VehicleLoadError::InvalidJson(_)) => (),
            _ => panic!("Expected InvalidJson error"),
        }
    }

    #[test]
    fn test_load_nonexistent_file() {
        let result = load_vehicle_config("nonexistent.json");
        match result {
            Err(VehicleLoadError::FileNotFound(_)) => (),
            _ => panic!("Expected FileNotFound error"),
        }
    }

    #[test]
    fn test_spawn_vehicle() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);

        let config = VehicleConfig::default();
        let transform = Transform::from_xyz(0.0, 1.0, 0.0);

        let vehicle_entity = app.world.spawn_empty().id();
        
        app.world.resource_scope(|world, mut commands: Commands| {
            let spawned_entity = spawn_vehicle(&mut commands, config.clone(), transform);
            assert_ne!(spawned_entity, vehicle_entity);
        });

        // Run systems to apply commands
        app.update();

        // Verify the spawned vehicle
        let vehicle = app.world.get::<Vehicle>(vehicle_entity);
        assert!(vehicle.is_none()); // Original entity should not have vehicle component
    }
}