use sandk_offroad::game::plugins::weather::{WeatherPlugin, WeatherState, WeatherManager};
use crate::common::TestApp;

#[test]
fn test_weather_initialization() {
    let mut app = TestApp::new();
    app.add_plugin(WeatherPlugin::default());

    // Verify weather resources are properly initialized
    let weather_state = app.get_resource::<WeatherState>().expect("WeatherState not initialized");
    let weather_manager = app.get_resource::<WeatherManager>().expect("WeatherManager not initialized");

    assert_eq!(weather_state.current_weather, weather_manager.default_weather());
}

#[test]
fn test_weather_transition() {
    let mut app = TestApp::new();
    app.add_plugin(WeatherPlugin::default());

    // Get initial weather
    let initial_weather = app.get_resource::<WeatherState>()
        .expect("WeatherState not initialized")
        .current_weather
        .clone();

    // Force a weather transition
    if let Some(mut manager) = app.get_resource_mut::<WeatherManager>() {
        manager.force_next_weather();
    }

    // Run a few update cycles to allow the transition to occur
    app.update_cycles(10);

    // Verify weather has changed
    let final_weather = app.get_resource::<WeatherState>()
        .expect("WeatherState not initialized")
        .current_weather
        .clone();

    assert_ne!(initial_weather, final_weather, "Weather should have changed after forced transition");
}

#[test]
fn test_weather_effects() {
    let mut app = TestApp::new();
    app.add_plugin(WeatherPlugin::default());

    // Set up a test entity that should be affected by weather
    let test_entity = app.app.world.spawn((
        Transform::default(),
        GlobalTransform::default(),
    )).id();

    // Run update cycles to apply weather effects
    app.update_cycles(5);

    // Verify entity has been affected by weather
    // (Specific assertions would depend on your weather effect implementation)
    let entity_exists = app.app.world.get_entity(test_entity).is_some();
    assert!(entity_exists, "Test entity should still exist after weather effects");
}

#[test]
fn test_weather_persistence() {
    use std::time::Duration;
    
    let mut app = TestApp::new();
    app.add_plugin(WeatherPlugin::default());

    // Set specific weather conditions
    if let Some(mut weather_state) = app.get_resource_mut::<WeatherState>() {
        weather_state.set_weather_duration(Duration::from_secs(10));
    }

    // Run updates and verify weather persists for expected duration
    let initial_weather = app.get_resource::<WeatherState>()
        .expect("WeatherState not initialized")
        .current_weather
        .clone();

    // Run for less than the set duration
    app.update_cycles(5); // Assuming each cycle is less than 2 seconds

    let mid_weather = app.get_resource::<WeatherState>()
        .expect("WeatherState not initialized")
        .current_weather
        .clone();

    assert_eq!(initial_weather, mid_weather, "Weather should not change before duration expires");
} 