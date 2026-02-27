use bevy::prelude::*;

/// Minimal TerrainPlugin for plugin registration
pub struct TerrainPlugin;

impl Plugin for TerrainPlugin {
    fn build(&self, _app: &mut App) {
        // Add terrain systems here
    }
}
