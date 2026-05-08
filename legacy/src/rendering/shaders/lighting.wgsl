// Lighting pass shader for deferred rendering with PBR lighting model

// G-Buffer inputs
@group(0) @binding(0) var t_albedo: texture_2d<f32>;
@group(0) @binding(1) var t_normal: texture_2d<f32>;
@group(0) @binding(2) var t_metallic_roughness: texture_2d<f32>;
@group(0) @binding(3) var t_emission: texture_2d<f32>;
@group(0) @binding(4) var t_depth: texture_depth_2d;
@group(0) @binding(5) var s_gbuffer: sampler;

// Shadow map inputs
@group(1) @binding(0) var t_shadow_cascades: texture_depth_2d_array;
@group(1) @binding(1) var s_shadow: sampler_comparison;

// Light data
struct Light {
    position: vec3<f32>,
    light_type: u32,
    direction: vec3<f32>,
    range: f32,
    color: vec3<f32>,
    intensity: f32,
    spot_angle_cos: f32,
    cast_shadows: u32,
    shadow_view: mat4x4<f32>,
    shadow_proj: mat4x4<f32>,
};

struct LightData {
    lights: array<Light, 64>,
    light_count: u32,
    ambient_intensity: f32,
    shadow_cascade_count: u32,
    padding: u32,
};

@group(2) @binding(0) var<uniform> light_data: LightData;
@group(2) @binding(1) var<uniform> camera_pos: vec3<f32>;

// Constants
const PI: f32 = 3.14159265359;
const LIGHT_TYPE_DIRECTIONAL: u32 = 0u;
const LIGHT_TYPE_POINT: u32 = 1u;
const LIGHT_TYPE_SPOT: u32 = 2u;

// PBR functions
fn fresnel_schlick(cos_theta: f32, F0: vec3<f32>) -> vec3<f32> {
    return F0 + (1.0 - F0) * pow(1.0 - cos_theta, 5.0);
}

fn distribution_ggx(N: vec3<f32>, H: vec3<f32>, roughness: f32) -> f32 {
    let a = roughness * roughness;
    let a2 = a * a;
    let NdotH = max(dot(N, H), 0.0);
    let NdotH2 = NdotH * NdotH;

    let nom = a2;
    var denom = (NdotH2 * (a2 - 1.0) + 1.0);
    denom = PI * denom * denom;

    return nom / denom;
}

fn geometry_schlick_ggx(NdotV: f32, roughness: f32) -> f32 {
    let r = (roughness + 1.0);
    let k = (r * r) / 8.0;

    let nom = NdotV;
    let denom = NdotV * (1.0 - k) + k;

    return nom / denom;
}

fn geometry_smith(N: vec3<f32>, V: vec3<f32>, L: vec3<f32>, roughness: f32) -> f32 {
    let NdotV = max(dot(N, V), 0.0);
    let NdotL = max(dot(N, L), 0.0);
    let ggx2 = geometry_schlick_ggx(NdotV, roughness);
    let ggx1 = geometry_schlick_ggx(NdotL, roughness);

    return ggx1 * ggx2;
}

// Shadow mapping
fn get_cascade_index(view_pos: vec3<f32>) -> u32 {
    let view_depth = abs(view_pos.z);
    for (var i: u32 = 0u; i < light_data.shadow_cascade_count - 1u; i = i + 1u) {
        if (view_depth < light_data.lights[0].shadow_splits[i]) {
            return i;
        }
    }
    return light_data.shadow_cascade_count - 1u;
}

fn sample_shadow_cascade(world_pos: vec3<f32>, cascade_idx: u32) -> f32 {
    let light = light_data.lights[0]; // Assuming first light is directional
    let shadow_pos = light.shadow_matrix * vec4<f32>(world_pos, 1.0);
    let proj_coords = shadow_pos.xyz / shadow_pos.w;
    
    // PCF sampling
    let texel_size = 1.0 / 2048.0; // Assuming 2048x2048 shadow maps
    var shadow: f32 = 0.0;
    
    for (var x: i32 = -1; x <= 1; x = x + 1) {
        for (var y: i32 = -1; y <= 1; y = y + 1) {
            let offset = vec2<f32>(f32(x), f32(y)) * texel_size;
            shadow = shadow + textureSampleCompareLevel(
                t_shadow_cascades,
                s_shadow,
                proj_coords.xy + offset,
                cascade_idx,
                proj_coords.z - 0.001
            );
        }
    }
    
    return shadow / 9.0;
}

// Vertex shader
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;
    let x = f32(vertex_index & 1u);
    let y = f32((vertex_index >> 1u) & 1u);
    out.position = vec4<f32>(x * 2.0 - 1.0, y * 2.0 - 1.0, 0.0, 1.0);
    out.uv = vec2<f32>(x, y);
    return out;
}

// Fragment shader
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Sample G-Buffer
    let albedo = textureSample(t_albedo, s_gbuffer, in.uv);
    let normal = textureSample(t_normal, s_gbuffer, in.uv).xyz * 2.0 - 1.0;
    let metallic_roughness = textureSample(t_metallic_roughness, s_gbuffer, in.uv);
    let emission = textureSample(t_emission, s_gbuffer, in.uv);
    let depth = textureSample(t_depth, s_gbuffer, in.uv);

    // Early exit if fragment is sky
    if (depth >= 1.0) {
        return vec4<f32>(0.0, 0.0, 0.0, 1.0);
    }

    // Reconstruct world position
    let clip_pos = vec4<f32>(in.uv * 2.0 - 1.0, depth, 1.0);
    let world_pos = clip_pos.xyz; // TODO: Implement proper position reconstruction

    // Material properties
    let metallic = metallic_roughness.x;
    let roughness = metallic_roughness.y;
    let V = normalize(camera_pos - world_pos);
    let F0 = mix(vec3<f32>(0.04), albedo.xyz, metallic);

    // Accumulate lighting
    var Lo = vec3<f32>(0.0);

    // Calculate lighting contribution from each light
    for (var i = 0u; i < light_data.light_count; i = i + 1u) {
        let light = light_data.lights[i];
        var L: vec3<f32>;
        var attenuation: f32 = 1.0;

        // Calculate light direction and attenuation based on light type
        switch (light.light_type) {
            case LIGHT_TYPE_DIRECTIONAL: {
                L = -normalize(light.direction);
                // Handle directional light shadows
                let cascade_idx = get_cascade_index(world_pos);
                attenuation = sample_shadow_cascade(world_pos, cascade_idx);
            }
            case LIGHT_TYPE_POINT: {
                let light_vec = light.position - world_pos;
                let distance = length(light_vec);
                L = light_vec / distance;
                attenuation = 1.0 / (distance * distance);
                attenuation = attenuation * smoothstep(light.range, 0.0, distance);
            }
            case LIGHT_TYPE_SPOT: {
                let light_vec = light.position - world_pos;
                let distance = length(light_vec);
                L = light_vec / distance;
                let theta = dot(L, -normalize(light.direction));
                attenuation = 1.0 / (distance * distance);
                attenuation = attenuation * smoothstep(light.spot_angle_cos, light.spot_angle_cos + 0.1, theta);
            }
            default: {
                L = vec3<f32>(0.0);
            }
        }

        let H = normalize(V + L);
        let NdotL = max(dot(normal, L), 0.0);

        if (NdotL > 0.0) {
            // Cook-Torrance BRDF
            let NDF = distribution_ggx(normal, H, roughness);
            let G = geometry_smith(normal, V, L, roughness);
            let F = fresnel_schlick(max(dot(H, V), 0.0), F0);

            let numerator = NDF * G * F;
            let denominator = 4.0 * max(dot(normal, V), 0.0) * NdotL + 0.001;
            let specular = numerator / denominator;

            let kS = F;
            let kD = (vec3<f32>(1.0) - kS) * (1.0 - metallic);

            Lo = Lo + (kD * albedo.xyz / PI + specular) * light.color * light.intensity * NdotL * attenuation;
        }
    }

    // Add ambient lighting
    let ambient = light_data.ambient_intensity * albedo.xyz;
    let color = ambient + Lo + emission.xyz;

    // Tone mapping and gamma correction
    let mapped = color / (color + vec3<f32>(1.0));
    let gamma = pow(mapped, vec3<f32>(1.0/2.2));

    return vec4<f32>(gamma, 1.0);
} 