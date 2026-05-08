#import bevy_pbr::mesh_view_bindings
#import bevy_pbr::pbr_bindings
#import bevy_pbr::pbr_functions
#import bevy_pbr::pbr_types

@group(1) @binding(0) var base_color_texture: texture_2d<f32>;
@group(1) @binding(1) var base_color_sampler: sampler;
@group(1) @binding(2) var normal_map: texture_2d<f32>;
@group(1) @binding(3) var normal_map_sampler: sampler;
@group(1) @binding(4) var metallic_roughness_texture: texture_2d<f32>;
@group(1) @binding(5) var metallic_roughness_sampler: sampler;
@group(1) @binding(6) var emissive_texture: texture_2d<f32>;
@group(1) @binding(7) var emissive_sampler: sampler;

struct FragmentInput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) tangent: vec4<f32>,
};

@fragment
fn fragment(in: FragmentInput) -> @location(0) vec4<f32> {
    // Sample textures
    let base_color = textureSample(base_color_texture, base_color_sampler, in.uv);
    let metallic_roughness = textureSample(metallic_roughness_texture, metallic_roughness_sampler, in.uv);
    let emissive = textureSample(emissive_texture, emissive_sampler, in.uv);
    
    // Calculate TBN matrix for normal mapping
    let world_normal = normalize(in.world_normal);
    let world_tangent = normalize(in.tangent.xyz);
    let world_bitangent = normalize(cross(world_normal, world_tangent) * in.tangent.w);
    let tbn = mat3x3<f32>(world_tangent, world_bitangent, world_normal);
    
    // Sample and transform normal from normal map
    var normal = textureSample(normal_map, normal_map_sampler, in.uv).xyz * 2.0 - 1.0;
    normal = normalize(tbn * normal);
    
    // Create PbrInput for lighting calculation
    var pbr_input: PbrInput;
    pbr_input.material.base_color = base_color;
    pbr_input.material.metallic = metallic_roughness.b;
    pbr_input.material.perceptual_roughness = metallic_roughness.g;
    pbr_input.material.reflectance = 0.5;
    pbr_input.material.emissive = emissive.rgb;
    pbr_input.frag_coord = in.clip_position;
    pbr_input.world_position = in.world_position;
    pbr_input.world_normal = normal;
    pbr_input.is_orthographic = view.projection[3].w == 1.0;
    
    // Calculate lighting
    let pbr_output = pbr(pbr_input);
    
    return vec4<f32>(pbr_output.color, base_color.a);
} 