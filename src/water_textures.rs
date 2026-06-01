// water_textures.rs — Sprint 82 / Sprint 90 (load-perf)
//
// Generates two procedural textures at Startup for the enhanced water surface:
//
//   water_normal  — tiling tangent-space normal map simulating rolling water
//                   ripples.  Two Perlin layers: low-freq broad swells +
//                   high-freq chop.  Stored as Rgba8Unorm (linear).
//
//   water_foam    — white foam texture used at the shoreline.  Near-white with
//                   low-freq noise giving a bubbly edge appearance.
//                   Stored as Rgba8UnormSrgb (for base_color use).
//
// Sprint 90: texture resolution is now tier-scaled (Low=128, Med=192, High=256)
// and generation is staggered via StartupQueue so it runs across two frames
// rather than all at once in the Startup burst.
//
// Both handles are stored in the WaterTextures resource.
// Used by water_reflective.rs (Medium+ quality gate).
//
// Public API
//   WaterTexturesPlugin
//   WaterTextures  (Resource)

use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};
use noise::{NoiseFn, Perlin};

use crate::graphics_quality::GraphicsQuality;
use crate::startup_stager::StartupQueue;

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct WaterTexturesPlugin;

impl Plugin for WaterTexturesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, queue_water_textures);
    }
}

// ---------------------------------------------------------------------------
// Resource
// ---------------------------------------------------------------------------

#[derive(Resource)]
pub struct WaterTextures {
    /// Tiling water normal map — assign to normal_map_texture on Medium+.
    pub normal_map: Handle<Image>,
    /// Foam texture — near-white with bubbly noise, used at shoreline quads.
    pub foam:       Handle<Image>,
}

// ---------------------------------------------------------------------------
// Startup system — enqueue generation into the stagger queue
// ---------------------------------------------------------------------------

fn queue_water_textures(
    quality: Res<GraphicsQuality>,
    mut queue: ResMut<StartupQueue>,
) {
    let tex_n = quality.proc_tex_size();
    info!(
        "water_textures: queuing 2 x {}x{} procedural water maps (staggered)",
        tex_n, tex_n
    );

    // Normal map — first slot (runs frame 1 of stagger).
    queue.push(move |world: &mut bevy::ecs::world::World| {
        let normal_img = build_water_normal(tex_n);
        let normal_map = world
            .resource_mut::<bevy::asset::Assets<Image>>()
            .add(normal_img);
        // If WaterTextures resource already exists (foam already inserted),
        // update it; otherwise store partial (foam will be set next frame).
        // We use a staging resource to coordinate the two-frame split.
        world.insert_resource(WaterTexNormalStage { normal_map });
        info!("water_textures: normal map generated ({}x{})", tex_n, tex_n);
    });

    // Foam — second slot (runs frame 2 of stagger).
    queue.push(move |world: &mut bevy::ecs::world::World| {
        let foam_img = build_water_foam(tex_n);
        let foam = world
            .resource_mut::<bevy::asset::Assets<Image>>()
            .add(foam_img);
        // Retrieve the normal map handle stored in the first stage.
        let normal_map = world
            .remove_resource::<WaterTexNormalStage>()
            .map(|s| s.normal_map);
        if let Some(normal_map) = normal_map {
            world.insert_resource(WaterTextures { normal_map, foam });
            info!("water_textures: foam generated; WaterTextures resource ready");
        } else {
            // Fallback: both should always be available, but be defensive.
            warn!("water_textures: normal stage resource missing; inserting foam-only placeholder");
            // Insert a duplicate foam handle as normal_map so the resource
            // is always present (materials guard with Option<Res<...>>).
            world.insert_resource(WaterTextures { normal_map: foam.clone(), foam });
        }
    });
}

// ---------------------------------------------------------------------------
// Staging resource (internal — coordinates the two-frame split)
// ---------------------------------------------------------------------------

#[derive(Resource)]
struct WaterTexNormalStage {
    normal_map: Handle<Image>,
}

// ---------------------------------------------------------------------------
// Water normal map builder
// ---------------------------------------------------------------------------

/// Two-layer Perlin water normal map.
///
/// Layer 1 (scale 3): broad ocean-swell bumps.
/// Layer 2 (scale 10): small chop / ripple detail (30% weight).
///
/// Stored as Rgba8Unorm (linear space) — Bevy interprets normal_map_texture
/// in linear space.  R = X tangent, G = Y tangent, B = Z (up), A = 255.
fn build_water_normal(n: usize) -> Image {
    let swell  = Perlin::new(0xA1B2_C3D4);
    let chop   = Perlin::new(0xD4C3_B2A1);
    let step   = 1.0 / n as f64;

    let mut data: Vec<u8> = Vec::with_capacity(n * n * 4);

    for y in 0..n {
        for x in 0..n {
            let fx = x as f64 / n as f64;
            let fy = y as f64 / n as f64;

            // Combined height field at this texel.
            let h = height_at(&swell, &chop, fx, fy);

            // Finite-difference neighbours (tiling wrap at boundary).
            let h_r = height_at(&swell, &chop, (fx + step).fract(), fy);
            let h_u = height_at(&swell, &chop, fx, (fy + step).fract());

            // Gradient → tangent-space normal.
            let dh_dx = (h_r - h) / step as f32;
            let dh_dy = (h_u - h) / step as f32;

            // Strength 0.7 keeps bumps subtle; water normals should be gentle.
            let strength = 0.7_f32;
            let nv = Vec3::new(-dh_dx * strength, -dh_dy * strength, 1.0).normalize();

            let r = ((nv.x * 0.5 + 0.5) * 255.0).clamp(0.0, 255.0) as u8;
            let g = ((nv.y * 0.5 + 0.5) * 255.0).clamp(0.0, 255.0) as u8;
            let b = ((nv.z * 0.5 + 0.5) * 255.0).clamp(0.0, 255.0) as u8;

            data.extend_from_slice(&[r, g, b, 255]);
        }
    }

    Image::new(
        Extent3d { width: n as u32, height: n as u32, depth_or_array_layers: 1 },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8Unorm,       // linear — correct for normal maps
        RenderAssetUsages::RENDER_WORLD,
    )
}

/// Evaluate the two-layer water height field at normalised (fx, fy).
#[inline]
fn height_at(swell: &Perlin, chop: &Perlin, fx: f64, fy: f64) -> f32 {
    let broad = swell.get([fx * 3.0, fy * 3.0]) as f32;
    let fine  = chop.get([fx * 10.0, fy * 10.0]) as f32 * 0.30;
    broad + fine
}

// ---------------------------------------------------------------------------
// Foam texture builder
// ---------------------------------------------------------------------------

/// Near-white bubbly foam texture for the shoreline strip.
///
/// Stored as Rgba8UnormSrgb (set as base_color_texture on the foam quads).
/// Values are near (255,255,255) with subtle low-freq noise to break up the
/// uniformity and simulate natural bubble clusters.
fn build_water_foam(n: usize) -> Image {
    let perlin = Perlin::new(0xF0A4_5E3D);
    let mut data: Vec<u8> = Vec::with_capacity(n * n * 4);

    for y in 0..n {
        for x in 0..n {
            let fx = x as f64 / n as f64;
            let fy = y as f64 / n as f64;

            // Low-frequency bubble clusters.
            let cluster = perlin.get([fx * 6.0, fy * 6.0]) as f32; // −1..1

            // High-frequency froth texture.
            let froth   = perlin.get([fx * 18.0, fy * 18.0]) as f32 * 0.4;

            // Combined 0..1 density (1 = fully opaque white foam).
            let density = ((cluster + froth) * 0.5 + 0.5).clamp(0.0, 1.0);

            // Foam is bright white; darken slightly where density is low.
            let brightness = (200.0 + density * 55.0).clamp(0.0, 255.0) as u8;
            // Alpha: more opaque where foam density is higher.
            let alpha = (density * 220.0 + 35.0).clamp(0.0, 255.0) as u8;

            data.extend_from_slice(&[brightness, brightness, brightness, alpha]);
        }
    }

    Image::new(
        Extent3d { width: n as u32, height: n as u32, depth_or_array_layers: 1 },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,   // sRGB — used as base_color_texture
        RenderAssetUsages::RENDER_WORLD,
    )
}
