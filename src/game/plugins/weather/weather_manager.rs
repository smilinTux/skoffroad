use bevy::prelude::*;
use std::time::Duration;
use crate::game::plugins::weather::weather_state::Weather;
use crate::game::plugins::weather::weather_state::WeatherState;

/// Resource that manages weather transitions and state
#[derive(Resource)]
pub struct WeatherManager {
    /// Current weather state
    state: WeatherState,
    /// Duration of weather transitions
    transition_duration: Duration,
    /// Minimum time between random weather changes
    min_change_interval: Duration,
    /// Time since last weather change
    time_since_change: Duration,
}

impl Default for WeatherManager {
    fn default() -> Self {
        Self {
            state: WeatherState::new(Weather::Clear),
            transition_duration: Duration::from_secs(30),
            min_change_interval: Duration::from_secs(300),
            time_since_change: Duration::ZERO,
        }
    }
}

impl WeatherManager {
    /// Update the weather manager with elapsed time
    pub fn update(&mut self, delta_seconds: f32) {
        let delta = Duration::from_secs_f32(delta_seconds);
        self.time_since_change += delta;

        // Update transition if in progress
        if let Some(target) = self.state.transitioning_to {
            self.state.transition_progress += delta_seconds / self.transition_duration.as_secs_f32();
            
            if self.state.transition_progress >= 1.0 {
                // Transition complete
                self.state.weather = target;
                self.state.transitioning_to = None;
                self.state.transition_progress = 0.0;
                // self.state.apply_weather_parameters(target); // Removed: method does not exist
            }
        }

        // Random weather changes (disabled for now, will be controlled by game logic)
        /*
        if self.time_since_change >= self.min_change_interval {
            // 5% chance per second to change weather
            if rand::random::<f32>() < 0.05 * delta_seconds {
                self.change_weather(self.random_weather());
            }
        }
        */
    }

    /// Get the current weather state
    pub fn current_state(&self) -> &WeatherState {
        &self.state
    }

    /// Change to a new weather type with transition
    pub fn change_weather(&mut self, weather: Weather) {
        if weather != self.state.weather && self.state.transitioning_to.is_none() {
            self.state.transitioning_to = Some(weather);
            self.state.transition_progress = 0.0;
            self.time_since_change = Duration::ZERO;
        }
    }

    /// Set the weather transition duration
    pub fn set_transition_duration(&mut self, duration: Duration) {
        self.transition_duration = duration;
    }

    /// Get a random weather type (excluding current)
    fn random_weather(&self) -> Weather {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        
        let options = [
            Weather::Clear,
            Weather::Cloudy,
            Weather::Rain,
            Weather::Storm,
            Weather::Fog,
            Weather::Snow,
        ];
        
        loop {
            let weather = options[rng.gen_range(0..options.len())];
            if weather != self.state.weather {
                return weather;
            }
        }
    }
} 