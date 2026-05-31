// prop_textures.rs — Sprint 87
//
// Generates a small set of tileable 256x256 procedural surface textures at
// Startup for world props (fence posts, wooden signs, rocks, metal surfaces).
// Handles are stored in the `PropTextures` resource so map_dressing.rs and
// fence_posts.rs can apply them (base_color_texture on Medium+).
//
// Textures generated:
//   wood_grain   -- warm brown planks with longitudinal grain lines
//                  Rgba8UnormSrgb (for base_color use)
//   weathered_metal -- mottled grey metal with corrosion patches
//                  Rgba8UnormSrgb (for base_color use)
//   rough_rock   -- stone-grey with high-freq pebble pattern
//                  Rgba8UnormSrgb (for base_color use)
//
// Technique: Perlin noise colour-mapped to surface tones. Same proven
// Bevy 0.18 API pattern as terrain_detail_tex.rs / water_textures.rs:
//   Image::new(Extent3d, TextureDimension::D2, data,
//              TextureFormat::Rgba8UnormSrgb, RenderAssetUsages::RENDER_WORLD)
//
// Public API
//   PropTexturesPlugin
//   PropTextures  (Resource) -- only inserted on Medium/High; Low tier skips.

use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};
use noise::{NoiseFn, Perlin};

use crate::graphics_quality::GraphicsQuality;

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct PropTexturesPlugin;

impl Plugin for PropTexturesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, generate_prop_textures);
    }
}

// ---------------------------------------------------------------------------
// Resource
// ---------------------------------------------------------------------------

/// Holds the three procedurally-generated tileable prop surface textures.
/// Only inserted on Medium/High quality tiers.
/// Low quality (and the headless drive_test path) leaves this absent; code
/// that reads it must use `Option<Res<PropTextures>>`.
#[derive(Resource)]
pub struct PropTextures {
    pub wood_grain:      Handle<Image>,
    pub weathered_metal: Handle<Image>,
    pub rough_rock:      Handle<Image>,
}

// ---------------------------------------------------------------------------
// Startup system
// ---------------------------------------------------------------------------

const TEX_N: usize = 256;

fn generate_prop_textures(
    mut commands: Commands,
    mut images:   ResMut<Assets<Image>>,
    quality:      Res<GraphicsQuality>,
) {
    // Low tier: skip texture generation entirely.
    // Props will use plain-color StandardMaterials (existing behaviour).
    // This also covers the headless simulation path which runs without a GPU.
    if matches!(*quality, GraphicsQuality::Low) {
        info!("prop_textures: Low quality -- skipping procedural prop surface textures");
        return;
    }

    let wood_grain      = images.add(build_wood_grain());
    let weathered_metal = images.add(build_weathered_metal());
    let rough_rock      = images.add(build_rough_rock());

    commands.insert_resource(PropTextures {
        wood_grain,
        weathered_metal,
        rough_rock,
    });

    info!("prop_textures: generated 3 x 256x256 tileable prop surface textures (wood/metal/rock)");
}

// ---------------------------------------------------------------------------
// Texture builders
// ---------------------------------------------------------------------------

/// Wood grain: warm brown planks with longitudinal Perlin grain streaks.
///
/// Two Perlin layers:
///   - Low-freq along X (2 waves) = broad plank colour bands.
///   - High-freq along Y (18 waves) = tight grain lines running length-wise.
/// Knot detail: extra Perlin at 6x adds occasional dark swirls.
/// Colour ramp: dark oak (0.28, 0.18, 0.10) to light birch (0.70, 0.52, 0.32).
fn build_wood_grain() -> Image {
    let n            = TEX_N;
    let perlin_plank = Perlin::new(0xBEEF_CAFE);
    let perlin_grain = Perlin::new(0xF00D_FACE);
    let mut data: Vec<u8> = Vec::with_capacity(n * n * 4);

    for y in 0..n {
        for x in 0..n {
            let fx = x as f64 / n as f64;
            let fy = y as f64 / n as f64;

            // Broad plank bands (2 plank-widths across tile width).
            let plank = perlin_plank.get([fx * 2.0, fy * 0.3]) as f32;
            // Tight grain lines running the length of the plank.
            let grain = perlin_grain.get([fx * 0.4, fy * 18.0]) as f32 * 0.40;
            // Occasional knot: a local dark swirl.
            let knot  = (perlin_plank.get([fx * 6.0, fy * 6.0]) as f32 * 0.20)
                .clamp(-0.10, 0.10);

            let h = (plank * 0.55 + grain + knot).clamp(-1.0, 1.0);
            let t = h * 0.5 + 0.5; // remap to [0,1]

            // Colour ramp: dark oak -> mid wood -> light birch grain.
            let r = lerp_u8(0.28, 0.70, t);
            let g = lerp_u8(0.18, 0.52, t);
            let b = lerp_u8(0.10, 0.32, t);

            data.extend_from_slice(&[r, g, b, 255]);
        }
    }

    Image::new(
        Extent3d { width: n as u32, height: n as u32, depth_or_array_layers: 1 },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb, // sRGB -- correct for base_color_texture
        RenderAssetUsages::RENDER_WORLD,
    )
}

/// Weathered metal: grey base with corrosion/rust patches and scratches.
///
/// Layers:
///   - Low-freq Perlin (4 Hz) = broad corrosion blotches.
///   - Medium-freq Perlin (1.5/12 Hz anisotropic) = panel seam lines.
///   - Per-texel hash = fine scratches.
/// Rust overlay blended in where Perlin > 0.25 (orange-brown shift).
fn build_weathered_metal() -> Image {
    let n            = TEX_N;
    let perlin_rust  = Perlin::new(0xDEAD_C0DE);
    let perlin_panel = Perlin::new(0x0ACE_BEEF);
    let mut data: Vec<u8> = Vec::with_capacity(n * n * 4);

    for y in 0..n {
        for x in 0..n {
            let fx = x as f64 / n as f64;
            let fy = y as f64 / n as f64;

            let rust_noise = perlin_rust.get([fx * 4.0, fy * 4.0]) as f32;
            let seam_noise = perlin_panel.get([fx * 1.5, fy * 12.0]) as f32 * 0.25;
            let scratch    = cheap_hash(x, y) * 0.10;

            let h = (rust_noise * 0.60 + seam_noise + scratch).clamp(-1.0, 1.0);
            let t = h * 0.5 + 0.5;

            // Rust strength: how much the corrosion tint applies.
            let rust_strength = (rust_noise - 0.25).clamp(0.0, 0.5) / 0.5;

            // Base: dark steel (0.35) to silver-grey (0.62).
            let base_r = lerp(0.35, 0.62, t);
            let base_g = lerp(0.35, 0.62, t);
            let base_b = lerp(0.38, 0.65, t);

            // Rust overlay colour (0.48, 0.28, 0.12) -- orange-brown.
            let r_f = lerp(base_r, 0.48, rust_strength * 0.65);
            let g_f = lerp(base_g, 0.28, rust_strength * 0.65);
            let b_f = lerp(base_b, 0.12, rust_strength * 0.65);

            data.extend_from_slice(&[
                (r_f * 255.0).clamp(0.0, 255.0) as u8,
                (g_f * 255.0).clamp(0.0, 255.0) as u8,
                (b_f * 255.0).clamp(0.0, 255.0) as u8,
                255,
            ]);
        }
    }

    Image::new(
        Extent3d { width: n as u32, height: n as u32, depth_or_array_layers: 1 },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    )
}

/// Rough rock: granite-grey with multi-scale pebble/crack noise.
///
/// Three Perlin octaves (3/8/20 Hz) summed with diminishing amplitude give a
/// convincing fractal stone appearance. Brightness mapped to cool grey-blue
/// granite palette.
fn build_rough_rock() -> Image {
    let n        = TEX_N;
    let perlin_a = Perlin::new(0xCAFE_BABE);
    let perlin_b = Perlin::new(0x1234_5678);
    let perlin_c = Perlin::new(0xABCD_EF01);
    let mut data: Vec<u8> = Vec::with_capacity(n * n * 4);

    for y in 0..n {
        for x in 0..n {
            let fx = x as f64 / n as f64;
            let fy = y as f64 / n as f64;

            let slab   = perlin_a.get([fx * 3.0,  fy * 3.0 ]) as f32;
            let pebble = perlin_b.get([fx * 8.0,  fy * 8.0 ]) as f32 * 0.45;
            let grit   = perlin_c.get([fx * 20.0, fy * 20.0]) as f32 * 0.18;

            let h = (slab * 0.60 + pebble + grit).clamp(-1.0, 1.0);
            let t = h * 0.5 + 0.5;

            // Granite grey-blue: dark (0.38,0.37,0.36) to light quartz (0.72,0.71,0.70).
            let r = lerp_u8(0.38, 0.72, t);
            let g = lerp_u8(0.37, 0.71, t);
            let b = lerp_u8(0.36, 0.70, t);

            data.extend_from_slice(&[r, g, b, 255]);
        }
    }

    Image::new(
        Extent3d { width: n as u32, height: n as u32, depth_or_array_layers: 1 },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    )
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

#[inline]
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t.clamp(0.0, 1.0)
}

#[inline]
fn lerp_u8(a: f32, b: f32, t: f32) -> u8 {
    (lerp(a, b, t) * 255.0).clamp(0.0, 255.0) as u8
}

/// Cheap per-texel deterministic hash in [-1, 1].
/// Same algorithm as vehicle_textures.rs.
#[inline]
fn cheap_hash(x: usize, y: usize) -> f32 {
    let mut v = (x as u32).wrapping_mul(2654435761).wrapping_add(y as u32);
    v ^= v >> 16;
    v = v.wrapping_mul(0x45d9f3b);
    v ^= v >> 16;
    v as f32 / (u32::MAX as f32) * 2.0 - 1.0
}
