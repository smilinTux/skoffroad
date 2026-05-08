use bevy::prelude::*;
use rand::Rng;
use crate::terrain::generation::TerrainGenerationSettings;

// pub mod generation;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WeatherType {
    Clear,
    Cloudy,
    LightSnow,
    HeavySnow,
    Blizzard,
    FreezingRain,
}

#[derive(Resource)]
pub struct WeatherState {
    pub current_weather: WeatherType,
    pub intensity: f32,
    pub temperature: f32,
    pub wind_speed: f32,
    pub wind_direction: Vec2,
    pub visibility: f32,
    pub transition_time: f32,
    pub time_in_state: f32,
}

impl Default for WeatherState {
    fn default() -> Self {
        Self {
            current_weather: WeatherType::Clear,
            intensity: 0.0,
            temperature: 0.0,
            wind_speed: 0.0,
            wind_direction: Vec2::new(1.0, 0.0),
            visibility: 1.0,
            transition_time: 0.0,
            time_in_state: 0.0,
        }
    }
}

#[derive(Event)]
pub struct WeatherChangeEvent {
    pub new_weather: WeatherType,
    pub transition_duration: f32,
}

pub struct WeatherPlugin;

impl Plugin for WeatherPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WeatherState>()
            .add_event::<WeatherChangeEvent>()
            .add_systems(Update, (
                update_weather_state,
                handle_weather_transitions,
                apply_weather_effects,
            ));
    }
}

fn update_weather_state(
    time: Res<Time>,
    mut weather: ResMut<WeatherState>,
    mut weather_events: EventWriter<WeatherChangeEvent>,
) {
    weather.time_in_state += time.delta_seconds();
    
    // Random weather changes
    if weather.time_in_state > 300.0 { // Check every 5 minutes
        let mut rng = rand::thread_rng();
        if rng.gen::<f32>() < 0.3 { // 30% chance to change weather
            let new_weather = match weather.current_weather {
                WeatherType::Clear => {
                    if weather.temperature < 0.0 {
                        WeatherType::LightSnow
                    } else {
                        WeatherType::Cloudy
                    }
                },
                WeatherType::Cloudy => {
                    if weather.temperature < 0.0 {
                        if rng.gen::<f32>() < 0.7 {
                            WeatherType::LightSnow
                        } else {
                            WeatherType::Clear
                        }
                    } else {
                        WeatherType::Clear
                    }
                },
                WeatherType::LightSnow => {
                    if rng.gen::<f32>() < 0.4 {
                        WeatherType::HeavySnow
                    } else {
                        WeatherType::Cloudy
                    }
                },
                WeatherType::HeavySnow => {
                    if rng.gen::<f32>() < 0.3 {
                        WeatherType::Blizzard
                    } else {
                        WeatherType::LightSnow
                    }
                },
                WeatherType::Blizzard => WeatherType::HeavySnow,
                WeatherType::FreezingRain => {
                    if weather.temperature < -2.0 {
                        WeatherType::LightSnow
                    } else {
                        WeatherType::Cloudy
                    }
                },
            };
            
            weather_events.send(WeatherChangeEvent {
                new_weather,
                transition_duration: rng.gen_range(60.0..180.0), // 1-3 minutes transition
            });
            weather.time_in_state = 0.0;
        }
    }
}

fn handle_weather_transitions(
    time: Res<Time>,
    mut weather: ResMut<WeatherState>,
    mut weather_events: EventReader<WeatherChangeEvent>,
) {
    for event in weather_events.read() {
        weather.current_weather = event.new_weather;
        weather.transition_time = event.transition_duration;
        
        // Update weather parameters based on type
        match event.new_weather {
            WeatherType::Clear => {
                weather.intensity = 0.0;
                weather.visibility = 1.0;
                weather.wind_speed *= 0.5;
            },
            WeatherType::Cloudy => {
                weather.intensity = 0.3;
                weather.visibility = 0.8;
                weather.wind_speed *= 1.2;
            },
            WeatherType::LightSnow => {
                weather.intensity = 0.5;
                weather.visibility = 0.6;
                weather.wind_speed *= 0.8;
            },
            WeatherType::HeavySnow => {
                weather.intensity = 0.8;
                weather.visibility = 0.3;
                weather.wind_speed *= 1.5;
            },
            WeatherType::Blizzard => {
                weather.intensity = 1.0;
                weather.visibility = 0.1;
                weather.wind_speed *= 2.5;
            },
            WeatherType::FreezingRain => {
                weather.intensity = 0.7;
                weather.visibility = 0.4;
                weather.wind_speed *= 1.3;
            },
        }
    }
}

fn apply_weather_effects(
    weather: Res<WeatherState>,
    mut terrain_settings: ResMut<crate::terrain::settings::TerrainFeatureSettings>,
    mut terrain_gen_settings: ResMut<TerrainGenerationSettings>,
) {
    // Update terrain settings based on weather
    terrain_settings.snowfall_intensity = match weather.current_weather {
        WeatherType::LightSnow => 0.3 * weather.intensity,
        WeatherType::HeavySnow => 0.7 * weather.intensity,
        WeatherType::Blizzard => 1.0 * weather.intensity,
        _ => 0.0,
    };
    
    terrain_settings.wind_direction = weather.wind_direction;
    // The following assignments are commented out because the fields do not exist or have type mismatches:
    // terrain_gen_settings.wind_speed = weather.wind_speed;
    // terrain_gen_settings.wind_direction = weather.wind_direction;
    // terrain_gen_settings.snow_accumulation_rate = match weather.current_weather {
    //     WeatherType::LightSnow => 0.2 * weather.intensity,
    //     WeatherType::HeavySnow => 0.5 * weather.intensity,
    //     WeatherType::Blizzard => 0.8 * weather.intensity,
    //     _ => 0.05,
    // };
    // terrain_gen_settings.snow_melt_rate = if weather.temperature > 0.0 {
    //     0.1 + (weather.temperature * 0.02)
    // } else {
    //     0.0
    // };
} 