use bevy::{
    prelude::*,
    render::{mesh::Indices, render_resource::PrimitiveTopology},
};
use crate::terrain::{TerrainChunk, TerrainType};
use std::f32::consts::PI;

/// Terrain mesh generation and mesh-only chunk spawner utilities.
///
/// - `spawn_terrain_chunk` (in chunk.rs/mod.rs): ECS-based, spawns full terrain chunk entities with all components and data.
/// - `spawn_lod_chunk_mesh` (here): Mesh-only, for LOD/visualization, does not create ECS terrain chunk components.

/// ECS-based chunk spawner vs. mesh-only/LOD chunk spawner:
/// - ECS-based chunk spawner (spawn_terrain_chunk) creates full terrain chunk entities with all components and data for gameplay.
/// - Mesh-only/LOD chunk spawner (spawn_lod_chunk_mesh) creates meshes for visualization/LOD purposes only, not full ECS entities.

#[derive(Component)]
pub struct TerrainMeshMarker {
    pub chunk_position: IVec2,
    pub lod_level: u32,
    pub edge_morph_factors: [f32; 4], // [left, right, top, bottom]
}

pub fn create_terrain_mesh(
    chunk: &TerrainChunk,
    lod_level: u32,
    neighbor_lods: Option<[u32; 4]>, // [left, right, top, bottom]
) -> Mesh {
    let size = chunk.size as usize;
    let vertex_count = size * size;
    let mut positions = Vec::with_capacity(vertex_count);
    let mut normals: Vec<[f32; 3]> = Vec::with_capacity(vertex_count);
    let mut uvs = Vec::with_capacity(vertex_count);
    let mut tangents = Vec::with_capacity(vertex_count);
    
    let step = 1 << lod_level; // Vertex skip factor for LOD
    
    // Generate vertex data with LOD
    for z in (0..size).step_by(step) {
        for x in (0..size).step_by(step) {
            // Calculate morph factor for LOD transition
            let morph_factor = if let Some(neighbor_lods) = neighbor_lods {
                calculate_morph_factor(x as u32, z as u32, size as u32, lod_level, neighbor_lods)
            } else {
                0.0
            };
            // Position with LOD morphing
            let height = chunk.get_height(x, z);
            let morph_height = if morph_factor > 0.0 {
                let next_x = (x + step).min(size - 1);
                let next_z = (z + step).min(size - 1);
                let h00 = height;
                let h10 = chunk.get_height(next_x, z);
                let h01 = chunk.get_height(x, next_z);
                let h11 = chunk.get_height(next_x, next_z);
                lerp(height, (h00 + h10 + h01 + h11) * 0.25, morph_factor)
            } else {
                height
            };
            let pos = Vec3::new(
                x as f32,
                morph_height,
                z as f32,
            );
            positions.push([pos.x, pos.y, pos.z]);
            // UV coordinates
            let uv = Vec2::new(
                x as f32 / (size - 1) as f32,
                z as f32 / (size - 1) as f32,
            );
            uvs.push([uv.x, uv.y]);
            // Calculate tangent vectors for normal mapping
            let tangent = calculate_tangent(x as u32, z as u32, chunk);
            tangents.push([tangent.x, tangent.y, tangent.z, tangent.w]);
        }
    }
    // Generate indices for triangles with LOD
    let mut indices = Vec::new();
    let lod_size = (size + step - 1) / step;
    for z in 0..(lod_size - 1) {
        for x in 0..(lod_size - 1) {
            let top_left = z * lod_size + x;
            let top_right = top_left + 1;
            let bottom_left = (z + 1) * lod_size + x;
            let bottom_right = bottom_left + 1;
            // First triangle
            indices.extend_from_slice(&[
                top_left as u32,
                bottom_left as u32,
                top_right as u32,
            ]);
            // Second triangle
            indices.extend_from_slice(&[
                bottom_left as u32,
                bottom_right as u32,
                top_right as u32,
            ]);
        }
    }
    // Calculate normals with consideration for terrain features
    let mut normals_vec = vec![Vec3::ZERO; positions.len()];
    for chunk in indices.chunks(3) {
        let [i1, i2, i3] = [chunk[0] as usize, chunk[1] as usize, chunk[2] as usize];
        let pos1 = Vec3::from(positions[i1]);
        let pos2 = Vec3::from(positions[i2]);
        let pos3 = Vec3::from(positions[i3]);
        let normal = (pos2 - pos1).cross(pos3 - pos1).normalize();
        // Add normal contribution with feature weighting
        let slope = normal.dot(Vec3::Y).abs();
        let weight = calculate_feature_weight(slope);
        normals_vec[i1] += normal * weight;
        normals_vec[i2] += normal * weight;
        normals_vec[i3] += normal * weight;
    }
    // Normalize all normals
    for normal in &mut normals_vec {
        *normal = normal.normalize();
    }
    // Convert normals to arrays for mesh
    let normals: Vec<[f32; 3]> = normals_vec.iter()
        .map(|n| [n.x, n.y, n.z])
        .collect();
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_attribute(Mesh::ATTRIBUTE_TANGENT, tangents);
    mesh.set_indices(Some(Indices::U32(indices)));
    mesh
}

fn calculate_morph_factor(x: u32, z: u32, size: u32, lod: u32, neighbor_lods: [u32; 4]) -> f32 {
    let border_size = 2 << lod;
    let mut factor = 0.0;
    
    // Check if we're near a chunk border
    if x < border_size && neighbor_lods[0] < lod {
        factor = 1.0 - (x as f32 / border_size as f32);
    } else if x > size - border_size && neighbor_lods[1] < lod {
        factor = 1.0 - ((size - x) as f32 / border_size as f32);
    }
    
    if z < border_size && neighbor_lods[2] < lod {
        factor = factor.max(1.0 - (z as f32 / border_size as f32));
    } else if z > size - border_size && neighbor_lods[3] < lod {
        factor = factor.max(1.0 - ((size - z) as f32 / border_size as f32));
    }
    
    factor
}

fn calculate_tangent(x: u32, z: u32, chunk: &TerrainChunk) -> Vec4 {
    let size = chunk.size as usize;
    let x_next = (x as usize + 1).min(size - 1);
    let z_next = (z as usize + 1).min(size - 1);
    let h00 = chunk.get_height(x as usize, z as usize);
    let h10 = chunk.get_height(x_next, z as usize);
    let h01 = chunk.get_height(x as usize, z_next);
    let tangent = Vec3::new(1.0, h10 - h00, 0.0).normalize();
    let bitangent = Vec3::new(0.0, h01 - h00, 1.0).normalize();
    let normal = tangent.cross(bitangent);
    Vec4::new(tangent.x, tangent.y, tangent.z, 1.0)
}

fn calculate_feature_weight(slope: f32) -> f32 {
    // Enhance normal contribution for steep slopes (cliffs)
    if slope < 0.3 { // Steep slope threshold
        2.0 // Increase normal influence for cliffs
    } else {
        1.0 // Normal weight for regular terrain
    }
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Spawns a mesh for a terrain chunk for LOD/visualization purposes only.
/// This does NOT create a full ECS terrain chunk entity.
///
/// Args:
///     commands: ECS Commands
///     meshes: Mesh asset storage
///     materials: StandardMaterial asset storage
///     chunk: TerrainChunk reference
///     lod_level: Level of detail
///     neighbor_lods: Optional neighbor LODs for edge morphing
///
/// Returns:
///     Entity: The spawned mesh-only chunk entity
pub fn spawn_lod_chunk_mesh(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    chunk: &TerrainChunk,
    lod_level: u32,
    neighbor_lods: Option<[u32; 4]>,
) -> Entity {
    let mesh = create_terrain_mesh(chunk, lod_level, neighbor_lods);
    
    // Create material based on terrain type
    let terrain_type = determine_terrain_type(chunk);
    let material = StandardMaterial {
        base_color: terrain_type.get_color(),
        perceptual_roughness: terrain_type.get_roughness(),
        metallic: 0.0,
        reflectance: 0.2,
        ..default()
    };

    commands.spawn((
        PbrBundle {
            mesh: meshes.add(mesh),
            material: materials.add(material),
            transform: Transform::from_xyz(
                chunk.position.x as f32 * chunk.size as f32,
                0.0,
                chunk.position.y as f32 * chunk.size as f32,
            ),
            ..default()
        },
        TerrainMeshMarker {
            chunk_position: IVec2::new(chunk.position.x as i32, chunk.position.y as i32),
            lod_level,
            edge_morph_factors: calculate_edge_morph_factors(chunk, lod_level, neighbor_lods),
        },
    )).id()
}

fn calculate_edge_morph_factors(
    chunk: &TerrainChunk,
    lod_level: u32,
    neighbor_lods: Option<[u32; 4]>,
) -> [f32; 4] {
    if let Some(lods) = neighbor_lods {
        lods.map(|neighbor_lod| {
            if neighbor_lod < lod_level {
                1.0
            } else {
                0.0
            }
        })
    } else {
        [0.0; 4]
    }
}

fn determine_terrain_type(chunk: &TerrainChunk) -> TerrainType {
    let size = chunk.size as usize;
    let mut avg_height = 0.0;
    let mut avg_temp = 0.0;
    let mut avg_moisture = 0.0;
    let mut max_slope: f32 = 0.0;
    let total = (size * size) as f32;
    for i in 0..size {
        for j in 0..size {
            avg_height += chunk.get_height(i, j);
            avg_temp += chunk.get_temperature(i, j);
            avg_moisture += chunk.get_moisture(i, j);
            // Calculate maximum slope for terrain features
            if i > 0 && j > 0 {
                let h00 = chunk.get_height(i-1, j-1);
                let h10 = chunk.get_height(i, j-1);
                let h01 = chunk.get_height(i-1, j);
                let h11 = chunk.get_height(i, j);
                let dx = ((h10 - h00) + (h11 - h01)) * 0.5;
                let dz = ((h01 - h00) + (h11 - h10)) * 0.5;
                let slope = (dx * dx + dz * dz).sqrt().atan() * (180.0 / PI);
                max_slope = max_slope.max(slope);
            }
        }
    }
    avg_height /= total;
    avg_temp /= total;
    avg_moisture /= total;
    // Determine terrain type based on height, temperature, moisture, and slope
    match (avg_height, avg_temp, avg_moisture, max_slope) {
        (h, _, _, s) if s > 45.0 => TerrainType::Mountain, // Steep slopes are mountains
        (h, _, _, _) if h < -0.3 => TerrainType::Ocean,
        (h, _, _, _) if h < -0.2 => TerrainType::Beach,
        (_, t, m, _) if t > 0.6 && m < -0.2 => TerrainType::Desert,
        (_, t, m, _) if t > 0.2 && m > 0.4 => TerrainType::Rainforest,
        (_, t, _,  _) if t < -0.3 => TerrainType::Tundra,
        (_, _, m, _) if m > 0.3 => TerrainType::Forest,
        _ => TerrainType::Plains,
    }
} 