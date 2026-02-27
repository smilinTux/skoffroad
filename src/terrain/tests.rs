use super::*;
use bevy::app::App;
use bevy::asset::AssetPlugin;
use bevy::render::RenderPlugin;
use bevy::pbr::PbrPlugin;
use bevy::core_pipeline::core_3d::Core3dPlugin;
use bevy::window::WindowPlugin;
use bevy::prelude::{Camera3dBundle, Transform, GlobalTransform};
use bevy::render::camera::Camera3d;
use bevy::render::view::VisibilityPlugin;
use noise::MultiFractal;

#[test]
fn test_terrain_chunk_generation() {
    // Setup the app with required plugins and systems
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins((
            WindowPlugin::default(),
            AssetPlugin::default(),
            RenderPlugin::default(),
            VisibilityPlugin::default(),
            Core3dPlugin,
            PbrPlugin::default(),
            TerrainPlugin,
        ))
        .add_systems(Startup, (setup_test_player, setup_test_camera));

    // Run the startup systems
    app.update();

    // Get the chunk manager
    let chunk_manager = app.world.resource::<TerrainChunkManager>();
    assert!(chunk_manager.chunks.is_empty(), "No chunks should be generated before player movement");

    // Add a player at a known position
    let player_pos = Vec3::new(25.0, 0.0, 25.0);
    let mut player_transform = app.world.query_filtered::<&mut Transform, With<crate::game::Player>>()
        .single_mut(&mut app.world);
    *player_transform = Transform::from_translation(player_pos);

    // Run the update systems
    app.update();

    // Verify chunks were generated
    let chunk_manager = app.world.resource::<TerrainChunkManager>();
    assert!(!chunk_manager.chunks.is_empty(), "Chunks should be generated around player");

    // Test chunk position calculation
    let chunk_pos = world_pos_to_chunk(player_pos);
    assert_eq!(chunk_pos, IVec2::new(0, 0), "Player should be in chunk (0,0)");

    // Move player far away
    let far_pos = Vec3::new(1000.0, 0.0, 1000.0);
    let mut player_transform = app.world.query_filtered::<&mut Transform, With<crate::game::Player>>()
        .single_mut(&mut app.world);
    *player_transform = Transform::from_translation(far_pos);

    // Run update to trigger cleanup
    app.update();

    // Verify old chunks were cleaned up
    let chunk_manager = app.world.resource::<TerrainChunkManager>();
    assert!(!chunk_manager.chunks.contains_key(&chunk_pos), "Old chunks should be cleaned up");
}

#[test]
fn test_terrain_height_generation() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins((
            WindowPlugin::default(),
            AssetPlugin::default(),
            RenderPlugin::default(),
            VisibilityPlugin::default(),
            Core3dPlugin,
            PbrPlugin::default(),
            TerrainPlugin,
        ))
        .add_systems(Startup, setup_test_camera);

    // Run startup systems
    app.update();

    let settings = TerrainSettings {
        noise_scale: 0.02,
        height_multiplier: 15.0,
        roughness: 0.55,
        persistence: 0.5,
        octaves: 6,
    };

    let noise: Fbm<Perlin> = Fbm::new(0)
        .set_octaves(settings.octaves)
        .set_persistence(settings.persistence.into())
        .set_lacunarity((settings.roughness * 2.0).into());

    // Test height at different positions
    let test_positions = vec![
        (0.0, 0.0),
        (25.0, 25.0),
        (-25.0, -25.0),
        (50.0, 0.0),
    ];

    for (x, z) in test_positions {
        let height = noise.get([
            x as f64 * settings.noise_scale as f64,
            z as f64 * settings.noise_scale as f64,
        ]) as f32 * settings.height_multiplier;

        assert!(height.is_finite(), "Height should be a finite number");
        assert!(height.abs() <= settings.height_multiplier, "Height should be within multiplier bounds");
    }
}

fn setup_test_player(mut commands: Commands) {
    commands.spawn((
        crate::game::Player { health: 100.0 },
        Transform::default(),
        GlobalTransform::default(),
    ));
}

fn setup_test_camera(mut commands: Commands) {
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(0.0, 10.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
} 