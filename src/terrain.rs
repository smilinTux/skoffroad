use bevy::{
    asset::RenderAssetUsages,
    mesh::{Indices, PrimitiveTopology},
    prelude::*,
};
use avian3d::prelude::*;
use noise::{NoiseFn, Perlin, Fbm};

use crate::graphics_quality::GraphicsQuality;
use crate::terrain_detail_tex::TerrainDetailTex;

pub struct TerrainPlugin;

impl Plugin for TerrainPlugin {
    fn build(&self, app: &mut App) {
        // Spawn after Startup so TerrainPbrPlugin's `load_terrain_assets`
        // has a chance to populate `TerrainPbrAssets::material`.
        app.add_systems(PostStartup, spawn_terrain);
    }
}

/// Marker component on the procedurally-generated terrain entity.
/// Custom-heightmap terrain swaps this out at runtime by despawning the entity
/// carrying this component and spawning a new one.
#[derive(Component)]
pub struct ProceduralTerrainMarker;

// Enlarged from 200 m to 720 m so the real terrain mesh + collider actually
// contain every gameplay area. The hillclimb (Z -180..-240), rock crawl
// (X 120, Z -120), obstacle course (Z +200..+260), trail rides, and the
// landmark props were all placed OUTSIDE the old 200 m terrain — so
// fast-travelling to them dropped the truck onto coordinates with no collider
// underneath (it floated over the void and couldn't get traction). 720 m
// (±360) covers all of them with margin. GRID bumped to keep ~3.75 m quads so
// the stretched terrain still drives well; the trimesh collider scales with it.
// GRID is now tier-scaled. The per-tier values come from GraphicsQuality:
//   High   → 192 vertices per side (~74 k tris)
//   Medium → 144 vertices per side (~41 k tris)
//   Low    →  96 vertices per side (~18 k tris)
// The module-level constant is only used by terrain_height_at (pure maths,
// no mesh) which always resamples the noise at any (x, z), so it is
// resolution-independent. Runtime mesh building reads quality.terrain_grid().
const SIZE: f32 = 720.0; // world-space width/depth in metres (spans [-360, +360])
const HEIGHT_SCALE: f32 = 12.0;
pub const TERRAIN_SEED: u32 = 42;

// Sample the layered heightmap at normalised [0,1] coords.
fn sample_height(fbm: &Fbm<Perlin>, nx: f64, nz: f64) -> f32 {
    // Two octave frequencies for gentle rolling hills plus fine detail.
    let coarse = fbm.get([nx * 2.0, nz * 2.0]) as f32;
    let fine   = fbm.get([nx * 8.0, nz * 8.0]) as f32 * 0.25;
    (coarse + fine) * HEIGHT_SCALE
}

/// Public helper used by the headless harness to replicate the terrain height
/// at an arbitrary world-space (x, z) position without spawning any entities.
pub fn terrain_height_at(x: f32, z: f32) -> f32 {
    let fbm: Fbm<Perlin> = Fbm::<Perlin>::new(TERRAIN_SEED);
    let nx = (x / SIZE + 0.5) as f64;
    let nz = (z / SIZE + 0.5) as f64;
    sample_height(&fbm, nx, nz)
}

fn spawn_terrain(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    quality: Res<GraphicsQuality>,
    detail_tex: Option<Res<TerrainDetailTex>>,
) {
    // Tier-scaled grid resolution: High=192, Medium=144, Low=96.
    let grid: usize = quality.terrain_grid();

    let fbm: Fbm<Perlin> = Fbm::<Perlin>::new(42);

    let vcount = grid + 1; // vertices per edge
    let mut positions: Vec<[f32; 3]> = Vec::with_capacity(vcount * vcount);
    let mut normals:   Vec<[f32; 3]> = Vec::with_capacity(vcount * vcount);
    let mut uvs:       Vec<[f32; 2]> = Vec::with_capacity(vcount * vcount);
    let mut colors:    Vec<[f32; 4]> = Vec::with_capacity(vcount * vcount);

    // Height values kept separately for collider construction.
    let mut heights: Vec<f32> = Vec::with_capacity(vcount * vcount);

    for z in 0..vcount {
        for x in 0..vcount {
            let nx = x as f64 / grid as f64;
            let nz = z as f64 / grid as f64;
            let h = sample_height(&fbm, nx, nz);

            let px = (x as f32 / grid as f32 - 0.5) * SIZE;
            let pz = (z as f32 / grid as f32 - 0.5) * SIZE;

            positions.push([px, h, pz]);
            normals.push([0.0, 1.0, 0.0]); // overwritten below
            uvs.push([nx as f32 * 8.0, nz as f32 * 8.0]);
            heights.push(h);
        }
    }

    // Smooth normals via finite differences.
    for z in 0..vcount {
        for x in 0..vcount {
            let h  = heights[z * vcount + x];
            let hx = if x + 1 < vcount { heights[z * vcount + x + 1] } else { h };
            let hz = if z + 1 < vcount { heights[(z + 1) * vcount + x] } else { h };
            let step = SIZE / grid as f32;
            let nx_v = Vec3::new(step, hx - h, 0.0).normalize();
            let nz_v = Vec3::new(0.0, hz - h, step).normalize();
            let n = nx_v.cross(nz_v).normalize();
            normals[z * vcount + x] = [n.x, n.y, n.z];
        }
    }

    // Improved vertex color blend — slope AND height give 5 distinct biomes:
    //
    //   sand   (low elevation, gentle slope) — sandy flats
    //   grass  (mid elevation, gentle slope) — green rolling hills
    //   dirt   (any elevation, moderate slope) — exposed bare earth
    //   rock   (any elevation, steep slope) — grey granite/shale
    //   snow   (high elevation, gentle/moderate slope) — white snow cap
    //
    // All transitions are cubic smoothstep so there are no hard colour bands.
    // The palette is tuned in linear sRGB to match the daylight post-FX
    // (TonyMcMapface tonemap + SSAO) introduced in v0.31.1.
    const SAND:  [f32; 3] = [0.72, 0.63, 0.43]; // sandy/dusty flats
    const GRASS: [f32; 3] = [0.22, 0.48, 0.15]; // mossy meadow green
    const DIRT:  [f32; 3] = [0.48, 0.36, 0.22]; // warm reddish-brown earth
    const ROCK:  [f32; 3] = [0.39, 0.39, 0.41]; // cool grey granite
    const SNOW:  [f32; 3] = [0.92, 0.93, 0.95]; // slightly blue-tinted snow

    // Minimum height of the terrain across the grid (approximate).  Used to
    // normalise the height value into a 0..1 elevation factor.  HEIGHT_SCALE
    // controls the amplitude; the Fbm output sits roughly in [-1, +1] so the
    // real range is about [-HEIGHT_SCALE, +HEIGHT_SCALE].
    let h_lo = -HEIGHT_SCALE;
    let h_hi =  HEIGHT_SCALE;

    for i in 0..(vcount * vcount) {
        let [nx, ny, nz] = normals[i];
        let normal = Vec3::new(nx, ny, nz);
        // slope = 0 on flat ground (normal points straight up), 1 on vertical.
        let slope = 1.0 - normal.dot(Vec3::Y).clamp(0.0, 1.0);

        let height = positions[i][1]; // world-space Y
        // Normalised elevation in [0, 1].
        let elev = ((height - h_lo) / (h_hi - h_lo)).clamp(0.0, 1.0);

        // ---- slope blends -----------------------------------------------
        // grass -> dirt  0.08..0.22
        let t_gd = slope_smooth_step(slope, 0.08, 0.22);
        // dirt  -> rock  0.28..0.52
        let t_dr = slope_smooth_step(slope, 0.28, 0.52);

        // Slope-derived base colour (identical for all elevations).
        let slope_col = lerp3(lerp3(GRASS, DIRT, t_gd), ROCK, t_dr);

        // ---- elevation blends -------------------------------------------
        // Low flats (elev 0..0.30): blend SAND into the slope colour.
        // Sandy colour is suppressed on steep slopes so sand doesn't appear
        // on cliff faces; we multiply the sand blend by (1 - slope*2).
        let sand_weight = slope_smooth_step(1.0 - slope * 2.0, 0.0, 1.0)
            .min(1.0)
            .max(0.0);
        let t_sand = slope_smooth_step(1.0 - elev, 0.70, 1.0) * sand_weight;

        // High peaks (elev 0.70..1.0): blend SNOW.  Steep cliffs stay rocky
        // (rock already dominates at high slope), so apply only when slope < 0.40.
        let snow_on_slope = 1.0 - slope_smooth_step(slope, 0.25, 0.40);
        let t_snow = slope_smooth_step(elev, 0.72, 1.0) * snow_on_slope;

        // Combine: start from slope colour, overlay sand at low elev, snow at high.
        let mid = lerp3(slope_col, SAND, t_sand);
        let c   = lerp3(mid, SNOW, t_snow);

        colors.push([c[0], c[1], c[2], 1.0]);
    }

    let mut indices: Vec<u32> = Vec::with_capacity(grid * grid * 6);
    for z in 0..grid {
        for x in 0..grid {
            let tl = (z * vcount + x) as u32;
            let tr = tl + 1;
            let bl = ((z + 1) * vcount + x) as u32;
            let br = bl + 1;
            // Two triangles per quad.
            indices.extend_from_slice(&[tl, bl, tr, tr, bl, br]);
        }
    }

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh.insert_indices(Indices::U32(indices));

    let mesh_handle = meshes.add(mesh);

    // Sprint 79: improved terrain PBR material.
    //
    // Low tier  — plain vertex-color material, very cheap.
    //             High roughness (terrain is never shiny), low reflectance.
    //
    // Medium+   — additionally applies the procedural tiling detail-normal
    //             generated by TerrainDetailTexPlugin.  The normal map adds
    //             sub-quad surface texture (pebbles/slabs) that reads well
    //             with SSAO + directional shadows at no extra geometry cost.
    //             The triplanar asset-server path is kept for future use but
    //             is gated behind `triplanar_terrain()` as before.
    //
    // `base_color WHITE` so vertex colors (ATTRIBUTE_COLOR) are passed through
    // unmodified — Bevy 0.18 multiplies base_color × vertex_color automatically.
    // `reflectance 0.2` keeps specular very low (real dirt reflects ~4 % light).
    let material = if quality.triplanar_terrain() {
        // Asset-server textures (Medium+ with real dirt PBR pack).
        let mut mat = StandardMaterial {
            base_color: Color::WHITE,
            base_color_texture: Some(asset_server.load("materials/terrain/dirt/albedo.jpg")),
            normal_map_texture: Some(asset_server.load("materials/terrain/dirt/normal.jpg")),
            metallic_roughness_texture: Some(
                asset_server.load("materials/terrain/dirt/roughness.jpg"),
            ),
            perceptual_roughness: 0.92,
            reflectance: 0.18,
            metallic: 0.0,
            ..default()
        };
        // Overlay the procedural detail-normal on top of the asset-server normal
        // only if no normal is already set and the resource is ready.
        // (In practice the asset-server normal wins; this is a safe guard.)
        if mat.normal_map_texture.is_none() {
            if let Some(ref tex) = detail_tex {
                mat.normal_map_texture = Some(tex.normal_map.clone());
            }
        }
        materials.add(mat)
    } else {
        // Vertex-color only path — used on Low and as headless fallback.
        // Apply the procedural detail-normal on Medium quality even when
        // triplanar is disabled (quality.triplanar_terrain() == false on Low).
        // The detail_tex resource may be None in headless; guard it.
        let detail_normal = if !matches!(*quality, GraphicsQuality::Low) {
            detail_tex.as_ref().map(|t| t.normal_map.clone())
        } else {
            None
        };
        materials.add(StandardMaterial {
            base_color: Color::WHITE,
            perceptual_roughness: 0.92,
            reflectance: 0.18,
            metallic: 0.0,
            normal_map_texture: detail_normal,
            ..default()
        })
    };

    commands.spawn((
        Mesh3d(mesh_handle.clone()),
        MeshMaterial3d(material),
        Transform::default(),
        RigidBody::Static,
        ColliderConstructor::TrimeshFromMesh,
        ProceduralTerrainMarker,
    ));
}

// ---------------------------------------------------------------------------
// Colour helpers (used only during mesh build, no runtime cost)
// ---------------------------------------------------------------------------

/// Smooth cubic ease mapping a value in [lo, hi] to [0, 1].
fn slope_smooth_step(x: f32, lo: f32, hi: f32) -> f32 {
    let t = ((x - lo) / (hi - lo)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Linear interpolate between two RGB triples.
fn lerp3(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}
