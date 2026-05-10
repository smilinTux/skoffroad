// vehicle_textures.rs — Sprint 60
//
// Procedural PBR maps for all four vehicles (Skrambler / Highland SK /
// Dune Skipper / Hauler SK).
//
// Three 256×256 textures are generated at Startup using the `noise` crate:
//
//   paint_normal    — subtle metallic flake (high-frequency hash) + low-freq
//                     Perlin "orange-peel" ripple, stored as Rgba8Unorm normal map
//   paint_roughness — base 0.32 with small Perlin variance (clearcoat look);
//                     stored as Rgba8Unorm (R channel is roughness)
//   dirt_streak     — downward gravity streaks + lower-corner darkening,
//                     stored as Rgba8UnormSrgb for linear-space multiply into baseColor
//
// The textures are put in the `VehicleTextureSet` resource.
// A one-shot Update system (`apply_vehicle_textures`) then finds every entity
// that carries `DefaultSkin` or `VariantSkin`, reads its `MeshMaterial3d`
// handle, and — if the material looks like body paint (metallic < 0.5 and
// no emissive and opaque) — upgrades it with the three maps.
//
// Public API
//   VehicleTexturesPlugin

use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};
use noise::{NoiseFn, Perlin};

use crate::vehicle::DefaultSkin;
use crate::variants::VariantSkin;

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct VehicleTexturesPlugin;

impl Plugin for VehicleTexturesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, generate_vehicle_textures)
           .add_systems(Update, apply_vehicle_textures);
    }
}

// ---------------------------------------------------------------------------
// Resource
// ---------------------------------------------------------------------------

/// Three PBR texture handles shared across all vehicle materials.
#[derive(Resource)]
pub struct VehicleTextureSet {
    pub paint_normal:    Handle<Image>,
    pub paint_roughness: Handle<Image>,
    pub dirt_streak:     Handle<Image>,
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const TEX_N: usize = 256;

// ---------------------------------------------------------------------------
// Startup: generate all three textures
// ---------------------------------------------------------------------------

fn generate_vehicle_textures(
    mut commands: Commands,
    mut images:   ResMut<Assets<Image>>,
) {
    let paint_normal    = images.add(build_paint_normal());
    let paint_roughness = images.add(build_paint_roughness());
    let dirt_streak     = images.add(build_dirt_streak());

    commands.insert_resource(VehicleTextureSet {
        paint_normal,
        paint_roughness,
        dirt_streak,
    });

    info!("vehicle_textures: generated 3 × 256×256 PBR maps for vehicle paint");
}

// ---------------------------------------------------------------------------
// Update (once): apply textures to paint materials on vehicle meshes
// ---------------------------------------------------------------------------

fn apply_vehicle_textures(
    texture_set: Option<Res<VehicleTextureSet>>,
    default_skin_q: Query<&MeshMaterial3d<StandardMaterial>, With<DefaultSkin>>,
    variant_skin_q: Query<&MeshMaterial3d<StandardMaterial>, With<VariantSkin>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut done: Local<bool>,
) {
    if *done { return; }

    let Some(textures) = texture_set else { return; };

    // Collect all material handles from both DefaultSkin and VariantSkin
    // entities, deduplicated, so we only mutate each asset once.
    let mut handles: Vec<AssetId<StandardMaterial>> = Vec::new();

    for mat_handle in default_skin_q.iter().chain(variant_skin_q.iter()) {
        let id = mat_handle.id();
        if !handles.contains(&id) {
            handles.push(id);
        }
    }

    if handles.is_empty() {
        // Vehicles haven't spawned yet — wait.
        return;
    }

    let normal_h    = textures.paint_normal.clone();
    let roughness_h = textures.paint_roughness.clone();
    let dirt_h      = textures.dirt_streak.clone();

    let mut upgraded = 0usize;
    for id in handles {
        let Some(mat) = materials.get_mut(id) else { continue };

        // Only upgrade body-paint materials:
        //   - metallic < 0.5          (excludes chrome, steel, rims)
        //   - emissive near zero      (excludes headlights)
        //   - alpha mode Opaque       (excludes glass / windshields)
        //   - no existing normal map  (don't double-apply)
        let is_paint = mat.metallic < 0.5
            && mat.emissive.red   < 0.1
            && mat.emissive.green < 0.1
            && mat.emissive.blue  < 0.1
            && matches!(mat.alpha_mode, AlphaMode::Opaque)
            && mat.normal_map_texture.is_none();

        if !is_paint { continue; }

        mat.normal_map_texture          = Some(normal_h.clone());
        mat.metallic_roughness_texture  = Some(roughness_h.clone());
        mat.base_color_texture          = Some(dirt_h.clone());

        // Keep the dirt as a subtle darkening multiply — raise the base
        // roughness slightly so it reads like clearcoat not raw paint.
        mat.perceptual_roughness = (mat.perceptual_roughness * 0.90 + 0.32 * 0.10)
            .clamp(0.12, 0.72);

        upgraded += 1;
    }

    if upgraded > 0 {
        info!("vehicle_textures: applied PBR maps to {} paint materials", upgraded);
        *done = true;
    }
}

// ---------------------------------------------------------------------------
// Texture builders
// ---------------------------------------------------------------------------

/// Normal map: metallic-flake high-freq hash + low-freq orange-peel Perlin.
///
/// Stored as `Rgba8Unorm` (not sRGB) — Bevy interprets normal maps in linear
/// space.  R = X tangent, G = Y tangent, B = Z (up), A = 255.
fn build_paint_normal() -> Image {
    let n = TEX_N;
    let perlin = Perlin::new(0xDEAD_BEEF);
    let mut data: Vec<u8> = Vec::with_capacity(n * n * 4);

    for y in 0..n {
        for x in 0..n {
            let fx = x as f64 / n as f64;
            let fy = y as f64 / n as f64;

            // Low-frequency orange-peel ripple (scale 3.0 ≈ 3 bumps across surface)
            let low_freq = perlin.get([fx * 3.0, fy * 3.0]) as f32;

            // High-frequency metallic-flake: tiny hash per texel
            let flake = cheap_hash(x, y);

            // Mix: 80% orange-peel, 20% metallic flake
            let combined = low_freq * 0.80 + flake * 0.20;

            // Finite-difference normal from combined height field.
            // We approximate ∂h/∂x and ∂h/∂y from neighbour samples.
            let right = perlin.get([(fx + 1.0 / n as f64) * 3.0, fy * 3.0]) as f32;
            let up    = perlin.get([fx * 3.0, (fy + 1.0 / n as f64) * 3.0]) as f32;
            let dx = (right - combined) * 4.0;   // tangent strength
            let dy = (up    - combined) * 4.0;

            // Tangent-space normal: (−∂h/∂x, −∂h/∂y, 1).normalize()
            let nv = Vec3::new(-dx, -dy, 1.0).normalize();

            // Encode: [0,1] range → [0,255] u8
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
        TextureFormat::Rgba8Unorm,      // linear — correct for normal maps
        RenderAssetUsages::RENDER_WORLD,
    )
}

/// Roughness map: base 0.32 + small low-freq Perlin variance ±0.08.
///
/// Stored as `Rgba8Unorm`.  Bevy's metallic_roughness_texture reads:
///   B channel → metallic (we set 0)
///   G channel → roughness
fn build_paint_roughness() -> Image {
    let n = TEX_N;
    let perlin = Perlin::new(0xC0FFEE);
    let mut data: Vec<u8> = Vec::with_capacity(n * n * 4);

    for y in 0..n {
        for x in 0..n {
            let fx = x as f64 / n as f64;
            let fy = y as f64 / n as f64;

            // Very low frequency — one gentle wave across the whole surface.
            let noise = perlin.get([fx * 2.0, fy * 2.0]) as f32; // −1..1

            // Base 0.32 roughness (clearcoat look), variance ±0.08
            let roughness = (0.32 + noise * 0.08).clamp(0.0, 1.0);

            let rough_u8 = (roughness * 255.0) as u8;
            // R=0, G=roughness, B=0 (metallic=0), A=255
            data.extend_from_slice(&[0, rough_u8, 0, 255]);
        }
    }

    Image::new(
        Extent3d { width: n as u32, height: n as u32, depth_or_array_layers: 1 },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::RENDER_WORLD,
    )
}

/// Dirt/streak overlay: downward-gravity streaks + lower-corner darkening.
///
/// Stored as `Rgba8UnormSrgb`.  When set as `base_color_texture`, Bevy
/// multiplies it against `base_color`.  Values near (1,1,1,1) are invisible;
/// darker values tint the paint.
fn build_dirt_streak() -> Image {
    let n = TEX_N;
    let perlin = Perlin::new(0x0FF_0AD);
    let mut data: Vec<u8> = Vec::with_capacity(n * n * 4);

    for y in 0..n {
        for x in 0..n {
            let fx = x as f64 / n as f64;
            let fy = y as f64 / n as f64;

            // ---- Downward streaks -------------------------------------------
            // Noise at high X-frequency + low Y-frequency produces vertical
            // ribbon shapes. `fy` goes 0 (top UV) → 1 (bottom UV); streaks
            // accumulate more dirt as they descend (multiply by fy²).
            let streak_noise = perlin.get([fx * 14.0, fy * 2.0]) as f32; // −1..1
            let streak_strength = (streak_noise * 0.5 + 0.5)             // 0..1
                * fy as f32 * fy as f32;                                  // stronger at bottom

            // ---- Lower-corner darkening -------------------------------------
            // Edges collect grime: dist from centre = max(|2u−1|, |2v−1|).
            let edge_x = (2.0 * fx as f32 - 1.0).abs();
            let edge_y = (2.0 * fy as f32 - 1.0).abs();
            let corner_dark = (edge_x.max(edge_y) - 0.55).clamp(0.0, 1.0) * 0.30;

            // ---- Combine and bias toward clean (mostly white) ---------------
            // Most of the surface is clean; streaks are subtle.
            let dirt = (streak_strength * 0.18 + corner_dark).clamp(0.0, 1.0);
            let brightness = ((1.0 - dirt) * 255.0).clamp(180.0, 255.0) as u8;

            data.extend_from_slice(&[brightness, brightness, brightness, 255]);
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

/// Cheap per-texel deterministic hash in [−1, 1] simulating metallic flake.
/// Uses integer bit-mixing (no allocation, no sin/cos).
#[inline]
fn cheap_hash(x: usize, y: usize) -> f32 {
    let mut v = (x as u32).wrapping_mul(2654435761).wrapping_add(y as u32);
    v ^= v >> 16;
    v = v.wrapping_mul(0x45d9f3b);
    v ^= v >> 16;
    // Map [0, u32::MAX] → [−1, 1]
    v as f32 / (u32::MAX as f32) * 2.0 - 1.0
}
