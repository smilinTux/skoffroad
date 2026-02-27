use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use bevy::prelude::Resource;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, Resource)]
pub struct WeatherSoundSettings {
    pub master_volume: f32,
    pub effect_volumes: HashMap<String, f32>,
}
