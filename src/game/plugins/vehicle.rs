use bevy::prelude::*;

/// Minimal VehiclePlugin for plugin registration
pub struct VehiclePlugin;

impl Plugin for VehiclePlugin {
    fn build(&self, _app: &mut App) {
        // Add vehicle systems here
    }
}
