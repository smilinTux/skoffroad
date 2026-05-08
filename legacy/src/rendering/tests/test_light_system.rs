use bevy::prelude::*;
use bevy::render::camera::Camera;
use bevy::render::render_resource::*;
use bevy::render::renderer::RenderDevice;
use bevy::render::RenderApp;
use bevy::asset::Assets;

use crate::rendering::light_manager::{LightManager, Light, LightType};
use crate::rendering::light_plugin::{LightPlugin, create_directional_light, create_point_light, create_spot_light};

#[test]
fn test_light_plugin_setup() {
    let mut app = App::new();
    app.add_plugins(LightPlugin);
    
    // Verify plugin setup
    assert!(app.world.get_resource::<LightManager>().is_none()); // Should be in render app
    let render_app = app.sub_app_mut(RenderApp);
    assert!(render_app.world.get_resource::<LightManager>().is_some());
}

#[test]
fn test_directional_light_creation() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(LightPlugin);
    
    let mut commands = app.world.spawn_empty().id();
    
    let transform = Transform::from_xyz(0.0, 10.0, 0.0)
        .looking_at(Vec3::ZERO, Vec3::Y);
    
    let light_entity = create_directional_light(
        &mut app.world.commands(),
        Vec3::new(1.0, 1.0, 1.0), // White light
        1.0, // Full intensity
        transform,
    );
    
    let light = app.world.get::<Light>(light_entity).unwrap();
    assert_eq!(light.params.light_type, 0); // Directional type
    assert_eq!(light.params.color, Vec3::new(1.0, 1.0, 1.0));
    assert_eq!(light.params.intensity, 1.0);
}

#[test]
fn test_point_light_creation() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(LightPlugin);
    
    let transform = Transform::from_xyz(1.0, 2.0, 3.0);
    
    let light_entity = create_point_light(
        &mut app.world.commands(),
        Vec3::new(1.0, 0.0, 0.0), // Red light
        2.0, // Double intensity
        5.0, // Range
        transform,
    );
    
    let light = app.world.get::<Light>(light_entity).unwrap();
    assert_eq!(light.params.light_type, 1); // Point type
    assert_eq!(light.params.color, Vec3::new(1.0, 0.0, 0.0));
    assert_eq!(light.params.intensity, 2.0);
    assert_eq!(light.params.range, 5.0);
}

#[test]
fn test_spot_light_creation() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(LightPlugin);
    
    let transform = Transform::from_xyz(0.0, 5.0, 0.0)
        .looking_at(Vec3::ZERO, Vec3::Y);
    
    let light_entity = create_spot_light(
        &mut app.world.commands(),
        Vec3::new(0.0, 1.0, 0.0), // Green light
        1.5, // 1.5x intensity
        10.0, // Range
        45.0, // 45 degree angle
        transform,
    );
    
    let light = app.world.get::<Light>(light_entity).unwrap();
    assert_eq!(light.params.light_type, 2); // Spot type
    assert_eq!(light.params.color, Vec3::new(0.0, 1.0, 0.0));
    assert_eq!(light.params.intensity, 1.5);
    assert_eq!(light.params.range, 10.0);
    assert!((light.params.spot_angle_cos - (22.5_f32.to_radians()).cos()).abs() < 0.001);
}

#[test]
fn test_light_update_system() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(LightPlugin);
    
    // Create a light
    let transform = Transform::from_xyz(1.0, 2.0, 3.0)
        .looking_at(Vec3::ZERO, Vec3::Y);
    let light_entity = create_directional_light(
        &mut app.world.commands(),
        Vec3::ONE,
        1.0,
        transform,
    );
    
    // Run systems
    app.update();
    
    // Verify light position and direction updated
    let light = app.world.get::<Light>(light_entity).unwrap();
    assert_eq!(light.params.position, Vec3::new(1.0, 2.0, 3.0));
    assert!((light.params.direction - -transform.forward()).length() < 0.001);
}

#[test]
fn test_shadow_matrix_updates() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(LightPlugin);
    
    let transform = Transform::from_xyz(0.0, 10.0, 0.0)
        .looking_at(Vec3::ZERO, Vec3::Y);
    
    let mut light_manager = LightManager::new(
        app.world.resource::<RenderDevice>().clone(),
        app.world.resource::<RenderQueue>().clone(),
        app.world.resource::<Assets<Image>>().clone(),
    );
    
    let mut light = light_manager.create_light(LightType::Directional);
    light.params.cast_shadows = 1;
    
    let light_entity = app.world.spawn((
        light,
        transform,
        GlobalTransform::default(),
    )).id();
    
    // Run systems
    app.update();
    
    // Verify shadow matrices updated
    let light = app.world.get::<Light>(light_entity).unwrap();
    assert!(light.shadow_view != Mat4::IDENTITY);
    assert!(light.shadow_proj != Mat4::IDENTITY);
} 