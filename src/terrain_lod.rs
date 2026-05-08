// Terrain LOD: distance-based skirt rings that visually extend the finite
// terrain past its 200 m playable boundary, hiding the hard edge.
//
// Two rings are spawned at startup as static low-poly meshes (no colliders):
//
//   Inner skirt — 100 m..200 m radius, 16×16 grid, grass-green tint.
//   Outer skirt — 200 m..600 m radius,  8×8  grid, atmospheric blue/gray.
//
// Both use procedural TriangleList meshes built from polar-annulus quads.
// Heights for the inner skirt are sampled from `terrain_height_at` so the
// geometry blends smoothly with the playable terrain edge; the outer skirt
// is flat at Y=0 (at that distance the height difference is imperceptible).
//
// Public API:
//   TerrainLodPlugin
//   TerrainSkirt  (marker component)

use bevy::{
    asset::RenderAssetUsages,
    mesh::{Indices, PrimitiveTopology},
    prelude::*,
};

use crate::terrain::terrain_height_at;

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct TerrainLodPlugin;

impl Plugin for TerrainLodPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_skirts);
    }
}

// ---------------------------------------------------------------------------
// Marker component
// ---------------------------------------------------------------------------

/// Marker attached to every terrain-skirt entity spawned by this module.
#[derive(Component)]
pub struct TerrainSkirt;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Number of angular subdivisions for each ring.
/// 64 gives smooth round edges without wasting triangles at distance.
const ANGULAR_STEPS: usize = 64;

/// Inner skirt: 100 m → 200 m, 16 radial bands.
const INNER_R_MIN:  f32   = 100.0;
const INNER_R_MAX:  f32   = 200.0;
const INNER_BANDS:  usize = 16;

/// Outer skirt: 200 m → 600 m, 8 radial bands.
const OUTER_R_MIN:  f32   = 200.0;
const OUTER_R_MAX:  f32   = 600.0;
const OUTER_BANDS:  usize = 8;

/// Grass-green tint for the inner skirt (matches terrain.rs GRASS colour).
const INNER_COLOR: Color = Color::srgb(0.32, 0.50, 0.20);

/// Faded atmospheric blue/gray for the outer skirt.
const OUTER_COLOR: Color = Color::srgb(0.55, 0.62, 0.70);

// ---------------------------------------------------------------------------
// Startup system
// ---------------------------------------------------------------------------

fn spawn_skirts(
    mut commands:  Commands,
    mut meshes:    ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Inner skirt ----------------------------------------------------------------
    {
        let mesh   = build_skirt_mesh(INNER_R_MIN, INNER_R_MAX, INNER_BANDS, true);
        let handle = meshes.add(mesh);
        let mat    = materials.add(StandardMaterial {
            base_color:           INNER_COLOR,
            perceptual_roughness: 0.9,
            ..default()
        });
        commands.spawn((
            TerrainSkirt,
            Mesh3d(handle),
            MeshMaterial3d(mat),
            Transform::default(),
        ));
    }

    // Outer skirt ----------------------------------------------------------------
    {
        let mesh   = build_skirt_mesh(OUTER_R_MIN, OUTER_R_MAX, OUTER_BANDS, false);
        let handle = meshes.add(mesh);
        let mat    = materials.add(StandardMaterial {
            base_color:           OUTER_COLOR,
            perceptual_roughness: 0.95,
            ..default()
        });
        commands.spawn((
            TerrainSkirt,
            Mesh3d(handle),
            MeshMaterial3d(mat),
            Transform::default(),
        ));
    }
}

// ---------------------------------------------------------------------------
// Procedural mesh builder
// ---------------------------------------------------------------------------

/// Build a polar-annulus mesh spanning `r_min`..`r_max` around the origin,
/// subdivided into `radial_bands` rings × `ANGULAR_STEPS` sectors.
///
/// When `sample_heights` is true the inner ring edge queries `terrain_height_at`
/// for a smooth blend at the terrain boundary; otherwise Y is always 0.
fn build_skirt_mesh(
    r_min:         f32,
    r_max:         f32,
    radial_bands:  usize,
    sample_heights: bool,
) -> Mesh {
    let rings   = radial_bands + 1;  // vertex rings (one more than quad bands)
    let sectors = ANGULAR_STEPS + 1; // vertex columns (one more than sector quads,
                                     // first == last angle for closed seam)

    let vert_count = rings * sectors;
    let mut positions: Vec<[f32; 3]> = Vec::with_capacity(vert_count);
    let mut normals:   Vec<[f32; 3]> = Vec::with_capacity(vert_count);
    let mut uvs:       Vec<[f32; 2]> = Vec::with_capacity(vert_count);

    for ri in 0..rings {
        let t = ri as f32 / radial_bands as f32;         // 0..=1 from inner to outer
        let r = r_min + t * (r_max - r_min);

        for si in 0..sectors {
            let angle = (si as f32 / ANGULAR_STEPS as f32) * std::f32::consts::TAU;
            let px = angle.cos() * r;
            let pz = angle.sin() * r;

            // Heights: sample terrain only at the inner boundary ring when
            // requested; elsewhere stay flat to avoid visible discontinuities
            // in the (already low-detail) outer bands.
            let py = if sample_heights && ri == 0 {
                terrain_height_at(px, pz)
            } else {
                0.0
            };

            positions.push([px, py, pz]);
            normals.push([0.0, 1.0, 0.0]);
            uvs.push([t, si as f32 / ANGULAR_STEPS as f32]);
        }
    }

    // Indices — two triangles per quad, wound CCW from above.
    let quad_count = radial_bands * ANGULAR_STEPS;
    let mut indices: Vec<u32> = Vec::with_capacity(quad_count * 6);

    for ri in 0..radial_bands {
        for si in 0..ANGULAR_STEPS {
            let tl = (ri       * sectors + si    ) as u32;
            let tr = (ri       * sectors + si + 1) as u32;
            let bl = ((ri + 1) * sectors + si    ) as u32;
            let br = ((ri + 1) * sectors + si + 1) as u32;
            // CCW winding: tl → bl → tr, then tr → bl → br
            indices.extend_from_slice(&[tl, bl, tr, tr, bl, br]);
        }
    }

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL,   normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0,     uvs);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}
