mod chunk;
mod mesh;
mod generator;
mod deformation;
mod material;
pub mod generation;
pub mod settings;

pub use chunk::TerrainChunk;
pub use mesh::{create_terrain_mesh, spawn_lod_chunk_mesh};
pub use generator::TerrainGenerator;
pub use deformation::ChunkMeshMapping;
pub use material::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TerrainType {
    Grass,
    Sand,
    Rock,
    Snow,
    Water,
    Forest,
    Plains,
    Tundra,
    Beach,
    Mountain,
    Desert,
    Rainforest,
    Ocean,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BiomeType {
    Mountains,
    Desert,
    Forest,
    Water,
    Sand,
    Snow,
    Rock,
    Plains,
    Tundra,
    Rainforest,
    Beach,
    Ocean,
}

pub mod terrain_features;

use bevy::{
    prelude::*,
    render::mesh::Mesh,
};
use bevy::math::IVec2;
use bevy::pbr::{StandardMaterial, PbrBundle};
use crate::terrain::settings::{TerrainFeatureSettings, TerrainConfig, TerrainGenerationConfig};
use crate::terrain::generation::TerrainGenerationSettings;
use crate::terrain::mesh::TerrainMeshMarker;
use crate::terrain::deformation::TerrainDeformationState;
// use crate::game::components::player::Player;
use crate::terrain::deformation::update_mesh_vertices;
use crate::terrain::settings::DifficultyLevel;
use crate::terrain::generation::generate_terrain_chunk;

pub use terrain_features::*;

pub struct TerrainPlugin;

impl Plugin for TerrainPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(TerrainConfig {
            chunk_resolution: 32,
            height_scale: 50.0,
            roughness: 0.5,
            chunk_size: 32,
            vertex_scale: 1.0,
        })
        .insert_resource(TerrainGenerationConfig {
            view_distance: 500.0,
            chunk_size: 32.0,
        })
        .insert_resource(TerrainFeatureSettings {
            feature_noise_scale: 0.005,
            min_feature_spacing: 50.0,
            difficulty_distribution: vec![
                (DifficultyLevel::Easy, 0.4),
                (DifficultyLevel::Medium, 0.3),
                (DifficultyLevel::Hard, 0.2),
                (DifficultyLevel::Extreme, 0.1),
            ],
            feature_probability: 0.3,
            temperature: 0.0,
            snowfall_intensity: 0.0,
            wind_direction: Vec2::new(1.0, 0.0),
            season_factor: 0.0,
            day_night_factor: 0.5,
            snow_compaction_rate: 0.1,
            ice_formation_threshold: -5.0,
        })
        // .add_systems(Update, update_visible_chunks) // TODO: Revisit system registration after signature review
        .add_systems(Update, cleanup_unloaded_chunks)
        .add_systems(Update, update_terrain_deformation);
    }
}

impl TerrainType {
    pub fn get_color(&self) -> bevy::prelude::Color {
        match self {
            TerrainType::Grass => Color::rgb(0.3, 0.7, 0.3),
            TerrainType::Sand => Color::rgb(0.9, 0.85, 0.6),
            TerrainType::Rock => Color::rgb(0.5, 0.5, 0.5),
            TerrainType::Snow => Color::rgb(0.95, 0.95, 1.0),
            TerrainType::Water => Color::rgb(0.2, 0.4, 0.8),
            TerrainType::Forest => Color::rgb(0.1, 0.5, 0.1),
            TerrainType::Plains => Color::rgb(0.6, 0.8, 0.4),
            TerrainType::Tundra => Color::rgb(0.8, 0.8, 0.7),
            TerrainType::Beach => Color::rgb(0.95, 0.9, 0.7),
            TerrainType::Mountain => Color::rgb(0.6, 0.6, 0.6),
            TerrainType::Desert => Color::rgb(0.95, 0.9, 0.6),
            TerrainType::Rainforest => Color::rgb(0.0, 0.4, 0.0),
            TerrainType::Ocean => Color::rgb(0.1, 0.3, 0.7),
        }
    }

    pub fn get_roughness(&self) -> f32 {
        match self {
            TerrainType::Grass => 0.7,
            TerrainType::Sand => 0.8,
            TerrainType::Rock => 0.9,
            TerrainType::Snow => 0.6,
            TerrainType::Water => 0.2,
            TerrainType::Forest => 0.8,
            TerrainType::Plains => 0.7,
            TerrainType::Tundra => 0.7,
            TerrainType::Beach => 0.6,
            TerrainType::Mountain => 0.9,
            TerrainType::Desert => 0.8,
            TerrainType::Rainforest => 0.85,
            TerrainType::Ocean => 0.3,
        }
    }
}

pub fn setup_terrain(mut commands: Commands) {
    // Initialize terrain generator with a random seed
    let generator = TerrainGenerator::new(32, 42);
    commands.insert_resource(generator);
}

pub fn spawn_initial_chunks(
    mut commands: Commands,
    generator: Res<TerrainGenerator>,
    config: Res<TerrainConfig>,
    settings: Res<TerrainGenerationSettings>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<TerrainMaterial>>,
    asset_server: Res<AssetServer>,
    feature_settings: Res<TerrainFeatureSettings>,
) {
    // Spawn the central chunk and its neighbors
    for z in -1..=1 {
        for x in -1..=1 {
            let chunk_data = generate_terrain_chunk(x, z, &settings);
            spawn_terrain_chunk(
                &mut commands,
                &mut meshes,
                &mut materials,
                &asset_server,
                chunk_data,
                &feature_settings,
                IVec2::new(x, z),
                0,
                None,
            );
        }
    }
}

pub fn update_terrain_chunks(
    mut commands: Commands,
    generator: Res<TerrainGenerator>,
    config: Res<TerrainConfig>,
    settings: Res<TerrainGenerationSettings>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<TerrainMaterial>>,
    asset_server: Res<AssetServer>,
    // player_query: Query<&Transform, With<Player>>,
    chunk_query: Query<(Entity, &TerrainChunkMarker)>,
    feature_settings: Res<TerrainFeatureSettings>,
) {
    // TODO: Replace player position logic with camera or other reference
    // if let Ok(player_transform) = player_query.get_single() {
    //     let player_pos = player_transform.translation;
    //     let current_chunk_x = (player_pos.x / (config.chunk_resolution as f32)).floor() as i32;
    //     let current_chunk_z = (player_pos.z / (config.chunk_resolution as f32)).floor() as i32;
    //     // ... existing code ...
    // }
}

#[derive(Component)]
pub struct TerrainChunkMarker {
    pub x: i32,
    pub z: i32,
}

fn update_visible_chunks(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<TerrainMaterial>>,
    mut chunk_mapping: ResMut<ChunkMeshMapping>,
    generator: Res<TerrainGenerator>,
    config: Res<TerrainGenerationConfig>,
    settings: Res<TerrainGenerationSettings>,
    camera: Query<&Transform, With<Camera>>,
    chunks: Query<(Entity, &Transform, &TerrainMeshMarker)>,
    feature_settings: Res<TerrainFeatureSettings>,
    asset_server: Res<AssetServer>,
) {
    if let Ok(camera_transform) = camera.get_single() {
        let camera_pos = camera_transform.translation;
        let chunk_size = config.chunk_size;
        let view_distance = 3; // Number of chunks to load in each direction
        
        // Calculate current chunk coordinates
        let current_chunk = IVec2::new(
            (camera_pos.x / chunk_size).floor() as i32,
            (camera_pos.z / chunk_size).floor() as i32,
        );
        
        // Track which chunks should exist
        let mut desired_chunks = Vec::new();
        
        // Generate chunks in view distance
        for x in -view_distance..=view_distance {
            for z in -view_distance..=view_distance {
                let chunk_pos = current_chunk + IVec2::new(x, z);
                desired_chunks.push(chunk_pos);
                
                let chunk_pos_3d = IVec3::new(chunk_pos.x, 0, chunk_pos.y);
                
                // Check if chunk already exists
                if !chunk_mapping.chunk_to_mesh.contains_key(&chunk_pos_3d) {
                    let chunk_data = generate_terrain_chunk(chunk_pos.x, chunk_pos.y, &settings);
                    spawn_terrain_chunk(
                        &mut commands,
                        &mut meshes,
                        &mut materials,
                        &asset_server,
                        chunk_data,
                        &feature_settings,
                        chunk_pos,
                        0,
                        None,
                    );
                }
            }
        }
        
        // Mark chunks for removal if they're too far
        for (entity, _, marker) in chunks.iter() {
            let chunk_pos_2d = IVec2::new(marker.chunk_position.x, marker.chunk_position.y);
            if !desired_chunks.contains(&chunk_pos_2d) {
                commands.entity(entity).insert(DespawnChunk);
            }
        }
    }
}

#[derive(Component)]
struct DespawnChunk;

fn cleanup_unloaded_chunks(
    mut commands: Commands,
    mut chunk_mapping: ResMut<ChunkMeshMapping>,
    query: Query<(Entity, &TerrainMeshMarker), With<DespawnChunk>>,
) {
    for (entity, marker) in query.iter() {
        // Unregister the chunk from our mapping
        chunk_mapping.unregister_chunk(IVec3::new(marker.chunk_position.x, 0, marker.chunk_position.y));
        // Despawn the entity
        commands.entity(entity).despawn();
    }
}

fn update_terrain_deformation(
    mut chunks: Query<(&TerrainMeshMarker, &Handle<Mesh>)>,
    mut meshes: ResMut<Assets<Mesh>>,
    deformation_state: Res<TerrainDeformationState>,
) {
    for (marker, mesh_handle) in chunks.iter() {
        if let Some(deformations) = deformation_state.active_deformations.get(&IVec3::new(marker.chunk_position.x, 0, marker.chunk_position.y)) {
            if let Some(mesh) = meshes.get_mut(mesh_handle) {
                update_mesh_vertices(mesh, deformations);
            }
        }
    }
}

pub fn spawn_terrain_chunk(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<TerrainMaterial>,
    asset_server: &AssetServer,
    chunk: TerrainChunk,
    feature_settings: &TerrainFeatureSettings,
    chunk_pos: IVec2,
    lod_level: u8,
    neighbor_lods: Option<[u8; 4]>,
) {
    let mesh = create_terrain_mesh(
        &chunk,
        lod_level as u32,
        neighbor_lods.map(|arr| [arr[0] as u32, arr[1] as u32, arr[2] as u32, arr[3] as u32]),
    );
    
    // Determine the dominant terrain type for this chunk
    let dominant_type = determine_dominant_terrain_type(&chunk);
    
    // Create the terrain material with proper texture arrays and configuration
    let material = TerrainMaterial::create_terrain_material(
        asset_server,
        dominant_type,
        feature_settings,
    );

    let position = Vec3::new(
        chunk.position.x,
        0.0,
        chunk.position.y,
    );

    commands.spawn((
        PbrBundle {
            mesh: meshes.add(mesh),
            transform: Transform::from_translation(position),
            ..default()
        },
        materials.add(material),
        TerrainChunkMarker {
            x: chunk.position.x as i32,
            z: chunk.position.y as i32,
        },
    ));
}

pub fn determine_dominant_terrain_type(chunk: &TerrainChunk) -> TerrainType {
    // Count occurrences of each terrain type
    let mut type_counts = std::collections::HashMap::new();
    
    for terrain_type in &chunk.terrain_types {
        *type_counts.entry(terrain_type).or_insert(0) += 1;
    }
    
    // Find the most common terrain type
    type_counts
        .into_iter()
        .max_by_key(|&(_, count)| count)
        .map(|(terrain_type, _)| *terrain_type)
        .unwrap_or(TerrainType::Grass)
}

pub fn update_terrain_chunk(
    chunk: &mut TerrainChunk,
    // ... existing code ...
) {
    // ... existing code ...
}

pub mod buffer_pool;
pub mod culling;

// Re-export moved types for external use
pub use buffer_pool::MeshKey;
pub use culling::ChunkBounds; 