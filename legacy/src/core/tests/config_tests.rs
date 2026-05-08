use std::path::PathBuf;
use tempfile::TempDir;
use serde_json::json;

use crate::core::config::{Config, Environment, AudioConfig, GraphicsConfig, GameplayConfig};

fn setup_test_env() -> (TempDir, Environment) {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let config_path = temp_dir.path().join("config");
    let assets_path = temp_dir.path().join("assets");
    std::fs::create_dir_all(&config_path).expect("Failed to create config directory");
    std::fs::create_dir_all(&assets_path).expect("Failed to create assets directory");

    let env = Environment {
        config_dir: config_path,
        assets_dir: assets_path,
    };

    (temp_dir, env)
}

fn create_test_config(path: &PathBuf) {
    let config = json!({
        "audio": {
            "master_volume": 0.8,
            "music_volume": 0.6,
            "sfx_volume": 0.7,
            "voice_volume": 0.9
        },
        "graphics": {
            "resolution": [1920, 1080],
            "fullscreen": false,
            "vsync": true,
            "shadow_quality": "High",
            "texture_quality": "High",
            "particle_quality": "High",
            "view_distance": 1000.0
        },
        "gameplay": {
            "difficulty": "Normal",
            "camera_sensitivity": 1.0,
            "invert_y_axis": false,
            "show_tutorials": true
        }
    });

    std::fs::write(
        path,
        serde_json::to_string_pretty(&config).unwrap(),
    ).expect("Failed to write test config");
}

#[test]
fn test_load_default_config() {
    let (temp_dir, env) = setup_test_env();
    let config_path = env.config_dir.join("config.json");
    create_test_config(&config_path);

    let config = Config::load(&env).expect("Failed to load config");
    
    assert_eq!(config.audio.master_volume, 0.8);
    assert_eq!(config.graphics.resolution, [1920, 1080]);
    assert_eq!(config.gameplay.difficulty, "Normal");
}

#[test]
fn test_load_user_config_override() {
    let (temp_dir, env) = setup_test_env();
    let config_path = env.config_dir.join("config.json");
    
    // Create user config with overridden values
    let user_config = json!({
        "audio": {
            "master_volume": 0.5,
        },
        "graphics": {
            "fullscreen": true,
        }
    });

    std::fs::write(
        &config_path,
        serde_json::to_string_pretty(&user_config).unwrap(),
    ).expect("Failed to write user config");

    let config = Config::load(&env).expect("Failed to load config");
    
    assert_eq!(config.audio.master_volume, 0.5);
    assert!(config.graphics.fullscreen);
}

#[test]
fn test_save_config() {
    let (temp_dir, env) = setup_test_env();
    let config_path = env.config_dir.join("config.json");
    create_test_config(&config_path);

    let mut config = Config::load(&env).expect("Failed to load config");
    config.audio.master_volume = 0.3;
    config.save(&env).expect("Failed to save config");

    // Load config again and verify changes persisted
    let reloaded_config = Config::load(&env).expect("Failed to reload config");
    assert_eq!(reloaded_config.audio.master_volume, 0.3);
}

#[test]
fn test_config_validation() {
    let (temp_dir, env) = setup_test_env();
    let config_path = env.config_dir.join("config.json");

    // Test invalid volume values
    let invalid_config = json!({
        "audio": {
            "master_volume": 1.5, // Invalid: > 1.0
        }
    });

    std::fs::write(
        &config_path,
        serde_json::to_string_pretty(&invalid_config).unwrap(),
    ).expect("Failed to write invalid config");

    assert!(Config::load(&env).is_err());
}

#[test]
fn test_missing_default_config() {
    let (temp_dir, env) = setup_test_env();
    
    // Don't create config file
    let result = Config::load(&env);
    
    // Should create default config
    assert!(result.is_ok());
    assert!(env.config_dir.join("config.json").exists());
}

#[test]
fn test_invalid_config_format() {
    let (temp_dir, env) = setup_test_env();
    let config_path = env.config_dir.join("config.json");

    // Write invalid JSON
    std::fs::write(&config_path, "invalid json").expect("Failed to write invalid config");

    assert!(Config::load(&env).is_err());
}

#[test]
fn test_audio_volume_bounds() {
    let (temp_dir, env) = setup_test_env();
    let config_path = env.config_dir.join("config.json");
    create_test_config(&config_path);

    let mut config = Config::load(&env).expect("Failed to load config");

    // Test volume bounds
    config.audio.master_volume = 2.0;
    assert!(config.save(&env).is_err());

    config.audio.master_volume = -0.5;
    assert!(config.save(&env).is_err());

    config.audio.master_volume = 0.5;
    assert!(config.save(&env).is_ok());
}

#[test]
fn test_graphics_settings() {
    let (temp_dir, env) = setup_test_env();
    let config_path = env.config_dir.join("config.json");
    create_test_config(&config_path);

    let mut config = Config::load(&env).expect("Failed to load config");

    // Test resolution bounds
    config.graphics.resolution = [0, 0];
    assert!(config.save(&env).is_err());

    config.graphics.resolution = [640, 480];
    assert!(config.save(&env).is_ok());

    // Test view distance bounds
    config.graphics.view_distance = -1.0;
    assert!(config.save(&env).is_err());

    config.graphics.view_distance = 500.0;
    assert!(config.save(&env).is_ok());
} 