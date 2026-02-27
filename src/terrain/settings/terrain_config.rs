use bevy::prelude::Resource;

// TerrainConfig for terrain system
#[derive(Debug, Clone, Copy, PartialEq, Resource)]
pub struct TerrainConfig {
    pub chunk_resolution: u32,
    pub height_scale: f32,
    pub roughness: f32,
    pub chunk_size: u32,
    pub vertex_scale: f32,
}
