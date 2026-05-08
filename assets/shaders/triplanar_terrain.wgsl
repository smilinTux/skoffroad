// Triplanar terrain fragment shader for Sprint 41.
//
// Single-layer for now (dirt). Commit 4 extends this to a 4-channel
// splatmap blend across dirt / grass / rock / mud.
//
// We sample world-space (X, Y, Z) projections of the same texture and
// blend them by abs(world_normal). On near-vertical cliffs the side
// projections dominate, so the texture never stretches. We also blend
// two scales (close + far) to break visible tiling.

#import bevy_pbr::pbr_fragment::pbr_input_from_standard_material
#import bevy_pbr::pbr_functions as fns
#import bevy_pbr::forward_io::{VertexOutput, FragmentOutput}

struct TriplanarUniforms {
    /// Inverse world-units per UV unit for the close-up sample (e.g. 0.5 = 2 m tile).
    tile_scale:    f32,
    /// Multiplier on tile_scale for the far / detail sample.
    detail_scale:  f32,
    /// 0..1 mix between close-up and detail samples (~0.35 looks nice).
    detail_blend:  f32,
    _pad:          f32,
};

@group(2) @binding(100) var<uniform> tri_u: TriplanarUniforms;
@group(2) @binding(101) var dirt_albedo:        texture_2d<f32>;
@group(2) @binding(102) var dirt_albedo_smp:    sampler;
@group(2) @binding(103) var dirt_roughness:     texture_2d<f32>;
@group(2) @binding(104) var dirt_roughness_smp: sampler;

fn triplanar_albedo(world_pos: vec3<f32>, weights: vec3<f32>, scale: f32) -> vec3<f32> {
    let uv_x = world_pos.zy * scale;
    let uv_y = world_pos.xz * scale;
    let uv_z = world_pos.xy * scale;

    let a_x = textureSample(dirt_albedo, dirt_albedo_smp, uv_x).rgb;
    let a_y = textureSample(dirt_albedo, dirt_albedo_smp, uv_y).rgb;
    let a_z = textureSample(dirt_albedo, dirt_albedo_smp, uv_z).rgb;

    return a_x * weights.x + a_y * weights.y + a_z * weights.z;
}

fn triplanar_roughness(world_pos: vec3<f32>, weights: vec3<f32>, scale: f32) -> f32 {
    let uv_x = world_pos.zy * scale;
    let uv_y = world_pos.xz * scale;
    let uv_z = world_pos.xy * scale;

    let r_x = textureSample(dirt_roughness, dirt_roughness_smp, uv_x).r;
    let r_y = textureSample(dirt_roughness, dirt_roughness_smp, uv_y).r;
    let r_z = textureSample(dirt_roughness, dirt_roughness_smp, uv_z).r;

    return r_x * weights.x + r_y * weights.y + r_z * weights.z;
}

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> FragmentOutput {
    var pbr_input = pbr_input_from_standard_material(in, is_front);

    let world_pos = in.world_position.xyz;

    // Heavy power keeps the dominant-axis projection clean on flat ground.
    let n = normalize(in.world_normal);
    let raw = pow(abs(n), vec3<f32>(4.0));
    let weights = raw / max(raw.x + raw.y + raw.z, 0.001);

    let s_close = tri_u.tile_scale;
    let s_far   = tri_u.tile_scale * tri_u.detail_scale;

    let albedo_close = triplanar_albedo(world_pos, weights, s_close);
    let albedo_far   = triplanar_albedo(world_pos, weights, s_far);
    let albedo = mix(albedo_close, albedo_far, tri_u.detail_blend);

    let rough_close = triplanar_roughness(world_pos, weights, s_close);
    let rough_far   = triplanar_roughness(world_pos, weights, s_far);
    let rough = mix(rough_close, rough_far, tri_u.detail_blend);

    // Preserve any vertex-color tint the legacy mesh provides as a soft mask.
    let vc = pbr_input.material.base_color.rgb;
    pbr_input.material.base_color = vec4<f32>(albedo * (0.4 + 0.6 * vc), 1.0);
    pbr_input.material.perceptual_roughness = clamp(rough, 0.05, 1.0);

    var out: FragmentOutput;
    out.color = fns::apply_pbr_lighting(pbr_input);
    out.color = fns::main_pass_post_lighting_processing(pbr_input, out.color);
    return out;
}
