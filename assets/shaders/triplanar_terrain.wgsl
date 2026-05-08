// Triplanar terrain fragment shader.
//
// 4-layer splat blend across dirt / grass / rock / mud, keyed off
// world-space slope and height. World-space triplanar projection on each
// layer eliminates UV stretch on cliffs. A two-frequency sample per layer
// (close + far) breaks visible tiling.
//
// Blend rules (procedural, no splatmap texture yet):
//   rock  ← steep slopes ∪ high altitude
//   mud   ← flat ground at low altitude (valley floors)
//   grass ← flat-to-gentle slopes at low-to-mid altitude
//   dirt  ← whatever the other three don't claim
//
// All four weights are normalised so they sum to 1. Roughness blends the
// same way; albedo is RGB-blended.

#import bevy_pbr::pbr_fragment::pbr_input_from_standard_material
#import bevy_pbr::pbr_functions as fns
#import bevy_pbr::forward_io::{VertexOutput, FragmentOutput}

struct TriplanarUniforms {
    /// World-units → UV scale for the close-up sample (0.25 = 4 m repeat).
    tile_scale:    f32,
    /// Multiplier on tile_scale for the far / detail sample.
    detail_scale:  f32,
    /// Mix factor close ↔ detail (0..1).
    detail_blend:  f32,
    /// 1.0 = full splat blend, 0.0 = dirt-only (used for Medium tier later).
    blend_strength: f32,
    /// 0..1, smoothed value driven by StormState. 1 = soaking wet.
    wetness:        f32,
    _pad0:          f32,
    _pad1:          f32,
    _pad2:          f32,
};

@group(2) @binding(100) var<uniform> tri_u: TriplanarUniforms;

@group(2) @binding(101) var dirt_albedo:     texture_2d<f32>;
@group(2) @binding(102) var dirt_albedo_smp: sampler;
@group(2) @binding(103) var dirt_rough:      texture_2d<f32>;
@group(2) @binding(104) var dirt_rough_smp:  sampler;

@group(2) @binding(105) var grass_albedo:     texture_2d<f32>;
@group(2) @binding(106) var grass_albedo_smp: sampler;
@group(2) @binding(107) var grass_rough:      texture_2d<f32>;
@group(2) @binding(108) var grass_rough_smp:  sampler;

@group(2) @binding(109) var rock_albedo:     texture_2d<f32>;
@group(2) @binding(110) var rock_albedo_smp: sampler;
@group(2) @binding(111) var rock_rough:      texture_2d<f32>;
@group(2) @binding(112) var rock_rough_smp:  sampler;

@group(2) @binding(113) var mud_albedo:      texture_2d<f32>;
@group(2) @binding(114) var mud_albedo_smp:  sampler;
@group(2) @binding(115) var mud_rough:       texture_2d<f32>;
@group(2) @binding(116) var mud_rough_smp:   sampler;

// ---------------------------------------------------------------------------
// Triplanar helpers — generic over (texture, sampler) per layer
// ---------------------------------------------------------------------------

fn tri_uvs(world_pos: vec3<f32>, scale: f32) -> array<vec2<f32>, 3> {
    return array<vec2<f32>, 3>(
        world_pos.zy * scale,
        world_pos.xz * scale,
        world_pos.xy * scale,
    );
}

fn blend_weights(world_normal: vec3<f32>) -> vec3<f32> {
    let raw = pow(abs(world_normal), vec3<f32>(4.0));
    return raw / max(raw.x + raw.y + raw.z, 0.001);
}

// One macro-like helper per layer; WGSL has no real generics over textures yet.

fn sample_dirt_albedo(world_pos: vec3<f32>, w: vec3<f32>, s: f32) -> vec3<f32> {
    let uv = tri_uvs(world_pos, s);
    return textureSample(dirt_albedo, dirt_albedo_smp, uv[0]).rgb * w.x
         + textureSample(dirt_albedo, dirt_albedo_smp, uv[1]).rgb * w.y
         + textureSample(dirt_albedo, dirt_albedo_smp, uv[2]).rgb * w.z;
}
fn sample_dirt_rough(world_pos: vec3<f32>, w: vec3<f32>, s: f32) -> f32 {
    let uv = tri_uvs(world_pos, s);
    return textureSample(dirt_rough, dirt_rough_smp, uv[0]).r * w.x
         + textureSample(dirt_rough, dirt_rough_smp, uv[1]).r * w.y
         + textureSample(dirt_rough, dirt_rough_smp, uv[2]).r * w.z;
}

fn sample_grass_albedo(world_pos: vec3<f32>, w: vec3<f32>, s: f32) -> vec3<f32> {
    let uv = tri_uvs(world_pos, s);
    return textureSample(grass_albedo, grass_albedo_smp, uv[0]).rgb * w.x
         + textureSample(grass_albedo, grass_albedo_smp, uv[1]).rgb * w.y
         + textureSample(grass_albedo, grass_albedo_smp, uv[2]).rgb * w.z;
}
fn sample_grass_rough(world_pos: vec3<f32>, w: vec3<f32>, s: f32) -> f32 {
    let uv = tri_uvs(world_pos, s);
    return textureSample(grass_rough, grass_rough_smp, uv[0]).r * w.x
         + textureSample(grass_rough, grass_rough_smp, uv[1]).r * w.y
         + textureSample(grass_rough, grass_rough_smp, uv[2]).r * w.z;
}

fn sample_rock_albedo(world_pos: vec3<f32>, w: vec3<f32>, s: f32) -> vec3<f32> {
    let uv = tri_uvs(world_pos, s);
    return textureSample(rock_albedo, rock_albedo_smp, uv[0]).rgb * w.x
         + textureSample(rock_albedo, rock_albedo_smp, uv[1]).rgb * w.y
         + textureSample(rock_albedo, rock_albedo_smp, uv[2]).rgb * w.z;
}
fn sample_rock_rough(world_pos: vec3<f32>, w: vec3<f32>, s: f32) -> f32 {
    let uv = tri_uvs(world_pos, s);
    return textureSample(rock_rough, rock_rough_smp, uv[0]).r * w.x
         + textureSample(rock_rough, rock_rough_smp, uv[1]).r * w.y
         + textureSample(rock_rough, rock_rough_smp, uv[2]).r * w.z;
}

fn sample_mud_albedo(world_pos: vec3<f32>, w: vec3<f32>, s: f32) -> vec3<f32> {
    let uv = tri_uvs(world_pos, s);
    return textureSample(mud_albedo, mud_albedo_smp, uv[0]).rgb * w.x
         + textureSample(mud_albedo, mud_albedo_smp, uv[1]).rgb * w.y
         + textureSample(mud_albedo, mud_albedo_smp, uv[2]).rgb * w.z;
}
fn sample_mud_rough(world_pos: vec3<f32>, w: vec3<f32>, s: f32) -> f32 {
    let uv = tri_uvs(world_pos, s);
    return textureSample(mud_rough, mud_rough_smp, uv[0]).r * w.x
         + textureSample(mud_rough, mud_rough_smp, uv[1]).r * w.y
         + textureSample(mud_rough, mud_rough_smp, uv[2]).r * w.z;
}

// ---------------------------------------------------------------------------
// Splat weight derivation from world position + slope
// ---------------------------------------------------------------------------
//
// Returns weights ordered (dirt, grass, rock, mud), normalised to sum to 1.
// terrain.rs uses HEIGHT_SCALE = 12.0; we map world.y∈[-12,+12] → 0..1.

fn splat_weights(world_pos: vec3<f32>, normal: vec3<f32>) -> vec4<f32> {
    let slope = 1.0 - clamp(normal.y, 0.0, 1.0);
    let h = clamp((world_pos.y + 12.0) / 24.0, 0.0, 1.0);

    let rock  = smoothstep(0.32, 0.62, slope) + 0.4 * smoothstep(0.65, 0.95, h);
    let mud   = (1.0 - smoothstep(0.0, 0.18, slope)) * (1.0 - smoothstep(0.05, 0.30, h));
    let grass = (1.0 - smoothstep(0.18, 0.42, slope))
              * (1.0 - smoothstep(0.55, 0.85, h))
              * (1.0 - mud * 0.8);
    let dirt  = max(0.05, 1.0 - clamp(rock + mud + grass, 0.0, 1.0));

    let raw = vec4<f32>(dirt, grass, clamp(rock, 0.0, 1.5), mud);
    return raw / max(raw.x + raw.y + raw.z + raw.w, 0.001);
}

// ---------------------------------------------------------------------------
// Fragment entry point
// ---------------------------------------------------------------------------

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> FragmentOutput {
    var pbr_input = pbr_input_from_standard_material(in, is_front);

    let world_pos = in.world_position.xyz;
    let n = normalize(in.world_normal);
    let w = blend_weights(n);

    let s_close = tri_u.tile_scale;
    let s_far   = tri_u.tile_scale * tri_u.detail_scale;

    // Per-layer triplanar samples at two frequencies.
    let dirt_a   = mix(sample_dirt_albedo(world_pos, w, s_close),
                       sample_dirt_albedo(world_pos, w, s_far),
                       tri_u.detail_blend);
    let grass_a  = mix(sample_grass_albedo(world_pos, w, s_close),
                       sample_grass_albedo(world_pos, w, s_far),
                       tri_u.detail_blend);
    let rock_a   = mix(sample_rock_albedo(world_pos, w, s_close),
                       sample_rock_albedo(world_pos, w, s_far),
                       tri_u.detail_blend);
    let mud_a    = mix(sample_mud_albedo(world_pos, w, s_close),
                       sample_mud_albedo(world_pos, w, s_far),
                       tri_u.detail_blend);

    let dirt_r   = mix(sample_dirt_rough(world_pos, w, s_close),
                       sample_dirt_rough(world_pos, w, s_far),
                       tri_u.detail_blend);
    let grass_r  = mix(sample_grass_rough(world_pos, w, s_close),
                       sample_grass_rough(world_pos, w, s_far),
                       tri_u.detail_blend);
    let rock_r   = mix(sample_rock_rough(world_pos, w, s_close),
                       sample_rock_rough(world_pos, w, s_far),
                       tri_u.detail_blend);
    let mud_r    = mix(sample_mud_rough(world_pos, w, s_close),
                       sample_mud_rough(world_pos, w, s_far),
                       tri_u.detail_blend);

    // 4-way splat blend (dirt, grass, rock, mud).
    let sw_full = splat_weights(world_pos, n);
    // For lower tiers we lerp toward dirt-only.
    let dirt_only = vec4<f32>(1.0, 0.0, 0.0, 0.0);
    let sw = mix(dirt_only, sw_full, tri_u.blend_strength);

    let albedo = dirt_a  * sw.x
               + grass_a * sw.y
               + rock_a  * sw.z
               + mud_a   * sw.w;

    let rough  = dirt_r  * sw.x
               + grass_r * sw.y
               + rock_r  * sw.z
               + mud_r   * sw.w;

    // Wet-surface tweak: water in pores darkens albedo and drops roughness so
    // bright specular highlights pop. Mud reads less wet because it's already
    // dark and mostly diffuse — bias the albedo darken toward grass/dirt/rock.
    let wet = clamp(tri_u.wetness, 0.0, 1.0);
    let mud_share = sw.w;
    let wet_albedo_mul = 1.0 - 0.30 * wet * (1.0 - 0.6 * mud_share);
    let wet_rough_mul  = 1.0 - 0.55 * wet * (1.0 - 0.4 * mud_share);
    let wet_albedo = albedo * wet_albedo_mul;
    let wet_rough  = rough  * wet_rough_mul;

    pbr_input.material.base_color = vec4<f32>(wet_albedo, 1.0);
    pbr_input.material.perceptual_roughness = clamp(wet_rough, 0.05, 1.0);

    var out: FragmentOutput;
    out.color = fns::apply_pbr_lighting(pbr_input);
    out.color = fns::main_pass_post_lighting_processing(pbr_input, out.color);
    return out;
}
