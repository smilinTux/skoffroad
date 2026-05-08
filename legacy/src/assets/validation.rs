use std::path::Path;
use bevy::prelude::*;
use thiserror::Error;

use super::GameAssets;

#[derive(Error, Debug)]
pub enum AssetValidationError {
    #[error("Required asset not found: {0}")]
    MissingAsset(String),
    #[error("Invalid asset format: {0}")]
    InvalidFormat(String),
    #[error("Asset load error: {0}")]
    LoadError(String),
}

/// Configuration for asset validation
#[derive(Resource)]
pub struct AssetValidationConfig {
    /// List of required asset paths relative to assets directory
    pub required_assets: Vec<String>,
    /// Whether to validate asset formats (more thorough but slower)
    pub validate_formats: bool,
    /// Whether to fail on missing optional assets
    pub strict_mode: bool,
}

impl Default for AssetValidationConfig {
    fn default() -> Self {
        Self {
            required_assets: vec![
                // Core vehicle assets
                "vehicles/offroad_truck.vehicle.json".to_string(),
                "vehicles/models/offroad_truck.glb".to_string(),
                
                // Core textures
                "textures/terrain/ground.png".to_string(),
                "textures/ui/loading.png".to_string(),
                
                // Core audio
                "audio/engine/idle.ogg".to_string(),
                "audio/effects/collision.ogg".to_string(),
            ],
            validate_formats: true,
            strict_mode: false,
        }
    }
}

/// System to validate assets on startup
pub fn validate_assets(
    asset_server: Res<AssetServer>,
    game_assets: Res<GameAssets>,
    validation_config: Res<AssetValidationConfig>,
) -> Result<(), AssetValidationError> {
    info!("Starting asset validation...");
    
    // Check required assets exist
    for asset_path in &validation_config.required_assets {
        let path = Path::new(asset_path);
        if !path.exists() {
            return Err(AssetValidationError::MissingAsset(asset_path.clone()));
        }
        
        if validation_config.validate_formats {
            match path.extension().and_then(|ext| ext.to_str()) {
                Some("json") => {
                    // Validate JSON format
                    if let Err(e) = std::fs::read_to_string(path)
                        .and_then(|content| serde_json::from_str::<serde_json::Value>(&content))
                    {
                        return Err(AssetValidationError::InvalidFormat(
                            format!("Invalid JSON in {}: {}", asset_path, e)
                        ));
                    }
                }
                Some("glb") | Some("gltf") => {
                    // Basic GLB/GLTF validation
                    if let Err(e) = std::fs::read(path) {
                        return Err(AssetValidationError::LoadError(
                            format!("Failed to read model {}: {}", asset_path, e)
                        ));
                    }
                }
                Some("png") | Some("jpg") => {
                    // Basic image validation
                    if let Err(e) = image::open(path) {
                        return Err(AssetValidationError::InvalidFormat(
                            format!("Invalid image format in {}: {}", asset_path, e)
                        ));
                    }
                }
                Some("ogg") | Some("wav") => {
                    // Basic audio file validation
                    if let Err(e) = std::fs::read(path) {
                        return Err(AssetValidationError::LoadError(
                            format!("Failed to read audio {}: {}", asset_path, e)
                        ));
                    }
                }
                _ => {} // Skip validation for unknown formats
            }
        }
    }
    
    info!("Asset validation completed successfully");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;
    
    #[test]
    fn test_asset_validation() {
        // Create temporary test assets
        let temp_dir = tempdir().unwrap();
        let assets_dir = temp_dir.path().join("assets");
        fs::create_dir_all(&assets_dir).unwrap();
        
        // Create test files
        let test_json = assets_dir.join("test.json");
        fs::write(&test_json, r#"{"test": "data"}"#).unwrap();
        
        let config = AssetValidationConfig {
            required_assets: vec![
                test_json.to_str().unwrap().to_string(),
            ],
            validate_formats: true,
            strict_mode: false,
        };
        
        let app = App::new();
        let asset_server = AssetServer::new();
        let game_assets = GameAssets::default();
        
        let result = validate_assets(
            Res::new(asset_server),
            Res::new(game_assets),
            Res::new(config),
        );
        
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_missing_asset_validation() {
        let config = AssetValidationConfig {
            required_assets: vec![
                "nonexistent.json".to_string(),
            ],
            validate_formats: true,
            strict_mode: false,
        };
        
        let app = App::new();
        let asset_server = AssetServer::new();
        let game_assets = GameAssets::default();
        
        let result = validate_assets(
            Res::new(asset_server),
            Res::new(game_assets),
            Res::new(config),
        );
        
        assert!(matches!(result, Err(AssetValidationError::MissingAsset(_))));
    }
} 