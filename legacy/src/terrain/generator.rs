use bevy::prelude::*;
use crate::terrain::chunk::TerrainChunk;
use noise::OpenSimplex;
use noise::NoiseFn;
use crate::terrain::generation::TerrainGenerationSettings;
use crate::terrain::TerrainType;

#[derive(Resource)]
pub struct TerrainGenerator {
    pub noise: OpenSimplex,
    pub temperature: Vec<f32>,
    pub moisture: Vec<f32>,
    pub height: Vec<f32>,
    pub chunk_size: usize,
}

impl TerrainGenerator {
    pub fn new(chunk_size: usize, seed: u32) -> Self {
        let noise = OpenSimplex::new(seed);
        let total_size = chunk_size * chunk_size;
        
        TerrainGenerator {
            noise,
            temperature: vec![0.0; total_size],
            moisture: vec![0.0; total_size],
            height: vec![0.0; total_size],
            chunk_size,
        }
    }

    pub fn generate_chunk(&self, x: i32, z: i32) -> TerrainChunk {
        let mut chunk = TerrainChunk::new_default(x, z, self.chunk_size);
        chunk.terrain_types = vec![TerrainType::Grass; self.chunk_size * self.chunk_size];
        
        // Generate height, temperature, and moisture maps
        for i in 0..self.chunk_size {
            for j in 0..self.chunk_size {
                let world_x = x as f32 * self.chunk_size as f32 + i as f32;
                let world_z = z as f32 * self.chunk_size as f32 + j as f32;
                
                let height = self.noise.get([world_x as f64 * 0.01, world_z as f64 * 0.01]) as f32;
                let temp = self.noise.get([world_x as f64 * 0.005 + 1000.0, world_z as f64 * 0.005]) as f32;
                let moisture = self.noise.get([world_x as f64 * 0.007 + 2000.0, world_z as f64 * 0.007]) as f32;
                
                let idx = i * self.chunk_size + j;
                
                chunk.set_height(i, j, height);
                chunk.set_temperature(i, j, temp);
                chunk.set_moisture(i, j, moisture);
                
                let terrain_type = if height > 0.5 {
                    TerrainType::Rock
                } else if height < -0.5 {
                    TerrainType::Water
                } else {
                    TerrainType::Grass
                };
                chunk.terrain_types[idx] = terrain_type;
            }
        }
        
        chunk
    }
} 