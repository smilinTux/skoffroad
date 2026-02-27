use bevy::prelude::*;

/// Minimal UiPlugin for plugin registration
pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, _app: &mut App) {
        // Add UI systems here
    }
}
