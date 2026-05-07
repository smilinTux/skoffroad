// Terrain normal map: generate high-quality per-vertex normals from the
// terrain heightmap via finite differences and write them into the terrain
// mesh's ATTRIBUTE_NORMAL slot.  The existing normals (computed during mesh
// build in terrain.rs) are replaced with normals derived directly from the
// authoritative `terrain_height_at` function, ensuring the lighting is
// consistent with the physics collider and any runtime deformation.
//
// The system runs in Update and fires exactly once (guarded by `Local<bool>`).
// It scans every mesh asset and picks the one with the most vertices as the
// terrain mesh – a simple heuristic that is robust as long as the terrain is
// by far the largest mesh in the scene (128×128 grid → 16 641 vertices).
//
// Public API:
//   TerrainNormalMapPlugin

use bevy::prelude::*;

use crate::terrain::terrain_height_at;

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct TerrainNormalMapPlugin;

impl Plugin for TerrainNormalMapPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, apply_normals_once);
    }
}

// ---------------------------------------------------------------------------
// System
// ---------------------------------------------------------------------------

/// Runs every Update frame but applies the finite-difference normals exactly
/// once, guarded by `Local<bool>`.
///
/// Algorithm for each vertex at world-space position (x, y, z):
///   - h  = y                         (height already in the mesh)
///   - hx = terrain_height_at(x+0.5, z)
///   - hz = terrain_height_at(x, z+0.5)
///   - tangent_x = Vec3(0.5, hx-h, 0.0).normalize()
///   - tangent_z = Vec3(0.0, hz-h, 0.5).normalize()
///   - normal    = tangent_x.cross(tangent_z).normalize()
fn apply_normals_once(
    mut meshes: ResMut<Assets<Mesh>>,
    mut done: Local<bool>,
) {
    if *done {
        return;
    }

    // -----------------------------------------------------------------------
    // Identify the terrain mesh: the one with the largest vertex count.
    // Use `as_float3()` to avoid matching on the private VertexAttributeValues
    // enum directly.
    // -----------------------------------------------------------------------
    let mut best_id: Option<AssetId<Mesh>> = None;
    let mut best_count: usize = 0;

    for (id, mesh) in meshes.iter() {
        let count = mesh
            .attribute(Mesh::ATTRIBUTE_POSITION)
            .and_then(|attr| attr.as_float3())
            .map(|v| v.len())
            .unwrap_or(0);

        if count > best_count {
            best_count = count;
            best_id = Some(id);
        }
    }

    let Some(terrain_id) = best_id else {
        info!("terrain_normal_map: no mesh assets found yet – skipping");
        return;
    };

    // Require a minimum size to avoid accidentally targeting a tiny mesh
    // before the terrain has been loaded (16 641 = (128+1)²).
    const MIN_TERRAIN_VERTS: usize = 1_000;
    if best_count < MIN_TERRAIN_VERTS {
        info!(
            "terrain_normal_map: largest mesh has only {} vertices – \
             terrain not ready yet, skipping",
            best_count
        );
        return;
    }

    // -----------------------------------------------------------------------
    // Read existing positions, compute new normals, write back.
    // -----------------------------------------------------------------------
    let mesh = meshes.get_mut(terrain_id).expect("mesh existed in iter");

    // Clone positions so we can mutably borrow the mesh to insert normals.
    let positions: Vec<[f32; 3]> = match mesh
        .attribute(Mesh::ATTRIBUTE_POSITION)
        .and_then(|attr| attr.as_float3())
    {
        Some(v) => v.to_vec(),
        None => {
            info!("terrain_normal_map: terrain mesh has no Float32x3 positions – skipping");
            return;
        }
    };

    let normals: Vec<[f32; 3]> = positions
        .iter()
        .map(|&[x, y, z]| {
            let h  = y;
            let hx = terrain_height_at(x + 0.5, z);
            let hz = terrain_height_at(x, z + 0.5);

            let tangent_x = Vec3::new(0.5, hx - h, 0.0).normalize();
            let tangent_z = Vec3::new(0.0, hz - h, 0.5).normalize();
            let normal    = tangent_x.cross(tangent_z).normalize();

            [normal.x, normal.y, normal.z]
        })
        .collect();

    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);

    info!(
        "terrain_normal_map: applied finite-difference normals to {} vertices",
        best_count
    );

    *done = true;
}
