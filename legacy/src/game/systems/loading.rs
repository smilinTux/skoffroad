use bevy::prelude::*;
use bevy::asset::LoadState;
use crate::GameState;

/// Resource to track loading progress
#[derive(Resource)]
pub struct LoadingProgress {
    pub total_assets: usize,
    pub loaded_assets: usize,
}

impl Default for LoadingProgress {
    fn default() -> Self {
        Self {
            total_assets: 0,
            loaded_assets: 0,
        }
    }
}

/// System to initialize loading screen and start asset loading
pub fn setup_loading(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut loading_progress: ResMut<LoadingProgress>,
) {
    // Initialize loading screen UI here
    info!("Setting up loading screen...");
    
    // Start loading assets
    // TODO: Add actual asset handles to track
    loading_progress.total_assets = 0;
    loading_progress.loaded_assets = 0;
}

/// System to update loading progress and transition when complete
pub fn update_loading_progress(
    asset_server: Res<AssetServer>,
    mut loading_progress: ResMut<LoadingProgress>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    // Check loading state of assets
    // TODO: Add actual asset handle checks
    
    if loading_progress.loaded_assets >= loading_progress.total_assets {
        info!("Loading complete!");
        next_state.set(GameState::MainMenu);
    }
}

/// System to cleanup loading screen when transitioning away
pub fn cleanup_loading(
    mut commands: Commands,
    query: Query<Entity, With<Node>>,
) {
    // Remove loading screen UI
    for entity in query.iter() {
        commands.entity(entity).despawn_recursive();
    }
} 