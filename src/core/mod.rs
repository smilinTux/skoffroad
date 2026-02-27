use bevy::prelude::*;

/// Minimal CorePlugin for plugin registration
pub struct CorePlugin;

impl Plugin for CorePlugin {
    fn build(&self, _app: &mut App) {
        // Add core systems here
    }
}
