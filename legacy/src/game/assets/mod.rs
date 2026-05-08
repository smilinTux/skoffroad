use bevy::prelude::*;
use bevy::asset::{LoadState, AssetServer, Asset, AssetLoader, LoadContext, BoxedFuture};
use bevy::pbr::StandardMaterial;
use bevy::scene::Scene;
use bevy::audio::AudioSource;
use std::collections::{HashMap, VecDeque};
use bevy::reflect::TypeUuid;
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;

/// Asset loading priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LoadPriority {
    Critical,   // Must be loaded before game starts (UI, core assets)
    High,       // Load as soon as possible (player vehicle, current level)
    Medium,     // Load during gameplay (other vehicles, effects)
    Low,        // Can be loaded in background (unused assets)
}

/// Represents an asset to be loaded with its priority
#[derive(Debug)]
struct PendingAsset<T: Asset> {
    path: String,
    priority: LoadPriority,
    handle: Handle<T>,
}

/// Tracks the loading state of game assets
#[derive(Resource)]
pub struct AssetLoadingState {
    pub total_assets: usize,
    pub loaded_assets: usize,
    pub failed_assets: usize,
    pub is_complete: bool,
    pub loading_queue: VecDeque<String>,
    pub current_priority: LoadPriority,
}

impl Default for AssetLoadingState {
    fn default() -> Self {
        Self {
            total_assets: 0,
            loaded_assets: 0,
            failed_assets: 0,
            is_complete: false,
            loading_queue: VecDeque::new(),
            current_priority: LoadPriority::Critical,
        }
    }
}

/// Type-safe asset collection
#[derive(Default)]
pub struct AssetCollection<T: Asset> {
    handles: HashMap<String, Handle<T>>,
    _phantom: PhantomData<T>,
}

impl<T: Asset> AssetCollection<T> {
    pub fn get(&self, key: &str) -> Option<&Handle<T>> {
        self.handles.get(key)
    }

    pub fn insert(&mut self, key: String, handle: Handle<T>) {
        self.handles.insert(key, handle);
    }

    pub fn values(&self) -> impl Iterator<Item = &Handle<T>> {
        self.handles.values()
    }

    pub fn is_empty(&self) -> bool {
        self.handles.is_empty()
    }
}

// Custom asset types
#[derive(Debug, Deserialize, Serialize, TypeUuid)]
#[uuid = "f9e6db21-a9e0-4e5a-b7d1-b5795b6c6c43"]
pub struct VehicleConfig {
    pub name: String,
    pub mass: f32,
    pub dimensions: Vec3,
    pub suspension_config: SuspensionConfig,
    pub engine_config: EngineConfig,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SuspensionConfig {
    pub spring_rate: f32,
    pub damping: f32,
    pub travel: f32,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct EngineConfig {
    pub max_power: f32,
    pub max_torque: f32,
    pub redline: f32,
}

// Custom asset loader for VehicleConfig
pub struct VehicleConfigLoader;

impl AssetLoader for VehicleConfigLoader {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<VehicleConfig, anyhow::Error>> {
        Box::pin(async move {
            let config: VehicleConfig = serde_json::from_slice(bytes)?;
            Ok(config)
        })
    }

    fn extensions(&self) -> &[&str] {
        &["json"]
    }
}

/// Resource that holds all game assets
#[derive(Resource)]
pub struct GameAssets {
    // Vehicle assets
    pub vehicle_models: AssetCollection<Scene>,
    pub vehicle_textures: AssetCollection<Image>,
    pub vehicle_materials: AssetCollection<StandardMaterial>,
    pub vehicle_configs: AssetCollection<VehicleConfig>,
    
    // Audio assets
    pub engine_sounds: AssetCollection<AudioSource>,
    pub environment_sounds: AssetCollection<AudioSource>,
    pub music_tracks: AssetCollection<AudioSource>,
    pub ui_sounds: AssetCollection<AudioSource>,
    pub radio_stations: AssetCollection<AudioSource>,
    pub voice_lines: AssetCollection<AudioSource>,
    
    // UI assets
    pub ui_textures: AssetCollection<Image>,
    pub ui_icons: AssetCollection<Image>,
    pub fonts: AssetCollection<Font>,
    pub ui_animations: AssetCollection<Scene>,
    
    // Effect assets
    pub particle_textures: AssetCollection<Image>,
    pub weather_effects: AssetCollection<Scene>,
    pub decal_textures: AssetCollection<Image>,
    pub trail_markers: AssetCollection<Scene>,
    
    // Terrain assets
    pub terrain_textures: AssetCollection<Image>,
    pub terrain_materials: AssetCollection<StandardMaterial>,
    pub terrain_heightmaps: AssetCollection<Image>,
    pub terrain_objects: AssetCollection<Scene>,
    
    // Shader assets
    pub custom_shaders: AssetCollection<Shader>,
    pub shader_materials: AssetCollection<StandardMaterial>,
    
    // Visualization assets
    pub visualization_textures: AssetCollection<Image>,
    pub visualization_materials: AssetCollection<StandardMaterial>,
    pub visualization_gradients: AssetCollection<ColorGradient>,
    pub visualization_icons: AssetCollection<Image>,
    
    // Performance metrics
    pub metrics_textures: AssetCollection<Image>,
    pub metrics_fonts: AssetCollection<Font>,
    
    // Cache validation
    cached_handles: HashMap<String, (Handle<Image>, LoadPriority)>,
    asset_validation_errors: Vec<String>,
}

impl Default for GameAssets {
    fn default() -> Self {
        Self {
            vehicle_models: Default::default(),
            vehicle_textures: Default::default(),
            vehicle_materials: Default::default(),
            vehicle_configs: Default::default(),
            
            engine_sounds: Default::default(),
            environment_sounds: Default::default(),
            music_tracks: Default::default(),
            ui_sounds: Default::default(),
            radio_stations: Default::default(),
            voice_lines: Default::default(),
            
            ui_textures: Default::default(),
            ui_icons: Default::default(),
            fonts: Default::default(),
            ui_animations: Default::default(),
            
            particle_textures: Default::default(),
            weather_effects: Default::default(),
            decal_textures: Default::default(),
            trail_markers: Default::default(),
            
            terrain_textures: Default::default(),
            terrain_materials: Default::default(),
            terrain_heightmaps: Default::default(),
            terrain_objects: Default::default(),
            
            custom_shaders: Default::default(),
            shader_materials: Default::default(),
            
            visualization_textures: AssetCollection::new(),
            visualization_materials: AssetCollection::new(),
            visualization_gradients: AssetCollection::new(),
            visualization_icons: AssetCollection::new(),
            
            metrics_textures: AssetCollection::new(),
            metrics_fonts: AssetCollection::new(),
            
            cached_handles: HashMap::new(),
            asset_validation_errors: Vec::new(),
        }
    }
}

impl GameAssets {
    /// Load all game assets from their respective directories with prioritization
    pub fn load_all(&mut self, asset_server: &AssetServer) -> AssetLoadingState {
        let mut loading_state = AssetLoadingState::default();
        
        // Helper closure to load assets into a collection
        let mut load_directory = |dir: &str, ext: &str, collection: &mut AssetCollection<impl Asset>, priority: LoadPriority| {
            if let Ok(paths) = std::fs::read_dir(dir) {
                for path in paths.flatten() {
                    if let Some(filename) = path.file_name().to_str() {
                        if filename.ends_with(ext) {
                            let key = filename.trim_end_matches(ext).trim_end_matches('.').to_string();
                            let asset_path = format!("{}/{}", dir, filename);
                            let handle = asset_server.load(&asset_path);
                            
                            collection.insert(key, handle.clone());
                            loading_state.loading_queue.push_back(asset_path);
                            loading_state.total_assets += 1;
                            
                            // Cache the handle with priority
                            self.cached_handles.insert(
                                asset_path,
                                (handle.clone_weak().typed(), priority)
                            );
                        }
                    }
                }
            }
        };
        
        // Load assets in priority order
        // Critical priority (UI, core assets)
        load_directory("ui/textures", "png", &mut self.ui_textures, LoadPriority::Critical);
        load_directory("ui/fonts", "ttf", &mut self.fonts, LoadPriority::Critical);
        load_directory("ui/icons", "png", &mut self.ui_icons, LoadPriority::Critical);
        
        // High priority (player vehicle, current level)
        load_directory("vehicles/models", "glb", &mut self.vehicle_models, LoadPriority::High);
        load_directory("vehicles/textures", "png", &mut self.vehicle_textures, LoadPriority::High);
        load_directory("vehicles/configs", "json", &mut self.vehicle_configs, LoadPriority::High);
        
        // Medium priority (effects, terrain)
        load_directory("effects/particles", "png", &mut self.particle_textures, LoadPriority::Medium);
        load_directory("effects/weather", "glb", &mut self.weather_effects, LoadPriority::Medium);
        load_directory("terrain/textures", "png", &mut self.terrain_textures, LoadPriority::Medium);
        load_directory("terrain/heightmaps", "png", &mut self.terrain_heightmaps, LoadPriority::Medium);
        
        // Low priority (audio, additional content)
        load_directory("audio/engine", "ogg", &mut self.engine_sounds, LoadPriority::Low);
        load_directory("audio/environment", "ogg", &mut self.environment_sounds, LoadPriority::Low);
        load_directory("audio/music", "ogg", &mut self.music_tracks, LoadPriority::Low);
        load_directory("audio/radio", "ogg", &mut self.radio_stations, LoadPriority::Low);
        load_directory("audio/voice", "ogg", &mut self.voice_lines, LoadPriority::Low);
        
        loading_state
    }
    
    /// Check the loading progress of all assets
    pub fn check_loading_progress(&self, asset_server: &AssetServer) -> AssetLoadingState {
        let mut state = AssetLoadingState::default();
        
        // Helper closure to check collection loading state
        let check_collection = |collection: &AssetCollection<impl Asset>| {
            for handle in collection.values() {
                state.total_assets += 1;
                match asset_server.get_load_state(handle) {
                    Some(LoadState::Loaded) => state.loaded_assets += 1,
                    Some(LoadState::Failed) => state.failed_assets += 1,
                    _ => {}
                }
            }
        };
        
        // Check all collections in priority order
        check_collection(&self.ui_textures);
        check_collection(&self.fonts);
        check_collection(&self.ui_icons);
        
        check_collection(&self.vehicle_models);
        check_collection(&self.vehicle_textures);
        check_collection(&self.vehicle_configs);
        
        check_collection(&self.particle_textures);
        check_collection(&self.weather_effects);
        check_collection(&self.terrain_textures);
        check_collection(&self.terrain_heightmaps);
        
        check_collection(&self.engine_sounds);
        check_collection(&self.environment_sounds);
        check_collection(&self.music_tracks);
        check_collection(&self.radio_stations);
        check_collection(&self.voice_lines);
        
        state.is_complete = state.loaded_assets + state.failed_assets == state.total_assets;
        state
    }
    
    /// Get an asset handle from cache or load it
    pub fn get_or_load<T: Asset>(&mut self, path: &str, asset_server: &AssetServer, priority: LoadPriority) -> Handle<T> {
        if let Some((handle, _)) = self.cached_handles.get(path) {
            handle.clone_weak().typed()
        } else {
            let handle = asset_server.load(path);
            self.cached_handles.insert(
                path.to_string(),
                (handle.clone_weak().typed(), priority)
            );
            handle
        }
    }
    
    /// Clear unused assets from cache based on priority
    pub fn clear_unused_assets(&mut self, min_priority: LoadPriority) {
        self.cached_handles.retain(|_, (_, priority)| {
            *priority >= min_priority
        });
    }
    
    /// Validate that required assets exist
    pub fn validate_assets(&mut self, asset_server: &AssetServer) -> Vec<String> {
        let mut errors = Vec::new();
        
        // Helper closure to validate a collection
        let validate_collection = |collection: &AssetCollection<impl Asset>, dir: &str, errors: &mut Vec<String>| {
            if collection.is_empty() {
                errors.push(format!("No assets loaded from directory: {}", dir));
            }
            
            for handle in collection.values() {
                if matches!(asset_server.get_load_state(handle), Some(LoadState::Failed)) {
                    errors.push(format!("Failed to load asset: {:?}", handle));
                }
            }
        };
        
        // Validate critical assets first
        validate_collection(&self.ui_textures, "ui/textures", &mut errors);
        validate_collection(&self.fonts, "ui/fonts", &mut errors);
        
        // Validate vehicle assets
        validate_collection(&self.vehicle_models, "vehicles/models", &mut errors);
        validate_collection(&self.vehicle_textures, "vehicles/textures", &mut errors);
        
        self.asset_validation_errors = errors.clone();
        errors
    }

    /// Load visualization assets with appropriate priorities
    pub fn load_visualization_assets(&mut self, asset_server: &AssetServer) {
        // Load visualization textures
        self.visualization_textures.load("normal_map", "textures/visualization/normal_map.png", asset_server);
        self.visualization_textures.load("slope_overlay", "textures/visualization/slope_overlay.png", asset_server);
        self.visualization_textures.load("curvature_map", "textures/visualization/curvature_map.png", asset_server);
        
        // Load visualization materials
        self.visualization_materials.load("wireframe", "materials/visualization/wireframe.mat", asset_server);
        self.visualization_materials.load("bounds", "materials/visualization/bounds.mat", asset_server);
        
        // Load visualization gradients
        self.visualization_gradients.load("height_gradient", "gradients/height_gradient.ron", asset_server);
        self.visualization_gradients.load("slope_gradient", "gradients/slope_gradient.ron", asset_server);
        self.visualization_gradients.load("biome_gradient", "gradients/biome_gradient.ron", asset_server);
        
        // Load visualization icons
        self.visualization_icons.load("terrain_icon", "icons/terrain.png", asset_server);
        self.visualization_icons.load("noise_icon", "icons/noise.png", asset_server);
        self.visualization_icons.load("biome_icon", "icons/biome.png", asset_server);
        
        // Load metrics textures and fonts
        self.metrics_textures.load("graph_bg", "textures/metrics/graph_bg.png", asset_server);
        self.metrics_fonts.load("metrics_font", "fonts/metrics.ttf", asset_server);
    }

    /// Get a visualization gradient texture, loading it if necessary
    pub fn get_visualization_gradient(&self, name: &str, asset_server: &AssetServer) -> Option<Handle<ColorGradient>> {
        self.visualization_gradients.get(name).cloned()
    }

    /// Get a visualization icon, loading it if necessary
    pub fn get_visualization_icon(&self, name: &str, asset_server: &AssetServer) -> Option<Handle<Image>> {
        self.visualization_icons.get(name).cloned()
    }

    /// Get a metrics font, loading it if necessary
    pub fn get_metrics_font(&self, name: &str, asset_server: &AssetServer) -> Option<Handle<Font>> {
        self.metrics_fonts.get(name).cloned()
    }
}

// Add this to your plugin setup
pub struct GameAssetsPlugin;

impl Plugin for GameAssetsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GameAssets>()
            .add_systems(Startup, (
                load_game_assets,
                load_visualization_assets,
            ))
            .add_systems(Update, (
                check_asset_loading,
                check_visualization_loading,
                update_asset_state,
            ));
    }
}

/// System to monitor asset loading progress
pub fn check_asset_loading_progress(
    asset_server: Res<AssetServer>,
    game_assets: Res<GameAssets>,
    mut loading_state: ResMut<AssetLoadingState>,
) {
    let new_state = game_assets.check_loading_progress(&asset_server);
    
    // Update loading state
    *loading_state = new_state;
    
    // Log progress changes
    if new_state.current_priority != loading_state.current_priority {
        info!(
            "Asset loading priority changed from {:?} to {:?}",
            loading_state.current_priority,
            new_state.current_priority
        );
    }
}

fn load_visualization_assets(
    mut game_assets: ResMut<GameAssets>,
    asset_server: Res<AssetServer>,
) {
    game_assets.load_visualization_assets(&asset_server);
}

fn check_visualization_loading(
    game_assets: Res<GameAssets>,
    asset_server: Res<AssetServer>,
) -> bool {
    // Check visualization textures
    if !game_assets.visualization_textures.is_loaded(&asset_server) {
        return false;
    }

    // Check visualization materials
    if !game_assets.visualization_materials.is_loaded(&asset_server) {
        return false;
    }

    // Check visualization gradients
    if !game_assets.visualization_gradients.is_loaded(&asset_server) {
        return false;
    }

    // Check visualization icons
    if !game_assets.visualization_icons.is_loaded(&asset_server) {
        return false;
    }

    // Check metrics assets
    if !game_assets.metrics_textures.is_loaded(&asset_server) {
        return false;
    }
    if !game_assets.metrics_fonts.is_loaded(&asset_server) {
        return false;
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::fs;
    
    fn setup_test_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
           .add_plugins(AssetPlugin::default())
           .add_plugins(GameAssetsPlugin);
        app
    }
    
    #[test]
    fn test_asset_collection() {
        let mut collection = AssetCollection::<Image>::default();
        let handle = Handle::<Image>::default();
        
        collection.insert("test".to_string(), handle.clone());
        assert!(collection.get("test").is_some());
        assert_eq!(collection.values().count(), 1);
    }
    
    #[test]
    fn test_loading_state() {
        let state = AssetLoadingState::default();
        assert_eq!(state.total_assets, 0);
        assert_eq!(state.loaded_assets, 0);
        assert_eq!(state.failed_assets, 0);
        assert!(!state.is_complete);
    }

    #[test]
    fn test_visualization_asset_loading() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(AssetPlugin::default())
            .add_plugins(GameAssetsPlugin);

        // Run startup systems
        app.update();

        let game_assets = app.world.resource::<GameAssets>();
        
        // Verify visualization textures are queued
        assert!(game_assets.visualization_textures.get("normal_map").is_some());
        assert!(game_assets.visualization_textures.get("slope_overlay").is_some());
        assert!(game_assets.visualization_textures.get("curvature_map").is_some());

        // Verify visualization materials are queued
        assert!(game_assets.visualization_materials.get("wireframe").is_some());
        assert!(game_assets.visualization_materials.get("bounds").is_some());

        // Verify visualization gradients are queued
        assert!(game_assets.visualization_gradients.get("height_gradient").is_some());
        assert!(game_assets.visualization_gradients.get("slope_gradient").is_some());
        assert!(game_assets.visualization_gradients.get("biome_gradient").is_some());

        // Verify visualization icons are queued
        assert!(game_assets.visualization_icons.get("terrain_icon").is_some());
        assert!(game_assets.visualization_icons.get("noise_icon").is_some());
        assert!(game_assets.visualization_icons.get("biome_icon").is_some());

        // Verify metrics assets are queued
        assert!(game_assets.metrics_textures.get("graph_bg").is_some());
        assert!(game_assets.metrics_fonts.get("metrics_font").is_some());
    }

    #[test]
    fn test_visualization_asset_retrieval() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(AssetPlugin::default())
            .add_plugins(GameAssetsPlugin);

        // Run startup systems
        app.update();

        let game_assets = app.world.resource::<GameAssets>();
        let asset_server = app.world.resource::<AssetServer>();

        // Test gradient retrieval
        let height_gradient = game_assets.get_visualization_gradient("height_gradient", &asset_server);
        assert!(height_gradient.is_some());

        let slope_gradient = game_assets.get_visualization_gradient("slope_gradient", &asset_server);
        assert!(slope_gradient.is_some());

        // Test icon retrieval
        let terrain_icon = game_assets.get_visualization_icon("terrain_icon", &asset_server);
        assert!(terrain_icon.is_some());

        let noise_icon = game_assets.get_visualization_icon("noise_icon", &asset_server);
        assert!(noise_icon.is_some());

        // Test font retrieval
        let metrics_font = game_assets.get_metrics_font("metrics_font", &asset_server);
        assert!(metrics_font.is_some());
    }
}