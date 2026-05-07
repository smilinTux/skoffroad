// Terrain splatmap: per-vertex color blending across grass / dirt / rock based
// on slope and elevation. Runs once after the terrain mesh is built and injects
// (or replaces) ATTRIBUTE_COLOR directly into the mesh asset.
//
// The terrain mesh already carries vertex colors from terrain.rs; this system
// supersedes them with the PRD v3.1 Sprint-38 palette and adds snow-tip
// blending for high elevations.
//
// Public API:
//   TerrainSplatmapPlugin

use bevy::prelude::*;
use bevy::mesh::VertexAttributeValues;
use crate::terrain::terrain_height_at;

pub struct TerrainSplatmapPlugin;

// Minimum vertex count used to identify the terrain mesh when no marker
// component is available.  The terrain grid is (128+1)^2 = 16,641 vertices.
const TERRAIN_MIN_VERTICES: usize = 5_000;

// Slope thresholds (dot(normal, Y) mapped to 0..1 slope value).
const GRASS_SLOPE_LO: f32 = 0.10;
const GRASS_SLOPE_HI: f32 = 0.25;
const DIRT_SLOPE_LO:  f32 = 0.30;
const DIRT_SLOPE_HI:  f32 = 0.55;

// PRD v3.1 palette (linear sRGB).
const GRASS: [f32; 3] = [0.20, 0.55, 0.25];
const DIRT:  [f32; 3] = [0.50, 0.40, 0.25];
const ROCK:  [f32; 3] = [0.42, 0.40, 0.38];
const SNOW:  [f32; 3] = [0.95, 0.95, 0.95];

// Elevation above which snow blending begins, and where it is full.
const SNOW_LO: f32 = 8.0;
const SNOW_HI: f32 = 14.0;

// Step (metres) used for finite-difference slope estimation when no normals are
// available on a given vertex (should not happen, but makes the code robust).
const FD_STEP: f32 = 1.5;

impl Plugin for TerrainSplatmapPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, apply_splatmap_once);
    }
}

/// Runs every frame until it successfully locates and patches the terrain mesh,
/// then sets `done = true` and becomes a no-op for the rest of the session.
fn apply_splatmap_once(
    mut done: Local<bool>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    if *done {
        return;
    }

    // Scan all mesh assets for the terrain (identified by vertex count).
    let mut patched = false;
    for (_id, mesh) in meshes.iter_mut() {
        let positions = match mesh.attribute(Mesh::ATTRIBUTE_POSITION) {
            Some(attr) => attr,
            None => continue,
        };

        // Downcast to the concrete Vec<[f32; 3]> representation.
        let positions: Vec<[f32; 3]> = match positions {
            VertexAttributeValues::Float32x3(v) => v.clone(),
            _ => continue,
        };

        if positions.len() < TERRAIN_MIN_VERTICES {
            continue;
        }

        // Optionally read existing normals; fall back to finite-difference
        // slope estimation if they are absent.
        let normals: Option<Vec<[f32; 3]>> =
            mesh.attribute(Mesh::ATTRIBUTE_NORMAL).and_then(|attr| {
                if let VertexAttributeValues::Float32x3(v) = attr {
                    Some(v.clone())
                } else {
                    None
                }
            });

        let n = positions.len();
        let mut colors: Vec<[f32; 4]> = Vec::with_capacity(n);

        for (i, &[px, py, pz]) in positions.iter().enumerate() {
            // --- Slope ---
            // Prefer pre-computed normals; fall back to finite-difference.
            let slope = if let Some(ref norms) = normals {
                let [nx, ny, nz] = norms[i];
                let normal = Vec3::new(nx, ny, nz).normalize_or_zero();
                1.0 - normal.dot(Vec3::Y).clamp(0.0, 1.0)
            } else {
                let h  = terrain_height_at(px, pz);
                let hx = terrain_height_at(px + FD_STEP, pz);
                let hz = terrain_height_at(px, pz + FD_STEP);
                let nx_v = Vec3::new(FD_STEP, hx - h, 0.0).normalize_or_zero();
                let nz_v = Vec3::new(0.0, hz - h, FD_STEP).normalize_or_zero();
                let n_vec = nx_v.cross(nz_v).normalize_or_zero();
                1.0 - n_vec.dot(Vec3::Y).clamp(0.0, 1.0)
            };

            // --- Slope blend: grass -> dirt -> rock ---
            let t_gd = smooth_step(slope, GRASS_SLOPE_LO, GRASS_SLOPE_HI);
            let t_dr = smooth_step(slope, DIRT_SLOPE_LO, DIRT_SLOPE_HI);
            let base = lerp3(lerp3(GRASS, DIRT, t_gd), ROCK, t_dr);

            // --- Elevation blend: high points -> snow ---
            let t_snow = smooth_step(py, SNOW_LO, SNOW_HI);
            let c = lerp3(base, SNOW, t_snow);

            colors.push([c[0], c[1], c[2], 1.0]);
        }

        let vertex_count = colors.len();
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
        patched = true;
        info!("terrain_splatmap: vertex colors applied ({} vertices)", vertex_count);
        // Patch only the first qualifying mesh (the terrain).
        break;
    }

    if patched {
        *done = true;
    }
    // If not patched: terrain mesh not yet in Assets<Mesh> — silent retry next frame.
}

// ---------------------------------------------------------------------------
// Colour helpers
// ---------------------------------------------------------------------------

/// Smooth cubic ease (Ken Perlin's smoothstep) mapping `x` in [lo, hi] to
/// [0, 1] with zero derivative at both ends.
#[inline]
fn smooth_step(x: f32, lo: f32, hi: f32) -> f32 {
    let t = ((x - lo) / (hi - lo)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Linear interpolate between two linear-sRGB RGB triples.
#[inline]
fn lerp3(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}
