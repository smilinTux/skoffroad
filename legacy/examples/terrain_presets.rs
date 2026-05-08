use bevy::prelude::*;
use sandk_offroad::{
    game::terrain::{
        TerrainBundle, TerrainConfig, TerrainPlugin, TerrainPreset,
        NoiseLayer, TerrainFeatureType,
    },
};

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, TerrainPlugin))
        .add_systems(Startup, setup)
        .add_systems(Update, (
            handle_input,
            update_terrain_preview,
        ))
        .run();
}

// Resource to track current terrain preset
#[derive(Resource)]
struct CurrentPreset {
    preset: TerrainPreset,
    terrain_entity: Option<Entity>,
}

impl Default for CurrentPreset {
    fn default() -> Self {
        Self {
            preset: TerrainPreset::Mountains,
            terrain_entity: None,
        }
    }
}

fn setup(mut commands: Commands) {
    // Add camera
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(-50.0, 50.0, -50.0)
            .looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });

    // Add light
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            illuminance: 10000.0,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_rotation(Quat::from_euler(
            EulerRot::XYZ,
            -45.0_f32.to_radians(),
            45.0_f32.to_radians(),
            0.0,
        )),
        ..default()
    });

    // Initialize current preset resource
    commands.insert_resource(CurrentPreset::default());

    // Create initial terrain
    spawn_terrain(&mut commands, TerrainPreset::Mountains);
}

fn spawn_terrain(commands: &mut Commands, preset: TerrainPreset) -> Entity {
    // Create terrain configuration from preset
    let config = TerrainConfig::new_preset(preset);
    
    // Spawn terrain
    commands.spawn((
        TerrainBundle::new(config),
        Name::new(format!("{:?} Terrain", preset)),
    )).id()
}

fn handle_input(
    keyboard: Res<Input<KeyCode>>,
    mut current_preset: ResMut<CurrentPreset>,
    mut commands: Commands,
) {
    // Change terrain preset with number keys
    let new_preset = if keyboard.just_pressed(KeyCode::Key1) {
        Some(TerrainPreset::Mountains)
    } else if keyboard.just_pressed(KeyCode::Key2) {
        Some(TerrainPreset::Desert)
    } else if keyboard.just_pressed(KeyCode::Key3) {
        Some(TerrainPreset::RiverValley)
    } else if keyboard.just_pressed(KeyCode::Key4) {
        Some(TerrainPreset::Volcanic)
    } else if keyboard.just_pressed(KeyCode::Key5) {
        Some(TerrainPreset::Coastal)
    } else if keyboard.just_pressed(KeyCode::Key6) {
        Some(TerrainPreset::Arctic)
    } else if keyboard.just_pressed(KeyCode::Key7) {
        Some(TerrainPreset::Canyonlands)
    } else if keyboard.just_pressed(KeyCode::Key8) {
        Some(TerrainPreset::Hills)
    } else if keyboard.just_pressed(KeyCode::Key9) {
        Some(TerrainPreset::Islands)
    } else if keyboard.just_pressed(KeyCode::Key0) {
        Some(TerrainPreset::Badlands)
    } else {
        None
    };

    if let Some(preset) = new_preset {
        // Remove old terrain
        if let Some(entity) = current_preset.terrain_entity {
            commands.entity(entity).despawn_recursive();
        }

        // Spawn new terrain
        let new_entity = spawn_terrain(&mut commands, preset);
        
        // Update current preset
        current_preset.preset = preset;
        current_preset.terrain_entity = Some(new_entity);
    }
}

fn update_terrain_preview(
    current_preset: Res<CurrentPreset>,
    mut gizmos: Gizmos,
) {
    // Draw preview information
    gizmos.text_3d(
        Vec3::new(0.0, 50.0, 0.0),
        Color::WHITE,
        format!("Current Preset: {:?}", current_preset.preset),
    );
}

// Example of creating a mixed terrain scene
fn create_mixed_terrain_example(mut commands: Commands) {
    // Create a mountainous region
    let mut mountain_config = TerrainConfig::new_preset(TerrainPreset::Mountains);
    mountain_config.size = Vec2::new(500.0, 500.0);
    commands.spawn((
        TerrainBundle::new(mountain_config),
        Transform::from_xyz(-250.0, 0.0, -250.0),
        Name::new("Mountain Region"),
    ));

    // Create a desert region
    let mut desert_config = TerrainConfig::new_preset(TerrainPreset::Desert);
    desert_config.size = Vec2::new(500.0, 500.0);
    commands.spawn((
        TerrainBundle::new(desert_config),
        Transform::from_xyz(250.0, 0.0, -250.0),
        Name::new("Desert Region"),
    ));

    // Create a coastal region
    let mut coastal_config = TerrainConfig::new_preset(TerrainPreset::Coastal);
    coastal_config.size = Vec2::new(500.0, 500.0);
    commands.spawn((
        TerrainBundle::new(coastal_config),
        Transform::from_xyz(-250.0, 0.0, 250.0),
        Name::new("Coastal Region"),
    ));

    // Create a volcanic region
    let mut volcanic_config = TerrainConfig::new_preset(TerrainPreset::Volcanic);
    volcanic_config.size = Vec2::new(500.0, 500.0);
    commands.spawn((
        TerrainBundle::new(volcanic_config),
        Transform::from_xyz(250.0, 0.0, 250.0),
        Name::new("Volcanic Region"),
    ));
}

// Example of creating a custom terrain preset
fn create_custom_terrain_example(mut commands: Commands) {
    // Start with the mountain preset as a base
    let mut config = TerrainConfig::new_preset(TerrainPreset::Mountains);
    
    // Customize basic parameters
    config.height_scale = 300.0;
    config.frequency = 0.002;
    config.persistence = 0.65;
    config.lacunarity = 2.2;
    
    // Add custom noise layers
    config.additional_layers.push(NoiseLayer {
        feature_type: TerrainFeatureType::Peak,
        frequency: 0.004,
        amplitude: 0.4,
        octaves: 4,
        persistence: 0.5,
        lacunarity: 2.0,
        enable_warping: true,
        warp_strength: 25.0,
        mask_frequency: 0.002,
        threshold: 0.7,
        smoothing: 0.15,
        erosion_iterations: 3,
    });

    config.additional_layers.push(NoiseLayer {
        feature_type: TerrainFeatureType::Canyon,
        frequency: 0.003,
        amplitude: 0.3,
        octaves: 3,
        persistence: 0.45,
        lacunarity: 2.1,
        enable_warping: true,
        warp_strength: 15.0,
        mask_frequency: 0.0015,
        threshold: 0.4,
        smoothing: 0.2,
        erosion_iterations: 2,
    });

    // Spawn the custom terrain
    commands.spawn((
        TerrainBundle::new(config),
        Name::new("Custom Mixed Terrain"),
    ));
} 