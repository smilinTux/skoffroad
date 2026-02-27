#[cfg(test)]
mod tests {
    use super::*;
    use bevy::prelude::*;
    use std::time::Duration;
    use bevy::app::App;
    use bevy::asset::AssetPlugin;
    use std::fs;
    use std::path::Path;
    use tempfile::tempdir;

    fn setup_test_assets() -> (App, AssetServer, GameAssets) {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(AssetPlugin::default());
        
        let asset_server = app.world.resource::<AssetServer>().clone();
        let game_assets = GameAssets::default();
        
        // Create test asset directories
        std::fs::create_dir_all("assets/test/vehicles/models").unwrap();
        std::fs::create_dir_all("assets/test/vehicles/textures").unwrap();
        std::fs::create_dir_all("assets/test/audio/engine").unwrap();
        std::fs::create_dir_all("assets/test/audio/environment").unwrap();
        std::fs::create_dir_all("assets/test/ui/textures").unwrap();
        std::fs::create_dir_all("assets/test/ui/fonts").unwrap();
        std::fs::create_dir_all("assets/test/terrain/textures").unwrap();
        std::fs::create_dir_all("assets/test/terrain/heightmaps").unwrap();
        std::fs::create_dir_all("assets/test/effects/particles").unwrap();
        std::fs::create_dir_all("assets/test/effects/weather").unwrap();
        std::fs::create_dir_all("assets/test/shaders/custom").unwrap();
        std::fs::create_dir_all("assets/test/shaders/materials").unwrap();

        (app, asset_server, game_assets)
    }

    #[test]
    fn test_asset_loading() {
        let (mut app, asset_server, mut game_assets) = setup_test_assets();
        
        // Create test assets
        std::fs::write("assets/test/ui/textures/loading.png", &[0; 100]).unwrap();
        std::fs::write("assets/test/ui/fonts/main.ttf", &[0; 100]).unwrap();
        std::fs::write("assets/test/vehicles/models/truck.glb", &[0; 100]).unwrap();
        std::fs::write("assets/test/audio/engine/idle.ogg", &[0; 100]).unwrap();
        
        game_assets.load_all(&asset_server);
        
        // Run the app to process asset loading
        app.update();
        
        let loading_state = app.world.resource::<AssetLoadingState>();
        assert_eq!(loading_state.total_assets, 4);
        assert!(game_assets.ui_textures.contains_key("loading.png"));
        assert!(game_assets.ui_fonts.contains_key("main.ttf"));
        assert!(game_assets.vehicle_models.contains_key("truck.glb"));
        assert!(game_assets.audio_engine.contains_key("idle.ogg"));
    }

    #[test]
    fn test_asset_loading_priority() {
        let (mut app, asset_server, mut game_assets) = setup_test_assets();
        
        // Create test assets with different priorities
        std::fs::write("assets/test/ui/textures/critical.png", &[0; 100]).unwrap();
        std::fs::write("assets/test/vehicles/models/high.glb", &[0; 100]).unwrap();
        std::fs::write("assets/test/terrain/textures/medium.png", &[0; 100]).unwrap();
        std::fs::write("assets/test/audio/environment/low.ogg", &[0; 100]).unwrap();
        
        game_assets.load_all(&asset_server);
        
        // Run one update to start loading
        app.update();
        
        let loading_state = app.world.resource::<AssetLoadingState>();
        assert_eq!(loading_state.current_priority, LoadPriority::Critical);
        
        // Run until critical assets are loaded
        while app.world.resource::<AssetLoadingState>().current_priority == LoadPriority::Critical {
            app.update();
        }
        
        assert!(game_assets.ui_textures.contains_key("critical.png"));
        assert_eq!(app.world.resource::<AssetLoadingState>().current_priority, LoadPriority::High);
    }

    #[test]
    fn test_asset_loading_progress() {
        let (mut app, asset_server, mut game_assets) = setup_test_assets();
        
        // Create multiple test assets
        for i in 0..5 {
            std::fs::write(format!("assets/test/ui/textures/test{}.png", i), &[0; 100]).unwrap();
        }
        
        game_assets.load_all(&asset_server);
        
        // Initial state
        app.update();
        let initial_state = app.world.resource::<AssetLoadingState>();
        assert_eq!(initial_state.total_assets, 5);
        assert_eq!(initial_state.loaded_assets, 0);
        
        // Run until complete
        while !app.world.resource::<AssetLoadingState>().is_complete {
            app.update();
        }
        
        let final_state = app.world.resource::<AssetLoadingState>();
        assert_eq!(final_state.loaded_assets, 5);
        assert!(final_state.is_complete);
    }

    #[test]
    fn test_hot_reload() {
        let (mut app, asset_server, mut game_assets) = setup_test_assets();
        
        // Create initial asset
        std::fs::write("assets/test/ui/textures/reload_test.png", &[0; 100]).unwrap();
        
        game_assets.load_all(&asset_server);
        app.update();
        
        // Modify the asset
        std::fs::write("assets/test/ui/textures/reload_test.png", &[1; 100]).unwrap();
        
        // Trigger hot reload
        #[cfg(debug_assertions)]
        game_assets.hot_reload(&asset_server);
        
        app.update();
        
        // Verify the asset was reloaded
        #[cfg(debug_assertions)]
        assert!(game_assets.ui_textures.contains_key("reload_test.png"));
    }

    #[test]
    fn test_failed_asset_loading() {
        let (mut app, asset_server, mut game_assets) = setup_test_assets();
        
        // Create an invalid asset
        std::fs::write("assets/test/vehicles/models/invalid.glb", &[0; 1]).unwrap();
        
        game_assets.load_all(&asset_server);
        
        // Run until loading completes or fails
        while !app.world.resource::<AssetLoadingState>().is_complete {
            app.update();
        }
        
        let final_state = app.world.resource::<AssetLoadingState>();
        assert_eq!(final_state.failed_assets, 1);
        assert!(!game_assets.vehicle_models.contains_key("invalid.glb"));
    }

    #[test]
    fn test_loading_queue() {
        let (mut app, asset_server, mut game_assets) = setup_test_assets();
        
        // Create test assets
        std::fs::write("assets/test/ui/textures/queue1.png", &[0; 100]).unwrap();
        std::fs::write("assets/test/ui/textures/queue2.png", &[0; 100]).unwrap();
        
        game_assets.load_all(&asset_server);
        app.update();
        
        let loading_state = app.world.resource::<AssetLoadingState>();
        assert_eq!(loading_state.loading_queue.len(), 2);
        
        // Run until queue is empty
        while !app.world.resource::<AssetLoadingState>().loading_queue.is_empty() {
            app.update();
        }
        
        assert!(game_assets.ui_textures.contains_key("queue1.png"));
        assert!(game_assets.ui_textures.contains_key("queue2.png"));
    }

    #[test]
    fn test_current_loading_priority() {
        let (mut app, asset_server, mut game_assets) = setup_test_assets();
        
        // Create assets of different priorities
        std::fs::write("assets/test/ui/textures/ui.png", &[0; 100]).unwrap(); // Critical
        std::fs::write("assets/test/vehicles/models/car.glb", &[0; 100]).unwrap(); // High
        std::fs::write("assets/test/terrain/textures/ground.png", &[0; 100]).unwrap(); // Medium
        std::fs::write("assets/test/audio/environment/ambient.ogg", &[0; 100]).unwrap(); // Low
        
        game_assets.load_all(&asset_server);
        
        // Check priority transitions
        let priorities = [
            LoadPriority::Critical,
            LoadPriority::High,
            LoadPriority::Medium,
            LoadPriority::Low,
        ];
        
        for expected_priority in priorities.iter() {
            while app.world.resource::<AssetLoadingState>().current_priority == *expected_priority {
                app.update();
            }
        }
        
        assert!(app.world.resource::<AssetLoadingState>().is_complete);
    }

    #[test]
    fn test_partial_asset_loading() {
        let (mut app, asset_server, mut game_assets) = setup_test_assets();
        
        // Only create critical assets
        std::fs::write("assets/test/ui/textures/loading.png", &[0; 100]).unwrap();
        std::fs::write("assets/test/ui/fonts/critical.ttf", &[0; 100]).unwrap();
        
        game_assets.load_all(&asset_server);
        
        // Run until complete
        while !app.world.resource::<AssetLoadingState>().is_complete {
            app.update();
        }
        
        let final_state = app.world.resource::<AssetLoadingState>();
        assert_eq!(final_state.total_assets, 2);
        assert_eq!(final_state.loaded_assets, 2);
        assert_eq!(final_state.failed_assets, 0);
        assert!(game_assets.ui_textures.contains_key("loading.png"));
        assert!(game_assets.ui_fonts.contains_key("critical.ttf"));
    }

    #[test]
    fn test_vehicle_config_loading() {
        let mut app = setup_test_app();
        
        // Create a temporary directory for test assets
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("test_vehicle.json");
        
        // Create a test vehicle config
        let test_config = VehicleConfig {
            name: "Test Vehicle".to_string(),
            mass: 1500.0,
            dimensions: Vec3::new(2.0, 1.5, 4.0),
            suspension_config: SuspensionConfig {
                spring_rate: 50000.0,
                damping: 5000.0,
                travel: 0.3,
            },
            engine_config: EngineConfig {
                max_power: 300.0,
                max_torque: 400.0,
                redline: 7000.0,
            },
        };
        
        // Write config to file
        fs::write(
            &config_path,
            serde_json::to_string(&test_config).unwrap(),
        ).unwrap();
        
        // Get the GameAssets resource
        let game_assets = app.world.resource_mut::<GameAssets>();
        let asset_server = app.world.resource::<AssetServer>();
        
        // Load the config
        let handle = game_assets.get_or_load::<VehicleConfig>(
            config_path.to_str().unwrap(),
            &asset_server,
            LoadPriority::Critical,
        );
        
        // Update the app to process asset loading
        app.update();
        
        // Verify the config was loaded
        assert!(game_assets.cached_handles.contains_key(config_path.to_str().unwrap()));
    }

    #[test]
    fn test_asset_validation() {
        let mut app = setup_test_app();
        
        // Create temporary test directories
        let temp_dir = tempdir().unwrap();
        let ui_textures_dir = temp_dir.path().join("ui/textures");
        let vehicles_dir = temp_dir.path().join("vehicles/models");
        
        fs::create_dir_all(&ui_textures_dir).unwrap();
        fs::create_dir_all(&vehicles_dir).unwrap();
        
        // Create some test files
        fs::write(ui_textures_dir.join("button.png"), "dummy data").unwrap();
        fs::write(vehicles_dir.join("jeep.glb"), "dummy data").unwrap();
        
        // Get the GameAssets resource
        let mut game_assets = app.world.resource_mut::<GameAssets>();
        let asset_server = app.world.resource::<AssetServer>();
        
        // Run validation
        let errors = game_assets.validate_assets(&asset_server);
        
        // We expect errors since we haven't loaded the required assets
        assert!(!errors.is_empty());
        assert!(errors.iter().any(|e| e.contains("button.png")));
        assert!(errors.iter().any(|e| e.contains("jeep.glb")));
    }

    #[test]
    fn test_cache_management() {
        let mut app = setup_test_app();
        
        // Get the GameAssets resource
        let mut game_assets = app.world.resource_mut::<GameAssets>();
        let asset_server = app.world.resource::<AssetServer>();
        
        // Load some test assets with different priorities
        game_assets.get_or_load::<Scene>(
            "test_high.glb",
            &asset_server,
            LoadPriority::High,
        );
        
        game_assets.get_or_load::<Scene>(
            "test_low.glb",
            &asset_server,
            LoadPriority::Low,
        );
        
        // Clear low priority assets
        game_assets.clear_unused_assets(LoadPriority::Medium);
        
        // Verify only high priority assets remain
        assert!(game_assets.cached_handles.contains_key("test_high.glb"));
        assert!(!game_assets.cached_handles.contains_key("test_low.glb"));
    }

    #[test]
    fn test_loading_state_tracking() {
        let mut app = setup_test_app();
        
        // Get the loading state
        let mut loading_state = app.world.resource_mut::<AssetLoadingState>();
        
        // Add some test assets to the queue
        loading_state.add_pending_asset("test1.png", LoadPriority::High);
        loading_state.add_pending_asset("test2.png", LoadPriority::Medium);
        
        assert_eq!(loading_state.total_assets, 2);
        assert_eq!(loading_state.loaded_assets, 0);
        
        // Simulate loading progress
        loading_state.mark_asset_loaded("test1.png");
        
        assert_eq!(loading_state.loaded_assets, 1);
        assert_eq!(loading_state.failed_assets, 0);
    }

    // Cleanup after tests
    impl Drop for GameAssets {
        fn drop(&mut self) {
            std::fs::remove_dir_all("assets/test").unwrap_or_default();
        }
    }
}

// Helper function to create test directories
fn create_test_directory(path: &Path) -> std::io::Result<()> {
    if !path.exists() {
        fs::create_dir_all(path)?;
    }
    Ok(())
} 