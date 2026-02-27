// use super::weather::Weather; // TODO: Fix or implement Weather module
// use super::weather::Weather;
use serde::{Serialize, Deserialize};
use bevy::prelude::Color;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct WeatherState {
    pub weather: Weather,
    pub transitioning_to: Option<Weather>,
    pub transition_progress: f32,
    pub cloud_coverage: f32,
    pub precipitation: f32,
    pub wind_speed: f32,
    pub wind_direction: f32,
    pub fog_density: f32,
    pub temperature: f32,
}

impl WeatherState {
    pub fn new(weather: Weather) -> Self {
        Self {
            weather,
            transitioning_to: None,
            transition_progress: 0.0,
            cloud_coverage: 0.0,
            precipitation: 0.0,
            wind_speed: 0.0,
            wind_direction: 0.0,
            fog_density: 0.0,
            temperature: 20.0,
        }
    }

    pub fn light_intensity_modifier(&self) -> f32 {
        match self.weather {
            Weather::Clear => 1.0,
            Weather::Cloudy => 0.7,
            Weather::Rain => 0.5,
            Weather::Storm => 0.3,
            Weather::Fog => 0.4,
            Weather::Snow => 0.6,
        }
    }

    pub fn ambient_color_modifier(&self) -> Color {
        match self.weather {
            Weather::Clear => Color::rgb(1.0, 1.0, 1.0),
            Weather::Cloudy => Color::rgb(0.8, 0.85, 0.9),
            Weather::Rain => Color::rgb(0.7, 0.8, 0.9),
            Weather::Storm => Color::rgb(0.6, 0.7, 0.8),
            Weather::Fog => Color::rgb(0.8, 0.8, 0.85),
            Weather::Snow => Color::rgb(0.95, 0.95, 1.0),
        }
    }

    pub fn ambient_intensity_modifier(&self) -> f32 {
        match self.weather {
            Weather::Clear => 1.0,
            Weather::Cloudy => 0.8,
            Weather::Rain => 0.7,
            Weather::Storm => 0.6,
            Weather::Fog => 0.7,
            Weather::Snow => 0.85,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Weather {
    Clear,
    Cloudy,
    Rain,
    Storm,
    Fog,
    Snow,
}

impl Default for Weather {
    fn default() -> Self {
        Weather::Clear
    }
}
