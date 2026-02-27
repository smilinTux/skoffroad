use bevy::prelude::*;

#[derive(Debug, Clone)]
pub enum TerrainFeatureType {
    RockCrawling {
        rock_density: f32,
        rock_size_range: (f32, f32),
    },
    WaterCrossing {
        width: f32,
        depth: f32,
        current_speed: f32,
    },
    HillClimb {
        steepness: f32,
        length: f32,
        switchbacks: u32,
    },
    MudPit {
        viscosity: f32,
        depth: f32,
        area: Vec2,
    },
    Snowfield {
        depth: f32,
        powder_factor: f32,
        ice_patches: bool,
        compaction: f32,
        surface_hardness: f32,
        temperature: f32,
    },
    SnowDrift {
        height: f32,
        length: f32,
        wind_direction: Vec2,
        stability: f32,
        density_gradient: f32,
        age: f32,
    },
    IceFormation {
        thickness: f32,
        roughness: f32,
        temperature: f32,
    },
}

#[derive(Debug, Clone)]
pub struct TerrainFeature {
    pub feature_type: TerrainFeatureType,
    pub difficulty: crate::terrain::settings::DifficultyLevel,
    pub position: Vec3,
    pub size: Vec3,
    pub rotation: Quat,
    pub metadata: TerrainFeatureMetadata,
}

#[derive(Debug, Clone, Default)]
pub struct TerrainFeatureMetadata {
    pub name: String,
    pub description: String,
    pub recommended_vehicle_type: String,
    pub completion_reward: u32,
} 