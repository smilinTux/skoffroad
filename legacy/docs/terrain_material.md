# Terrain Material System Documentation

## Overview
The terrain material system provides advanced texture blending and noise-based variation for realistic terrain rendering. It supports multiple texture layers (snow, rock, grass, dirt) with normal mapping, dynamic blending based on height and slope, and noise-based variation to reduce repetition.

## Core Components

### BlendingConfig
Controls how different textures blend together based on terrain properties.

```rust
pub struct BlendingConfig {
    pub snow_height: f32,      // Height above which snow appears
    pub rock_height: f32,      // Height for rock textures
    pub grass_height: f32,     // Height for grass transition
    pub steep_slope: f32,      // Slope threshold for rock textures
    pub blend_sharpness: f32,  // Controls blend transition sharpness
    
    // Texture tiling parameters
    pub snow_tiling: f32,      // Snow texture repeat frequency
    pub rock_tiling: f32,      // Rock texture repeat frequency
    pub grass_tiling: f32,     // Grass texture repeat frequency
    pub dirt_tiling: f32,      // Dirt texture repeat frequency
    
    // Noise variation parameters
    pub noise_scale: f32,      // Overall scale of the noise
    pub noise_strength: f32,   // How much noise affects blending
    pub noise_octaves: u32,    // Number of noise layers
    pub noise_persistence: f32, // How much each octave contributes
    pub noise_lacunarity: f32, // How much detail increases per octave
}
```

Default values are optimized for realistic appearance while maintaining performance.

### NoiseConfig
Controls texture variation to reduce tiling artifacts.

```rust
pub struct NoiseConfig {
    pub noise_scale: Vec4,     // Scale for each texture's noise
    pub noise_strength: Vec4,  // Strength of noise effect per texture
    pub detail_scale: Vec4,    // Scale for detail/micro tiling
    pub detail_strength: Vec4, // Strength of detail tiling
}
```

Each Vec4 component corresponds to snow, rock, grass, and dirt textures respectively.

### TerrainMaterial
The main material struct that combines all textures and configuration.

```rust
pub struct TerrainMaterial {
    pub blending: BlendingConfig,
    pub noise_config: NoiseConfig,
    
    // Base color textures
    pub snow_texture: Handle<Image>,
    pub rock_texture: Handle<Image>,
    pub grass_texture: Handle<Image>,
    pub dirt_texture: Handle<Image>,
    
    // Normal maps
    pub snow_normal: Handle<Image>,
    pub rock_normal: Handle<Image>,
    pub grass_normal: Handle<Image>,
    pub dirt_normal: Handle<Image>,
    
    // Noise texture
    pub noise_texture: Handle<Image>,
}
```

## Debug and Analysis Tools

### TerrainDebugUi
Provides real-time visualization and analysis of terrain metrics.

```rust
pub struct TerrainDebugUi {
    // Correlation analysis settings
    pub correlation_threshold: f32,
    pub confidence_level: f32,
    pub show_significance: bool,
    pub show_confidence_intervals: bool,
    pub history_window: f32,
    pub metric_pairs: Vec<(String, String)>,
    
    // Visualization settings
    pub show_correlation_matrix: bool,
    pub show_correlation_history: bool,
    pub show_trend_analysis: bool,
    pub selected_metrics: Vec<String>,
    pub export_format: String,
}
```

Features:
- Real-time correlation matrix visualization
- Historical correlation tracking
- Trend analysis
- Data export in JSON/CSV formats
- Statistical significance testing
- Confidence interval display

## Usage Examples

### Basic Setup
```rust
// Create a terrain material
let material = TerrainMaterial {
    blending: BlendingConfig::default(),
    noise_config: NoiseConfig::default(),
    // ... load textures ...
};

// Add to assets
let material_handle = materials.add(material);
```

### Customizing Blending
```rust
let mut config = BlendingConfig::default();
config.snow_height = 120.0;
config.blend_sharpness = 0.2;
config.noise_strength = 0.4;
```

### Using the Debug UI
```rust
let mut debug_ui = TerrainDebugUi::new();
debug_ui.correlation_threshold = 0.8;
debug_ui.show_significance = true;
debug_ui.history_window = 120.0; // 2 minutes of history
```

### Exporting Correlation Data
```rust
// Export as JSON
debug_ui.export_format = "json".to_string();
debug_ui.export_correlation_data(&metrics);

// Export as CSV
debug_ui.export_format = "csv".to_string();
debug_ui.export_correlation_data(&metrics);
```

## Performance Considerations

1. **Texture Sizes**: Keep texture dimensions power-of-two (e.g., 1024x1024, 2048x2048).
2. **Noise Texture**: Default 512x512 resolution balances quality and memory usage.
3. **Blend Sharpness**: Higher values (>0.5) may cause visible transitions.
4. **Tiling Scales**: Lower values increase texture repetition but improve performance.

## Implementation Details

### Noise Generation
The system uses Perlin noise with multiple octaves for natural variation:
```rust
pub fn generate_noise_texture(size: u32) -> Image {
    // ... generates RGBA noise texture ...
}
```

### Material Updates
The system automatically updates materials when chunks are modified:
```rust
pub fn update_terrain_material(
    mut materials: ResMut<Assets<TerrainMaterial>>,
    modified_chunks: Res<ModifiedChunks>,
    terrain_chunks: Query<(&TerrainChunk, &Handle<TerrainMaterial>)>,
) {
    // ... updates material properties ...
}
```

## Testing

The system includes comprehensive tests:
- Blending configuration validation
- Noise configuration validation
- Material creation verification
- Noise texture generation testing

Run tests with:
```bash
cargo test --package sandk-offroad
```

## Debugging Tips

1. Use the correlation matrix to identify performance bottlenecks
2. Export data for offline analysis
3. Adjust the history window for different analysis timeframes
4. Monitor trend analysis for stability issues

## Future Improvements

1. Dynamic LOD for texture detail
2. Automated parameter optimization
3. Enhanced statistical analysis
4. Real-time performance monitoring
5. Machine learning-based texture selection 