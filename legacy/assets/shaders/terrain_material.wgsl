#import bevy_pbr::mesh_view_bindings
#import bevy_pbr::mesh_bindings
#import bevy_pbr::pbr_bindings
#import bevy_pbr::pbr_functions

struct BlendingConfig {
    snow_height: f32,
    rock_height: f32,
    grass_height: f32,
    steep_slope: f32,
    blend_sharpness: f32,
}

struct TextureIndices {
    snow: u32,
    rock: u32,
    grass: u32,
    dirt: u32,
}

struct TessellationConfig {
    min_distance: f32,
    max_distance: f32,
    min_factor: f32,
    max_factor: f32,
    edge_size: f32,
}

struct NoiseConfig {
    noise_scale: vec4<f32>,  // Scale for each texture's noise
    noise_strength: vec4<f32>,  // Strength of noise effect per texture
    detail_scale: vec4<f32>,  // Scale for detail/micro tiling
    detail_strength: vec4<f32>,  // Strength of detail tiling
}

@group(1) @binding(0)
var<uniform> blending: BlendingConfig;

// Texture arrays for PBR maps
@group(1) @binding(1)
var albedo_array: texture_2d_array<f32>;
@group(1) @binding(2)
var albedo_sampler: sampler;

@group(1) @binding(3)
var normal_array: texture_2d_array<f32>;
@group(1) @binding(4)
var normal_sampler: sampler;

@group(1) @binding(5)
var roughness_array: texture_2d_array<f32>;
@group(1) @binding(6)
var roughness_sampler: sampler;

@group(1) @binding(7)
var ao_array: texture_2d_array<f32>;
@group(1) @binding(8)
var ao_sampler: sampler;

@group(1) @binding(9)
var height_array: texture_2d_array<f32>;
@group(1) @binding(10)
var height_sampler: sampler;

@group(1) @binding(11)
var<uniform> texture_indices: TextureIndices;

@group(1) @binding(12)
var<uniform> texture_scales: vec4<f32>;

@group(1) @binding(13)
var<uniform> height_scales: vec4<f32>;

@group(1) @binding(14)
var<uniform> parallax_quality: u32;

@group(1) @binding(15)
var<uniform> tessellation: TessellationConfig;

@group(1) @binding(8)
var noise_texture: texture_2d<f32>;
@group(1) @binding(9)
var noise_sampler: sampler;

struct FragmentInput {
    @builtin(front_facing) is_front: bool,
    @builtin(position) frag_coord: vec4<f32>,
    #import bevy_pbr::mesh_vertex_output
}

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) tangent: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) world_tangent: vec4<f32>,
}

struct TessControlOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) world_tangent: vec4<f32>,
    @builtin(patch_size) patch_size: u32,
    @builtin(tess_level_outer) tess_level_outer: array<f32, 4>,
    @builtin(tess_level_inner) tess_level_inner: array<f32, 2>,
}

fn calculate_slope(normal: vec3<f32>) -> f32 {
    return 1.0 - normal.y; // 0 = flat, 1 = vertical
}

fn calculate_blend_weights(height: f32, slope: f32) -> vec4<f32> {
    let snow_blend = smoothstep(
        blending.snow_height - blending.blend_sharpness,
        blending.snow_height + blending.blend_sharpness,
        height
    );
    
    let rock_blend = smoothstep(
        blending.rock_height - blending.blend_sharpness,
        blending.rock_height + blending.blend_sharpness,
        height
    ) * (1.0 - snow_blend);
    
    let steep_blend = smoothstep(
        blending.steep_slope - blending.blend_sharpness,
        blending.steep_slope + blending.blend_sharpness,
        slope
    );
    
    let grass_dirt_height = smoothstep(
        blending.grass_height - blending.blend_sharpness,
        blending.grass_height + blending.blend_sharpness,
        height
    );
    
    let grass_weight = (1.0 - rock_blend) * (1.0 - snow_blend) * (1.0 - steep_blend);
    let dirt_weight = (1.0 - grass_dirt_height) * (1.0 - rock_blend) * (1.0 - snow_blend) * steep_blend;
    
    return vec4<f32>(
        snow_blend,
        rock_blend,
        grass_weight,
        dirt_weight
    );
}

fn get_height_map_value(uv: vec2<f32>, weights: vec4<f32>) -> f32 {
    let snow_height = textureSample(height_array, height_sampler, uv * texture_scales.x, texture_indices.snow).r;
    let rock_height = textureSample(height_array, height_sampler, uv * texture_scales.y, texture_indices.rock).r;
    let grass_height = textureSample(height_array, height_sampler, uv * texture_scales.z, texture_indices.grass).r;
    let dirt_height = textureSample(height_array, height_sampler, uv * texture_scales.w, texture_indices.dirt).r;
    
    return snow_height * weights.x * height_scales.x +
           rock_height * weights.y * height_scales.y +
           grass_height * weights.z * height_scales.z +
           dirt_height * weights.w * height_scales.w;
}

fn parallax_mapping(uv: vec2<f32>, view_dir: vec3<f32>, weights: vec4<f32>) -> vec2<f32> {
    let num_layers = f32(parallax_quality);
    let layer_depth = 1.0 / num_layers;
    let current_layer_depth = 0.0;
    let delta_uv = view_dir.xy * 0.1 / (view_dir.z * num_layers);
    var current_uv = uv;
    var current_depth = 0.0;
    
    let height = get_height_map_value(current_uv, weights);
    
    // Parallax occlusion mapping loop
    for (var i = 0u; i < parallax_quality; i = i + 1u) {
        current_depth = current_depth + layer_depth;
        current_uv = current_uv - delta_uv;
        let sample_height = get_height_map_value(current_uv, weights);
        
        if (sample_height < current_depth) {
            break;
        }
    }
    
    // Interpolate between the last two layers
    let prev_uv = current_uv + delta_uv;
    let next_height = get_height_map_value(current_uv, weights);
    let prev_height = get_height_map_value(prev_uv, weights);
    
    let weight = (prev_height - current_depth) / 
                 (prev_height - next_height + 0.0001);
                 
    return mix(prev_uv, current_uv, weight);
}

fn sample_noise(uv: vec2<f32>, scale: f32) -> f32 {
    return textureSample(noise_texture, noise_sampler, uv * scale).r;
}

fn apply_noise_variation(uv: vec2<f32>, texture_index: u32, noise_config: NoiseConfig) -> vec2<f32> {
    let base_noise = sample_noise(uv, noise_config.noise_scale[texture_index]);
    let detail_noise = sample_noise(uv, noise_config.detail_scale[texture_index]);
    
    let noise_offset = (base_noise - 0.5) * noise_config.noise_strength[texture_index];
    let detail_offset = (detail_noise - 0.5) * noise_config.detail_strength[texture_index];
    
    return uv + vec2<f32>(noise_offset + detail_offset);
}

fn sample_terrain_textures(uv: vec2<f32>, weights: vec4<f32>) -> TerrainMaterial {
    var material: TerrainMaterial;
    
    // Apply noise variation to UVs for each texture
    let snow_uv = apply_noise_variation(uv, texture_indices.snow, noise_config);
    let rock_uv = apply_noise_variation(uv, texture_indices.rock, noise_config);
    let grass_uv = apply_noise_variation(uv, texture_indices.grass, noise_config);
    let dirt_uv = apply_noise_variation(uv, texture_indices.dirt, noise_config);
    
    // Sample albedo with varied UVs and tiling
    let snow_albedo = textureSample(albedo_array, albedo_sampler, snow_uv * texture_scales.x, texture_indices.snow);
    let rock_albedo = textureSample(albedo_array, albedo_sampler, rock_uv * texture_scales.y, texture_indices.rock);
    let grass_albedo = textureSample(albedo_array, albedo_sampler, grass_uv * texture_scales.z, texture_indices.grass);
    let dirt_albedo = textureSample(albedo_array, albedo_sampler, dirt_uv * texture_scales.w, texture_indices.dirt);
    
    material.albedo = snow_albedo * weights.x + 
                     rock_albedo * weights.y + 
                     grass_albedo * weights.z + 
                     dirt_albedo * weights.w;
    
    // Sample normal maps with enhanced detail
    let snow_normal = textureSample(normal_array, normal_sampler, snow_uv * texture_scales.x, texture_indices.snow);
    let rock_normal = textureSample(normal_array, normal_sampler, rock_uv * texture_scales.y, texture_indices.rock);
    let grass_normal = textureSample(normal_array, normal_sampler, grass_uv * texture_scales.z, texture_indices.grass);
    let dirt_normal = textureSample(normal_array, normal_sampler, dirt_uv * texture_scales.w, texture_indices.dirt);
    
    // Apply detail normal mapping
    let snow_detail = textureSample(normal_array, normal_sampler, snow_uv * texture_scales.x * 4.0, texture_indices.snow);
    let rock_detail = textureSample(normal_array, normal_sampler, rock_uv * texture_scales.y * 4.0, texture_indices.rock);
    let grass_detail = textureSample(normal_array, normal_sampler, grass_uv * texture_scales.z * 4.0, texture_indices.grass);
    let dirt_detail = textureSample(normal_array, normal_sampler, dirt_uv * texture_scales.w * 4.0, texture_indices.dirt);
    
    // Blend base and detail normals
    let snow_final_normal = normalize(snow_normal + snow_detail * 0.5);
    let rock_final_normal = normalize(rock_normal + rock_detail * 0.5);
    let grass_final_normal = normalize(grass_normal + grass_detail * 0.5);
    let dirt_final_normal = normalize(dirt_normal + dirt_detail * 0.5);
    
    material.normal = normalize(
        snow_final_normal * weights.x +
        rock_final_normal * weights.y +
        grass_final_normal * weights.z +
        dirt_final_normal * weights.w
    );
    
    // Sample roughness maps
    let snow_roughness = textureSample(roughness_array, roughness_sampler, snow_uv * texture_scales.x, texture_indices.snow);
    let rock_roughness = textureSample(roughness_array, roughness_sampler, rock_uv * texture_scales.y, texture_indices.rock);
    let grass_roughness = textureSample(roughness_array, roughness_sampler, grass_uv * texture_scales.z, texture_indices.grass);
    let dirt_roughness = textureSample(roughness_array, roughness_sampler, dirt_uv * texture_scales.w, texture_indices.dirt);
    
    material.roughness = snow_roughness * weights.x +
                        rock_roughness * weights.y +
                        grass_roughness * weights.z +
                        dirt_roughness * weights.w;
    
    // Sample ambient occlusion maps
    let snow_ao = textureSample(ao_array, ao_sampler, snow_uv * texture_scales.x, texture_indices.snow);
    let rock_ao = textureSample(ao_array, ao_sampler, rock_uv * texture_scales.y, texture_indices.rock);
    let grass_ao = textureSample(ao_array, ao_sampler, grass_uv * texture_scales.z, texture_indices.grass);
    let dirt_ao = textureSample(ao_array, ao_sampler, dirt_uv * texture_scales.w, texture_indices.dirt);
    
    material.ao = snow_ao * weights.x +
                 rock_ao * weights.y +
                 grass_ao * weights.z +
                 dirt_ao * weights.w;
    
    return material;
}

@vertex
fn vertex(in: VertexInput) -> VertexOutput {
    let world_position = (mesh.model * vec4<f32>(in.position, 1.0)).xyz;
    let world_normal = normalize((mesh.model * vec4<f32>(in.normal, 0.0)).xyz);
    let world_tangent = vec4<f32>(
        normalize((mesh.model * vec4<f32>(in.tangent.xyz, 0.0)).xyz),
        in.tangent.w
    );

    var out: VertexOutput;
    out.position = view.view_proj * vec4<f32>(world_position, 1.0);
    out.world_position = world_position;
    out.world_normal = world_normal;
    out.uv = in.uv;
    out.world_tangent = world_tangent;
    return out;
}

fn calculate_tess_level(position: vec3<f32>) -> f32 {
    let distance = length(position - view.world_position);
    let normalized_distance = clamp(
        (distance - tessellation.min_distance) / 
        (tessellation.max_distance - tessellation.min_distance),
        0.0,
        1.0
    );
    
    return mix(
        tessellation.max_factor,
        tessellation.min_factor,
        normalized_distance
    );
}

@tessellation_control
fn tessellation_control(@builtin(invocation_id) invocation_id: u32, in: array<VertexOutput, 3>) -> TessControlOutput {
    var out: TessControlOutput;
    
    // Pass through vertex data
    out.position = in[invocation_id].position;
    out.world_position = in[invocation_id].world_position;
    out.world_normal = in[invocation_id].world_normal;
    out.uv = in[invocation_id].uv;
    out.world_tangent = in[invocation_id].world_tangent;
    
    // Set patch size
    out.patch_size = 3u;
    
    // Calculate tessellation levels based on distance
    if (invocation_id == 0u) {
        let center_position = (in[0].world_position + in[1].world_position + in[2].world_position) / 3.0;
        let tess_level = calculate_tess_level(center_position);
        
        // Set outer tessellation levels
        out.tess_level_outer[0] = tess_level;
        out.tess_level_outer[1] = tess_level;
        out.tess_level_outer[2] = tess_level;
        out.tess_level_outer[3] = tess_level;
        
        // Set inner tessellation levels
        out.tess_level_inner[0] = tess_level;
        out.tess_level_inner[1] = tess_level;
    }
    
    return out;
}

@tessellation_evaluation
fn tessellation_evaluation(
    @builtin(position) position: vec4<f32>,
    @builtin(tess_coord) tess_coord: vec3<f32>,
    @builtin(patch_vertex_count) patch_vertex_count: u32,
    in: array<TessControlOutput, 3>,
) -> VertexOutput {
    // Interpolate vertex attributes using barycentric coordinates
    let world_position = 
        in[0].world_position * tess_coord.x +
        in[1].world_position * tess_coord.y +
        in[2].world_position * tess_coord.z;
        
    let world_normal = normalize(
        in[0].world_normal * tess_coord.x +
        in[1].world_normal * tess_coord.y +
        in[2].world_normal * tess_coord.z
    );
    
    let uv = 
        in[0].uv * tess_coord.x +
        in[1].uv * tess_coord.y +
        in[2].uv * tess_coord.z;
        
    let world_tangent = normalize(
        in[0].world_tangent * tess_coord.x +
        in[1].world_tangent * tess_coord.y +
        in[2].world_tangent * tess_coord.z
    );
    
    // Sample height map at the interpolated UV
    let blend_weights = calculate_blend_weights(world_position.y, calculate_slope(world_normal));
    let height_offset = get_height_map_value(uv, blend_weights) * tessellation.edge_size;
    
    // Displace vertex along normal by height map value
    let final_position = world_position + world_normal * height_offset;
    
    var out: VertexOutput;
    out.position = view.view_proj * vec4<f32>(final_position, 1.0);
    out.world_position = final_position;
    out.world_normal = world_normal;
    out.uv = uv;
    out.world_tangent = world_tangent;
    return out;
}

@fragment
fn fragment(in: FragmentInput) -> @location(0) vec4<f32> {
    let height = in.world_position.y;
    let normal = normalize(in.world_normal);
    let slope = abs(dot(normal, vec3<f32>(0.0, 1.0, 0.0)));
    
    let blend_weights = calculate_blend_weights(height, slope);
    
    // Calculate view direction in tangent space
    let view_pos = view.world_position;
    let view_dir = normalize(view_pos - in.world_position);
    let tangent_view_dir = normalize(vec3<f32>(
        dot(view_dir, in.world_tangent.xyz),
        dot(view_dir, cross(in.world_normal, in.world_tangent.xyz)),
        dot(view_dir, in.world_normal)
    ));
    
    // Apply parallax mapping
    let parallax_uv = parallax_mapping(in.uv, tangent_view_dir, blend_weights);
    let material = sample_terrain_textures(parallax_uv, blend_weights);
    
    var pbr_input: PbrInput;
    pbr_input.material.base_color = material.albedo;
    pbr_input.material.roughness = material.roughness.r;
    pbr_input.material.metallic = 0.0;
    pbr_input.material.reflectance = 0.5;
    pbr_input.material.emissive = vec4<f32>(0.0);
    pbr_input.material.alpha_cutoff = 0.5;
    
    pbr_input.world_position = in.world_position;
    pbr_input.world_normal = apply_normal_mapping(
        material.normal.xyz,
        in.world_normal,
        in.world_tangent
    );
    pbr_input.is_front = in.is_front;
    pbr_input.frag_coord = in.frag_coord;
    pbr_input.uv = in.uv;
    
    pbr_input.occlusion = material.ao.r;
    
    return pbr_functions::pbr(pbr_input);
}

fn apply_normal_mapping(normal_map: vec3<f32>, world_normal: vec3<f32>, world_tangent: vec4<f32>) -> vec3<f32> {
    let bitangent = cross(world_normal, world_tangent.xyz) * world_tangent.w;
    let tangent_matrix = mat3x3<f32>(
        world_tangent.xyz,
        bitangent,
        world_normal
    );
    
    // Convert normal from tangent space to world space
    let normal = normal_map * 2.0 - 1.0;
    return normalize(tangent_matrix * normal);
} 