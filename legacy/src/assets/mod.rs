use bevy::prelude::*;
use bevy::asset::LoadState;
use bevy::pbr::StandardMaterial;
use bevy::scene::Scene;
use bevy::audio::AudioSource;

pub struct AssetPlugin;

impl Plugin for AssetPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GameAssets>()
           .init_resource::<AssetLoadingState>()
           .add_systems(Update, check_asset_loading_progress);
    }
}

/// Resource to hold all game asset handles
#[derive(Resource)]
pub struct GameAssets {
    // Vehicle assets
    pub vehicle_models: Vec<Handle<Scene>>,
    pub vehicle_textures: Vec<Handle<Image>>,
    pub vehicle_materials: Vec<Handle<StandardMaterial>>,
    
    // Terrain assets
    pub terrain_textures: Vec<Handle<Image>>,
    pub terrain_materials: Vec<Handle<StandardMaterial>>,
    
    // Audio assets
    pub engine_sounds: Vec<Handle<AudioSource>>,
    pub environment_sounds: Vec<Handle<AudioSource>>,
    pub impact_sounds: Vec<Handle<AudioSource>>,
    
    // UI assets
    pub ui_textures: Vec<Handle<Image>>,
    pub fonts: Vec<Handle<Font>>,
    
    // Effect assets
    pub particle_textures: Vec<Handle<Image>>,
    pub skybox: Option<Handle<Image>>,
}

impl Default for GameAssets {
    fn default() -> Self {
        Self {
            vehicle_models: Vec::new(),
            vehicle_textures: Vec::new(),
            vehicle_materials: Vec::new(),
            terrain_textures: Vec::new(),
            terrain_materials: Vec::new(),
            engine_sounds: Vec::new(),
            environment_sounds: Vec::new(),
            impact_sounds: Vec::new(),
            ui_textures: Vec::new(),
            fonts: Vec::new(),
            particle_textures: Vec::new(),
            skybox: None,
        }
    }
}

/// Resource to track asset loading progress
#[derive(Resource, Default)]
pub struct AssetLoadingState {
    pub total_assets: usize,
    pub loaded_assets: usize,
    pub failed_assets: Vec<String>,
    pub loading_complete: bool,
}

impl AssetLoadingState {
    /// Get loading progress as a percentage
    pub fn progress(&self) -> f32 {
        if self.total_assets == 0 {
            return 1.0;
        }
        self.loaded_assets as f32 / self.total_assets as f32
    }
}

/// System to check asset loading progress
fn check_asset_loading_progress(
    asset_server: Res<AssetServer>,
    mut loading_state: ResMut<AssetLoadingState>,
    game_assets: Res<GameAssets>,
) {
    // Collect all asset handles as AssetHandle enum
    let mut handles = Vec::new();
    handles.extend(game_assets.vehicle_models.iter().cloned().map(AssetHandle::Scene));
    handles.extend(game_assets.vehicle_textures.iter().cloned().map(AssetHandle::Image));
    handles.extend(game_assets.vehicle_materials.iter().cloned().map(AssetHandle::StandardMaterial));
    handles.extend(game_assets.terrain_textures.iter().cloned().map(AssetHandle::Image));
    handles.extend(game_assets.terrain_materials.iter().cloned().map(AssetHandle::StandardMaterial));
    handles.extend(game_assets.engine_sounds.iter().cloned().map(AssetHandle::AudioSource));
    handles.extend(game_assets.environment_sounds.iter().cloned().map(AssetHandle::AudioSource));
    handles.extend(game_assets.impact_sounds.iter().cloned().map(AssetHandle::AudioSource));
    handles.extend(game_assets.ui_textures.iter().cloned().map(AssetHandle::Image));
    handles.extend(game_assets.fonts.iter().cloned().map(AssetHandle::Font));
    handles.extend(game_assets.particle_textures.iter().cloned().map(AssetHandle::Image));
    if let Some(skybox) = &game_assets.skybox {
        handles.push(AssetHandle::Image(skybox.clone()));
    }

    // Update loading state
    loading_state.total_assets = handles.len();
    loading_state.loaded_assets = handles
        .iter()
        .filter(|handle| {
            matches!(
                asset_server.get_load_state(handle.id()),
                Some(LoadState::Loaded)
            )
        })
        .count();

    // Check for failed assets
    loading_state.failed_assets = handles
        .iter()
        .filter_map(|handle| {
            if matches!(
                asset_server.get_load_state(handle.id()),
                Some(LoadState::Failed)
            ) {
                Some(format!("{:?}", handle.id()))
            } else {
                None
            }
        })
        .collect();

    // Update completion state
    loading_state.loading_complete = loading_state.loaded_assets == loading_state.total_assets;
}

/// Helper functions for loading assets
impl GameAssets {
    /// Load all game assets from their respective directories
    pub fn load_all(&mut self, asset_server: &AssetServer) {
        // Load vehicle assets
        let vehicle_models_folder = asset_server.load_folder("models/vehicles");
        let vehicle_textures_folder = asset_server.load_folder("textures/vehicles");
        let terrain_textures_folder = asset_server.load_folder("textures/terrain");
        let engine_sounds_folder = asset_server.load_folder("audio/engine");
        let environment_sounds_folder = asset_server.load_folder("audio/environment");
        let impact_sounds_folder = asset_server.load_folder("audio/impacts");
        let ui_textures_folder = asset_server.load_folder("textures/ui");
        let fonts_folder = asset_server.load_folder("fonts");
        let particle_textures_folder = asset_server.load_folder("textures/particles");
        let skybox_folder = asset_server.load_folder("textures/skybox");
        // TODO: You must use asset events or AssetServer to enumerate assets in these folders and assign to the Vec<Handle<T>> fields after loading. For now, clear the vectors and store the folder handles for tracking.
        self.vehicle_models.clear();
        self.vehicle_textures.clear();
        self.terrain_textures.clear();
        self.engine_sounds.clear();
        self.environment_sounds.clear();
        self.impact_sounds.clear();
        self.ui_textures.clear();
        self.fonts.clear();
        self.particle_textures.clear();
        self.skybox = None;
        // Optionally, store the folder handles somewhere if you want to track loading completion.
    }

    /// Hot reload all assets (useful during development)
    pub fn hot_reload(&self, _asset_server: &AssetServer) {
        // Bevy's AssetServer::reload requires an AssetPath, not a handle or UntypedAssetId.
        // Since we only store handles, we cannot reliably reload assets by handle.
        // If hot reload is needed, consider tracking asset paths alongside handles.
        warn!("GameAssets::hot_reload: Hot reloading by handle is not supported. No action taken.");
    }

    /// Get all asset handles as a vector
    fn get_all_handles(&self) -> Vec<AssetHandle> {
        let mut handles = Vec::new();
        handles.extend(self.vehicle_models.iter().cloned().map(AssetHandle::Scene));
        handles.extend(self.vehicle_textures.iter().cloned().map(AssetHandle::Image));
        handles.extend(self.vehicle_materials.iter().cloned().map(AssetHandle::StandardMaterial));
        handles.extend(self.terrain_textures.iter().cloned().map(AssetHandle::Image));
        handles.extend(self.terrain_materials.iter().cloned().map(AssetHandle::StandardMaterial));
        handles.extend(self.engine_sounds.iter().cloned().map(AssetHandle::AudioSource));
        handles.extend(self.environment_sounds.iter().cloned().map(AssetHandle::AudioSource));
        handles.extend(self.impact_sounds.iter().cloned().map(AssetHandle::AudioSource));
        handles.extend(self.ui_textures.iter().cloned().map(AssetHandle::Image));
        handles.extend(self.fonts.iter().cloned().map(AssetHandle::Font));
        handles.extend(self.particle_textures.iter().cloned().map(AssetHandle::Image));
        if let Some(skybox) = &self.skybox {
            handles.push(AssetHandle::Image(skybox.clone()));
        }
        handles
    }
}

// Move AssetHandle enum and its impl outside of impl GameAssets
pub enum AssetHandle {
    Scene(Handle<Scene>),
    Image(Handle<Image>),
    StandardMaterial(Handle<StandardMaterial>),
    AudioSource(Handle<AudioSource>),
    Font(Handle<Font>),
}

impl AssetHandle {
    pub fn id(&self) -> bevy::asset::UntypedAssetId {
        match self {
            AssetHandle::Scene(h) => h.id().untyped(),
            AssetHandle::Image(h) => h.id().untyped(),
            AssetHandle::StandardMaterial(h) => h.id().untyped(),
            AssetHandle::AudioSource(h) => h.id().untyped(),
            AssetHandle::Font(h) => h.id().untyped(),
        }
    }
} 