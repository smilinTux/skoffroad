use bevy::prelude::*;
use crate::terrain::TerrainType;
use crate::terrain::BiomeType;
use crate::terrain::{MeshKey, ChunkBounds};
use std::collections::HashMap;
use std::collections::HashSet;
use crate::terrain::settings::TerrainFeatureSettings;
use crate::terrain::generation::TerrainGenerationSettings;
use bevy::pbr::StandardMaterial;

#[derive(Component, Debug)]
pub struct TerrainChunk {
    pub mesh: Handle<Mesh>,
    pub material: Handle<StandardMaterial>,
    pub transform: Transform,
    pub position: Vec2,
    pub size: f32,
    pub height_map: Vec<f32>,
    pub biome_map: HashMap<(i32, i32), BiomeType>,
    pub lod_level: u32,
    pub bounds: ChunkBounds,
    pub temperature: Vec<f32>,
    pub moisture: Vec<f32>,
    pub snow_coverage: Vec<f32>,
    pub needs_update: bool,
    pub mesh_key: Option<MeshKey>,
    pub terrain_types: Vec<TerrainType>,
    pub wind_speed: Vec<f32>,
    pub wind_direction: Vec<Vec2>,
    pub erosion: Vec<f32>,
    pub vegetation: Vec<f32>,
}

impl TerrainChunk {
    pub fn new(
        mesh: Handle<Mesh>,
        material: Handle<StandardMaterial>,
        position: Vec2,
        size: f32,
    ) -> Self {
        let total_size = size as usize * size as usize;
        Self {
            mesh,
            material,
            transform: Transform::from_xyz(position.x, 0.0, position.y),
            position,
            size,
            height_map: vec![0.0; total_size],
            biome_map: HashMap::new(),
            lod_level: 0,
            bounds: ChunkBounds::from_min_max(
                Vec3::new(position.x, 0.0, position.y),
                Vec3::new(position.x + size, 0.0, position.y + size),
            ),
            temperature: vec![0.0; total_size],
            moisture: vec![0.0; total_size],
            snow_coverage: vec![0.0; total_size],
            needs_update: true,
            mesh_key: None,
            terrain_types: vec![TerrainType::Grass; total_size],
            wind_speed: vec![0.0; total_size],
            wind_direction: vec![Vec2::ZERO; total_size],
            erosion: vec![0.0; total_size],
            vegetation: vec![0.0; total_size],
        }
    }

    pub fn new_default(x: i32, z: i32, size: usize) -> Self {
        let position = Vec2::new(x as f32 * size as f32, z as f32 * size as f32);
        let total_size = size * size;
        Self {
            mesh: Handle::default(),
            material: Handle::default(),
            transform: Transform::from_xyz(position.x, 0.0, position.y),
            position,
            size: size as f32,
            height_map: vec![0.0; total_size],
            biome_map: HashMap::new(),
            lod_level: 0,
            bounds: ChunkBounds::from_min_max(
                Vec3::new(position.x, 0.0, position.y),
                Vec3::new(position.x + size as f32, 0.0, position.y + size as f32),
            ),
            temperature: vec![0.0; total_size],
            moisture: vec![0.0; total_size],
            snow_coverage: vec![0.0; total_size],
            needs_update: true,
            mesh_key: None,
            terrain_types: vec![TerrainType::Grass; total_size],
            wind_speed: vec![0.0; total_size],
            wind_direction: vec![Vec2::ZERO; total_size],
            erosion: vec![0.0; total_size],
            vegetation: vec![0.0; total_size],
        }
    }

    pub fn get_height(&self, x: usize, z: usize) -> f32 {
        let idx = z * self.size as usize + x;
        self.height_map[idx]
    }

    pub fn set_height(&mut self, x: usize, z: usize, height: f32) {
        let idx = z * self.size as usize + x;
        self.height_map[idx] = height;
        self.needs_update = true;
    }

    pub fn get_temperature(&self, x: usize, z: usize) -> f32 {
        let idx = z * self.size as usize + x;
        self.temperature[idx]
    }

    pub fn set_temperature(&mut self, x: usize, z: usize, temp: f32) {
        let idx = z * self.size as usize + x;
        self.temperature[idx] = temp;
    }

    pub fn get_moisture(&self, x: usize, z: usize) -> f32 {
        let idx = z * self.size as usize + x;
        self.moisture[idx]
    }

    pub fn set_moisture(&mut self, x: usize, z: usize, moisture: f32) {
        let idx = z * self.size as usize + x;
        self.moisture[idx] = moisture;
    }

    pub fn get_biome(&self, x: usize, z: usize) -> BiomeType {
        self.biome_map[&(x as i32, z as i32)]
    }

    pub fn set_biome(&mut self, x: usize, z: usize, biome: BiomeType) {
        self.biome_map.insert((x as i32, z as i32), biome);
    }

    pub fn get_snow_coverage(&self, x: usize, z: usize) -> f32 {
        let idx = z * self.size as usize + x;
        self.snow_coverage[idx]
    }

    pub fn set_snow_coverage(&mut self, x: usize, z: usize, coverage: f32) {
        let idx = z * self.size as usize + x;
        self.snow_coverage[idx] = coverage;
    }

    pub fn update_bounds(&mut self) {
        let mut min_height = f32::MAX;
        let mut max_height = f32::MIN;

        for height in &self.height_map {
            min_height = min_height.min(*height);
            max_height = max_height.max(*height);
        }

        let chunk_size = self.size;
        let min = Vec3::new(
            self.position.x,
            min_height,
            self.position.y
        );
        let max = Vec3::new(
            self.position.x + chunk_size,
            max_height,
            self.position.y + chunk_size
        );

        self.bounds = ChunkBounds::from_min_max(min, max);
    }

    pub fn get_average_height(&self) -> f32 {
        let sum: f32 = self.height_map.iter().sum();
        sum / (self.size * self.size) as f32
    }

    pub fn get_dominant_biome(&self) -> BiomeType {
        let mut biome_counts = std::collections::HashMap::new();
        
        for biome in self.biome_map.values() {
            *biome_counts.entry(*biome).or_insert(0) += 1;
        }
        
        biome_counts
            .into_iter()
            .max_by_key(|&(_, count)| count)
            .map(|(biome, _)| biome)
            .unwrap_or(BiomeType::Forest)
    }
}

#[derive(Component)]
pub struct TerrainChunkComponent {
    pub chunk: TerrainChunk,
}

impl TerrainChunkComponent {
    pub fn new(
        mesh: Handle<Mesh>,
        material: Handle<StandardMaterial>,
        position: Vec2,
        size: f32,
    ) -> Self {
        Self {
            chunk: TerrainChunk::new(mesh, material, position, size),
        }
    }
}

pub fn spawn_terrain_chunk(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    asset_server: &Res<AssetServer>,
    chunk_data: TerrainChunk,
    settings: &TerrainFeatureSettings,
) -> Entity {
    // Create mesh from chunk data
    let mesh = meshes.add(
        crate::terrain::create_terrain_mesh(
            &chunk_data,
            0, // Default LOD
            None, // No neighbor LODs for now
        )
    );
    // Determine dominant terrain type
    let dominant_type = crate::terrain::determine_dominant_terrain_type(&chunk_data);
    // Create material (use StandardMaterial for now)
    let material = materials.add(StandardMaterial {
        base_color: Color::rgb(0.5, 0.8, 0.5), // Example color, replace as needed
        perceptual_roughness: 0.9,
        metallic: 0.0,
        ..default()
    });
    // Use chunk_data.position for placement
    let position = chunk_data.position;
    let size = chunk_data.size as f32;
    let entity = commands
        .spawn(PbrBundle {
            mesh: mesh.clone(),
            material: material.clone(),
            transform: Transform::from_translation(Vec3::new(position.x, 0.0, position.y)),
            ..default()
        })
        .insert(TerrainChunkComponent {
            // Store the full chunk data for later use
            chunk: TerrainChunk {
                mesh,
                material,
                transform: Transform::from_translation(Vec3::new(position.x, 0.0, position.y)),
                position,
                size,
                height_map: Vec::new(),
                biome_map: HashMap::new(),
                lod_level: 0,
                bounds: ChunkBounds::from_min_max(
                    Vec3::new(position.x, 0.0, position.y),
                    Vec3::new(position.x + size, 0.0, position.y + size),
                ),
                temperature: Vec::new(),
                moisture: Vec::new(),
                snow_coverage: Vec::new(),
                needs_update: true,
                mesh_key: None,
                terrain_types: Vec::new(),
                wind_speed: Vec::new(),
                wind_direction: Vec::new(),
                erosion: Vec::new(),
                vegetation: Vec::new(),
            },
        })
        .id();
    entity
}

pub fn update_terrain_chunks(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    camera_query: Query<&Transform, With<Camera>>,
    terrain_settings: Res<TerrainGenerationSettings>,
    mut terrain_chunks: Query<(Entity, &TerrainChunkComponent)>,
    feature_settings: Res<TerrainFeatureSettings>,
    asset_server: Res<AssetServer>,
) {
    // Get camera position
    let camera_transform = camera_query.single();
    let camera_pos = Vec2::new(camera_transform.translation.x, camera_transform.translation.z);

    // Calculate visible chunk range
    let chunk_size = terrain_settings.chunk_size as f32;
    let view_distance = terrain_settings.view_distance as f32;
    let chunks_in_view = (view_distance / chunk_size).ceil() as i32;

    let current_chunk_x = (camera_pos.x / chunk_size).floor() as i32;
    let current_chunk_z = (camera_pos.y / chunk_size).floor() as i32;

    // Create a HashSet of existing chunk positions
    let existing_chunks: HashSet<(i32, i32)> = terrain_chunks
        .iter()
        .map(|(_, chunk)| {
            let pos = chunk.chunk.position;
            (
                (pos.x / chunk_size).floor() as i32,
                (pos.y / chunk_size).floor() as i32,
            )
        })
        .collect();

    // Spawn new chunks
    for x in (current_chunk_x - chunks_in_view)..=(current_chunk_x + chunks_in_view) {
        for z in (current_chunk_z - chunks_in_view)..=(current_chunk_z + chunks_in_view) {
            if !existing_chunks.contains(&(x, z)) {
                let chunk_pos = Vec2::new(x as f32 * chunk_size, z as f32 * chunk_size);
                let distance_to_camera = chunk_pos.distance(camera_pos);

                if distance_to_camera <= view_distance {
                    let _chunk_data = todo!("generate or obtain TerrainChunk");
                }
            }
        }
    }

    // Remove chunks that are too far
    for (entity, chunk) in terrain_chunks.iter() {
        let chunk_pos = chunk.chunk.position;
        let distance_to_camera = chunk_pos.distance(camera_pos);

        if distance_to_camera > view_distance {
            commands.entity(entity).despawn();
        }
    }
}

fn determine_terrain_type(position: Vec2, settings: &TerrainGenerationSettings) -> TerrainType {
    // Get height and moisture at this position
    // let height = settings.noise.get_height(position);
    // let moisture = settings.noise.get_moisture(position);
    
    // Determine biome based on height and moisture
    match (position.x, position.y) {
        (_, _) if position.x > 0.8 => TerrainType::Snow,
        (_, _) if position.x > 0.6 => TerrainType::Rock,
        (_, _) if position.x > 0.3 && position.y > 0.6 => TerrainType::Forest,
        (_, _) if position.x > 0.3 => TerrainType::Plains,
        (_, _) if position.x > 0.2 => TerrainType::Grass,
        (_, _) if position.y > 0.6 => TerrainType::Tundra,
        _ => TerrainType::Sand,
    }
} 