// Fragment shader for PBR materials with advanced features

// Import common structures and functions
// ... existing code ...

struct FragmentInput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_pos: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) tangent: vec4<f32>,
    @location(4) prev_pos: vec4<f32>,
};

struct Camera {
    view_proj: mat4x4<f32>,
    view: mat4x4<f32>,
    position: vec3<f32>,
    prev_view_proj: mat4x4<f32>,
};

const PI: f32 = 3.14159265359;
const MIN_ROUGHNESS: f32 = 0.045;

// PBR functions
fn calculate_normal(input: FragmentInput) -> vec3<f32> {
    let N = normalize(input.world_normal);
    let T = normalize(input.tangent.xyz);
    let B = normalize(cross(N, T) * input.tangent.w);
    let TBN = mat3x3<f32>(T, B, N);
    
    let normal_sample = textureLoad(normal_texture, vec2<i32>(input.uv * vec2<f32>(textureDimensions(normal_texture))), 0).xyz;
    let normal = normalize(normal_sample * 2.0 - 1.0);
    
    return normalize(TBN * normal);
}

fn calculate_f0(base_metallic: f32, base_color: vec3<f32>) -> vec3<f32> {
    let dielectric_f0 = vec3<f32>(0.04);
    return mix(dielectric_f0, base_color, base_metallic);
}

fn calculate_light_radiance(light: Light, world_pos: vec3<f32>) -> vec3<f32> {
    var radiance = light.color * light.intensity;
    var attenuation = 1.0;
    
    switch(light.type) {
        case 0u: { // Point light
            let distance = length(light.position - world_pos);
            attenuation = 1.0 / (distance * distance);
        }
        case 1u: { // Directional light
            // No attenuation for directional lights
        }
        case 2u: { // Spot light
            let L = normalize(light.position - world_pos);
            let theta = dot(L, -light.direction);
            if theta < light.angle_cos {
                attenuation = 0.0;
            } else {
                let distance = length(light.position - world_pos);
                attenuation = 1.0 / (distance * distance);
            }
        }
        default: {}
    }
    
    return radiance * attenuation;
}

@fragment
fn fs_main(input: FragmentInput) -> GBufferOutput {
    // Sample material textures
    let albedo = textureLoad(albedo_texture, vec2<i32>(input.uv * vec2<f32>(textureDimensions(albedo_texture))), 0).rgb;
    let metallic_roughness = textureLoad(metallic_roughness_texture, vec2<i32>(input.uv * vec2<f32>(textureDimensions(metallic_roughness_texture))), 0).bg;
    let emission = textureLoad(emission_texture, vec2<i32>(input.uv * vec2<f32>(textureDimensions(emission_texture))), 0).rgb;
    
    let material = materials[instances[0].material_index];
    
    // Material properties
    let metallic = metallic_roughness.x * material.metallic;
    let roughness = max(metallic_roughness.y * material.roughness, MIN_ROUGHNESS);
    let f0 = calculate_f0(metallic, albedo);
    
    // Calculate normal and view direction
    let N = calculate_normal(input);
    let V = normalize(camera.position - input.world_pos);
    let NdotV = max(dot(N, V), 0.0);
    
    var Lo = vec3<f32>(0.0);
    
    // Calculate direct lighting
    for(var i = 0u; i < arrayLength(&lights); i = i + 1u) {
        let light = lights[i];
        let L = normalize(light.position - input.world_pos);
        let H = normalize(V + L);
        
        let NdotL = max(dot(N, L), 0.0);
        let NdotH = max(dot(N, H), 0.0);
        let HdotV = max(dot(H, V), 0.0);
        
        let radiance = calculate_light_radiance(light, input.world_pos);
        
        // Cook-Torrance BRDF
        let D = distribution_ggx(NdotH, roughness);
        let G = geometry_smith(NdotV, NdotL, roughness);
        let F = fresnel_schlick(HdotV, f0);
        
        let numerator = D * G * F;
        let denominator = 4.0 * NdotV * NdotL;
        let specular = numerator / max(denominator, 0.001);
        
        let kD = (vec3<f32>(1.0) - F) * (1.0 - metallic);
        
        Lo += (kD * albedo / PI + specular) * radiance * NdotL;
    }
    
    // Add subsurface scattering if enabled
    if material.use_subsurface != 0u {
        Lo += calc_subsurface(input.world_pos, N, material);
    }
    
    // Add emission
    Lo += emission * material.emission * material.emission_strength;
    
    // Calculate motion vectors for temporal effects
    let clip_pos = camera.view_proj * vec4<f32>(input.world_pos, 1.0);
    let prev_clip_pos = camera.prev_view_proj * input.prev_pos;
    let motion = (clip_pos.xy / clip_pos.w - prev_clip_pos.xy / prev_clip_pos.w) * 0.5;
    
    // Output G-Buffer data
    var output: GBufferOutput;
    output.albedo = vec4<f32>(albedo, material.alpha);
    output.normal = vec4<f32>(N * 0.5 + 0.5, 1.0);
    output.metallic_roughness = vec4<f32>(metallic, roughness, 0.0, 1.0);
    output.emission = vec4<f32>(emission * material.emission_strength, 1.0);
    output.motion_vectors = motion;
    
    return output;
} 