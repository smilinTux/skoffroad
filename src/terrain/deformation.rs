use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use bevy::render::mesh::Mesh;
use bevy::utils::HashMap;
use crate::terrain::settings::TerrainConfig;
use crate::weather;

#[derive(Debug, Clone)]
pub struct DeformationPoint {
    pub position: Vec3,
    pub radius: f32,
    pub strength: f32,
    pub smoothing: f32,
}

#[derive(Event)]
pub struct TerrainDeformationEvent {
    pub chunk_pos: IVec3,
    pub world_pos: Vec3,
    pub radius: f32,
    pub strength: f32,
    pub deformation_type: DeformationType,
}

#[derive(Debug, Clone, Copy)]
pub enum DeformationType {
    Generic,
    SnowTrack {
        vehicle_weight: f32,
        vehicle_speed: f32,
        track_width: f32,
    },
    SnowPile {
        density: f32,
    },
    SnowCompression {
        pressure: f32,
    },
}

#[derive(Resource, Default)]
pub struct TerrainDeformationState {
    pub active_deformations: HashMap<IVec3, Vec<DeformationPoint>>,
}

#[derive(Resource, Default)]
pub struct ChunkMeshMapping {
    pub chunk_to_mesh: HashMap<IVec3, Handle<Mesh>>,
    pub mesh_to_chunk: HashMap<Handle<Mesh>, IVec3>,
}

#[derive(Resource)]
pub struct SnowDeformationSettings {
    pub track_persistence: f32,      // How long tracks remain visible
    pub max_track_depth: f32,        // Maximum depth of vehicle tracks
    pub compression_factor: f32,     // How much snow compresses under weight
    pub displacement_spread: f32,    // How far displaced snow spreads
    pub healing_rate: f32,          // Rate at which snow naturally smooths out
    pub weather_influence: f32,     // How much weather affects deformation
}

impl Default for SnowDeformationSettings {
    fn default() -> Self {
        Self {
            track_persistence: 300.0,  // 5 minutes
            max_track_depth: 0.5,
            compression_factor: 0.7,
            displacement_spread: 1.5,
            healing_rate: 0.01,
            weather_influence: 1.0,
        }
    }
}

#[derive(Component)]
pub struct SnowDeformation {
    pub tracks: Vec<SnowTrack>,
    pub compression_map: Vec<f32>,
    pub last_update: f32,
}

#[derive(Debug, Clone)]
pub struct SnowTrack {
    pub start_pos: Vec3,
    pub end_pos: Vec3,
    pub width: f32,
    pub depth: f32,
    pub age: f32,
    pub compressed: bool,
}

impl ChunkMeshMapping {
    pub fn register_chunk(&mut self, chunk_pos: IVec3, mesh_handle: Handle<Mesh>) {
        self.chunk_to_mesh.insert(chunk_pos, mesh_handle.clone());
        self.mesh_to_chunk.insert(mesh_handle, chunk_pos);
    }

    pub fn unregister_chunk(&mut self, chunk_pos: IVec3) {
        if let Some(mesh_handle) = self.chunk_to_mesh.remove(&chunk_pos) {
            self.mesh_to_chunk.remove(&mesh_handle);
        }
    }

    pub fn get_mesh_handle(&self, chunk_pos: &IVec3) -> Option<&Handle<Mesh>> {
        self.chunk_to_mesh.get(chunk_pos)
    }

    pub fn get_chunk_pos(&self, mesh_handle: &Handle<Mesh>) -> Option<&IVec3> {
        self.mesh_to_chunk.get(mesh_handle)
    }
}

fn apply_deformation(
    mut meshes: ResMut<Assets<Mesh>>,
    mut deformation_state: ResMut<TerrainDeformationState>,
    chunk_mapping: Res<ChunkMeshMapping>,
    mut deformation_events: EventReader<TerrainDeformationEvent>,
) {
    for event in deformation_events.read() {
        let chunk_key = IVec3::new(
            event.chunk_pos.x as i32,
            event.chunk_pos.y as i32,
            event.chunk_pos.z as i32,
        );

        let deformation = DeformationPoint {
            position: event.world_pos,
            radius: event.radius,
            strength: event.strength,
            smoothing: 0.5, // Default smoothing factor
        };

        // Store deformation point for the chunk
        deformation_state.active_deformations
            .entry(chunk_key)
            .or_default()
            .push(deformation);

        // Apply deformation to the mesh if we have a mapping
        if let Some(mesh_handle) = chunk_mapping.get_mesh_handle(&chunk_key) {
            if let Some(mesh) = meshes.get_mut(mesh_handle) {
                if let Some(deformations) = deformation_state.active_deformations.get(&chunk_key) {
                    update_mesh_vertices(mesh, deformations);
                }
            }
        }
    }
}

pub fn update_mesh_vertices(mesh: &mut Mesh, deformations: &[DeformationPoint]) {
    let mut recalc_normals = false;
    if let Some(vertices) = mesh.attribute_mut(Mesh::ATTRIBUTE_POSITION) {
        match vertices {
            bevy::render::mesh::VertexAttributeValues::Float32x3(positions) => {
                for position in positions.iter_mut() {
                    let vertex_pos = Vec3::new(position[0], position[1], position[2]);
                    
                    // Apply all deformations that affect this vertex
                    for deform in deformations.iter() {
                        let distance = vertex_pos.distance(deform.position);
                        if distance <= deform.radius {
                            // Calculate falloff based on distance
                            let falloff = 1.0 - (distance / deform.radius).powf(deform.smoothing);
                            
                            // Apply deformation along the up vector (Y-axis)
                            position[1] -= deform.strength * falloff;
                        }
                    }
                }
                recalc_normals = true;
            }
            _ => warn!("Unexpected vertex format"),
        }
    }
    // Now, after mutable borrow ends, recalc normals if needed
    if recalc_normals {
        if let Some(indices) = mesh.indices() {
            let positions = match mesh.attribute(Mesh::ATTRIBUTE_POSITION) {
                Some(bevy::render::mesh::VertexAttributeValues::Float32x3(positions)) => positions,
                _ => return,
            };
            let indices_array: Vec<[u32; 3]> = match indices {
                bevy::render::mesh::Indices::U16(ref vec) => vec
                    .chunks(3)
                    .map(|chunk| [chunk[0] as u32, chunk[1] as u32, chunk[2] as u32])
                    .collect(),
                bevy::render::mesh::Indices::U32(ref vec) => vec
                    .chunks(3)
                    .map(|chunk| [chunk[0], chunk[1], chunk[2]])
                    .collect(),
            };
            let normals = calculate_normals(positions, &indices_array);
            mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
        }
    }
}

fn calculate_normals(positions: &[[f32; 3]], indices: &[[u32; 3]]) -> Vec<[f32; 3]> {
    let mut normals = vec![[0.0, 0.0, 0.0]; positions.len()];
    
    for triangle in indices.iter() {
        let [i0, i1, i2] = *triangle;
        
        let v0 = Vec3::from(positions[i0 as usize]);
        let v1 = Vec3::from(positions[i1 as usize]);
        let v2 = Vec3::from(positions[i2 as usize]);
        
        let normal = (v1 - v0).cross(v2 - v0).normalize();
        
        // Add the face normal to each vertex normal
        normals[i0 as usize] = (Vec3::from(normals[i0 as usize]) + normal).normalize().into();
        normals[i1 as usize] = (Vec3::from(normals[i1 as usize]) + normal).normalize().into();
        normals[i2 as usize] = (Vec3::from(normals[i2 as usize]) + normal).normalize().into();
    }
    
    normals
}

pub struct TerrainDeformationPlugin;

impl Plugin for TerrainDeformationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TerrainDeformationState>()
            .init_resource::<ChunkMeshMapping>()
            .init_resource::<SnowDeformationSettings>()
            .add_event::<TerrainDeformationEvent>()
            .add_systems(Update, (
                apply_deformation,
                handle_snow_deformation,
                update_snow_tracks,
                heal_snow_deformation,
            ));
    }
}

fn handle_snow_deformation(
    mut _commands: Commands,
    mut deformation_events: EventReader<TerrainDeformationEvent>,
    mut query: Query<(&mut SnowDeformation, &Handle<Mesh>)>,
    mut meshes: ResMut<Assets<Mesh>>,
    settings: Res<SnowDeformationSettings>,
    _weather: Res<weather::WeatherState>,
    _time: Res<Time>,
) {
    for event in deformation_events.read() {
        match event.deformation_type {
            DeformationType::SnowTrack { vehicle_weight, vehicle_speed, track_width } => {
                // Calculate track depth based on vehicle properties and snow conditions
                let depth = (vehicle_weight * 0.001).min(settings.max_track_depth)
                    * (1.0 - (vehicle_speed * 0.1).min(0.8))
                    * settings.compression_factor;
                
                // Create new track
                let track = SnowTrack {
                    start_pos: event.world_pos,
                    end_pos: event.world_pos + Vec3::new(track_width, 0.0, 0.0),
                    width: track_width,
                    depth,
                    age: 0.0,
                    compressed: true,
                };
                
                // Apply track to mesh and terrain
                for (mut snow_deform, mesh_handle) in query.iter_mut() {
                    snow_deform.tracks.push(track.clone());
                    apply_track_to_mesh(&mut meshes, mesh_handle, &track, &settings);
                }
            },
            DeformationType::SnowPile { density } => {
                // Handle snow piling up (e.g., from plowing or displacement)
                let pile_height = event.strength * density;
                apply_snow_pile(
                    &mut meshes,
                    &mut query,
                    event.world_pos,
                    event.radius,
                    pile_height,
                    &settings,
                );
            },
            DeformationType::SnowCompression { pressure } => {
                // Handle snow compression without displacement
                let compression = pressure * settings.compression_factor;
                apply_snow_compression(
                    &mut meshes,
                    &mut query,
                    event.world_pos,
                    event.radius,
                    compression,
                    &settings,
                );
            },
            DeformationType::Generic => {
                // Handle generic deformation
                apply_generic_deformation(
                    &mut meshes,
                    &mut query,
                    event.world_pos,
                    event.radius,
                    event.strength,
                );
            },
        }
    }
}

fn update_snow_tracks(
    mut query: Query<&mut SnowDeformation>,
    settings: Res<SnowDeformationSettings>,
    _weather: Res<weather::WeatherState>,
    _time: Res<Time>,
) {
    let dt = _time.delta_seconds();
    
    for mut snow_deform in query.iter_mut() {
        // Update track age and remove old tracks
        snow_deform.tracks.retain_mut(|track| {
            track.age += dt;
            
            // Modify track based on weather
            match _weather.current_weather {
                weather::WeatherType::HeavySnow | weather::WeatherType::Blizzard => {
                    track.depth *= 0.95; // Tracks fill in faster in heavy snow
                },
                weather::WeatherType::FreezingRain => {
                    track.compressed = true; // Tracks become icy
                },
                _ => {}
            }
            
            // Keep track if it's still visible
            track.age < settings.track_persistence
        });
    }
}

fn heal_snow_deformation(
    mut query: Query<&mut SnowDeformation>,
    settings: Res<SnowDeformationSettings>,
    _weather: Res<weather::WeatherState>,
    _time: Res<Time>,
) {
    let dt = _time.delta_seconds();
    let healing_rate = settings.healing_rate * settings.weather_influence;
    
    for mut snow_deform in query.iter_mut() {
        // Gradually smooth out compression
        for compression in snow_deform.compression_map.iter_mut() {
            *compression = lerp(*compression, 1.0, healing_rate * dt);
        }
    }
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn apply_track_to_mesh(
    meshes: &mut Assets<Mesh>,
    mesh_handle: &Handle<Mesh>,
    _track: &SnowTrack,
    _settings: &SnowDeformationSettings,
) {
    if let Some(_mesh) = meshes.get_mut(mesh_handle) {
        // Implementation of mesh deformation for tracks
        // This would modify the vertex positions along the track path
    }
}

fn apply_snow_pile(
    _meshes: &mut Assets<Mesh>,
    _query: &mut Query<(&mut SnowDeformation, &Handle<Mesh>)>,
    _position: Vec3,
    _radius: f32,
    _height: f32,
    _settings: &SnowDeformationSettings,
) {
    // Implementation of snow piling mechanics
}

fn apply_snow_compression(
    _meshes: &mut Assets<Mesh>,
    _query: &mut Query<(&mut SnowDeformation, &Handle<Mesh>)>,
    _position: Vec3,
    _radius: f32,
    _compression: f32,
    _settings: &SnowDeformationSettings,
) {
    // Implementation of snow compression mechanics
}

fn apply_generic_deformation(
    _meshes: &mut Assets<Mesh>,
    _query: &mut Query<(&mut SnowDeformation, &Handle<Mesh>)>,
    _position: Vec3,
    _radius: f32,
    _strength: f32,
) {
    // Implementation of generic deformation mechanics
}

pub fn create_terrain_mesh(settings: &TerrainConfig) -> Mesh {
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut indices = Vec::new();
    
    for z in 0..settings.chunk_size {
        for x in 0..settings.chunk_size {
            let pos = [
                x as f32 * settings.vertex_scale,
                0.0,
                z as f32 * settings.vertex_scale,
            ];
            positions.push(pos);
            normals.push([0.0, 1.0, 0.0]);
            uvs.push([
                x as f32 / settings.chunk_size as f32,
                z as f32 / settings.chunk_size as f32,
            ]);
            
            if x < settings.chunk_size - 1 && z < settings.chunk_size - 1 {
                let i = z * settings.chunk_size + x;
                indices.push([
                    i as u32,
                    (i + 1) as u32,
                    (i + settings.chunk_size) as u32,
                ]);
                indices.push([
                    (i + 1) as u32,
                    (i + settings.chunk_size + 1) as u32,
                    (i + settings.chunk_size) as u32,
                ]);
            }
        }
    }
    
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.set_indices(Some(Indices::U32(indices.into_iter().flatten().collect())));
    
    mesh
}