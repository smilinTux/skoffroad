use bevy::prelude::*;
use bevy::diagnostic::{Diagnostics, FrameTimeDiagnosticsPlugin};
use bevy::input::keyboard::KeyCode;
use crate::game::debug::{DebugInfo, DebugPlugin};

/// Helper function to setup a test app with debug plugin
fn setup_test_app() -> App {
    let mut app = App::new();
    app.add_plugins(DebugPlugin);
    app
}

#[test]
fn test_debug_info_default() {
    let debug_info = DebugInfo::default();
    assert!(!debug_info.show_fps);
    assert!(!debug_info.show_physics_debug);
    assert!(!debug_info.show_vehicle_debug);
    assert!(!debug_info.show_particle_debug);
}

#[test]
fn test_debug_plugin_setup() {
    let mut app = setup_test_app();
    app.update();

    assert!(app.world.contains_resource::<DebugInfo>());
    assert!(app.world.contains_resource::<Diagnostics>());
}

#[test]
fn test_debug_toggle_system() {
    let mut app = setup_test_app();
    
    // Test FPS toggle
    app.update();
    let mut input = app.world.resource_mut::<Input<KeyCode>>();
    input.press(KeyCode::F3);
    app.update();
    
    let debug_info = app.world.resource::<DebugInfo>();
    assert!(debug_info.show_fps);
    
    // Test physics debug toggle
    input.clear();
    input.press(KeyCode::F4);
    app.update();
    
    let debug_info = app.world.resource::<DebugInfo>();
    assert!(debug_info.show_physics_debug);
}

#[test]
fn test_debug_display_system() {
    let mut app = setup_test_app();
    app.add_plugins(FrameTimeDiagnosticsPlugin::default());
    
    // Enable FPS display
    {
        let mut debug_info = app.world.resource_mut::<DebugInfo>();
        debug_info.show_fps = true;
    }
    
    app.update();
    
    let diagnostics = app.world.resource::<Diagnostics>();
    assert!(diagnostics.get(FrameTimeDiagnosticsPlugin::FPS).is_some());
}

#[test]
fn test_multiple_toggles() {
    let mut app = setup_test_app();
    
    // Test multiple toggles
    let mut input = app.world.resource_mut::<Input<KeyCode>>();
    input.press(KeyCode::F3);
    input.press(KeyCode::F4);
    app.update();
    
    let debug_info = app.world.resource::<DebugInfo>();
    assert!(debug_info.show_fps);
    assert!(debug_info.show_physics_debug);
    
    // Test toggle off
    input.clear();
    input.press(KeyCode::F3);
    app.update();
    
    let debug_info = app.world.resource::<DebugInfo>();
    assert!(!debug_info.show_fps);
    assert!(debug_info.show_physics_debug);
}

#[test]
fn test_vehicle_debug_toggle() {
    let mut app = setup_test_app();
    
    let mut input = app.world.resource_mut::<Input<KeyCode>>();
    input.press(KeyCode::F5);
    app.update();
    
    let debug_info = app.world.resource::<DebugInfo>();
    assert!(debug_info.show_vehicle_debug);
}

#[test]
fn test_particle_debug_toggle() {
    let mut app = setup_test_app();
    
    let mut input = app.world.resource_mut::<Input<KeyCode>>();
    input.press(KeyCode::F6);
    app.update();
    
    let debug_info = app.world.resource::<DebugInfo>();
    assert!(debug_info.show_particle_debug);
} 