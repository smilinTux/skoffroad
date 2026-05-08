# Terrain Material Shader Documentation

## Overview
The terrain material shader implements advanced texture blending with noise-based variation, normal mapping, and tessellation. It supports multiple texture layers with dynamic blending based on height and slope.

## Shader Structure

### Binding Groups
```wgsl
@group(1) @binding(0) var<uniform> blending: BlendingConfig;
@group(1) @binding(1) var<uniform> noise_config: NoiseConfig;
// Texture bindings 2-19
```

### Configuration Structs
```wgsl
struct BlendingConfig {
    snow_height: f32,
    rock_height: f32,
    grass_height: f32,
    steep_slope: f32,
    blend_sharpness: f32,
}

struct NoiseConfig {
    noise_scale: vec4<f32>,    // Per-texture noise scales
    noise_strength: vec4<f32>, // Per-texture noise strengths
    detail_scale: vec4<f32>,   // Per-texture detail scales
    detail_strength: vec4<f32>, // Per-texture detail strengths
}
```

### Pipeline Stages

#### 1. Vertex Stage
```wgsl
@vertex
fn vertex(in: VertexInput) -> VertexOutput {
    // Transforms vertices and prepares for tessellation
}
```

#### 2. Tessellation Control
```wgsl
@tessellation_control
fn tessellation_control(...) -> TessControlOutput {
    // Calculates tessellation levels based on distance
}
```

#### 3. Tessellation Evaluation
```wgsl
@tessellation_evaluation
fn tessellation_evaluation(...) -> VertexOutput {
    // Applies displacement mapping and interpolates attributes
}
```

#### 4. Fragment Stage
```wgsl
@fragment
fn fragment(in: FragmentInput) -> @location(0) vec4<f32> {
    // Performs texture blending and lighting calculations
}
```

## Key Functions

### Texture Blending
```wgsl
fn calculate_blend_weights(height: f32, slope: f32) -> vec4<f32> {
    // Calculates blend weights for each texture layer
    // Returns (snow, rock, grass, dirt) weights
}
```

### Noise Application
```wgsl
fn sample_noise(uv: vec2<f32>, scale: f32) -> f32 {
    // Samples noise texture with given scale
}

fn apply_noise_variation(
    uv: vec2<f32>, 
    texture_index: u32,
    noise_config: NoiseConfig
) -> vec2<f32> {
    // Applies both base noise and detail variation
}
```

### Normal Mapping
```wgsl
fn apply_normal_mapping(
    normal_map: vec3<f32>,
    world_normal: vec3<f32>,
    world_tangent: vec4<f32>
) -> vec3<f32> {
    // Converts normal from tangent space to world space
}
```

## Texture Sampling

### Base Color Sampling
```wgsl
// Sample with noise variation
let snow_uv = apply_noise_variation(uv, texture_indices.snow, noise_config);
let snow_albedo = textureSample(
    albedo_array,
    albedo_sampler,
    snow_uv * texture_scales.x,
    texture_indices.snow
);
```

### Normal Map Sampling
```wgsl
// Sample with detail enhancement
let base_normal = textureSample(
    normal_array,
    normal_sampler,
    uv * texture_scales.x,
    texture_indices.snow
);
let detail_normal = textureSample(
    normal_array,
    normal_sampler,
    uv * texture_scales.x * 4.0,
    texture_indices.snow
);
```

## Performance Optimizations

1. **Texture Coordinates**
   - UV coordinates are computed once and reused
   - Detail sampling uses scaled base UVs
   - Noise sampling is optimized for each texture type

2. **Normal Mapping**
   - Detail normals are sampled at 4x the base scale
   - Normal blending is optimized for performance
   - Tangent space conversion is done efficiently

3. **Tessellation**
   - Dynamic LOD based on distance
   - Edge factor calculation is optimized
   - Displacement uses efficient height sampling

## Usage Guidelines

### Texture Setup
1. Provide power-of-two textures
2. Include mipmaps for all textures
3. Use compressed formats when possible
4. Ensure consistent texture scales

### Noise Configuration
1. Adjust noise scales based on terrain size
2. Balance detail strength for visual quality
3. Use appropriate noise octaves for performance
4. Consider distance-based detail reduction

### Blending Parameters
1. Set height thresholds for natural transitions
2. Adjust blend sharpness for smooth results
3. Configure slope threshold for rock exposure
4. Fine-tune tiling scales for each texture

## Example Configuration

```rust
// Rust-side configuration
let config = NoiseConfig {
    noise_scale: Vec4::new(0.5, 0.7, 0.3, 0.4),
    noise_strength: Vec4::new(0.2, 0.3, 0.15, 0.25),
    detail_scale: Vec4::new(4.0, 3.0, 5.0, 3.5),
    detail_strength: Vec4::new(0.1, 0.15, 0.08, 0.12),
};
```

```wgsl
// Shader-side usage
let varied_uv = apply_noise_variation(
    base_uv,
    texture_index,
    noise_config
);
```

## Debugging

### Visual Debugging
1. Use the correlation matrix for performance analysis
2. Export metrics for detailed investigation
3. Monitor frame times with different configurations
4. Check texture sampling patterns

### Common Issues
1. **Visible Seams**
   - Check texture tiling settings
   - Verify blend sharpness values
   - Ensure proper mipmap generation

2. **Performance Issues**
   - Reduce noise octaves
   - Optimize texture sizes
   - Adjust LOD distances
   - Check tessellation factors

3. **Blending Artifacts**
   - Verify height thresholds
   - Check slope calculations
   - Adjust blend sharpness
   - Validate noise parameters

## Future Improvements

1. **Planned Features**
   - Dynamic texture LOD system
   - Improved parallax mapping
   - Automated parameter optimization
   - Enhanced detail texturing

2. **Optimization Opportunities**
   - Compute shader for height calculations
   - Texture array optimizations
   - Improved tessellation patterns
   - Cached noise calculations 