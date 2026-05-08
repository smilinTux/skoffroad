# Terrain Material API Reference

## Structs and Types

### TerrainMaterial
```rust
#[derive(AsBindGroup, Debug, Clone, TypePath)]
pub struct TerrainMaterial {
    #[uniform(0)]
    pub blending: BlendingConfig,
    
    #[uniform(1)]
    pub noise_config: NoiseConfig,
    
    // Textures and their samplers
    #[texture(2)] #[sampler(3)]
    pub snow_texture: Handle<Image>,
    // ... other textures at bindings 4-17 ...
    
    #[texture(18)] #[sampler(19)]
    pub noise_texture: Handle<Image>,
}
```

#### Shader Bindings
| Resource | Binding Group | Binding Index |
|----------|---------------|---------------|
| BlendingConfig | 1 | 0 |
| NoiseConfig | 1 | 1 |
| Snow Texture/Sampler | 1 | 2/3 |
| Rock Texture/Sampler | 1 | 4/5 |
| Grass Texture/Sampler | 1 | 6/7 |
| Dirt Texture/Sampler | 1 | 8/9 |
| Snow Normal/Sampler | 1 | 10/11 |
| Rock Normal/Sampler | 1 | 12/13 |
| Grass Normal/Sampler | 1 | 14/15 |
| Dirt Normal/Sampler | 1 | 16/17 |
| Noise Texture/Sampler | 1 | 18/19 |

### Configuration Types

#### BlendingConfig
```rust
#[derive(Clone)]
pub struct BlendingConfig {
    // Height thresholds
    pub snow_height: f32,
    pub rock_height: f32,
    pub grass_height: f32,
    pub steep_slope: f32,
    pub blend_sharpness: f32,
    
    // Tiling parameters
    pub snow_tiling: f32,
    pub rock_tiling: f32,
    pub grass_tiling: f32,
    pub dirt_tiling: f32,
    
    // Noise parameters
    pub noise_scale: f32,
    pub noise_strength: f32,
    pub noise_octaves: u32,
    pub noise_persistence: f32,
    pub noise_lacunarity: f32,
}
```

Default values:
```rust
BlendingConfig {
    snow_height: 100.0,
    rock_height: 50.0,
    grass_height: 10.0,
    steep_slope: 0.7,
    blend_sharpness: 0.1,
    snow_tiling: 32.0,
    rock_tiling: 24.0,
    grass_tiling: 48.0,
    dirt_tiling: 32.0,
    noise_scale: 0.1,
    noise_strength: 0.3,
    noise_octaves: 4,
    noise_persistence: 0.5,
    noise_lacunarity: 2.0,
}
```

#### NoiseConfig
```rust
#[derive(Clone)]
pub struct NoiseConfig {
    pub noise_scale: Vec4,
    pub noise_strength: Vec4,
    pub detail_scale: Vec4,
    pub detail_strength: Vec4,
}
```

Default values:
```rust
NoiseConfig {
    noise_scale: Vec4::new(0.5, 0.7, 0.3, 0.4),
    noise_strength: Vec4::new(0.2, 0.3, 0.15, 0.25),
    detail_scale: Vec4::new(4.0, 3.0, 5.0, 3.5),
    detail_strength: Vec4::new(0.1, 0.15, 0.08, 0.12),
}
```

## Debug UI Components

### TerrainDebugUi
```rust
#[derive(Component)]
pub struct TerrainDebugUi {
    // Analysis settings
    pub correlation_threshold: f32,
    pub confidence_level: f32,
    pub show_significance: bool,
    pub show_confidence_intervals: bool,
    pub history_window: f32,
    pub metric_pairs: Vec<(String, String)>,
    
    // UI state
    pub show_correlation_matrix: bool,
    pub show_correlation_history: bool,
    pub show_trend_analysis: bool,
    pub selected_metrics: Vec<String>,
    pub export_format: String,
}
```

## Public Functions

### Noise Generation
```rust
/// Generate a noise texture using Perlin noise
/// 
/// # Arguments
/// * `size` - Width and height of the texture (must be power of 2)
/// 
/// # Returns
/// * `Image` - RGBA noise texture
pub fn generate_noise_texture(size: u32) -> Image
```

### Material System
```rust
/// Update terrain materials for modified chunks
/// 
/// # System Parameters
/// * `materials: ResMut<Assets<TerrainMaterial>>`
/// * `modified_chunks: Res<ModifiedChunks>`
/// * `terrain_chunks: Query<(&TerrainChunk, &Handle<TerrainMaterial>)>`
pub fn update_terrain_material(...)

/// Initialize noise texture for all terrain materials
/// 
/// # System Parameters
/// * `commands: Commands`
/// * `images: ResMut<Assets<Image>>`
/// * `materials: ResMut<Assets<TerrainMaterial>>`
/// * `terrain_materials: Query<&Handle<TerrainMaterial>>`
pub fn setup_noise_texture(...)
```

### Debug UI Methods
```rust
impl TerrainDebugUi {
    /// Create new debug UI with default settings
    pub fn new() -> Self

    /// Draw correlation matrix with current metrics
    /// 
    /// # Arguments
    /// * `ui` - egui UI context
    /// * `metrics` - Current GPU metrics
    pub fn draw_correlation_matrix(&mut self, ui: &mut egui::Ui, metrics: &GpuMetrics)

    /// Export correlation data in selected format
    /// 
    /// # Arguments
    /// * `metrics` - GPU metrics to export
    pub fn export_correlation_data(&self, metrics: &GpuMetrics)
}
```

## Plugin Setup

```rust
/// Plugin to register the terrain material and systems
pub struct TerrainMaterialPlugin;

impl Plugin for TerrainMaterialPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<TerrainMaterial>::default())
           .add_systems(Startup, setup_noise_texture)
           .add_systems(Update, update_terrain_material);
    }
}
```

## Integration Example

```rust
use bevy::prelude::*;

fn setup_terrain_material(
    mut commands: Commands,
    mut materials: ResMut<Assets<TerrainMaterial>>,
    asset_server: Res<AssetServer>,
) {
    // Load textures
    let snow_texture = asset_server.load("textures/snow.png");
    let rock_texture = asset_server.load("textures/rock.png");
    // ... load other textures ...

    // Create material
    let material = TerrainMaterial {
        blending: BlendingConfig {
            snow_height: 150.0,
            rock_height: 75.0,
            ..Default::default()
        },
        noise_config: NoiseConfig {
            noise_strength: Vec4::new(0.3, 0.4, 0.2, 0.3),
            ..Default::default()
        },
        snow_texture,
        rock_texture,
        // ... assign other textures ...
        noise_texture: Handle::default(), // Will be set by setup_noise_texture
    };

    // Add material to assets
    let material_handle = materials.add(material);

    // Create terrain mesh with material
    commands.spawn(MaterialMeshBundle {
        mesh: // ... create or load terrain mesh ...,
        material: material_handle,
        transform: Transform::from_xyz(0.0, 0.0, 0.0),
        ..Default::default()
    });
}
```

## Testing

### Unit Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blending_config_defaults() {
        // Validates default blending parameters
    }

    #[test]
    fn test_noise_config_defaults() {
        // Validates noise configuration
    }

    #[test]
    fn test_material_creation() {
        // Verifies material setup
    }

    #[test]
    fn test_noise_texture_generation() {
        // Checks noise texture properties
    }
}
```

## Performance Guidelines

1. **Texture Management**
   - Use power-of-two textures (1024x1024, 2048x2048)
   - Consider using texture arrays for better batching
   - Compress normal maps when possible

2. **Noise Configuration**
   - Keep noise octaves ≤ 4 for performance
   - Use larger tiling values for distant terrain
   - Balance detail scales with performance needs

3. **Debug Features**
   - Disable correlation tracking in release builds
   - Limit history window for large terrains
   - Use CSV export for large datasets

4. **Memory Optimization**
   - Share noise textures between materials
   - Use mipmaps for distant textures
   - Clear correlation history periodically

## Error Handling

The system includes error handling for:
- Texture loading failures
- Invalid noise parameters
- Export errors
- Resource cleanup

Example error handling:
```rust
pub fn export_correlation_data(&self, metrics: &GpuMetrics) {
    let filename = format!("correlation_data_{}.{}", 
        Local::now().format("%Y%m%d_%H%M%S"),
        self.export_format
    );

    if let Err(e) = std::fs::write(&filename, data) {
        eprintln!("Failed to write file: {}", e);
        // Handle error appropriately
    }
}
``` 