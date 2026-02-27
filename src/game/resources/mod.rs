use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

mod game_status;
mod game_settings;

pub use game_status::*;
pub use game_settings::*;

/// Represents the current state of the game
#[derive(Resource, Debug, Clone, Default)]
pub struct GameStatus {
    /// Current game mode (e.g., FreeRoam, Race, etc.)
    pub mode: GameMode,
    /// Whether the game is paused
    pub paused: bool,
    /// Current score or progress
    pub score: u32,
    /// Time elapsed since game start
    pub time_elapsed: f32,
    /// Game difficulty
    pub difficulty: Difficulty,
}

/// Different game modes available
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub enum GameMode {
    #[default]
    FreeRoam,
    Race,
    Challenge,
    Tutorial,
}

/// Game settings that can be configured by the player
#[derive(Resource, Debug, Clone, Serialize, Deserialize)]
pub struct GameSettings {
    /// Graphics settings
    pub graphics: GraphicsSettings,
    /// Audio settings
    pub audio: AudioSettings,
    /// Control settings
    pub controls: ControlSettings,
    /// Physics settings
    pub physics: PhysicsSettings,
}

impl Default for GameSettings {
    fn default() -> Self {
        Self {
            graphics: GraphicsSettings::default(),
            audio: AudioSettings::default(),
            controls: ControlSettings::default(),
            physics: PhysicsSettings::default(),
        }
    }
}

/// Graphics-related settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphicsSettings {
    pub resolution: (u32, u32),
    pub fullscreen: bool,
    pub vsync: bool,
    pub shadow_quality: ShadowQuality,
    pub texture_quality: TextureQuality,
}

impl Default for GraphicsSettings {
    fn default() -> Self {
        Self {
            resolution: (1920, 1080),
            fullscreen: false,
            vsync: true,
            shadow_quality: ShadowQuality::High,
            texture_quality: TextureQuality::High,
        }
    }
}

/// Shadow quality levels
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ShadowQuality {
    Low,
    Medium,
    High,
}

/// Texture quality levels
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum TextureQuality {
    Low,
    Medium,
    High,
}

/// Audio-related settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioSettings {
    pub master_volume: f32,
    pub music_volume: f32,
    pub sfx_volume: f32,
}

impl Default for AudioSettings {
    fn default() -> Self {
        Self {
            master_volume: 1.0,
            music_volume: 0.8,
            sfx_volume: 0.9,
        }
    }
}

/// Control-related settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlSettings {
    pub mouse_sensitivity: f32,
    pub invert_y: bool,
    pub controller_deadzone: f32,
}

impl Default for ControlSettings {
    fn default() -> Self {
        Self {
            mouse_sensitivity: 1.0,
            invert_y: false,
            controller_deadzone: 0.1,
        }
    }
}

/// Physics-related settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhysicsSettings {
    pub gravity: f32,
    pub time_scale: f32,
    pub simulation_rate: u32,
}

impl Default for PhysicsSettings {
    fn default() -> Self {
        Self {
            gravity: -9.81,
            time_scale: 1.0,
            simulation_rate: 60,
        }
    }
}

/// Resource for managing input state
#[derive(Resource)]
pub struct InputState {
    pub throttle: f32,
    pub brake: f32,
    pub steering: f32,
    pub handbrake: bool,
}

impl Default for InputState {
    fn default() -> Self {
        Self {
            throttle: 0.0,
            brake: 0.0,
            steering: 0.0,
            handbrake: false,
        }
    }
}

/// Resource for managing vehicle state
#[derive(Resource)]
pub struct VehicleState {
    pub velocity: Vec3,
    pub angular_velocity: Vec3,
    pub wheel_speeds: [f32; 4],
    pub suspension_forces: [f32; 4],
    pub ground_contacts: [bool; 4],
}

impl Default for VehicleState {
    fn default() -> Self {
        Self {
            velocity: Vec3::ZERO,
            angular_velocity: Vec3::ZERO,
            wheel_speeds: [0.0; 4],
            suspension_forces: [0.0; 4],
            ground_contacts: [false; 4],
        }
    }
}

/// Resource for managing debug information
#[derive(Resource)]
pub struct DebugInfo {
    pub fps: f32,
    pub frame_time: f32,
    pub physics_time: f32,
    pub render_time: f32,
    pub custom_metrics: HashMap<String, f32>,
}

impl Default for DebugInfo {
    fn default() -> Self {
        Self {
            fps: 0.0,
            frame_time: 0.0,
            physics_time: 0.0,
            render_time: 0.0,
            custom_metrics: HashMap::new(),
        }
    }
}

/// Game difficulty levels
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Difficulty {
    Easy,
    Normal,
    Hard,
    Expert,
}

impl GameStatus {
    /// Toggle the pause state
    pub fn toggle_pause(&mut self) {
        self.paused = !self.paused;
    }

    /// Update the game state
    pub fn update(&mut self, delta_time: f32) {
        if !self.paused {
            self.time_elapsed += delta_time;
        }
    }

    /// Add points to the score
    pub fn add_score(&mut self, points: u32) {
        self.score += points;
    }

    /// Set the game difficulty
    pub fn set_difficulty(&mut self, difficulty: Difficulty) {
        self.difficulty = difficulty;
    }
} 