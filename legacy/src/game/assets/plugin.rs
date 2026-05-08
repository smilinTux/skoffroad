use bevy::prelude::*;
use super::{GameAssets, AssetLoadingState, check_asset_loading_progress};

/// Plugin that handles asset loading and management
pub struct GameAssetsPlugin;

impl Plugin for GameAssetsPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<GameAssets>()
            .init_resource::<AssetLoadingState>()
            .add_systems(Startup, setup_assets)
            .add_systems(Update, check_asset_loading_progress);
    }
}

/// System to initialize asset loading on startup
fn setup_assets(
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