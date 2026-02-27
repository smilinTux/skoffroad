use bevy::prelude::*;
use bevy::asset::{AssetServer, LoadState};
use crate::game::assets::{GameAssets, AssetLoadingState, LoadPriority};

/// Plugin that handles asset loading and management
pub struct AssetLoaderPlugin;

impl Plugin for AssetLoaderPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GameAssets>()
            .init_resource::<AssetLoadingState>()
            .add_systems(Startup, setup_asset_loading)
            .add_systems(Update, (
                check_asset_loading_progress,
                handle_asset_loading_queue,
            ).chain());
    }
}

/// System to initialize asset loading on startup
fn setup_asset_loading(
    mut game_assets: ResMut<GameAssets>,
    asset_server: Res<AssetServer>,
    mut loading_state: ResMut<AssetLoadingState>,
) {
    info!("Starting asset loading...");
    *loading_state = game_assets.load_all(&asset_server);
    info!(
        "Asset loading initialized. Total assets to load: {}",
        loading_state.total_assets
    );
}

/// System to check asset loading progress and update loading state
fn check_asset_loading_progress(
    asset_server: Res<AssetServer>,
    game_assets: Res<GameAssets>,
    mut loading_state: ResMut<AssetLoadingState>,
) {
    let state = game_assets.check_loading_progress(&asset_server);
    
    // Update loading state
    loading_state.loaded_assets = state.loaded_assets;
    loading_state.failed_assets = state.failed_assets;
    loading_state.is_complete = state.is_complete;
    
    // Log progress
    if loading_state.loaded_assets > 0 && loading_state.loaded_assets % 10 == 0 {
        info!(
            "Loading progress: {}/{} assets loaded ({:.1}%)",
            loading_state.loaded_assets,
            loading_state.total_assets,
            (loading_state.loaded_assets as f32 / loading_state.total_assets as f32) * 100.0
        );
    }
    
    // Log failed assets
    if state.failed_assets > 0 {
        warn!("Failed to load {} assets", state.failed_assets);
    }
}

/// System to handle the asset loading queue based on priority
fn handle_asset_loading_queue(
    asset_server: Res<AssetServer>,
    mut loading_state: ResMut<AssetLoadingState>,
) {
    // Process the loading queue based on priority
    while let Some(asset_path) = loading_state.loading_queue.pop_front() {
        // Load the asset
        asset_server.load(&asset_path);
        
        // Update loading state
        loading_state.loaded_assets += 1;
        
        // Break if we've loaded enough assets for this frame
        // This prevents loading too many assets at once and causing frame drops
        if loading_state.loaded_assets % 5 == 0 {
            break;
        }
    }
    
    // Update completion state
    loading_state.is_complete = loading_state.loading_queue.is_empty() &&
        loading_state.loaded_assets == loading_state.total_assets;
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::app::App;
    use bevy::asset::AssetPlugin;
    use std::time::Duration;

    #[test]
    fn test_asset_loading_plugin() {
        let mut app = App::new();
        
        // Add required plugins
        app.add_plugins((
            MinimalPlugins,
            AssetPlugin::default(),
        ));
        
        // Add our asset loading plugin
        app.add_plugin(AssetLoaderPlugin);
        
        // Run the app for a few frames to test loading
        for _ in 0..10 {
            app.update();
        }
        
        // Check that resources were initialized
        assert!(app.world.contains_resource::<GameAssets>());
        assert!(app.world.contains_resource::<AssetLoadingState>());
    }

    #[test]
    fn test_loading_progress() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.add_plugin(AssetLoaderPlugin);
        
        // Add some test assets
        let mut game_assets = GameAssets::default();
        let asset_server = app.world.resource::<AssetServer>();
        
        // Create test directories and files
        std::fs::create_dir_all("assets/test").unwrap();
        std::fs::write("assets/test/test.png", vec![0; 100]).unwrap();
        
        // Load test assets
        let loading_state = game_assets.load_all(&asset_server);
        assert!(loading_state.total_assets > 0);
        
        // Run the app to process loading
        for _ in 0..20 {
            app.update();
        }
        
        // Clean up test files
        std::fs::remove_dir_all("assets/test").unwrap();
    }
} 