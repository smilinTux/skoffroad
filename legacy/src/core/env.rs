use std::env;
use std::path::PathBuf;

/// Environment configuration for the game
#[derive(Debug, Clone)]
pub struct Environment {
    /// Path to configuration files
    pub config_path: PathBuf,
    /// Path to asset files
    pub asset_path: PathBuf,
    /// Development mode flag
    pub dev_mode: bool,
}

impl Environment {
    /// Create a new Environment instance
    pub fn new() -> Self {
        let config_path = env::var("SANDK_CONFIG_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("config"));

        let asset_path = env::var("SANDK_ASSET_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("assets"));

        let dev_mode = env::var("SANDK_DEV_MODE")
            .map(|v| v.to_lowercase() == "true")
            .unwrap_or(false);

        Self {
            config_path,
            asset_path,
            dev_mode,
        }
    }

    /// Get the path to the default configuration file
    pub fn default_config_path(&self) -> PathBuf {
        self.config_path.join("default.toml")
    }

    /// Get the path to the user configuration file
    pub fn user_config_path(&self) -> PathBuf {
        self.config_path.join("user.toml")
    }

    /// Check if development mode is enabled
    pub fn is_dev_mode(&self) -> bool {
        self.dev_mode
    }
}

impl Default for Environment {
    fn default() -> Self {
        Self::new()
    }
} 