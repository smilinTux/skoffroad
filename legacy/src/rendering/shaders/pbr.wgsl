// PBR shader with support for ray tracing and advanced material features

// Vertex shader inputs and outputs
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
    @location(3) tangent: vec3<f32>,
    @location(4) bitangent: vec3<f32>,
    @location(5) prev_pos: vec3<f32>,
};

// Uniform buffer bindings
struct Camera {
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    view_proj: mat4x4<f32>,
    inverse_view: mat4x4<f32>,
    position: vec3<f32>,
    prev_view_proj: mat4x4<f32>,
};

struct Model {
    model: mat4x4<f32>,
    normal: mat4x4<f32>,
    prev_model: mat4x4<f32>,
};

@group(0) @binding(0) var<uniform> camera: Camera;
@group(1) @binding(0) var<uniform> model: Model;

// Material bindings
@group(2) @binding(0) var t_albedo: texture_2d<f32>;
@group(2) @binding(1) var s_albedo: sampler;
@group(2) @binding(2) var t_normal: texture_2d<f32>;
@group(2) @binding(3) var s_normal: sampler;
@group(2) @binding(4) var t_metallic_roughness: texture_2d<f32>;
@group(2) @binding(5) var s_metallic_roughness: sampler;
@group(2) @binding(6) var t_emission: texture_2d<f32>;
@group(2) @binding(7) var s_emission: sampler;

// Vertex shader
@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    
    // Calculate world space position
    let world_pos = (model.model * vec4<f32>(in.position, 1.0)).xyz;
    out.world_pos = world_pos;
    
    // Transform to clip space
    out.position = camera.view_proj * vec4<f32>(world_pos, 1.0);
    
    // Calculate previous position for motion vectors
    let prev_world_pos = (model.prev_model * vec4<f32>(in.position, 1.0)).xyz;
    out.prev_pos = (camera.prev_view_proj * vec4<f32>(prev_world_pos, 1.0)).xyz;
    
    // Transform normal and tangent
    out.normal = normalize((model.normal * vec4<f32>(in.normal, 0.0)).xyz);
    out.tangent = normalize((model.normal * vec4<f32>(in.tangent.xyz, 0.0)).xyz);
    out.bitangent = cross(out.normal, out.tangent) * in.tangent.w;
    
    // Pass through texture coordinates
    out.uv = in.uv;
    
    return out;
}

// Fragment shader outputs
struct FragmentOutput {
    @location(0) albedo: vec4<f32>,
    @location(1) normal: vec4<f32>,
    @location(2) metallic_roughness: vec2<f32>,
    @location(3) emission: vec4<f32>,
    @location(4) motion: vec2<f32>,
};

// PBR functions
fn calculate_normal(in: VertexOutput) -> vec3<f32> {
    let tangent_normal = textureSample(t_normal, s_normal, in.uv).xyz * 2.0 - 1.0;
    
    let N = normalize(in.normal);
    let T = normalize(in.tangent);
    let B = normalize(in.bitangent);
    let TBN = mat3x3<f32>(T, B, N);
    
    return normalize(TBN * tangent_normal);
}

fn calculate_motion_vector(in: VertexOutput) -> vec2<f32> {
    let clip_pos = in.position.xyz / in.position.w;
    let prev_clip_pos = in.prev_pos / in.prev_pos.z;
    
    return (clip_pos.xy - prev_clip_pos.xy) * 0.5;
}

// Fragment shader
@fragment
fn fs_main(in: VertexOutput) -> FragmentOutput {
    var out: FragmentOutput;
    
    // Sample material textures
    let albedo = textureSample(t_albedo, s_albedo, in.uv);
    let metallic_roughness = textureSample(t_metallic_roughness, s_metallic_roughness, in.uv);
    let emission = textureSample(t_emission, s_emission, in.uv);
    
    // Calculate normal in world space
    let normal = calculate_normal(in);
    
    // Calculate motion vectors for temporal effects
    let motion = calculate_motion_vector(in);
    
    // Output G-Buffer data
    out.albedo = albedo;
    out.normal = vec4<f32>(normal * 0.5 + 0.5, 1.0);
    out.metallic_roughness = metallic_roughness.bg;
    out.emission = emission;
    out.motion = motion;
    
    return out;
} 