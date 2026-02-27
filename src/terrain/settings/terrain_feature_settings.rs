use bevy::prelude::Resource;
use bevy::math::Vec2;

// TerrainFeatureSettings for terrain system
#[derive(Debug, Clone, PartialEq, Resource)]
pub struct TerrainFeatureSettings {
    pub feature_noise_scale: f32,
    pub min_feature_spacing: f32,
    pub difficulty_distribution: Vec<(DifficultyLevel, f32)>,
    pub feature_probability: f32,
    pub temperature: f32,
    pub snowfall_intensity: f32,
    pub wind_direction: Vec2,
    pub season_factor: f32,
    pub day_night_factor: f32,
    pub snow_compaction_rate: f32,
    pub ice_formation_threshold: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Resource)]
pub enum DifficultyLevel {
    Easy,
    Medium,
    Hard,
    Extreme,
}

impl Default for TerrainFeatureSettings {
    fn default() -> Self {
        Self {
            feature_noise_scale: 0.005,
            min_feature_spacing: 50.0,
            difficulty_distribution: vec![
                (DifficultyLevel::Easy, 0.4),
                (DifficultyLevel::Medium, 0.3),
                (DifficultyLevel::Hard, 0.2),
                (DifficultyLevel::Extreme, 0.1),
            ],
            feature_probability: 0.3,
            temperature: 0.0,
            snowfall_intensity: 0.0,
            wind_direction: Vec2::new(1.0, 0.0),
            season_factor: 0.0,
            day_night_factor: 0.5,
            snow_compaction_rate: 0.1,
            ice_formation_threshold: -5.0,
        }
    }
}
