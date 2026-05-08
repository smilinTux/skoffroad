# Graphics Pipeline Documentation

## Overview

The SandK Offroad graphics pipeline is built on WGPU with custom extensions for ray tracing and advanced post-processing. The pipeline is designed for high performance and photorealistic rendering while maintaining scalability across different hardware capabilities.

## Pipeline Stages

### 1. G-Buffer Generation
```rust
pub struct GBufferTarget {
    albedo: TextureView,
    normal: TextureView,
    metallic_roughness: TextureView,
    motion_vectors: TextureView,
    depth: TextureView,
}
```

- **Geometry Pass**: Renders scene geometry with material properties
- **Motion Vectors**: Calculates per-pixel motion for temporal effects
- **Material Properties**: Stores PBR material parameters

### 2. Shadow Maps

#### Cascaded Shadow Maps
```rust
pub struct CascadedShadowMap {
    cascades: Vec<ShadowCascade>,
    matrix_buffer: Buffer,
    depth_texture: Texture,
}
```

- Multiple cascades for different detail levels
- PCF filtering for soft shadows
- Stable cascade transitions

#### Contact Shadows
- Screen-space ray marching
- High-frequency detail preservation
- Adaptive sample count

### 3. Lighting Pass

#### Direct Lighting
```rust
pub struct LightingPass {
    bind_group_layout: BindGroupLayout,
    pipeline: RenderPipeline,
    light_buffer: Buffer,
}
```

- Clustered forward+ lighting
- Area light support
- Dynamic light count scaling

#### Global Illumination
- Ray-traced indirect lighting (optional)
- Light probe system for static GI
- Dynamic probe updates

#### Ray-Traced Reflections
```rust
pub struct RTReflections {
    acceleration_structure: AccelerationStructure,
    ray_pipeline: RayTracingPipeline,
    denoiser: Denoiser,
}
```

- Hybrid ray-traced reflections
- Temporal accumulation
- AI-powered denoising

### 4. Post-Processing Stack

#### Temporal Anti-Aliasing
```rust
pub struct TAA {
    history_buffer: Texture,
    velocity_buffer: Texture,
    pipeline: ComputePipeline,
}
```

- Temporal sample accumulation
- Velocity-based rejection
- Anti-ghosting measures

#### Motion Blur
- Per-object motion vectors
- Tile-based computation
- Quality/performance scaling

#### Depth of Field
- Bokeh simulation
- Physical camera parameters
- Adaptive blur radius

#### Color Grading
- HDR tone mapping
- LUT-based grading
- Film grain simulation

## Shader System

### Base Shaders

#### Vertex Shader (WGSL)
```wgsl
struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) tangent: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_pos: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    // Transform vertex position and normal
    out.position = camera.view_proj * model * vec4<f32>(in.position, 1.0);
    out.world_pos = (model * vec4<f32>(in.position, 1.0)).xyz;
    out.normal = normalize((model * vec4<f32>(in.normal, 0.0)).xyz);
    out.uv = in.uv;
    return out;
}
```

#### Fragment Shader (WGSL)
```wgsl
struct FragmentOutput {
    @location(0) albedo: vec4<f32>,
    @location(1) normal: vec4<f32>,
    @location(2) metallic_roughness: vec2<f32>,
};

@fragment
fn fs_main(in: VertexOutput) -> FragmentOutput {
    var out: FragmentOutput;
    
    // Sample textures
    let base_color = textureSample(t_albedo, s_albedo, in.uv);
    let normal_map = textureSample(t_normal, s_normal, in.uv);
    let metallic_roughness = textureSample(t_metallic_roughness, s_metallic_roughness, in.uv);
    
    // Output G-Buffer data
    out.albedo = base_color;
    out.normal = vec4<f32>(normalize(in.normal), 1.0);
    out.metallic_roughness = metallic_roughness.bg;
    
    return out;
}
```

### Custom Shaders

#### Terrain Deformation
```wgsl
@compute
fn terrain_deform(@builtin(global_invocation_id) id: vec3<u32>) {
    let pos = vec2<u32>(id.xy);
    let height = heightmap[pos];
    let deform = deformation_buffer[pos];
    
    // Apply deformation
    heightmap[pos] = height + deform * deform_strength;
    
    // Update normals
    update_terrain_normals(pos);
}
```

#### Vehicle Paint
```wgsl
struct PaintProperties {
    base_color: vec4<f32>,
    metallic: f32,
    roughness: f32,
    clearcoat: f32,
    flake_density: f32,
};

@fragment
fn paint_shader(in: VertexOutput) -> FragmentOutput {
    var out: FragmentOutput;
    
    // Calculate paint layers
    let base = calculate_base_paint(in.uv);
    let metallic = calculate_metallic_flakes(in.uv);
    let clearcoat = calculate_clearcoat(in.normal);
    
    // Combine layers
    out.albedo = mix(base, metallic, paint.flake_density);
    out.metallic_roughness = vec2<f32>(paint.metallic, paint.roughness);
    out.normal = calculate_paint_normal(in.normal, clearcoat);
    
    return out;
}
```

## Material System

### PBR Materials
```rust
pub struct PBRMaterial {
    albedo: Handle<Texture>,
    normal: Handle<Texture>,
    metallic_roughness: Handle<Texture>,
    emission: Handle<Texture>,
    properties: MaterialProperties,
}
```

### Terrain Materials
- Layer-based blending
- Dynamic wetness
- Displacement mapping
- Tessellation support

### Vehicle Materials
- Multi-layer paint system
- Damage mapping
- Dirt accumulation
- Dynamic weathering

## Performance Optimizations

### GPU-Driven Rendering
- Indirect drawing
- Mesh clustering
- Visibility buffer

### Memory Management
- Texture streaming
- Mesh LOD system
- Material instancing

### Pipeline Optimizations
- Async compute
- Pipeline caching
- Shader permutation reduction

## Debug Features

### Visual Debugging
- Material channels
- Light visualization
- Performance overlays
- Ray visualization

### Performance Metrics
- Draw call counts
- Memory usage
- Pipeline statistics
- Frame timing 