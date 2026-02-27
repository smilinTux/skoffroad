use bevy::prelude::*;
use super::WeatherState;
use std::f32::consts::PI;

/// Represents different times of day with their associated lighting parameters
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TimeOfDay {
    Dawn,
    Morning,
    Noon,
    Afternoon,
    Dusk,
    Night,
}

impl TimeOfDay {
    /// Get the base illuminance for this time of day
    pub fn base_illuminance(&self) -> f32 {
        match self {
            TimeOfDay::Dawn => 15000.0,
            TimeOfDay::Morning => 25000.0,
            TimeOfDay::Noon => 32000.0,
            TimeOfDay::Afternoon => 28000.0,
            TimeOfDay::Dusk => 12000.0,
            TimeOfDay::Night => 5000.0,
        }
    }

    /// Get the ambient light color for this time of day
    pub fn ambient_color(&self) -> Color {
        match self {
            TimeOfDay::Dawn => Color::rgb(0.8, 0.7, 0.7),
            TimeOfDay::Morning => Color::rgb(1.0, 0.95, 0.9),
            TimeOfDay::Noon => Color::rgb(1.0, 1.0, 1.0),
            TimeOfDay::Afternoon => Color::rgb(1.0, 0.95, 0.8),
            TimeOfDay::Dusk => Color::rgb(0.9, 0.7, 0.6),
            TimeOfDay::Night => Color::rgb(0.2, 0.2, 0.3),
        }
    }
}

/// Resource that manages the time of day and related lighting parameters
#[derive(Resource)]
pub struct TimeManager {
    /// Current time in hours (0-24)
    current_time: f32,
    /// How many real seconds per game hour
    seconds_per_hour: f32,
    /// Current time of day period
    time_of_day: TimeOfDay,
    /// Sun position parameters
    sun_angle: f32,
    sun_height: f32,
}

impl Default for TimeManager {
    fn default() -> Self {
        Self {
            current_time: 12.0, // Start at noon
            seconds_per_hour: 60.0, // 1 game hour = 60 real seconds
            time_of_day: TimeOfDay::Noon,
            sun_angle: 0.0,
            sun_height: 1.0,
        }
    }
}

impl TimeManager {
    /// Update the time manager with the elapsed time
    pub fn update(&mut self, delta_seconds: f32) {
        // Update current time
        self.current_time += delta_seconds / self.seconds_per_hour;
        if self.current_time >= 24.0 {
            self.current_time -= 24.0;
        }

        // Update sun position
        self.sun_angle = (self.current_time / 24.0) * 2.0 * PI;
        self.sun_height = (self.sun_angle.sin() + 1.0) * 0.5;

        // Update time of day period
        self.time_of_day = match self.current_time {
            t if t < 6.0 => TimeOfDay::Night,
            t if t < 8.0 => TimeOfDay::Dawn,
            t if t < 10.0 => TimeOfDay::Morning,
            t if t < 14.0 => TimeOfDay::Noon,
            t if t < 16.0 => TimeOfDay::Afternoon,
            t if t < 18.0 => TimeOfDay::Dusk,
            _ => TimeOfDay::Night,
        };
    }

    /// Get the current time of day
    pub fn time_of_day(&self) -> TimeOfDay {
        self.time_of_day
    }

    /// Get the current time in hours (0-24)
    pub fn current_time(&self) -> f32 {
        self.current_time
    }

    /// Set the game time speed (seconds per hour)
    pub fn set_time_speed(&mut self, seconds_per_hour: f32) {
        self.seconds_per_hour = seconds_per_hour.max(1.0);
    }

    /// Set the current time directly
    pub fn set_time(&mut self, hours: f32) {
        self.current_time = hours.clamp(0.0, 24.0);
        self.update(0.0); // Update derived values
    }

    /// Get the main directional light parameters based on time of day and weather
    pub fn get_main_light_params(&self, weather_state: &WeatherState) -> (Vec3, f32) {
        // Calculate sun direction
        let direction = Vec3::new(
            self.sun_angle.cos(),
            self.sun_height,
            self.sun_angle.sin(),
        ).normalize();

        // Get base illuminance and apply weather modifier
        let base_illuminance = self.time_of_day.base_illuminance();
        let weather_modifier = weather_state.light_intensity_modifier();
        let illuminance = base_illuminance * weather_modifier;

        (direction, illuminance)
    }

    /// Get the ambient light parameters based on time of day and weather
    pub fn get_ambient_light_params(&self, weather_state: &WeatherState) -> (Color, f32) {
        let base_color = self.time_of_day.ambient_color();
        let weather_color = weather_state.ambient_color_modifier();
        let intensity = weather_state.ambient_intensity_modifier();

        (base_color * [weather_color.r(), weather_color.g(), weather_color.b()], intensity)
    }
} 