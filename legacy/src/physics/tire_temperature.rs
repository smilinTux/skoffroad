use bevy::prelude::*;

#[derive(Component, Debug)]
pub struct TireTemperature {
    pub core_temp: f32,      // Internal tire temperature
    pub surface_temp: f32,   // Surface temperature
    pub optimal_temp: f32,   // Temperature for peak performance
    pub heat_capacity: f32,  // How quickly the tire heats up/cools down
    pub cooling_rate: f32,   // How quickly heat dissipates
    pub wear: f32,          // Tire wear (0.0 = new, 1.0 = worn out)
    pub wear_rate: f32,     // Base rate of wear accumulation
    pub temperature_memory: [f32; 10], // Recent temperature history
    pub memory_index: usize, // Index for circular temperature history buffer
}

impl Default for TireTemperature {
    fn default() -> Self {
        Self {
            core_temp: 20.0,      // Start at ambient temperature (20°C)
            surface_temp: 20.0,    // Start at ambient temperature
            optimal_temp: 80.0,    // Optimal operating temperature
            heat_capacity: 0.001,  // Heat gained per unit of work
            cooling_rate: 0.1,     // Heat lost per second
            wear: 0.0,            // Start with new tire
            wear_rate: 0.0001,    // Base wear rate
            temperature_memory: [20.0; 10], // Start with ambient temperature history
            memory_index: 0,
        }
    }
}

impl TireTemperature {
    pub fn update(&mut self, slip_power: f32, load: f32, dt: f32, ambient_temp: f32) {
        // Calculate heat generation from slip and load
        let heat_generation = slip_power * load * self.heat_capacity;
        
        // Update surface temperature first
        self.surface_temp += heat_generation * dt;
        
        // Heat transfer between surface and core
        let temp_difference = self.surface_temp - self.core_temp;
        let heat_transfer = temp_difference * 0.5 * dt;
        
        self.surface_temp -= heat_transfer;
        self.core_temp += heat_transfer;
        
        // Cooling based on difference from ambient temperature
        let surface_cooling = (self.surface_temp - ambient_temp) * self.cooling_rate * dt;
        let core_cooling = (self.core_temp - ambient_temp) * (self.cooling_rate * 0.5) * dt;
        
        self.surface_temp -= surface_cooling;
        self.core_temp -= core_cooling;
        
        // Update temperature history
        self.temperature_memory[self.memory_index] = (self.surface_temp + self.core_temp) * 0.5;
        self.memory_index = (self.memory_index + 1) % self.temperature_memory.len();
        
        // Calculate wear based on temperature and slip power
        let avg_temp = (self.surface_temp + self.core_temp) * 0.5;
        let temp_factor = if avg_temp > 120.0 {
            // Exponential wear increase when overheated
            ((avg_temp - 120.0) * 0.1).exp()
        } else {
            1.0
        };
        
        // Accumulate wear
        self.wear += self.wear_rate * temp_factor * slip_power * dt;
        self.wear = self.wear.min(1.0); // Clamp wear to maximum
    }
    
    pub fn get_grip_multiplier(&self) -> f32 {
        let avg_temp = (self.surface_temp + self.core_temp) * 0.5;
        
        // Continuous grip curve based on temperature
        let temp_factor = if avg_temp < 40.0 {
            // Cold tires
            0.7 + (avg_temp / 40.0) * 0.15
        } else if avg_temp < 60.0 {
            // Warming up
            0.85 + ((avg_temp - 40.0) / 20.0) * 0.15
        } else if avg_temp < 100.0 {
            // Optimal range
            1.0
        } else if avg_temp < 120.0 {
            // Getting hot
            1.0 - ((avg_temp - 100.0) / 20.0) * 0.1
        } else {
            // Overheated
            0.9 - ((avg_temp - 120.0) / 20.0).min(0.15)
        };
        
        // Apply wear degradation
        let wear_factor = 1.0 - (self.wear * 0.3); // Up to 30% grip loss when fully worn
        
        temp_factor * wear_factor
    }
    
    pub fn get_temperature_state(&self) -> TireTemperatureState {
        let avg_temp = (self.surface_temp + self.core_temp) * 0.5;
        
        if avg_temp < 40.0 {
            TireTemperatureState::Cold
        } else if avg_temp < 60.0 {
            TireTemperatureState::Cool
        } else if avg_temp < 100.0 {
            TireTemperatureState::Optimal
        } else if avg_temp < 120.0 {
            TireTemperatureState::Hot
        } else {
            TireTemperatureState::Overheated
        }
    }
    
    pub fn get_wear_state(&self) -> TireWearState {
        if self.wear < 0.3 {
            TireWearState::Good
        } else if self.wear < 0.6 {
            TireWearState::Fair
        } else if self.wear < 0.9 {
            TireWearState::Poor
        } else {
            TireWearState::Critical
        }
    }
    
    pub fn get_average_temperature(&self) -> f32 {
        self.temperature_memory.iter().sum::<f32>() / self.temperature_memory.len() as f32
    }
}

#[derive(Debug, PartialEq)]
pub enum TireTemperatureState {
    Cold,
    Cool,
    Optimal,
    Hot,
    Overheated,
}

#[derive(Debug, PartialEq)]
pub enum TireWearState {
    Good,
    Fair,
    Poor,
    Critical,
} 