// Binding layouts for material system

// Camera uniform buffer
@group(0) @binding(0)
var<uniform> camera: Camera;

// Material buffer
@group(1) @binding(0)
var<storage> materials: array<Material>;

// Per-instance data
struct Instance {
    model_matrix: mat4x4<f32>,
    normal_matrix: mat3x3<f32>,
    material_index: u32,
    prev_model_matrix: mat4x4<f32>,  // For motion vectors
};

@group(1) @binding(1)
var<storage> instances: array<Instance>;

// Textures
@group(2) @binding(0)
var albedo_texture: texture_2d<f32>;
@group(2) @binding(1)
var normal_texture: texture_2d<f32>;
@group(2) @binding(2)
var metallic_roughness_texture: texture_2d<f32>;
@group(2) @binding(3)
var emission_texture: texture_2d<f32>;

// Samplers
@group(2) @binding(4)
var texture_sampler: sampler;

// Light data
struct Light {
    position: vec3<f32>,
    intensity: f32,
    color: vec3<f32>,
    type: u32,  // 0: point, 1: directional, 2: spot
    direction: vec3<f32>,  // For directional and spot lights
    angle_cos: f32,        // For spot lights
};

@group(3) @binding(0)
var<storage> lights: array<Light>;

// Environment map for image-based lighting
@group(3) @binding(1)
var environment_map: texture_cube<f32>;
@group(3) @binding(2)
var environment_sampler: sampler;

// Output targets for deferred rendering
struct GBufferOutput {
    @location(0) albedo: vec4<f32>,
    @location(1) normal: vec4<f32>,
    @location(2) metallic_roughness: vec4<f32>,
    @location(3) emission: vec4<f32>,
    @location(4) motion_vectors: vec2<f32>,
};

// Ray tracing acceleration structure (when supported)
@group(4) @binding(0)
var acceleration_structure: acceleration_structure; 