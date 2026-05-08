use bevy::prelude::*;
use bevy::diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin};
use bevy::log::LogPlugin;

/// Resource for managing debug visualization states
#[derive(Resource, Default)]
pub struct DebugInfo {
    pub show_fps: bool,
    pub show_physics_debug: bool,
    pub show_vehicle_debug: bool,
    pub show_particle_debug: bool,
}

/// Plugin for managing debug features and visualization
pub struct DebugPlugin;

impl Plugin for DebugPlugin {
    fn build(&self, app: &mut App) {
        info!("Initializing Debug Plugin");
        
        app.init_resource::<DebugInfo>()
           .add_plugins(LogDiagnosticsPlugin::default())
           .add_plugins(FrameTimeDiagnosticsPlugin::default())
           .add_systems(Update, (
               toggle_debug_info,
               update_debug_display.after(toggle_debug_info)
           ));

        debug!("Debug Plugin initialized successfully");
    }
}

/// System for toggling debug information based on key input
fn toggle_debug_info(
    mut debug_info: ResMut<DebugInfo>,
    keyboard: Res<Input<KeyCode>>,
) {
    if keyboard.just_pressed(KeyCode::F3) {
        debug_info.show_fps = !debug_info.show_fps;
        info!("FPS display toggled: {}", debug_info.show_fps);
    }
    if keyboard.just_pressed(KeyCode::F4) {
        debug_info.show_physics_debug = !debug_info.show_physics_debug;
        info!("Physics debug toggled: {}", debug_info.show_physics_debug);
    }
    if keyboard.just_pressed(KeyCode::F5) {
        debug_info.show_vehicle_debug = !debug_info.show_vehicle_debug;
        info!("Vehicle debug toggled: {}", debug_info.show_vehicle_debug);
    }
    if keyboard.just_pressed(KeyCode::F6) {
        debug_info.show_particle_debug = !debug_info.show_particle_debug;
        info!("Particle debug toggled: {}", debug_info.show_particle_debug);
    }
}

/// System for updating debug display based on active debug flags
fn update_debug_display(
    debug_info: Res<DebugInfo>,
    diagnostics: Res<bevy::diagnostic::Diagnostics>,
    mut gizmos: Gizmos,
) {
    if debug_info.show_fps {
        if let Some(fps) = diagnostics.get(bevy::diagnostic::FrameTimeDiagnosticsPlugin::FPS) {
            if let Some(value) = fps.smoothed() {
                trace!("Current FPS: {:.2}", value);
            }
        }
    }

    if debug_info.show_physics_debug {
        // Draw physics debug visualization
        trace!("Drawing physics debug visualization");
    }

    if debug_info.show_vehicle_debug {
        // Draw vehicle debug visualization
        trace!("Drawing vehicle debug visualization");
    }

    if debug_info.show_particle_debug {
        // Draw particle debug visualization
        trace!("Drawing particle debug visualization");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_debug_info_creation() {
        let debug_info = DebugInfo::default();
        assert!(!debug_info.show_fps);
        assert!(!debug_info.show_physics_debug);
        assert!(!debug_info.show_vehicle_debug);
        assert!(!debug_info.show_particle_debug);
    }
} 