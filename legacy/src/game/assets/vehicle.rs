use bevy::prelude::*;
use bevy::asset::{AssetLoader, LoadContext, LoadedAsset};
use bevy::reflect::TypeUuid;
use serde::{Deserialize, Serialize};
use anyhow::Result;
use std::path::PathBuf;

/// Asset type for vehicle configurations
#[derive(Debug, Deserialize, Serialize, TypeUuid, Clone)]
#[uuid = "817c1b34-7bc4-4c62-9d0f-a2f6a9c4b491"] // Generated UUID for this asset type
pub struct VehicleConfig {
    pub name: String,
    pub model_path: PathBuf,
    pub mass: f32,
    pub dimensions: Vec3,
    pub wheel_radius: f32,
    pub wheel_positions: Vec<Vec3>,
    pub suspension_stiffness: f32,
    pub suspension_damping: f32,
    pub max_engine_force: f32,
    pub max_brake_force: f32,
    pub steering_angle: f32,
}

impl Default for VehicleConfig {
    fn default() -> Self {
        Self {
            name: "Default Vehicle".to_string(),
            model_path: PathBuf::from("models/default_vehicle.glb"),
            mass: 1500.0,
            dimensions: Vec3::new(2.0, 1.5, 4.0),
            wheel_radius: 0.4,
            wheel_positions: vec![
                Vec3::new(-1.0, -0.5, 1.5),  // Front left
                Vec3::new(1.0, -0.5, 1.5),   // Front right
                Vec3::new(-1.0, -0.5, -1.5), // Rear left
                Vec3::new(1.0, -0.5, -1.5),  // Rear right
            ],
            suspension_stiffness: 50.0,
            suspension_damping: 2.0,
            max_engine_force: 2000.0,
            max_brake_force: 100.0,
            steering_angle: 0.5,
        }
    }
}

/// Asset loader for vehicle configuration files
#[derive(Default)]
pub struct VehicleConfigLoader;

impl AssetLoader for VehicleConfigLoader {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut LoadContext,
    ) -> bevy::utils::BoxedFuture<'a, Result<()>> {
        Box::pin(async move {
            // Parse the JSON configuration
            let config: VehicleConfig = serde_json::from_slice(bytes)?;
            
            // Create the asset
            load_context.set_default_asset(LoadedAsset::new(config));
            Ok(())
        })
    }

    fn extensions(&self) -> &[&str] {
        &["vehicle.json"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::asset::AssetPlugin;
    use std::fs;

    #[test]
    fn test_vehicle_config_loading() {
        // Create a test app
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        
        // Add the vehicle config loader
        app.init_asset::<VehicleConfig>();
        app.init_asset_loader::<VehicleConfigLoader>();
        
        // Create a test vehicle config file
        let test_config = VehicleConfig {
            name: "Test Vehicle".to_string(),
            ..Default::default()
        };
        
        // Create test directory
        fs::create_dir_all("assets/test").unwrap();
        
        // Write test config to file
        let config_json = serde_json::to_string_pretty(&test_config).unwrap();
        fs::write("assets/test/test.vehicle.json", config_json).unwrap();
        
        // Get the asset server
        let asset_server = app.world.resource::<AssetServer>();
        
        // Load the test config
        let handle = asset_server.load("test/test.vehicle.json");
        
        // Run the app to process loading
        for _ in 0..10 {
            app.update();
        }
        
        // Clean up test files
        fs::remove_dir_all("assets/test").unwrap();
    }

    #[test]
    fn test_vehicle_config_defaults() {
        let config = VehicleConfig::default();
        assert_eq!(config.name, "Default Vehicle");
        assert_eq!(config.wheel_positions.len(), 4);
        assert!(config.mass > 0.0);
    }
} 