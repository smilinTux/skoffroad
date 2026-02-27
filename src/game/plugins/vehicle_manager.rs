use bevy::prelude::*;
use crate::game::assets::vehicle::{VehicleConfig, VehicleConfigLoader};
use std::collections::HashMap;

/// Resource to track loaded vehicle configurations
#[derive(Resource, Default)]
pub struct VehicleAssets {
    configs: HashMap<String, Handle<VehicleConfig>>,
    loading_complete: bool,
}

/// Plugin for managing vehicle assets
pub struct VehicleManagerPlugin;

impl Plugin for VehicleManagerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<VehicleAssets>()
            .init_asset::<VehicleConfig>()
            .init_asset_loader::<VehicleConfigLoader>()
            .add_systems(Startup, setup_vehicle_assets)
            .add_systems(Update, check_vehicle_loading);
    }
}

/// System to initialize vehicle asset loading
fn setup_vehicle_assets(
    mut vehicle_assets: ResMut<VehicleAssets>,
    asset_server: Res<AssetServer>,
) {
    // Load all vehicle configurations from the assets directory
    let vehicles = [
        "offroad_truck",
        "buggy",
        "trophy_truck",
        // Add more vehicle types here
    ];

    for vehicle in vehicles.iter() {
        let path = format!("vehicles/{}.vehicle.json", vehicle);
        let handle = asset_server.load(&path);
        vehicle_assets.configs.insert(vehicle.to_string(), handle);
    }
}

/// System to check vehicle asset loading progress
fn check_vehicle_loading(
    mut vehicle_assets: ResMut<VehicleAssets>,
    asset_server: Res<AssetServer>,
) {
    if vehicle_assets.loading_complete {
        return;
    }

    // Check if all vehicle configs are loaded
    let handles: Vec<_> = vehicle_assets.configs.values().cloned().collect();
    if asset_server.is_loaded_with_dependencies(handles.as_slice()) {
        info!("All vehicle configurations loaded successfully");
        vehicle_assets.loading_complete = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_vehicle_manager_plugin() {
        // Create test directory and files
        fs::create_dir_all("assets/vehicles").unwrap();
        
        let test_config = VehicleConfig {
            name: "Test Offroad Truck".to_string(),
            ..Default::default()
        };
        
        let config_json = serde_json::to_string_pretty(&test_config).unwrap();
        fs::write(
            "assets/vehicles/offroad_truck.vehicle.json",
            config_json
        ).unwrap();

        // Create test app
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.add_plugins(VehicleManagerPlugin);

        // Initial state check
        app.update();
        let vehicle_assets = app.world.resource::<VehicleAssets>();
        assert!(!vehicle_assets.loading_complete);
        assert!(vehicle_assets.configs.contains_key("offroad_truck"));

        // Run updates to process loading
        for _ in 0..10 {
            app.update();
        }

        // Clean up test files
        fs::remove_dir_all("assets/vehicles").unwrap();
    }

    #[test]
    fn test_vehicle_assets_resource() {
        let assets = VehicleAssets::default();
        assert!(assets.configs.is_empty());
        assert!(!assets.loading_complete);
    }
} 