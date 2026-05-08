#[cfg(test)]
mod tests {
    use super::*;
    use bevy::app::App;
    use bevy::asset::AssetPlugin;
    use bevy::log::LogPlugin;
    use std::path::PathBuf;

    fn setup_test_app() -> App {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            AssetPlugin {
                asset_folder: PathBuf::from("assets"),
                watch_for_changes: false,
            },
            LogPlugin::default(),
        ));
        app.init_resource::<GameAssets>();
        app.init_resource::<AssetLoadingState>();
        app
    }

    #[test]
    fn test_asset_loading_state_progress() {
        let state = AssetLoadingState {
            total_assets: 10,
            loaded_assets: 5,
            failed_assets: Vec::new(),
            loading_complete: false,
        };
        assert_eq!(state.progress(), 0.5);

        let empty_state = AssetLoadingState::default();
        assert_eq!(empty_state.progress(), 1.0);
    }

    #[test]
    fn test_game_assets_initialization() {
        let assets = GameAssets::default();
        assert!(assets.vehicle_models.is_empty());
        assert!(assets.vehicle_textures.is_empty());
        assert!(assets.engine_sounds.is_empty());
        assert!(assets.skybox.is_none());
    }

    #[test]
    fn test_asset_loading_system() {
        let mut app = setup_test_app();
        
        // Add test system to load some assets
        app.add_systems(Startup, |mut assets: ResMut<GameAssets>, asset_server: Res<AssetServer>| {
            assets.vehicle_textures.push(asset_server.load("textures/test.png"));
            assets.engine_sounds.push(asset_server.load("audio/test.ogg"));
        });
        
        // Add our asset loading progress system
        app.add_systems(Update, check_asset_loading_progress);
        
        // Run the app for a few frames
        app.update();
        
        // Check loading state
        let loading_state = app.world.resource::<AssetLoadingState>();
        assert_eq!(loading_state.total_assets, 2);
        assert!(!loading_state.loading_complete);
    }

    #[test]
    fn test_hot_reload() {
        let mut app = setup_test_app();
        let mut assets = GameAssets::default();
        let asset_server = app.world.resource::<AssetServer>();
        
        // Load some test assets
        assets.vehicle_textures.push(asset_server.load("textures/test.png"));
        assets.engine_sounds.push(asset_server.load("audio/test.ogg"));
        
        // Test hot reload
        assets.hot_reload(&asset_server);
        
        // Verify handles are still valid
        assert_eq!(assets.vehicle_textures.len(), 1);
        assert_eq!(assets.engine_sounds.len(), 1);
    }

    #[test]
    fn test_load_all() {
        let mut app = setup_test_app();
        let mut assets = GameAssets::default();
        let asset_server = app.world.resource::<AssetServer>();
        
        // Test loading all assets
        assets.load_all(&asset_server);
        
        // Run a few frames to process asset loading
        app.update();
        
        // Check that loading state is tracking
        let loading_state = app.world.resource::<AssetLoadingState>();
        assert!(loading_state.total_assets > 0);
    }

    #[test]
    fn test_failed_asset_tracking() {
        let mut app = setup_test_app();
        
        // Add test system to load a non-existent asset
        app.add_systems(Startup, |mut assets: ResMut<GameAssets>, asset_server: Res<AssetServer>| {
            assets.vehicle_textures.push(asset_server.load("textures/nonexistent.png"));
        });
        
        app.add_systems(Update, check_asset_loading_progress);
        
        // Run the app for a few frames
        app.update();
        
        // Check that the failed asset is tracked
        let loading_state = app.world.resource::<AssetLoadingState>();
        assert!(!loading_state.failed_assets.is_empty());
    }
} 