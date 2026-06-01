// terrain_detail_tex.rs — Sprint 79 / Sprint 90 (load-perf)
//
// Generates a tiling detail-normal texture at Startup using the `noise` crate.
// The texture simulates coarse rocky/gravelly ground detail: a mix of
// low-frequency Perlin "slab" bumps and high-frequency hash "pebble" grain.
//
// Sprint 90: resolution is tier-scaled (Low=128, Med=192, High=256) and
// generation is staggered via StartupQueue so it runs in a dedicated frame
// instead of blocking the Startup burst.
//
// IMPORTANT: terrain.rs PostStartup reads TerrainDetailTex via
// Option<Res<TerrainDetailTex>>.  The stagger queue processes its FIRST item
// before PostStartup runs (Bevy schedule order: Update → PostUpdate → ...
// but PostStartup runs before first Update; we therefore generate the terrain
// detail texture SYNCHRONOUSLY in Startup so it is available in PostStartup).
//
// The handle is stored in `TerrainDetailTex` (Resource) so terrain.rs can
// assign it to the `StandardMaterial::normal_map_texture` field on Medium+.
//
// Public API
//   TerrainDetailTexPlugin   — registers Startup system + Resource
//   TerrainDetailTex         — Resource carrying the Handle<Image>

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

pub struct TerrainDetailTexPlugin;

impl Plugin for TerrainDetailTexPlugin {
    fn build(&self, app: &mut App) {
        // Must run SYNCHRONOUSLY in Startup so the handle is available in
        // PostStartup (spawn_terrain reads it via Option<Res<TerrainDetailTex>>).
        app.add_systems(Startup, generate_terrain_detail_tex);
    }
}

// ---------------------------------------------------------------------------
// Resource
// ---------------------------------------------------------------------------

/// Holds the procedurally-generated tiling detail-normal `Handle<Image>`.
#[derive(Resource)]
pub struct TerrainDetailTex {
    pub normal_map: Handle<Image>,
}

// ---------------------------------------------------------------------------
// Startup system
// ---------------------------------------------------------------------------

fn generate_terrain_detail_tex(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    quality: Res<GraphicsQuality>,
) {
    let tex_n = quality.proc_tex_size();
    let normal_map = images.add(build_terrain_detail_normal(tex_n));
    commands.insert_resource(TerrainDetailTex { normal_map });
    info!(
        "terrain_detail_tex: generated {}x{} tiling detail-normal texture",
        tex_n, tex_n
    );
}

// ---------------------------------------------------------------------------
// Texture builder
// ---------------------------------------------------------------------------

/// Build a tangent-space normal map for coarse rocky ground detail.
///
/// Height field = 70% low-freq Perlin "slab" + 30% high-freq hash "pebble".
/// Stored as `Rgba8Unorm` (linear, not sRGB) — Bevy interprets normal maps
/// in linear space.  R = X, G = Y, B = Z (pointing up), A = 255.
fn build_terrain_detail_normal(n: usize) -> Image {
    // Two Perlin samplers at different seeds for variety.
    let perlin_slab   = Perlin::new(0x5EED_FACE);
    let perlin_detail = Perlin::new(0xCAFE_BEEF);

    let mut data: Vec<u8> = Vec::with_capacity(n * n * 4);
    let nf = n as f64;
    let step = 1.0 / nf;

    for y in 0..n {
        for x in 0..n {
            let fx = x as f64 / nf;
            let fy = y as f64 / nf;

            // ---- Low-frequency "slab" bumps (3 waves across the tile) --------
            let slab = perlin_slab.get([fx * 3.0, fy * 3.0]) as f32;

            // ---- High-frequency "pebble" grain (12 waves across the tile) ----
            let grain = perlin_detail.get([fx * 12.0, fy * 12.0]) as f32 * 0.35;

            // Combined height = weighted sum of the two layers.
            let h = slab * 0.70 + grain;

            // Finite-difference neighbours (tiling: wrap at boundary).
            let fx_r = ((x + 1) % n) as f64 / nf;
            let fy_u = ((y + 1) % n) as f64 / nf;

            let h_r = perlin_slab.get([fx_r * 3.0, fy * 3.0]) as f32 * 0.70
                + perlin_detail.get([fx_r * 12.0, fy * 12.0]) as f32 * 0.35;
            let h_u = perlin_slab.get([fx * 3.0, fy_u * 3.0]) as f32 * 0.70
                + perlin_detail.get([fx * 12.0, fy_u * 12.0]) as f32 * 0.35;

            // Tangent-space normal from finite differences.
            let dh_dx = (h_r - h) / step as f32;
            let dh_dy = (h_u - h) / step as f32;

            // Strength factor: keeps bumps subtle so they complement SSAO/shadows
            // without flattening the per-vertex color contribution.
            let strength = 0.6_f32;
            let nv = Vec3::new(-dh_dx * strength, -dh_dy * strength, 1.0).normalize();

            // Encode [−1,1] → [0,255].
            let r = ((nv.x * 0.5 + 0.5) * 255.0).clamp(0.0, 255.0) as u8;
            let g = ((nv.y * 0.5 + 0.5) * 255.0).clamp(0.0, 255.0) as u8;
            let b = ((nv.z * 0.5 + 0.5) * 255.0).clamp(0.0, 255.0) as u8;

            data.extend_from_slice(&[r, g, b, 255]);
        }
    }

    Image::new(
        Extent3d {
            width:               n as u32,
            height:              n as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8Unorm, // linear — correct for normal maps
        RenderAssetUsages::RENDER_WORLD,
    )
}
