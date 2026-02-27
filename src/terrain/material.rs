use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderRef, ShaderType};
use bevy::render::texture::Image;
use crate::terrain::TerrainType;
use crate::terrain::settings::TerrainFeatureSettings;
use crate::terrain::BiomeType;

// Configuration for terrain blending based on height and slope
#[derive(Clone, Debug, ShaderType)]
pub struct BlendingConfig {
    pub height_blend_strength: f32,
    pub slope_blend_strength: f32,
    pub noise_scale: f32,
    pub noise_strength: f32,
}

impl Default for BlendingConfig {
    fn default() -> Self {
        Self {
            height_blend_strength: 0.5,
            slope_blend_strength: 0.3,
            noise_scale: 0.1,
            noise_strength: 0.2,
        }
    }
}

// Configuration for terrain tessellation
#[derive(Clone, Debug)]
pub struct TessellationConfig {
    pub tessellation_factor: f32,
    pub displacement_scale: f32,
    pub lod_transition_region: f32,
}

impl Default for TessellationConfig {
    fn default() -> Self {
        Self {
            tessellation_factor: 1.0,
            displacement_scale: 1.0,
            lod_transition_region: 0.2,
        }
    }
}

// Configuration for noise-based detail mapping
#[derive(Clone, Debug, ShaderType)]
pub struct NoiseConfig {
    pub detail_noise_scale: f32,
    pub detail_noise_strength: f32,
    pub macro_noise_scale: f32,
    pub macro_variation: f32,
}

impl Default for NoiseConfig {
    fn default() -> Self {
        Self {
            detail_noise_scale: 0.5,
            detail_noise_strength: 0.3,
            macro_noise_scale: 0.1,
            macro_variation: 0.2,
        }
    }
}

#[derive(AsBindGroup, Debug, Clone, Asset, TypePath, Component)]
pub struct TerrainMaterial {
    #[uniform(0)]
    pub blending: BlendingConfig,
    
    #[uniform(1)]
    pub noise_config: NoiseConfig,
    
    // Base color (albedo) textures
    #[texture(2)] #[sampler(3)]
    pub grass_albedo: Handle<Image>,
    #[texture(4)] #[sampler(5)]
    pub rock_albedo: Handle<Image>,
    #[texture(6)] #[sampler(7)]
    pub sand_albedo: Handle<Image>,
    #[texture(8)] #[sampler(9)]
    pub snow_albedo: Handle<Image>,
    #[texture(10)] #[sampler(11)]
    pub water_albedo: Handle<Image>,
    #[texture(12)] #[sampler(13)]
    pub forest_albedo: Handle<Image>,
    #[texture(14)] #[sampler(15)]
    pub plains_albedo: Handle<Image>,
    #[texture(16)] #[sampler(17)]
    pub tundra_albedo: Handle<Image>,
    
    // Normal maps
    #[texture(18)] #[sampler(19)]
    pub grass_normal: Handle<Image>,
    #[texture(20)] #[sampler(21)]
    pub rock_normal: Handle<Image>,
    #[texture(22)] #[sampler(23)]
    pub sand_normal: Handle<Image>,
    #[texture(24)] #[sampler(25)]
    pub snow_normal: Handle<Image>,
    #[texture(26)] #[sampler(27)]
    pub water_normal: Handle<Image>,
    #[texture(28)] #[sampler(29)]
    pub forest_normal: Handle<Image>,
    #[texture(30)] #[sampler(31)]
    pub plains_normal: Handle<Image>,
    #[texture(32)] #[sampler(33)]
    pub tundra_normal: Handle<Image>,
    
    // Roughness maps
    #[texture(34)] #[sampler(35)]
    pub grass_roughness: Handle<Image>,
    #[texture(36)] #[sampler(37)]
    pub rock_roughness: Handle<Image>,
    #[texture(38)] #[sampler(39)]
    pub sand_roughness: Handle<Image>,
    #[texture(40)] #[sampler(41)]
    pub snow_roughness: Handle<Image>,
    #[texture(42)] #[sampler(43)]
    pub water_roughness: Handle<Image>,
    #[texture(44)] #[sampler(45)]
    pub forest_roughness: Handle<Image>,
    #[texture(46)] #[sampler(47)]
    pub plains_roughness: Handle<Image>,
    #[texture(48)] #[sampler(49)]
    pub tundra_roughness: Handle<Image>,
    
    // Metallic maps
    #[texture(50)] #[sampler(51)]
    pub grass_metallic: Handle<Image>,
    #[texture(52)] #[sampler(53)]
    pub rock_metallic: Handle<Image>,
    #[texture(54)] #[sampler(55)]
    pub sand_metallic: Handle<Image>,
    #[texture(56)] #[sampler(57)]
    pub snow_metallic: Handle<Image>,
    #[texture(58)] #[sampler(59)]
    pub water_metallic: Handle<Image>,
    #[texture(60)] #[sampler(61)]
    pub forest_metallic: Handle<Image>,
    #[texture(62)] #[sampler(63)]
    pub plains_metallic: Handle<Image>,
    #[texture(64)] #[sampler(65)]
    pub tundra_metallic: Handle<Image>,
    
    // Height maps
    #[texture(66)] #[sampler(67)]
    pub grass_height: Handle<Image>,
    #[texture(68)] #[sampler(69)]
    pub rock_height: Handle<Image>,
    #[texture(70)] #[sampler(71)]
    pub sand_height: Handle<Image>,
    #[texture(72)] #[sampler(73)]
    pub snow_height: Handle<Image>,
    #[texture(74)] #[sampler(75)]
    pub water_height: Handle<Image>,
    #[texture(76)] #[sampler(77)]
    pub forest_height: Handle<Image>,
    #[texture(78)] #[sampler(79)]
    pub plains_height: Handle<Image>,
    #[texture(80)] #[sampler(81)]
    pub tundra_height: Handle<Image>,
    
    // Ambient Occlusion maps
    #[texture(82)] #[sampler(83)]
    pub grass_ao: Handle<Image>,
    #[texture(84)] #[sampler(85)]
    pub rock_ao: Handle<Image>,
    #[texture(86)] #[sampler(87)]
    pub sand_ao: Handle<Image>,
    #[texture(88)] #[sampler(89)]
    pub snow_ao: Handle<Image>,
    #[texture(90)] #[sampler(91)]
    pub water_ao: Handle<Image>,
    #[texture(92)] #[sampler(93)]
    pub forest_ao: Handle<Image>,
    #[texture(94)] #[sampler(95)]
    pub plains_ao: Handle<Image>,
    #[texture(96)] #[sampler(97)]
    pub tundra_ao: Handle<Image>,
}

impl Material for TerrainMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/terrain_material.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Opaque
    }
}

impl Default for TerrainMaterial {
    fn default() -> Self {
        Self {
            blending: BlendingConfig::default(),
            noise_config: NoiseConfig::default(),
            grass_albedo: Handle::default(),
            rock_albedo: Handle::default(),
            sand_albedo: Handle::default(),
            snow_albedo: Handle::default(),
            water_albedo: Handle::default(),
            forest_albedo: Handle::default(),
            plains_albedo: Handle::default(),
            tundra_albedo: Handle::default(),
            grass_normal: Handle::default(),
            rock_normal: Handle::default(),
            sand_normal: Handle::default(),
            snow_normal: Handle::default(),
            water_normal: Handle::default(),
            forest_normal: Handle::default(),
            plains_normal: Handle::default(),
            tundra_normal: Handle::default(),
            grass_roughness: Handle::default(),
            rock_roughness: Handle::default(),
            sand_roughness: Handle::default(),
            snow_roughness: Handle::default(),
            water_roughness: Handle::default(),
            forest_roughness: Handle::default(),
            plains_roughness: Handle::default(),
            tundra_roughness: Handle::default(),
            grass_metallic: Handle::default(),
            rock_metallic: Handle::default(),
            sand_metallic: Handle::default(),
            snow_metallic: Handle::default(),
            water_metallic: Handle::default(),
            forest_metallic: Handle::default(),
            plains_metallic: Handle::default(),
            tundra_metallic: Handle::default(),
            grass_height: Handle::default(),
            rock_height: Handle::default(),
            sand_height: Handle::default(),
            snow_height: Handle::default(),
            water_height: Handle::default(),
            forest_height: Handle::default(),
            plains_height: Handle::default(),
            tundra_height: Handle::default(),
            grass_ao: Handle::default(),
            rock_ao: Handle::default(),
            sand_ao: Handle::default(),
            snow_ao: Handle::default(),
            water_ao: Handle::default(),
            forest_ao: Handle::default(),
            plains_ao: Handle::default(),
            tundra_ao: Handle::default(),
        }
    }
}

impl TerrainMaterial {
    pub fn create_terrain_material(
        asset_server: &AssetServer,
        terrain_type: TerrainType,
        settings: &TerrainFeatureSettings,
    ) -> Self {
        let mut material = TerrainMaterial::default();
        
        // Load all textures for the specified terrain type
        let base_path = "textures/terrain";
        match terrain_type {
            TerrainType::Grass => {
                material.grass_albedo = asset_server.load(format!("{}/grass_albedo.png", base_path));
                material.grass_normal = asset_server.load(format!("{}/grass_normal.png", base_path));
                material.grass_roughness = asset_server.load(format!("{}/grass_roughness.png", base_path));
                material.grass_metallic = asset_server.load(format!("{}/grass_metallic.png", base_path));
                material.grass_height = asset_server.load(format!("{}/grass_height.png", base_path));
                material.grass_ao = asset_server.load(format!("{}/grass_ao.png", base_path));
            },
            TerrainType::Rock => {
                material.rock_albedo = asset_server.load(format!("{}/rock_albedo.png", base_path));
                material.rock_normal = asset_server.load(format!("{}/rock_normal.png", base_path));
                material.rock_roughness = asset_server.load(format!("{}/rock_roughness.png", base_path));
                material.rock_metallic = asset_server.load(format!("{}/rock_metallic.png", base_path));
                material.rock_height = asset_server.load(format!("{}/rock_height.png", base_path));
                material.rock_ao = asset_server.load(format!("{}/rock_ao.png", base_path));
            },
            TerrainType::Sand => {
                material.sand_albedo = asset_server.load(format!("{}/sand_albedo.png", base_path));
                material.sand_normal = asset_server.load(format!("{}/sand_normal.png", base_path));
                material.sand_roughness = asset_server.load(format!("{}/sand_roughness.png", base_path));
                material.sand_metallic = asset_server.load(format!("{}/sand_metallic.png", base_path));
                material.sand_height = asset_server.load(format!("{}/sand_height.png", base_path));
                material.sand_ao = asset_server.load(format!("{}/sand_ao.png", base_path));
            },
            TerrainType::Snow => {
                material.snow_albedo = asset_server.load(format!("{}/snow_albedo.png", base_path));
                material.snow_normal = asset_server.load(format!("{}/snow_normal.png", base_path));
                material.snow_roughness = asset_server.load(format!("{}/snow_roughness.png", base_path));
                material.snow_metallic = asset_server.load(format!("{}/snow_metallic.png", base_path));
                material.snow_height = asset_server.load(format!("{}/snow_height.png", base_path));
                material.snow_ao = asset_server.load(format!("{}/snow_ao.png", base_path));
            },
            TerrainType::Water => {
                material.water_albedo = asset_server.load(format!("{}/water_albedo.png", base_path));
                material.water_normal = asset_server.load(format!("{}/water_normal.png", base_path));
                material.water_roughness = asset_server.load(format!("{}/water_roughness.png", base_path));
                material.water_metallic = asset_server.load(format!("{}/water_metallic.png", base_path));
                material.water_height = asset_server.load(format!("{}/water_height.png", base_path));
                material.water_ao = asset_server.load(format!("{}/water_ao.png", base_path));
            },
            TerrainType::Forest => {
                material.forest_albedo = asset_server.load(format!("{}/forest_albedo.png", base_path));
                material.forest_normal = asset_server.load(format!("{}/forest_normal.png", base_path));
                material.forest_roughness = asset_server.load(format!("{}/forest_roughness.png", base_path));
                material.forest_metallic = asset_server.load(format!("{}/forest_metallic.png", base_path));
                material.forest_height = asset_server.load(format!("{}/forest_height.png", base_path));
                material.forest_ao = asset_server.load(format!("{}/forest_ao.png", base_path));
            },
            TerrainType::Plains => {
                material.plains_albedo = asset_server.load(format!("{}/plains_albedo.png", base_path));
                material.plains_normal = asset_server.load(format!("{}/plains_normal.png", base_path));
                material.plains_roughness = asset_server.load(format!("{}/plains_roughness.png", base_path));
                material.plains_metallic = asset_server.load(format!("{}/plains_metallic.png", base_path));
                material.plains_height = asset_server.load(format!("{}/plains_height.png", base_path));
                material.plains_ao = asset_server.load(format!("{}/plains_ao.png", base_path));
            },
            TerrainType::Tundra => {
                material.tundra_albedo = asset_server.load(format!("{}/tundra_albedo.png", base_path));
                material.tundra_normal = asset_server.load(format!("{}/tundra_normal.png", base_path));
                material.tundra_roughness = asset_server.load(format!("{}/tundra_roughness.png", base_path));
                material.tundra_metallic = asset_server.load(format!("{}/tundra_metallic.png", base_path));
                material.tundra_height = asset_server.load(format!("{}/tundra_height.png", base_path));
                material.tundra_ao = asset_server.load(format!("{}/tundra_ao.png", base_path));
            },
            TerrainType::Beach => { /* TODO: implement Beach material */ },
            TerrainType::Mountain => { /* TODO: implement Mountain material */ },
            TerrainType::Desert => { /* TODO: implement Desert material */ },
            TerrainType::Rainforest => { /* TODO: implement Rainforest material */ },
            TerrainType::Ocean => { /* TODO: implement Ocean material */ },
        }
        
        // Configure blending and noise settings based on terrain type
        material.blending = match terrain_type {
            TerrainType::Rock => BlendingConfig {
                height_blend_strength: 0.8,
                slope_blend_strength: 0.6,
                noise_scale: 0.15,
                noise_strength: 0.3,
            },
            TerrainType::Sand => BlendingConfig {
                height_blend_strength: 0.4,
                slope_blend_strength: 0.2,
                noise_scale: 0.2,
                noise_strength: 0.4,
            },
            TerrainType::Water => BlendingConfig {
                height_blend_strength: 0.3,
                slope_blend_strength: 0.1,
                noise_scale: 0.1,
                noise_strength: 0.2,
            },
            _ => BlendingConfig::default(),
        };
        
        // Configure noise settings based on terrain type
        material.noise_config = match terrain_type {
            TerrainType::Sand => NoiseConfig {
                detail_noise_scale: 0.8,
                detail_noise_strength: 0.4,
                macro_noise_scale: 0.15,
                macro_variation: 0.3,
            },
            TerrainType::Grass | TerrainType::Plains => NoiseConfig {
                detail_noise_scale: 0.6,
                detail_noise_strength: 0.5,
                macro_noise_scale: 0.12,
                macro_variation: 0.25,
            },
            TerrainType::Water => NoiseConfig {
                detail_noise_scale: 0.3,
                detail_noise_strength: 0.6,
                macro_noise_scale: 0.08,
                macro_variation: 0.4,
            },
            _ => NoiseConfig::default(),
        };
        
        material
    }
}