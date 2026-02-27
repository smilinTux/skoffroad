use crate::math::*;
use crate::terrain::Heightmap;
use crate::render::mesh::*;
use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use crate::terrain::{MeshKey, ChunkBounds};
use crate::game::terrain::buffer_pool::TerrainBufferPool;
use crate::game::terrain::culling::TerrainCullingSystem;

/// Trait for heightmap sampling
pub trait HeightmapSampler {
    fn sample_height_at(&self, x: f32, z: f32, lod: Option<u32>) -> f32;
    fn calculate_normal_at(&self, x: f32, z: f32, lod: Option<u32>) -> [f32; 3];
    fn get_dimensions(&self) -> UVec2;
    fn get_size(&self) -> Vec2;
}

impl HeightmapSampler for Heightmap {
    fn sample_height_at(&self, x: f32, z: f32, _lod: Option<u32>) -> f32 {
        self.get_height_at(Vec2::new(x, z)).unwrap_or(0.0)
    }

    fn calculate_normal_at(&self, x: f32, z: f32, _lod: Option<u32>) -> [f32; 3] {
        let grid_x = ((x + self.size.x / 2.0) / self.size.x * self.dimensions.x as f32) as u32;
        let grid_y = ((z + self.size.y / 2.0) / self.size.y * self.dimensions.y as f32) as u32;
        calculate_normal(self, grid_x, grid_y, self.size.x / self.dimensions.x as f32, self.size.y / self.dimensions.y as f32)
    }

    fn get_dimensions(&self) -> UVec2 {
        self.dimensions
    }

    fn get_size(&self) -> Vec2 {
        self.size
    }
}

impl HeightmapSampler for TerrainHeightmap {
    fn sample_height_at(&self, x: f32, z: f32, lod: Option<u32>) -> f32 {
        match lod {
            Some(level) => self.sample_height_at_lod(x, z, level),
            None => self.sample_height(x, z)
        }
    }

    fn calculate_normal_at(&self, x: f32, z: f32, lod: Option<u32>) -> [f32; 3] {
        match lod {
            Some(level) => self.calculate_normal_at_lod(x, z, level),
            None => self.calculate_normal(x, z)
        }
    }

    fn get_dimensions(&self) -> UVec2 {
        // TerrainHeightmap is continuous, so we use a default high resolution
        UVec2::new(1024, 1024)
    }

    fn get_size(&self) -> Vec2 {
        // Return the world size of the terrain
        Vec2::new(1000.0, 1000.0) // This should be configurable
    }
}

/// Configuration for LOD transitions
#[derive(Debug, Clone)]
pub struct LodTransitionConfig {
    /// Distance at which to start transitioning to the next LOD level
    pub transition_start: f32,
    /// Distance at which to complete the transition
    pub transition_end: f32,
    /// Current transition factor (0.0 = current LOD, 1.0 = next LOD)
    pub morph_factor: f32,
}

impl Default for LodTransitionConfig {
    fn default() -> Self {
        Self {
            transition_start: 50.0,
            transition_end: 100.0,
            morph_factor: 0.0,
        }
    }
}

/// Generate skirt vertices for a chunk edge to hide seams between chunks
fn generate_skirt_vertices<H: HeightmapSampler>(
    heightmap: &H,
    positions: &[[f32; 3]],
    normals: &[[f32; 3]],
    uvs: &[[f32; 2]],
    vertices_per_side: u32,
    edge: usize, // 0=left, 1=right, 2=top, 3=bottom
    skirt_depth: f32,
    lod_level: Option<u32>,
) -> (Vec<[f32; 3]>, Vec<[f32; 3]>, Vec<[f32; 2]>, Vec<u32>) {
    let mut skirt_positions = Vec::new();
    let mut skirt_normals = Vec::new();
    let mut skirt_uvs = Vec::new();
    let mut skirt_indices = Vec::new();
    
    // Get edge vertices based on edge type
    let edge_vertices = match edge {
        0 => (0..vertices_per_side).map(|i| i * vertices_per_side).collect::<Vec<_>>(), // Left
        1 => (0..vertices_per_side).map(|i| (i + 1) * vertices_per_side - 1).collect(), // Right
        2 => (0..vertices_per_side).collect(), // Top
        3 => ((vertices_per_side - 1) * vertices_per_side..(vertices_per_side * vertices_per_side)).collect(), // Bottom
        _ => return (Vec::new(), Vec::new(), Vec::new(), Vec::new()),
    };
    
    // Create skirt vertices
    for (i, &vertex_idx) in edge_vertices.iter().enumerate() {
        let vertex_idx = vertex_idx as usize;
        let pos = positions[vertex_idx];
        let normal = normals[vertex_idx];
        let uv = uvs[vertex_idx];
        
        // Add original vertex
        skirt_positions.push(pos);
        skirt_normals.push(normal);
        skirt_uvs.push(uv);
        
        // Add lowered vertex
        let mut lowered_pos = pos;
        lowered_pos[1] -= skirt_depth; // Lower the vertex
        
        // Sample height and normal at lowered position
        let world_x = lowered_pos[0];
        let world_z = lowered_pos[2];
        lowered_pos[1] = heightmap.sample_height_at(world_x, world_z, lod) - skirt_depth;
        let lowered_normal = heightmap.calculate_normal_at(world_x, world_z, lod);
        
        skirt_positions.push(lowered_pos);
        skirt_normals.push(lowered_normal);
        skirt_uvs.push(uv); // Reuse UV from original vertex
        
        // Add indices for skirt triangles (two triangles per edge vertex)
        if i < edge_vertices.len() - 1 {
            let base_idx = (i * 2) as u32;
            skirt_indices.extend_from_slice(&[
                base_idx,
                base_idx + 1,
                base_idx + 2,
                base_idx + 2,
                base_idx + 1,
                base_idx + 3,
            ]);
        }
    }
    
    (skirt_positions, skirt_normals, skirt_uvs, skirt_indices)
}

/// Blend two vectors with a weight factor
fn blend_vectors(v1: [f32; 3], v2: [f32; 3], weight: f32) -> [f32; 3] {
    let w1 = 1.0 - weight;
    let w2 = weight;
    [
        v1[0] * w1 + v2[0] * w2,
        v1[1] * w1 + v2[1] * w2,
        v1[2] * w1 + v2[2] * w2,
    ]
}

/// Generate a mesh for a terrain chunk with LOD transition support
pub fn generate_chunk_mesh<H: HeightmapSampler>(
    heightmap: &H,
    chunk_pos: UVec2,
    lod_level: u32,
    reduction_factor: u32,
    transition_config: Option<&LodTransitionConfig>,
    buffer_pool: &mut TerrainBufferPool,
    culling_system: &mut TerrainCullingSystem,
    entity: Entity,
) -> (Mesh, ChunkBounds) {
    let vertices_per_side = CHUNK_SIZE / reduction_factor + 1;
    let mut positions = Vec::new();
    let mut morph_targets = Vec::new();
    let mut morph_factors = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut indices = Vec::new();
    
    // Calculate chunk bounds in world space
    let size = heightmap.get_size();
    let dimensions = heightmap.get_dimensions();
    let cell_size_x = size.x / dimensions.x as f32;
    let cell_size_y = size.y / dimensions.y as f32;
    
    let start_x = chunk_pos.x * CHUNK_SIZE;
    let start_y = chunk_pos.y * CHUNK_SIZE;
    
    // Calculate next LOD reduction factor if transitioning
    let next_reduction_factor = if transition_config.is_some() {
        reduction_factor * 2
    } else {
        reduction_factor
    };
    
    // Generate vertices
    for y in 0..vertices_per_side {
        for x in 0..vertices_per_side {
            let world_x = start_x as f32 * cell_size_x - size.x / 2.0;
            let world_z = start_y as f32 * cell_size_y - size.y / 2.0;
            
            // Sample height and normal at current LOD
            let height = heightmap.sample_height_at(world_x, world_z, Some(lod_level));
            let normal = heightmap.calculate_normal_at(world_x, world_z, Some(lod_level));
            
            let position = [world_x, height, world_z];
            positions.push(position);
            normals.push(normal);
            
            // Calculate morph target position for next LOD if transitioning
            if let Some(config) = transition_config {
                if (x % 2 == 0) && (y % 2 == 0) {
                    morph_targets.push(position);
                    morph_factors.push(config.morph_factor);
                } else {
                    // Sample height at next LOD for morphing
                    let morph_height = heightmap.sample_height_at(world_x, world_z, Some(lod_level + 1));
                    let morph_target = [world_x, morph_height, world_z];
                    morph_targets.push(morph_target);
                    morph_factors.push(config.morph_factor);
                }
            }
            
            // Calculate UV coordinates
            let u = x as f32 / (vertices_per_side - 1) as f32;
            let v = y as f32 / (vertices_per_side - 1) as f32;
            uvs.push([u, v]);
        }
    }
    
    // Generate indices for triangles
    for y in 0..vertices_per_side - 1 {
        for x in 0..vertices_per_side - 1 {
            let top_left = y * vertices_per_side + x;
            let top_right = top_left + 1;
            let bottom_left = (y + 1) * vertices_per_side + x;
            let bottom_right = bottom_left + 1;
            
            indices.extend_from_slice(&[
                top_left as u32,
                bottom_left as u32,
                top_right as u32,
                top_right as u32,
                bottom_left as u32,
                bottom_right as u32,
            ]);
        }
    }
    
    // Add skirt meshes to hide seams
    let skirt_depth = 50.0; // Adjust based on terrain scale
    for edge in 0..4 {
        let (mut skirt_positions, mut skirt_normals, mut skirt_uvs, mut skirt_indices) = 
            generate_skirt_vertices(
                heightmap,
                &positions,
                &normals,
                &uvs,
                vertices_per_side,
                edge,
                skirt_depth,
                Some(lod_level),
            );
        
        // Add morph targets for skirt vertices if transitioning
        if transition_config.is_some() {
            let skirt_morph_targets = skirt_positions.clone();
            morph_targets.extend(skirt_morph_targets);
            morph_factors.extend(vec![0.0; skirt_positions.len()]); // Skirts don't morph
        }
        
        // Offset indices by current vertex count
        let vertex_offset = positions.len() as u32;
        for idx in &mut skirt_indices {
            *idx += vertex_offset;
        }
        
        // Append skirt data
        positions.extend(skirt_positions);
        normals.extend(skirt_normals);
        uvs.extend(skirt_uvs);
        indices.extend(skirt_indices);
    }
    
    // Create mesh
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    if !morph_targets.is_empty() {
        mesh.insert_attribute(Mesh::ATTRIBUTE_MORPH_TARGET_POSITION_0, morph_targets);
        mesh.insert_attribute(Mesh::ATTRIBUTE_MORPH_FACTOR, morph_factors);
    }
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.set_indices(Some(Indices::U32(indices)));
    
    // Calculate bounds
    let bounds = calculate_chunk_bounds(&mesh);
    
    // Update culling system
    culling_system.update_chunk_bounds(entity, bounds.clone());
    
    // Register mesh with pool
    let mesh_key = MeshKey {
        lod_level,
        has_skirt: true,
        vertex_count: positions.len() as usize,
    };
    buffer_pool.register_mesh(mesh_key, mesh.clone());

    (mesh, bounds)
}

/// Get neighboring vertex positions for interpolation
fn get_neighbor_positions(
    heightmap: &Heightmap,
    x: u32,
    y: u32,
    reduction_factor: u32,
    cell_size_x: f32,
    cell_size_y: f32,
) -> Vec<[f32; 3]> {
    let mut neighbors = Vec::new();
    let offsets = [
        (-1, -1), (0, -1), (1, -1),
        (-1, 0),           (1, 0),
        (-1, 1),  (0, 1),  (1, 1),
    ];
    
    for (dx, dy) in offsets.iter() {
        let nx = (x as i32 + dx * reduction_factor as i32) as u32;
        let ny = (y as i32 + dy * reduction_factor as i32) as u32;
        
        if nx < heightmap.dimensions.x && ny < heightmap.dimensions.y {
            let height = heightmap.get_height(nx, ny).unwrap_or(0.0);
            let pos_x = nx as f32 * cell_size_x - heightmap.size.x / 2.0;
            let pos_z = ny as f32 * cell_size_y - heightmap.size.y / 2.0;
            neighbors.push([pos_x, height, pos_z]);
        }
    }
    
    neighbors
}

/// Interpolate position from neighboring vertices
fn interpolate_position(neighbors: &[[f32; 3]]) -> [f32; 3] {
    if neighbors.is_empty() {
        return [0.0, 0.0, 0.0];
    }
    
    let mut result = [0.0, 0.0, 0.0];
    let weight = 1.0 / neighbors.len() as f32;
    
    for neighbor in neighbors {
        for i in 0..3 {
            result[i] += neighbor[i] * weight;
        }
    }
    
    result
}

/// Calculate chunk bounds for culling
fn calculate_chunk_bounds(mesh: &Mesh) -> ChunkBounds {
    let positions = mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap().as_float3().unwrap();
    
    let mut min = Vec3::splat(f32::MAX);
    let mut max = Vec3::splat(f32::MIN);
    
    for position in positions.iter() {
        min = min.min(Vec3::from(*position));
        max = max.max(Vec3::from(*position));
    }
    
    ChunkBounds::from_min_max(min, max)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ... existing tests ...

    #[test]
    fn test_skirt_generation() {
        let vertices_per_side = 4;
        let positions = vec![
            [0.0, 1.0, 0.0], [1.0, 1.0, 0.0], [2.0, 1.0, 0.0], [3.0, 1.0, 0.0],
            [0.0, 1.0, 1.0], [1.0, 1.0, 1.0], [2.0, 1.0, 1.0], [3.0, 1.0, 1.0],
            [0.0, 1.0, 2.0], [1.0, 1.0, 2.0], [2.0, 1.0, 2.0], [3.0, 1.0, 2.0],
            [0.0, 1.0, 3.0], [1.0, 1.0, 3.0], [2.0, 1.0, 3.0], [3.0, 1.0, 3.0],
        ];
        
        let normals = vec![[0.0, 1.0, 0.0]; 16];
        let uvs = vec![[0.0, 0.0]; 16];
        
        // Test left edge skirt
        let (skirt_pos, skirt_normals, skirt_uvs, indices) = generate_skirt_vertices(
            &positions,
            &normals,
            &uvs,
            vertices_per_side,
            0, // Left edge
            1.0, // Skirt depth
        );
        
        // Verify skirt vertices
        assert_eq!(skirt_pos.len(), 8); // 4 edge vertices + 4 lowered vertices
        assert_eq!(skirt_normals.len(), 8); // Should match position count
        assert_eq!(skirt_uvs.len(), 8); // Should match position count
        assert!(!indices.is_empty());
        
        // Verify lowered vertices
        for i in (1..skirt_pos.len()).step_by(2) {
            assert_eq!(skirt_pos[i][1], 0.0); // Check lowered height
        }
        
        // Verify normal blending
        for i in (0..skirt_normals.len()).step_by(2) {
            let normal = skirt_normals[i];
            assert!(normal[0] < 0.0); // Should be blended with left-facing normal
            assert!(normal[1] > 0.0); // Should maintain some upward component
        }
    }

    #[test]
    fn test_lod_transition() {
        // Create a simple heightmap for testing
        let mut heightmap = Heightmap::new(UVec2::new(4, 4), Vec2::new(4.0, 4.0));
        for y in 0..4 {
            for x in 0..4 {
                heightmap.set_height(x, y, 1.0).unwrap();
            }
        }
        
        let config = LodTransitionConfig {
            transition_start: 50.0,
            transition_end: 100.0,
            morph_factor: 0.5,
        };
        
        // Generate mesh with LOD transition
        let mesh = generate_chunk_mesh(
            &heightmap,
            UVec2::new(0, 0),
            0,
            1,
            Some(&config),
        );
        
        // Verify morph attributes exist
        assert!(mesh.attribute(Mesh::ATTRIBUTE_MORPH_TARGET_POSITION_0).is_some());
        assert!(mesh.attribute(Mesh::ATTRIBUTE_MORPH_FACTOR).is_some());
        
        // Get morph factors
        let morph_factors: Vec<f32> = mesh
            .attribute(Mesh::ATTRIBUTE_MORPH_FACTOR)
            .unwrap()
            .as_float()
            .unwrap()
            .to_vec();
        
        // Verify morph factors are set correctly
        assert!(morph_factors.iter().any(|&f| f == config.morph_factor));
    }

    #[test]
    fn test_mesh_generation_with_buffer_pool() {
        let resolution = UVec2::new(4, 4);
        let heightmap = vec![0.0; (resolution.x * resolution.y) as usize];
        let chunk_position = Vec2::ZERO;
        let chunk_size = Vec2::new(10.0, 10.0);
        let lod_level = 0;
        
        let mut buffer_pool = TerrainBufferPool::default();
        let mut culling_system = TerrainCullingSystem::default();
        let entity = Entity::from_raw(1);

        // Generate initial mesh
        let (mesh1, bounds1) = generate_chunk_mesh(
            &heightmap,
            resolution,
            chunk_position,
            chunk_size,
            lod_level,
            Some(&LodTransitionConfig::default()),
            &mut buffer_pool,
            &mut culling_system,
            entity,
        );

        // Verify mesh data
        assert!(mesh1.attribute(Mesh::ATTRIBUTE_POSITION).is_some());
        assert!(mesh1.attribute(Mesh::ATTRIBUTE_NORMAL).is_some());
        assert!(mesh1.attribute(Mesh::ATTRIBUTE_UV_0).is_some());
        assert!(mesh1.indices().is_some());

        // Generate second mesh with same parameters
        let (mesh2, bounds2) = generate_chunk_mesh(
            &heightmap,
            resolution,
            chunk_position,
            chunk_size,
            lod_level,
            Some(&LodTransitionConfig::default()),
            &mut buffer_pool,
            &mut culling_system,
            entity,
        );

        // Verify buffer reuse
        assert_eq!(
            mesh1.count_vertices(),
            mesh2.count_vertices(),
            "Reused mesh should have same vertex count"
        );

        // Verify bounds calculation
        assert_eq!(bounds1.center, bounds2.center);
        assert_eq!(bounds1.extents, bounds2.extents);
    }
}