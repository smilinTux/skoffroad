use bevy::prelude::*;

/// Minimal DebugPlugin for plugin registration
pub struct DebugPlugin;

impl Plugin for DebugPlugin {
    fn build(&self, _app: &mut App) {
        // Add debug systems here
    }
}
