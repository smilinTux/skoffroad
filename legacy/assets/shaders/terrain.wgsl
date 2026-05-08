struct TerrainMaterial {
    height_scale: f32,
    texture_scale: f32,
    normal_strength: f32,
};

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) morph_target: vec3<f32>,
    @location(4) morph_factor: f32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
};

@group(0) @binding(0)
var<uniform> view_proj: mat4x4<f32>;

@group(0) @binding(1)
var<uniform> model: mat4x4<f32>;

@group(1) @binding(0)
var height_map: texture_2d<f32>;
@group(1) @binding(1)
var height_sampler: sampler;

@vertex
fn vertex(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    
    // Interpolate between current position and morph target
    let morphed_position = mix(
        input.position,
        input.morph_target,
        input.morph_factor
    );
    
    let world_position = (model * vec4<f32>(morphed_position, 1.0)).xyz;
    output.clip_position = view_proj * vec4<f32>(world_position, 1.0);
    output.world_position = world_position;
    output.world_normal = normalize((model * vec4<f32>(input.normal, 0.0)).xyz);
    output.uv = input.uv;
    
    return output;
}

@fragment
fn fragment(
    @location(0) world_position: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
) -> @location(0) vec4<f32> {
    let height = textureSample(height_map, height_sampler, uv).r;
    let color = mix(
        vec3<f32>(0.2, 0.5, 0.1), // grass color
        vec3<f32>(0.6, 0.5, 0.4), // dirt color
        smoothstep(0.3, 0.7, height)
    );
    
    return vec4<f32>(color, 1.0);
} 