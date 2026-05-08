use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use config::{Config, ConfigError, Environment, File};
use std::fs;
use std::path::Path;
use anyhow::{Context, Result};

use super::env::Environment;

#[derive(Debug, Error)]
pub enum ConfigurationError {
    #[error("Failed to load configuration: {0}")]
    LoadError(#[from] ConfigError),
    #[error("Invalid configuration value: {0}")]
    ValidationError(String),
}

/// Window configuration settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowConfig {
    pub width: u32,
    pub height: u32,
    pub fullscreen: bool,
    pub vsync: bool,
}

/// Graphics quality settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphicsConfig {
    pub shadow_quality: String,
    pub texture_quality: String,
    pub particle_quality: String,
    pub view_distance: f32,
    pub fov: f32,
}

/// Physics simulation settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhysicsConfig {
    pub fps: u32,
    pub substeps: u32,
    pub gravity: f32,
}

/// Audio volume settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioConfig {
    pub master_volume: f32,
    pub music_volume: f32,
    pub sfx_volume: f32,
}

/// Main configuration struct containing all settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub window: WindowConfig,
    pub graphics: GraphicsConfig,
    pub physics: PhysicsConfig,
    pub audio: AudioConfig,
}

impl Config {
    /// Load configuration from default and user config files
    pub fn load(env: &Environment) -> Result<Self> {
        // Load default config first
        let default_path = env.default_config_path();
        let mut config = Self::load_from_file(&default_path)
            .context("Failed to load default config")?;

        // Try to load and merge user config if it exists
        let user_path = env.user_config_path();
        if user_path.exists() {
            if let Ok(user_config) = Self::load_from_file(&user_path) {
                config.merge(user_config);
            }
        }

        Ok(config)
    }

    /// Load configuration from a specific file
    fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let contents = fs::read_to_string(path)?;
        let config = toml::from_str(&contents)?;
        Ok(config)
    }

    /// Save current configuration to the user config file
    pub fn save(&self, env: &Environment) -> Result<()> {
        let contents = toml::to_string_pretty(self)?;
        fs::write(env.user_config_path(), contents)?;
        Ok(())
    }

    /// Merge another config into this one, overwriting existing values
    fn merge(&mut self, other: Self) {
        self.window = other.window;
        self.graphics = other.graphics;
        self.physics = other.physics;
        self.audio = other.audio;
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            window: WindowConfig {
                width: 1920,
                height: 1080,
                fullscreen: false,
                vsync: true,
            },
            graphics: GraphicsConfig {
                shadow_quality: "high".to_string(),
                texture_quality: "high".to_string(),
                particle_quality: "high".to_string(),
                view_distance: 1000.0,
                fov: 90.0,
            },
            physics: PhysicsConfig {
                fps: 60,
                substeps: 2,
                gravity: -9.81,
            },
            audio: AudioConfig {
                master_volume: 1.0,
                music_volume: 0.8,
                sfx_volume: 1.0,
            },
        }
    }
}

impl Config {
    pub fn validate(&self) -> Result<(), ConfigurationError> {
        if self.window.width < 640 || self.window.height < 480 {
            return Err(ConfigurationError::ValidationError(
                "Window dimensions too small".to_string(),
            ));
        }
        if self.physics.fps < 30 {
            return Err(ConfigurationError::ValidationError(
                "Physics FPS too low".to_string(),
            ));
        }
        if self.audio.master_volume < 0.0 || self.audio.master_volume > 1.0 {
            return Err(ConfigurationError::ValidationError(
                "Invalid master volume".to_string(),
            ));
        }
        Ok(())
    }
} 