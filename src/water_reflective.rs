// water_reflective.rs — Sprint 82
//
// Upgrades the water plane with convincing reflective/refractive appearance
// and adds shoreline foam, quality-gated to Medium+.
//
// ── Reflection approach (FAKE — no custom shader) ────────────────────────────
// Uses StandardMaterial with:
//   - reflectance = 0.7 (strong specular)
//   - perceptual_roughness = 0.06 (nearly mirror smooth on High)
//   - metallic = 0.0 (dielectric Fresnel — correct for water)
//   - alpha_mode = Blend, base_color blue-green tint (A=0.75)
//   - normal_map_texture = procedural water normal (Medium+)
//   - emissive: subtle blue-white glint to fake sky reflections on Low
//
// UV scroll is driven by animate_water_uvs which offsets a UV attribute on
// the WaterMesh each frame, simulating the normal map scrolling across the
// surface.  Ripple direction follows WindState (X/Z components).
//
// ── Shoreline foam ───────────────────────────────────────────────────────────
// At Startup (Medium+), we sample terrain_height_at on a 60×60 grid across
// the 720 m playable area and spawn a thin horizontal quad for each sample
// where terrain height falls within FOAM_BAND m of WATER_LEVEL.  Each quad
// is placed at WATER_LEVEL + 0.02 m and uses a near-white emissive material
// to fake a frothy waterline.  Low tier: no foam.
//
// ── Quality gating ───────────────────────────────────────────────────────────
// Low    → simple flat blue plane, no normal map, no foam
// Medium → normal-map texture (scrolling), reflectance=0.7, foam quads
// High   → same + slightly lower roughness
//
// Public API
//   WaterReflectivePlugin

use bevy::prelude::*;

use crate::graphics_quality::GraphicsQuality;
use crate::terrain::terrain_height_at;
use crate::water::WATER_LEVEL;
use crate::water_textures::WaterTextures;
use crate::wind::WindState;

// ── Constants ─────────────────────────────────────────────────────────────────

/// Shoreline foam band half-width in metres relative to WATER_LEVEL.
const FOAM_BAND: f32 = 1.8;

/// Grid resolution for the shoreline foam scan.
const FOAM_GRID: usize = 60;

/// World-space extent of the foam scan area (matches terrain SIZE).
const FOAM_AREA: f32 = 720.0;

/// Foam quad half-width in metres.
const FOAM_QUAD_HALF: f32 = 6.0;

/// Foam quad height above WATER_LEVEL (thin sliver above the waterplane).
const FOAM_Y_OFFSET: f32 = 0.04;

// ── Components ────────────────────────────────────────────────────────────────

/// Marker on the water entity so we can reach its material handle.
#[derive(Component)]
pub struct WaterSurface;

// ── Plugin ────────────────────────────────────────────────────────────────────

pub struct WaterReflectivePlugin;

impl Plugin for WaterReflectivePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostStartup, (setup_water_material, spawn_shoreline_foam))
           .add_systems(Update, animate_water_uvs.after(crate::water::animate_water));
    }
}

// ── Startup: upgrade the water material ──────────────────────────────────────

fn setup_water_material(
    mut commands: Commands,
    quality: Res<GraphicsQuality>,
    water_textures: Option<Res<WaterTextures>>,
    // Query WaterMesh from water.rs — it carries Mesh3d + MeshMaterial3d.
    water_q: Query<(Entity, &MeshMaterial3d<StandardMaterial>), With<crate::water::WaterMesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let Ok((entity, mat_handle)) = water_q.single() else {
        warn!("water_reflective: WaterMesh not found in PostStartup");
        return;
    };

    let Some(mat) = materials.get_mut(&mat_handle.0) else {
        warn!("water_reflective: WaterMesh material not found");
        return;
    };

    match *quality {
        GraphicsQuality::Low => {
            // Low: simple flat, cheap.  Just ensure alpha blend is set.
            mat.base_color        = Color::srgba(0.10, 0.28, 0.55, 0.65);
            mat.alpha_mode        = AlphaMode::Blend;
            mat.perceptual_roughness = 0.40;
            mat.metallic          = 0.0;
            mat.reflectance       = 0.3;
            // Subtle emissive fake-sky glint even on Low.
            mat.emissive          = LinearRgba::new(0.02, 0.05, 0.10, 1.0);
        }
        GraphicsQuality::Medium | GraphicsQuality::High => {
            // Medium/High: high-quality reflective water.
            mat.base_color        = Color::srgba(0.06, 0.22, 0.48, 0.72);
            mat.alpha_mode        = AlphaMode::Blend;
            mat.perceptual_roughness = if *quality == GraphicsQuality::High {
                0.06
            } else {
                0.10
            };
            mat.metallic          = 0.0;
            // reflectance=0.7 pushes the specular F0 well above dielectric
            // default (0.5), giving a noticeably mirror-like surface under
            // directional sunlight while keeping Fresnel correct (metallic=0).
            mat.reflectance       = 0.7;
            mat.double_sided      = true;
            mat.cull_mode         = None;
            // Subtle blue-white emissive to fake sky reflection base.
            mat.emissive          = LinearRgba::new(0.04, 0.08, 0.18, 1.0);

            // Assign the procedural normal map when the texture resource is ready.
            if let Some(ref textures) = water_textures {
                mat.normal_map_texture = Some(textures.normal_map.clone());
            }
        }
    }

    // Tag the entity so animate_water_uvs can find it.
    commands.entity(entity).insert(WaterSurface);

    info!("water_reflective: water material upgraded for {:?} quality", *quality);
}

// ── Update: scroll UV coordinates to animate the normal map ──────────────────

/// On Medium+, offsets the ATTRIBUTE_UV_0 of the WaterMesh each frame so the
/// normal map appears to scroll across the surface in the wind direction.
/// This is purely a UV shift — no vertex position is changed here.
fn animate_water_uvs(
    quality: Res<GraphicsQuality>,
    time: Res<Time>,
    wind: Option<Res<WindState>>,
    water_q: Query<&Mesh3d, With<WaterSurface>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    // Low tier: no UV animation.
    if *quality == GraphicsQuality::Low {
        return;
    }

    let Ok(mesh3d) = water_q.single() else { return };
    let Some(mesh) = meshes.get_mut(&mesh3d.0) else { return };

    let t = time.elapsed_secs();

    // Derive scroll velocity from wind if available (subtle, ~0.015 UV/s max).
    let (scroll_x, scroll_z) = if let Some(ref w) = wind {
        let speed_scale = (w.speed_mps / 6.5).clamp(0.0, 1.0);
        (
            w.direction.x * speed_scale * 0.012,
            w.direction.z * speed_scale * 0.012,
        )
    } else {
        (0.008_f32, 0.005_f32)
    };

    // UV tiling scale — how many times the 256×256 texture tiles across the
    // 720 m water plane.  8 tiles gives a reasonable ripple scale.
    let tile = 8.0_f32;

    let vcount = 31_usize; // WATER_GRID(30) + 1
    let mut uvs: Vec<[f32; 2]> = Vec::with_capacity(vcount * vcount);

    for row in 0..vcount {
        for col in 0..vcount {
            let base_u = col as f32 / 30.0;
            let base_v = row as f32 / 30.0;
            // Scroll in wind direction; second layer scrolls at 60% speed at
            // a slight perpendicular offset for a more complex interference.
            let u = (base_u * tile + scroll_x * t).rem_euclid(1.0);
            let v = (base_v * tile + scroll_z * t).rem_euclid(1.0);
            uvs.push([u, v]);
        }
    }

    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
}

// ── Startup: shoreline foam quads ────────────────────────────────────────────

/// Samples terrain_height_at on a FOAM_GRID×FOAM_GRID lattice.
/// Wherever the terrain height is within FOAM_BAND metres of WATER_LEVEL,
/// spawns a small near-white emissive quad at the waterline.
///
/// Low tier: no-op.
fn spawn_shoreline_foam(
    mut commands: Commands,
    quality: Res<GraphicsQuality>,
    water_textures: Option<Res<WaterTextures>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if *quality == GraphicsQuality::Low {
        return;
    }

    // Build a shared foam material.
    let foam_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(0.95, 0.97, 1.0, 0.55),
        base_color_texture: water_textures.as_ref().map(|t| t.foam.clone()),
        alpha_mode: AlphaMode::Blend,
        // Subtle emissive so the foam glows slightly (brighter near sunlight).
        emissive: LinearRgba::new(0.25, 0.28, 0.30, 1.0),
        perceptual_roughness: 0.9,
        metallic: 0.0,
        double_sided: true,
        cull_mode: None,
        unlit: false,
        ..default()
    });

    // Flat quad mesh (XZ plane, Y=0) — thin strip lying on the water surface.
    let quad_mesh = meshes.add(build_foam_quad());

    let foam_y = WATER_LEVEL + FOAM_Y_OFFSET;
    let half   = FOAM_AREA / 2.0;
    let step   = FOAM_AREA / FOAM_GRID as f32;

    let mut spawned = 0usize;

    for gz in 0..FOAM_GRID {
        for gx in 0..FOAM_GRID {
            let wx = -half + (gx as f32 + 0.5) * step;
            let wz = -half + (gz as f32 + 0.5) * step;

            let terrain_h = terrain_height_at(wx, wz);
            // Foam appears where the terrain is just above or at the water level.
            if (terrain_h - WATER_LEVEL).abs() <= FOAM_BAND {
                commands.spawn((
                    Mesh3d(quad_mesh.clone()),
                    MeshMaterial3d(foam_mat.clone()),
                    Transform::from_translation(Vec3::new(wx, foam_y, wz)),
                ));
                spawned += 1;
            }
        }
    }

    info!("water_reflective: spawned {} shoreline foam quads", spawned);
}

// ---------------------------------------------------------------------------
// Mesh helper: flat XZ quad for foam
// ---------------------------------------------------------------------------

fn build_foam_quad() -> Mesh {
    use bevy::mesh::{Indices, PrimitiveTopology};
    use bevy::asset::RenderAssetUsages;

    let h = FOAM_QUAD_HALF;
    let positions: Vec<[f32; 3]> = vec![
        [-h, 0.0, -h],
        [ h, 0.0, -h],
        [ h, 0.0,  h],
        [-h, 0.0,  h],
    ];
    let normals: Vec<[f32; 3]> = vec![
        [0.0, 1.0, 0.0]; 4
    ];
    let uvs: Vec<[f32; 2]> = vec![
        [0.0, 0.0],
        [1.0, 0.0],
        [1.0, 1.0],
        [0.0, 1.0],
    ];
    let indices = Indices::U32(vec![0, 1, 2, 0, 2, 3]);

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(indices);
    mesh
}
