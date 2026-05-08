use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use bevy::prelude::*;
use super::{EnvironmentType, WeatherType, TransitionCurve};

#[derive(Debug, Serialize, Deserialize)]
pub struct EnvironmentAudioConfig {
    pub environments: HashMap<String, EnvironmentSettings>,
    pub global_settings: GlobalSettings,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EnvironmentSettings {
    pub environment_type: String,
    pub base_volume: f32,
    pub transition_radius: f32,
    pub ambient_sounds: Vec<SoundConfig>,
    pub oneshot_sounds: Vec<SoundConfig>,
    pub reverb: ReverbConfig,
    pub weather_transitions: Vec<WeatherTransitionConfig>,
    pub time_volumes: Vec<TimeVolumePoint>,
    pub distance_attenuation: f32,
    pub occlusion_factor: f32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SoundConfig {
    pub name: String,
    pub volume: f32,
    pub pitch_variation: f32,
    pub min_interval: f32,
    pub max_interval: f32,
    pub distance_range: (f32, f32),
    #[serde(default)]
    pub height_range: Option<(f32, f32)>,
    pub weather_conditions: Vec<String>,
    #[serde(default)]
    pub time_range: Option<(f32, f32)>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReverbConfig {
    pub room_size: f32,
    pub damping: f32,
    pub wet_level: f32,
    pub dry_level: f32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WeatherTransitionConfig {
    pub from_weather: String,
    pub to_weather: String,
    pub fade_duration: f32,
    pub curve_type: String,
    pub intensity: f32,
    pub transition_sounds: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TimeVolumePoint {
    pub hour: f32,
    pub volume: f32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GlobalSettings {
    pub default_transition_duration: f32,
    pub default_oneshot_interval: (f32, f32),
    pub max_concurrent_oneshots: usize,
    pub distance_scale: f32,
    pub debug_visualization: DebugVisualizationSettings,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DebugVisualizationSettings {
    pub enabled: bool,
    pub show_volume_levels: bool,
    pub show_transition_zones: bool,
    pub show_overlap_regions: bool,
    pub show_active_sounds: bool,
    pub text_scale: f32,
}

impl EnvironmentAudioConfig {
    pub fn load(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let contents = std::fs::read_to_string(path)?;
        let config: Self = ron::from_str(&contents)?;
        Ok(config)
    }

    pub fn save(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let contents = ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default())?;
        std::fs::write(path, contents)?;
        Ok(())
    }

    pub fn get_environment_config(&self, env_type: &str) -> Option<&EnvironmentSettings> {
        self.environments.get(env_type)
    }
}

// Example configuration
pub fn create_example_config() -> EnvironmentAudioConfig {
    let mut environments = HashMap::new();
    
    // Desert environment example
    environments.insert("desert".to_string(), EnvironmentSettings {
        environment_type: "Desert".to_string(),
        base_volume: 0.4,
        transition_radius: 25.0,
        ambient_sounds: vec![
            SoundConfig {
                name: "desert_wind".to_string(),
                volume: 0.6,
                pitch_variation: 0.2,
                min_interval: 0.0,
                max_interval: 0.0,
                distance_range: (10.0, 50.0),
                height_range: None,
                weather_conditions: vec!["Clear".to_string(), "Wind".to_string()],
                time_range: None,
            },
        ],
        oneshot_sounds: vec![
            SoundConfig {
                name: "dust_devil".to_string(),
                volume: 0.5,
                pitch_variation: 0.3,
                min_interval: 10.0,
                max_interval: 30.0,
                distance_range: (15.0, 40.0),
                height_range: Some((0.0, 20.0)),
                weather_conditions: vec!["Wind".to_string()],
                time_range: Some((10.0, 16.0)),
            },
        ],
        reverb: ReverbConfig {
            room_size: 0.95,
            damping: 0.2,
            wet_level: 0.3,
            dry_level: 0.7,
        },
        weather_transitions: vec![
            WeatherTransitionConfig {
                from_weather: "Clear".to_string(),
                to_weather: "Wind".to_string(),
                fade_duration: 6.0,
                curve_type: "EaseIn".to_string(),
                intensity: 0.9,
                transition_sounds: vec!["wind_rising".to_string(), "sand_swirl".to_string()],
            },
        ],
        time_volumes: vec![
            TimeVolumePoint { hour: 0.0, volume: 0.2 },
            TimeVolumePoint { hour: 6.0, volume: 0.7 },
            TimeVolumePoint { hour: 12.0, volume: 1.0 },
            TimeVolumePoint { hour: 18.0, volume: 0.5 },
        ],
        distance_attenuation: 0.8,
        occlusion_factor: 0.3,
    });

    EnvironmentAudioConfig {
        environments,
        global_settings: GlobalSettings {
            default_transition_duration: 2.0,
            default_oneshot_interval: (5.0, 15.0),
            max_concurrent_oneshots: 3,
            distance_scale: 1.0,
            debug_visualization: DebugVisualizationSettings {
                enabled: true,
                show_volume_levels: true,
                show_transition_zones: true,
                show_overlap_regions: true,
                show_active_sounds: true,
                text_scale: 1.0,
            },
        },
    }
}

// Helper function to convert string to WeatherType
pub fn parse_weather_type(weather: &str) -> Option<WeatherType> {
    match weather.to_lowercase().as_str() {
        "clear" => Some(WeatherType::Clear),
        "rain" => Some(WeatherType::Rain),
        "storm" => Some(WeatherType::Storm),
        "wind" => Some(WeatherType::Wind),
        "snow" => Some(WeatherType::Snow),
        _ => None,
    }
}

// Helper function to convert string to TransitionCurve
pub fn parse_transition_curve(curve: &str) -> Option<TransitionCurve> {
    match curve.to_lowercase().as_str() {
        "linear" => Some(TransitionCurve::Linear),
        "easein" => Some(TransitionCurve::EaseIn),
        "easeout" => Some(TransitionCurve::EaseOut),
        "easeinout" => Some(TransitionCurve::EaseInOut),
        _ => None,
    }
} 