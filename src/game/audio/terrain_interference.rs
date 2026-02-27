use bevy::prelude::*;
use crate::physics::terrain_properties::PhysicsTerrainType;
use crate::game::terrain::TerrainManager;

/// Factors affecting CB radio signal interference from terrain and environment
#[derive(Resource)]
pub struct TerrainInterferenceConfig {
    /// Base signal reduction per meter of elevation difference
    pub elevation_factor: f32,
    /// Signal reduction in different terrain types
    pub terrain_factors: TerrainFactors,
    /// Impact of weather conditions
    pub weather_factors: WeatherFactors,
    /// Time of day effects (e.g. ionospheric propagation)
    pub time_factors: TimeFactors,
}

#[derive(Clone, Debug)]
pub struct TerrainFactors {
    pub mountain: f32,      // Heavy reduction in mountainous areas
    pub urban: f32,         // Moderate reduction from buildings
    pub forest: f32,        // Light reduction from vegetation
    pub tunnel: f32,        // Severe reduction underground
    pub water: f32,         // Enhanced propagation over water
    pub desert: f32,        // Minimal reduction in open desert
}

#[derive(Clone, Debug)]
pub struct WeatherFactors {
    pub clear: f32,         // Base condition
    pub rain: f32,         // Moderate reduction
    pub snow: f32,         // Light reduction
    pub fog: f32,          // Minor reduction
    pub storm: f32,        // Heavy reduction
    pub dust: f32,         // Moderate reduction
}

#[derive(Clone, Debug)]
pub struct TimeFactors {
    pub day: f32,          // Base propagation
    pub night: f32,        // Enhanced skip propagation
    pub dawn: f32,         // Transition period
    pub dusk: f32,         // Transition period
}

impl Default for TerrainInterferenceConfig {
    fn default() -> Self {
        Self {
            elevation_factor: 0.01,  // 1% reduction per meter of elevation difference
            terrain_factors: TerrainFactors {
                mountain: 0.7,   // 30% reduction
                urban: 0.6,      // 40% reduction
                forest: 0.8,     // 20% reduction
                tunnel: 0.1,     // 90% reduction
                water: 1.2,      // 20% enhancement
                desert: 1.0,     // No reduction
            },
            weather_factors: WeatherFactors {
                clear: 1.0,      // No reduction
                rain: 0.8,       // 20% reduction
                snow: 0.9,       // 10% reduction
                fog: 0.95,       // 5% reduction
                storm: 0.6,      // 40% reduction
                dust: 0.7,       // 30% reduction
            },
            time_factors: TimeFactors {
                day: 1.0,        // Base propagation
                night: 1.3,      // 30% enhancement
                dawn: 1.1,       // 10% enhancement
                dusk: 1.1,       // 10% enhancement
            },
        }
    }
}

/// Calculate terrain interference between two positions
pub fn calculate_terrain_interference(
    source_pos: Vec3,
    target_pos: Vec3,
    terrain_manager: &TerrainManager,
    config: &TerrainInterferenceConfig,
    current_weather: &str,
    time_of_day: f32, // 0.0-24.0 hours
) -> f32 {
    let mut interference = 1.0;
    
    // Calculate elevation difference
    let elevation_diff = (source_pos.y - target_pos.y).abs();
    interference *= 1.0 - (elevation_diff * config.elevation_factor);
    
    // Sample terrain types along path
    let direction = (target_pos - source_pos).normalize();
    let distance = source_pos.distance(target_pos);
    let samples = (distance / 50.0).ceil() as i32; // Sample every 50 meters
    
    let mut terrain_reduction = 1.0;
    for i in 0..samples {
        let t = i as f32 / samples as f32;
        let sample_pos = source_pos + direction * (distance * t);
        
        // Get terrain type at sample point
        let terrain_type = terrain_manager.get_terrain_type_at(sample_pos);
        
        // Apply terrain factor
        let factor = match terrain_type {
            PhysicsTerrainType::Mountain => config.terrain_factors.mountain,
            PhysicsTerrainType::Urban => config.terrain_factors.urban,
            PhysicsTerrainType::Forest => config.terrain_factors.forest,
            PhysicsTerrainType::Tunnel => config.terrain_factors.tunnel,
            PhysicsTerrainType::Water => config.terrain_factors.water,
            PhysicsTerrainType::Desert => config.terrain_factors.desert,
            _ => 1.0,
        };
        terrain_reduction *= factor.powf(1.0 / samples as f32);
    }
    interference *= terrain_reduction;
    
    // Apply weather factor
    let weather_factor = match current_weather {
        "clear" => config.weather_factors.clear,
        "rain" => config.weather_factors.rain,
        "snow" => config.weather_factors.snow,
        "fog" => config.weather_factors.fog,
        "storm" => config.weather_factors.storm,
        "dust" => config.weather_factors.dust,
        _ => 1.0,
    };
    interference *= weather_factor;
    
    // Apply time of day factor
    let time_factor = match time_of_day {
        t if t < 6.0 => config.time_factors.night,
        t if t < 8.0 => config.time_factors.dawn,
        t if t < 18.0 => config.time_factors.day,
        t if t < 20.0 => config.time_factors.dusk,
        _ => config.time_factors.night,
    };
    interference *= time_factor;
    
    // Clamp final value
    interference.clamp(0.1, 1.5)
} 