use bevy::prelude::*;
use crate::game::terrain::mesh::generate_chunk_mesh;
use crate::game::terrain::buffer_pool::TerrainBufferPool;
use crate::game::terrain::culling::TerrainCullingSystem;
use crate::terrain::{MeshKey, ChunkBounds};
use crate::game::terrain::heightmap::Heightmap;
use noise::{FastNoise, NoiseType, NoiseFn, Perlin, Fbm, RidgedMulti, Billow, MultiFractal};
use std::f64;
use crate::terrain::TerrainChunk;

/// Configuration for terrain noise generation
#[derive(Resource, Clone, Debug)]
pub struct TerrainNoiseConfig {
    /// Base frequency for the noise
    pub frequency: f64,
    /// Number of octaves for fractal noise
    pub octaves: usize,
    /// Persistence factor for octaves
    pub persistence: f64,
    /// Lacunarity factor for octaves
    pub lacunarity: f64,
    /// Overall height scale
    pub height_scale: f32,
    /// Seed for noise generation
    pub seed: u32,
    /// Biome influence factor (0-1)
    pub biome_blend: f32,
    /// Erosion iterations
    pub erosion_iterations: u32,
    /// Ridge noise influence (0-1)
    pub ridge_influence: f32,
}

impl Default for TerrainNoiseConfig {
    fn default() -> Self {
        Self {
            frequency: 0.01,
            octaves: 6,
            persistence: 0.5,
            lacunarity: 2.0,
            height_scale: 50.0,
            seed: 42,
            biome_blend: 0.5,
            erosion_iterations: 3,
            ridge_influence: 0.3,
        }
    }
}

/// Biome type for terrain variation
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BiomeType {
    Plains,
    Mountains,
    Desert,
    Hills,
}

impl BiomeType {
    fn height_modifier(&self) -> f32 {
        match self {
            BiomeType::Plains => 0.5,
            BiomeType::Mountains => 2.0,
            BiomeType::Desert => 0.3,
            BiomeType::Hills => 1.0,
        }
    }

    fn roughness_modifier(&self) -> f32 {
        match self {
            BiomeType::Plains => 0.3,
            BiomeType::Mountains => 1.0,
            BiomeType::Desert => 0.1,
            BiomeType::Hills => 0.6,
        }
    }
}

#[derive(Component)]
pub struct TerrainChunkComponent {
    pub chunk: TerrainChunk,
}

impl TerrainChunkComponent {
    pub fn new(chunk: TerrainChunk) -> Self {
        Self { chunk }
    }
}

pub fn update_terrain_chunks(
    mut commands: Commands,
    mut chunks: Query<(Entity, &mut TerrainChunk)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut buffer_pool: ResMut<TerrainBufferPool>,
    mut culling_system: ResMut<TerrainCullingSystem>,
    // Add other resources needed for heightmap/LOD
) {
    for (entity, mut chunk) in chunks.iter_mut() {
        if chunk.needs_update {
            // Generate heightmap data for chunk
            let heightmap = generate_heightmap_for_chunk(chunk.position, chunk.size);
            let resolution = calculate_resolution(chunk.lod_level);

            // Generate or update mesh
            let (mesh, bounds) = generate_chunk_mesh(
                &heightmap,
                resolution,
                chunk.position,
                chunk.size,
                chunk.lod_level,
                &mut buffer_pool,
                &mut culling_system,
                entity,
            );

            // Update chunk bounds and mesh key
            chunk.bounds = bounds;
            chunk.mesh_key = Some(MeshKey {
                lod_level: chunk.lod_level,
                has_skirts: true,
                vertex_count: mesh.count_vertices(),
            });
            
            // Insert or update mesh component
            if let Some(mesh_entity) = commands.get_entity(entity) {
                mesh_entity.insert(meshes.add(mesh));
            }

            chunk.needs_update = false;
        }
    }
}

pub fn cleanup_terrain_chunks(
    mut commands: Commands,
    chunks: Query<(Entity, &TerrainChunk)>,
    mut buffer_pool: ResMut<TerrainBufferPool>,
    mut culling_system: ResMut<TerrainCullingSystem>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    for (entity, chunk) in chunks.iter() {
        // Remove chunk from culling system
        culling_system.remove_chunk(entity);
        
        // Return mesh to buffer pool if we have a key
        if let Some(mesh_key) = chunk.mesh_key {
            if let Some(mesh_handle) = commands.get_entity(entity)
                .and_then(|e| e.get::<Handle<Mesh>>()) {
                if let Some(mesh) = meshes.remove(mesh_handle) {
                    buffer_pool.return_buffer(entity, mesh);
                }
            }
        }
        
        commands.entity(entity).despawn();
    }
}

// Helper function implementations
fn generate_heightmap_for_chunk(position: Vec2, size: Vec2) -> Heightmap {
    let config = TerrainNoiseConfig::default();
    generate_heightmap_with_config(position, size, &config)
}

fn generate_heightmap_with_config(position: Vec2, size: Vec2, config: &TerrainNoiseConfig) -> Heightmap {
    let dimensions = UVec2::new(
        (size.x / TERRAIN_CELL_SIZE) as u32 + 1,
        (size.y / TERRAIN_CELL_SIZE) as u32 + 1
    );
    
    let mut heightmap = Heightmap::new(dimensions, size);
    
    // Create different noise generators
    let base_noise = Fbm::<Perlin>::new(config.seed)
        .set_octaves(config.octaves)
        .set_persistence(config.persistence)
        .set_lacunarity(config.lacunarity)
        .set_frequency(config.frequency);
    
    let ridge_noise = RidgedMulti::<Perlin>::new(config.seed + 1)
        .set_octaves(config.octaves)
        .set_persistence(config.persistence)
        .set_lacunarity(config.lacunarity)
        .set_frequency(config.frequency * 2.0);
    
    let biome_noise = Billow::<Perlin>::new(config.seed + 2)
        .set_octaves(3)
        .set_frequency(config.frequency * 0.5);
    
    for y in 0..dimensions.y {
        for x in 0..dimensions.x {
            let world_x = position.x + x as f32 * TERRAIN_CELL_SIZE;
            let world_y = position.y + y as f32 * TERRAIN_CELL_SIZE;
            
            // Generate base terrain
            let base_value = base_noise.get([world_x as f64, world_y as f64]);
            
            // Add ridge features
            let ridge_value = ridge_noise.get([world_x as f64, world_y as f64]);
            let ridge_contribution = ridge_value * config.ridge_influence as f64;
            
            // Determine biome
            let biome_value = biome_noise.get([world_x as f64, world_y as f64]);
            let biome = determine_biome(biome_value);
            
            // Combine noise with biome influence
            let combined_height = (base_value + ridge_contribution) as f32 
                * biome.height_modifier() 
                * config.height_scale;
            
            heightmap.set_height(x, y, combined_height).unwrap();
        }
    }
    
    // Apply thermal erosion if configured
    if config.erosion_iterations > 0 {
        apply_thermal_erosion(&mut heightmap, config.erosion_iterations);
    }
    
    heightmap
}

fn determine_biome(noise_value: f64) -> BiomeType {
    match noise_value {
        n if n < -0.25 => BiomeType::Plains,
        n if n < 0.0 => BiomeType::Hills,
        n if n < 0.25 => BiomeType::Mountains,
        _ => BiomeType::Desert,
    }
}

fn apply_thermal_erosion(heightmap: &mut Heightmap, iterations: u32) {
    let talus = 0.5; // Maximum height difference between adjacent cells
    let dimensions = heightmap.dimensions;
    
    for _ in 0..iterations {
        for y in 1..dimensions.y - 1 {
            for x in 1..dimensions.x - 1 {
                let current_height = heightmap.get_height(x, y).unwrap();
                let mut max_diff = 0.0;
                let mut steepest_dir = (0, 0);
                
                // Check all neighbors
                for (dx, dy) in &[(0, 1), (1, 0), (0, -1), (-1, 0)] {
                    let nx = (x as i32 + dx) as u32;
                    let ny = (y as i32 + dy) as u32;
                    let neighbor_height = heightmap.get_height(nx, ny).unwrap();
                    let diff = current_height - neighbor_height;
                    
                    if diff > max_diff && diff > talus {
                        max_diff = diff;
                        steepest_dir = (*dx, *dy);
                    }
                }
                
                // Move material if slope is too steep
                if max_diff > talus {
                    let nx = (x as i32 + steepest_dir.0) as u32;
                    let ny = (y as i32 + steepest_dir.1) as u32;
                    let transfer = (max_diff - talus) * 0.5;
                    
                    heightmap.set_height(x, y, current_height - transfer).unwrap();
                    heightmap.set_height(nx, ny, 
                        heightmap.get_height(nx, ny).unwrap() + transfer).unwrap();
                }
            }
        }
    }
}

const TERRAIN_CELL_SIZE: f32 = 1.0;
const BASE_RESOLUTION: u32 = 64; // Base resolution for LOD level 0

fn calculate_resolution(lod_level: u32) -> UVec2 {
    let reduction_factor = 1u32 << lod_level; // 2^lod_level
    let resolution = BASE_RESOLUTION / reduction_factor;
    
    // Ensure minimum resolution
    let min_resolution = 4;
    let resolution = resolution.max(min_resolution);
    
    UVec2::new(resolution, resolution)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_terrain_chunk_lifecycle() {
        let mut app = App::new();
        
        // Add required resources
        app.init_resource::<TerrainBufferPool>()
           .init_resource::<TerrainCullingSystem>()
           .init_resource::<Assets<Mesh>>();
           
        // Add systems
        app.add_systems(Update, (
            update_terrain_chunks,
            cleanup_terrain_chunks,
        ));
        
        // Create test chunk
        let chunk = TerrainChunk::new(
            Vec2::new(0.0, 0.0),
            Vec2::new(100.0, 100.0),
            0,
        );
        
        let chunk_entity = app.world.spawn(chunk).id();
        
        // Run systems
        app.update();
        
        // Verify chunk was processed
        let chunk_query = app.world.query::<&TerrainChunk>();
        let chunks: Vec<_> = chunk_query.iter(&app.world).collect();
        
        assert_eq!(chunks.len(), 1);
        assert!(!chunks[0].needs_update);
        assert!(chunks[0].mesh_key.is_some());
        
        // Cleanup
        app.world.despawn(chunk_entity);
        app.update();
        
        // Verify cleanup
        let remaining_chunks: Vec<_> = chunk_query.iter(&app.world).collect();
        assert_eq!(remaining_chunks.len(), 0);
    }
    
    #[test]
    fn test_resolution_calculation() {
        // Test LOD level 0 (base resolution)
        let res0 = calculate_resolution(0);
        assert_eq!(res0, UVec2::new(BASE_RESOLUTION, BASE_RESOLUTION));
        
        // Test LOD level 1 (half resolution)
        let res1 = calculate_resolution(1);
        assert_eq!(res1, UVec2::new(BASE_RESOLUTION / 2, BASE_RESOLUTION / 2));
        
        // Test minimum resolution enforcement
        let high_lod = calculate_resolution(10);
        assert_eq!(high_lod, UVec2::new(4, 4));
    }
    
    #[test]
    fn test_heightmap_generation() {
        let pos = Vec2::new(0.0, 0.0);
        let size = Vec2::new(10.0, 10.0);
        let heightmap = generate_heightmap_for_chunk(pos, size);
        
        // Verify dimensions
        assert_eq!(heightmap.dimensions.x, (size.x / TERRAIN_CELL_SIZE) as u32 + 1);
        assert_eq!(heightmap.dimensions.y, (size.y / TERRAIN_CELL_SIZE) as u32 + 1);
        
        // Verify height range
        let mut min_height = f32::MAX;
        let mut max_height = f32::MIN;
        
        for y in 0..heightmap.dimensions.y {
            for x in 0..heightmap.dimensions.x {
                let height = heightmap.get_height(x, y).unwrap();
                min_height = min_height.min(height);
                max_height = max_height.max(height);
            }
        }
        
        assert!(max_height > min_height, "Heightmap should have variation");
        assert!(max_height <= TERRAIN_HEIGHT_SCALE, "Heights should be within scale");
    }

    #[test]
    fn test_biome_generation() {
        let config = TerrainNoiseConfig {
            frequency: 0.05,
            height_scale: 100.0,
            biome_blend: 0.8,
            ..Default::default()
        };
        
        let pos = Vec2::new(0.0, 0.0);
        let size = Vec2::new(100.0, 100.0);
        let heightmap = generate_heightmap_with_config(pos, size, &config);
        
        // Verify height variations match biome characteristics
        let mut height_distribution = vec![0; 4]; // Count points in different height ranges
        
        for y in 0..heightmap.dimensions.y {
            for x in 0..heightmap.dimensions.x {
                let height = heightmap.get_height(x, y).unwrap();
                let range_idx = match height {
                    h if h < -25.0 => 0,
                    h if h < 0.0 => 1,
                    h if h < 25.0 => 2,
                    _ => 3,
                };
                height_distribution[range_idx] += 1;
            }
        }
        
        // Ensure we have a good distribution of heights
        assert!(height_distribution.iter().all(|&count| count > 0), 
            "All height ranges should be represented");
    }

    #[test]
    fn test_erosion() {
        let mut config = TerrainNoiseConfig::default();
        config.erosion_iterations = 5;
        
        let pos = Vec2::new(0.0, 0.0);
        let size = Vec2::new(50.0, 50.0);
        let heightmap = generate_heightmap_with_config(pos, size, &config);
        
        // Verify that erosion has smoothed out extreme height differences
        let mut max_diff = 0.0;
        
        for y in 1..heightmap.dimensions.y - 1 {
            for x in 1..heightmap.dimensions.x - 1 {
                let center = heightmap.get_height(x, y).unwrap();
                let right = heightmap.get_height(x + 1, y).unwrap();
                let down = heightmap.get_height(x, y + 1).unwrap();
                
                max_diff = max_diff.max((center - right).abs());
                max_diff = max_diff.max((center - down).abs());
            }
        }
        
        assert!(max_diff < config.height_scale * 0.5, 
            "Erosion should prevent extreme height differences");
    }
}