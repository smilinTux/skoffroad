use bevy::prelude::Resource;

// TerrainGenerationConfig for terrain system
#[derive(Debug, Clone, Copy, PartialEq, Resource)]
pub struct TerrainGenerationConfig {
    pub view_distance: f32,
    pub chunk_size: f32,
}
