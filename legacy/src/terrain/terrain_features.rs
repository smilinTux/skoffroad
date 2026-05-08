use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use noise::{NoiseFn, Perlin};
use rand;
use rand::Rng;
use crate::terrain::{TerrainType, TerrainChunk, BiomeType};
use crate::terrain::settings::{DifficultyLevel, TerrainFeatureSettings};
// use crate::terrain::feature_types::{TerrainFeatureType, TerrainFeature, TerrainFeatureMarker};

pub struct TerrainFeaturePlugin;

impl Plugin for TerrainFeaturePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(TerrainFeatureSettings::default())
           .add_systems(Update, generate_terrain_features);
    }
}

#[derive(Clone, Debug)]
pub enum TerrainFeatureType {
    Snowfield {
        depth: f32,
        powder_factor: f32,
        ice_patches: bool,
        compaction: f32,
        surface_hardness: f32,
        temperature: f32,
    },
    SnowDrift {
        height: f32,
        length: f32,
        wind_direction: Vec2,
        stability: f32,
        density_gradient: f32,
        age: f32,
    },
    RockCrawling {
        rock_density: f32,
        rock_size_range: (f32, f32),
    },
    WaterCrossing {
        width: f32,
        depth: f32,
        current_speed: f32,
    },
    HillClimb {
        steepness: f32,
        length: f32,
        switchbacks: u32,
    },
    MudPit {
        viscosity: f32,
        depth: f32,
        area: Vec2,
    },
    IceFormation {
        thickness: f32,
        roughness: f32,
        temperature: f32,
    },
}

#[derive(Clone, Debug)]
pub struct TerrainFeature {
    pub feature_type: TerrainFeatureType,
    pub difficulty: DifficultyLevel,
    pub position: Vec3,
    pub size: Vec3,
    pub rotation: Quat,
    pub metadata: TerrainFeatureMetadata,
}

#[derive(Clone, Debug)]
pub struct TerrainFeatureMetadata {
    pub name: String,
    pub description: String,
    pub recommended_vehicle_type: String,
    pub completion_reward: u32,
}

fn generate_terrain_features(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    settings: Res<TerrainFeatureSettings>,
    chunks: Query<(Entity, &TerrainChunk)>,
) {
    let feature_noise = Perlin::new(42);

    for (entity, chunk) in chunks.iter() {
        let noise_val = feature_noise.get([
            chunk.position.x as f64 * settings.feature_noise_scale as f64,
            chunk.position.y as f64 * settings.feature_noise_scale as f64,
        ]) as f32;

        if noise_val > 1.0 - settings.feature_probability {
            let feature = generate_feature_for_chunk(&settings, chunk, noise_val);
            
            if let Some(feature) = feature {
                apply_feature_to_terrain(
                    &asset_server,
                    &settings,
                    &mut commands,
                    entity,
                    chunk,
                    &feature,
                );
            }
        }
    }
}

fn generate_feature_for_chunk(
    settings: &TerrainFeatureSettings,
    chunk: &TerrainChunk,
    noise_val: f32,
) -> Option<TerrainFeature> {
    let difficulty = determine_difficulty(settings, noise_val);
    
    let feature_type = match (difficulty, noise_val) {
        // Add snow-based features when temperature is low
        (_, n) if settings.temperature < 0.0 && n > 0.8 => TerrainFeatureType::Snowfield {
            depth: 1.0 + (settings.snowfall_intensity * 2.0),
            powder_factor: 0.8,
            ice_patches: settings.temperature < -5.0,
            compaction: 0.0,
            surface_hardness: 0.0,
            temperature: settings.temperature,
        },
        (DifficultyLevel::Hard, n) if settings.temperature < 0.0 && n > 0.6 => TerrainFeatureType::SnowDrift {
            height: 2.0 + (settings.snowfall_intensity * 3.0),
            length: 15.0,
            wind_direction: settings.wind_direction,
            stability: 0.0,
            density_gradient: 0.0,
            age: 0.0,
        },
        (DifficultyLevel::Extreme, _) => TerrainFeatureType::RockCrawling {
            rock_density: 0.8,
            rock_size_range: (2.0, 5.0),
        },
        (DifficultyLevel::Hard, n) if n > 0.7 => TerrainFeatureType::HillClimb {
            steepness: 0.8,
            length: 100.0,
            switchbacks: 3,
        },
        (DifficultyLevel::Medium, n) if n < 0.3 => TerrainFeatureType::WaterCrossing {
            width: 15.0,
            depth: 2.0,
            current_speed: 2.0,
        },
        (_, _) => TerrainFeatureType::MudPit {
            viscosity: 0.6,
            depth: 1.5,
            area: Vec2::new(20.0, 20.0),
        },
    };

    Some(TerrainFeature {
        feature_type,
        difficulty,
        position: Vec3::new(chunk.position.x, 0.0, chunk.position.y),
        size: Vec3::new(30.0, 10.0, 30.0),
        rotation: Quat::IDENTITY,
        metadata: TerrainFeatureMetadata {
            name: "Generated Feature".to_string(),
            description: "A challenging terrain feature".to_string(),
            recommended_vehicle_type: "Off-road".to_string(),
            completion_reward: 100,
        },
    })
}

fn determine_difficulty(settings: &TerrainFeatureSettings, noise_val: f32) -> DifficultyLevel {
    let mut cumulative_prob = 0.0;
    for (difficulty, prob) in &settings.difficulty_distribution {
        cumulative_prob += prob;
        if noise_val <= cumulative_prob {
            return *difficulty;
        }
    }
    DifficultyLevel::Easy
}

fn apply_feature_to_terrain(
    asset_server: &AssetServer,
    settings: &TerrainFeatureSettings,
    commands: &mut Commands,
    chunk_entity: Entity,
    chunk: &TerrainChunk,
    feature: &TerrainFeature,
) {
    match &feature.feature_type {
        TerrainFeatureType::Snowfield { depth, powder_factor, ice_patches, compaction, surface_hardness, temperature } => {
            apply_snowfield(asset_server, settings, commands, chunk_entity, chunk, *depth, *powder_factor, *ice_patches, *compaction, *surface_hardness, *temperature);
        },
        TerrainFeatureType::SnowDrift { height, length, wind_direction, stability, density_gradient, age } => {
            apply_snow_drift(asset_server, settings, commands, chunk_entity, chunk, *height, *length, *wind_direction, *stability, *density_gradient, *age);
        },
        TerrainFeatureType::RockCrawling { rock_density, rock_size_range } => {
            apply_rock_crawling(asset_server, settings, commands, chunk_entity, chunk, *rock_density, *rock_size_range);
        }
        TerrainFeatureType::WaterCrossing { width, depth, current_speed } => {
            apply_water_crossing(asset_server, settings, commands, chunk_entity, chunk, *width, *depth, *current_speed);
        }
        TerrainFeatureType::HillClimb { steepness, length, switchbacks } => {
            apply_hill_climb(asset_server, settings, commands, chunk_entity, chunk, *steepness, *length, *switchbacks);
        }
        TerrainFeatureType::MudPit { viscosity, depth, area } => {
            apply_mud_pit(asset_server, settings, commands, chunk_entity, chunk, *viscosity, *depth, *area);
        }
        TerrainFeatureType::IceFormation { thickness, roughness, temperature } => {
            apply_ice_formation(asset_server, settings, commands, chunk_entity, chunk, *thickness, *roughness, *temperature);
        }
    }
}

fn apply_rock_crawling(
    asset_server: &AssetServer,
    settings: &TerrainFeatureSettings,
    commands: &mut Commands,
    chunk_entity: Entity,
    chunk: &TerrainChunk,
    density: f32,
    size_range: (f32, f32),
) {
    // Create rock formations using noise-based placement
    let noise = Perlin::new(43);
    let (min_size, max_size) = size_range;

    for x in 0..chunk.height_map.len() {
        let noise_val = noise.get([
            (chunk.position.x + x as f32) as f64 * 0.1,
            chunk.position.y as f64 * 0.1,
        ]) as f32;

        if noise_val > 1.0 - density {
            let size = min_size + (max_size - min_size) * noise_val;
            let rock_collider = Collider::ball(size);
            let rock_material = crate::terrain::material::TerrainMaterial::default();
            let rock_physics = PhysicsMaterial::default();
            commands.spawn(TerrainFeatureBundle {
                collider: rock_collider,
                material: rock_material,
                physics_material: rock_physics,
                transform_bundle: TransformBundle::from_transform(
                    Transform::from_xyz(
                        chunk.position.x + x as f32,
                        chunk.height_map[x],
                        chunk.position.y,
                    )
                ),
                rigid_body: RigidBody::Fixed,
            });
        }
    }
}

fn apply_water_crossing(
    asset_server: &AssetServer,
    settings: &TerrainFeatureSettings,
    commands: &mut Commands,
    chunk_entity: Entity,
    chunk: &TerrainChunk,
    width: f32,
    depth: f32,
    current_speed: f32,
) {
    // Create water volume and modify terrain for river banks
    let water_collider = Collider::cuboid(width / 2.0, depth / 2.0, chunk.height_map.len() as f32 / 2.0);
    
    commands.spawn((
        water_collider,
        TransformBundle::from_transform(
            Transform::from_xyz(
                chunk.position.x + width / 2.0,
                chunk.position.y - depth / 2.0,
                chunk.position.y + chunk.height_map.len() as f32 / 2.0,
            )
        ),
        RigidBody::Fixed,
    ));
}

fn apply_hill_climb(
    asset_server: &AssetServer,
    settings: &TerrainFeatureSettings,
    commands: &mut Commands,
    chunk_entity: Entity,
    chunk: &TerrainChunk,
    steepness: f32,
    length: f32,
    switchbacks: u32,
) {
    // Modify terrain heights to create switchback path
    let path_width = 5.0;
    let switchback_length = length / switchbacks as f32;
    
    // Add guide markers and difficulty indicators
    commands.spawn((
        Collider::cylinder(2.0, 0.5),
        TransformBundle::from_transform(
            Transform::from_xyz(
                chunk.position.x,
                chunk.height_map[0],
                chunk.position.y,
            )
        ),
    ));
}

fn apply_mud_pit(
    asset_server: &AssetServer,
    settings: &TerrainFeatureSettings,
    commands: &mut Commands,
    chunk_entity: Entity,
    chunk: &TerrainChunk,
    viscosity: f32,
    depth: f32,
    area: Vec2,
) {
    let mud_material = crate::terrain::material::TerrainMaterial::default();
    let mud_physics = PhysicsMaterial {
        friction: 0.2,
        density: 1.2,
        restitution: 0.1,
    };
    // Create mud volume with physics material
    let mud_collider = Collider::cuboid(area.x / 2.0, depth / 2.0, area.y / 2.0);
    commands.spawn(TerrainFeatureBundle {
        collider: mud_collider,
        material: mud_material,
        physics_material: mud_physics,
        transform_bundle: TransformBundle::from_transform(
            Transform::from_xyz(
                chunk.position.x + area.x / 2.0,
                chunk.position.y - depth / 2.0,
                chunk.position.y + area.y / 2.0,
            )
        ),
        rigid_body: RigidBody::Fixed,
    });
}

fn apply_snowfield(
    asset_server: &AssetServer,
    settings: &TerrainFeatureSettings,
    commands: &mut Commands,
    chunk_entity: Entity,
    chunk: &TerrainChunk,
    depth: f32,
    powder_factor: f32,
    ice_patches: bool,
    compaction: f32,
    surface_hardness: f32,
    temperature: f32,
) {
    let mut rng = rand::thread_rng();
    
    // Calculate snow properties based on environmental conditions
    let surface_hardness = if temperature < settings.ice_formation_threshold {
        0.8 + (rng.gen::<f32>() * 0.2) // Harder surface when very cold
    } else {
        0.2 + (powder_factor * 0.3) + (compaction * 0.5)
    };

    let mut base_physics = PhysicsMaterial {
        friction: 0.3,
        density: 0.4,
        restitution: 0.1,
    };

    let mut surface_physics = PhysicsMaterial {
        friction: 0.1 + (surface_hardness * 0.3),
        density: 0.2 + (compaction * 0.4),
        restitution: 0.15,
    };

    if ice_patches {
        let mut snow_physics = PhysicsMaterial {
            friction: 0.05,
            density: 0.9,
            restitution: 0.2,
        };
        // ... rest of the function ...
    }

    // Create layered snow colliders for more realistic physics
    let base_snow = Collider::cuboid(15.0, depth * 0.7, 15.0);
    let surface_snow = Collider::cuboid(15.0, depth * 0.3, 15.0);
    
    // Create materials
    let base_material = crate::terrain::material::TerrainMaterial::create_terrain_material(asset_server, TerrainType::Snow, settings);
    
    let surface_material = crate::terrain::material::TerrainMaterial::create_terrain_material(asset_server, TerrainType::Snow, settings);
    
    // Spawn base layer
    commands.spawn(TerrainFeatureBundle {
        collider: base_snow,
        physics_material: base_physics,
        material: base_material,
        transform_bundle: TransformBundle::from_transform(
            Transform::from_xyz(
                chunk.position.x + 15.0,
                chunk.position.y + (depth * 0.35),
                chunk.position.y + 15.0,
            )
        ),
        rigid_body: RigidBody::Fixed,
    });
    
    // Spawn surface layer
    commands.spawn(TerrainFeatureBundle {
        collider: surface_snow,
        physics_material: surface_physics,
        material: surface_material,
        transform_bundle: TransformBundle::from_transform(
            Transform::from_xyz(
                chunk.position.x + 15.0,
                chunk.position.y + (depth * 0.85),
                chunk.position.y + 15.0,
            )
        ),
        rigid_body: RigidBody::Fixed,
    });
}

fn apply_snow_drift(
    asset_server: &AssetServer,
    settings: &TerrainFeatureSettings,
    commands: &mut Commands,
    chunk_entity: Entity,
    chunk: &TerrainChunk,
    height: f32,
    length: f32,
    wind_direction: Vec2,
    stability: f32,
    density_gradient: f32,
    age: f32,
) {
    let mut rng = rand::thread_rng();
    // Calculate drift properties
    let age = settings.season_factor * 10.0; // Older drifts in later winter
    let stability = (age * 0.1).min(0.8) + (rng.gen::<f32>() * 0.2);
    let density_gradient = (1.0 - stability) * 0.5;
    // Create multiple layers for the drift with varying properties
    let layers = 3;
    for i in 0..layers {
        let layer_height = height * (1.0 - (i as f32 / layers as f32));
        let layer_length = length * (1.0 - (i as f32 / layers as f32) * 0.2);
        let drift_collider = Collider::cuboid(layer_length / 2.0, layer_height / 2.0, 5.0);
        let drift_material = crate::terrain::material::TerrainMaterial::default();
        let drift_physics = PhysicsMaterial {
            friction: 0.2 + (stability * 0.4),
            density: 0.5 + (density_gradient * (i as f32 / layers as f32)),
            restitution: 0.1 + ((1.0 - stability) * 0.2),
        };
        // Calculate drift angle based on wind
        let angle = wind_direction.y.atan2(wind_direction.x);
        let rotation = Quat::from_rotation_y(angle);
        commands.spawn(TerrainFeatureBundle {
            collider: drift_collider,
            material: drift_material,
            physics_material: drift_physics,
            transform_bundle: TransformBundle::from_transform(
                Transform::from_xyz(
                    chunk.position.x + layer_length / 2.0,
                    chunk.position.y + (i as f32 * layer_height / layers as f32),
                    chunk.position.y + 5.0,
                ).with_rotation(rotation)
            ),
            rigid_body: RigidBody::Fixed,
        });
    }
}

fn apply_ice_formation(
    asset_server: &AssetServer,
    settings: &TerrainFeatureSettings,
    commands: &mut Commands,
    chunk_entity: Entity,
    chunk: &TerrainChunk,
    thickness: f32,
    roughness: f32,
    temperature: f32,
) {
    // Implementation of ice formation feature
}

#[derive(Component, Clone, Debug)]
pub struct PhysicsMaterial {
    pub density: f32,
    pub friction: f32,
    pub restitution: f32,
}

impl Default for PhysicsMaterial {
    fn default() -> Self {
        Self {
            density: 1.0,
            friction: 0.5,
            restitution: 0.1,
        }
    }
}

#[derive(Bundle)]
struct TerrainFeatureBundle {
    collider: Collider,
    physics_material: PhysicsMaterial,
    material: crate::terrain::material::TerrainMaterial,
    transform_bundle: TransformBundle,
    rigid_body: RigidBody,
}

fn spawn_mud_patch(
    commands: &mut Commands,
    mud_collider: Collider,
    mud_material: crate::terrain::material::TerrainMaterial,
    transform: Transform,
) {
    commands.spawn(TerrainFeatureBundle {
        collider: mud_collider,
        material: mud_material,
        transform_bundle: TransformBundle::from_transform(transform),
        rigid_body: RigidBody::Fixed,
        physics_material: PhysicsMaterial::default(),
    });
}

fn spawn_snow_patch(
    commands: &mut Commands,
    base_snow: Collider,
    base_material: crate::terrain::material::TerrainMaterial,
    transform: Transform,
) {
    commands.spawn(TerrainFeatureBundle {
        collider: base_snow,
        material: base_material,
        transform_bundle: TransformBundle::from_transform(transform),
        rigid_body: RigidBody::Fixed,
        physics_material: PhysicsMaterial::default(),
    });
}

fn spawn_surface_snow(
    commands: &mut Commands,
    surface_snow: Collider,
    surface_material: crate::terrain::material::TerrainMaterial,
    transform: Transform,
) {
    commands.spawn(TerrainFeatureBundle {
        collider: surface_snow,
        material: surface_material,
        transform_bundle: TransformBundle::from_transform(transform),
        rigid_body: RigidBody::Fixed,
        physics_material: PhysicsMaterial::default(),
    });
} 