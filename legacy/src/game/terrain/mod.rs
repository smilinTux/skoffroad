use bevy::prelude::*;
use bevy::render::primitives::Frustum;
use bevy::render::view::VisibleEntities;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use bevy::render::render_resource::{FilterMode, TextureDimension, TextureFormat};
use bevy::gizmos::gizmos::Gizmos;
use crate::math::*;
use super::lod::{TerrainLODManager, update_terrain_lod, update_lod_transitions};
use bevy::utils::HashMap;
use std::collections::VecDeque;
use bevy_rapier3d::prelude::*;

mod collision;
mod heightmap;
mod lod;
mod physics_lod;

use collision::{TerrainCollider, TerrainCollisionPlugin};
use heightmap::{TerrainHeightmap, HeightmapSettings};
use lod::{TerrainLODManager, TerrainLODPlugin};
use physics_lod::{TerrainPhysicsLODPlugin, PhysicsLODSettings};

/// Plugin for terrain generation and management
pub struct TerrainPlugin;

impl Plugin for TerrainPlugin {
    fn build(&self, app: &mut App) {
        app
            // Add required plugins
            .add_plugins((
                TerrainLODPlugin,
                TerrainCollisionPlugin,
                TerrainPhysicsLODPlugin,
            ))
            // Add resources
            .insert_resource(HeightmapSettings {
                seed: 12345,
                amplitude: 100.0,
                frequency: 0.01,
                octaves: 6,
                persistence: 0.5,
                lacunarity: 2.0,
            })
            .insert_resource(PhysicsLODSettings {
                distance_thresholds: vec![50.0, 100.0, 200.0],
                update_frequencies: vec![0.1, 0.25, 0.5],
                max_collision_points: 1024,
                cache_lifetime: 5.0,
            })
            // Initialize terrain resources
            .init_resource::<TerrainHeightmap>()
            .init_resource::<TerrainLODManager>();
    }
}

/// Component bundle for terrain chunks
#[derive(Bundle)]
pub struct TerrainChunkBundle {
    /// Spatial transform
    pub transform: Transform,
    /// Mesh instance
    pub mesh: Handle<Mesh>,
    /// Material instance
    pub material: Handle<StandardMaterial>,
    /// Visibility
    pub visibility: Visibility,
    /// Computed visibility
    pub computed_visibility: ComputedVisibility,
    /// LOD configuration
    pub lod: lod::TerrainLODConfig,
    /// Physics LOD data
    pub physics_lod: physics_lod::TerrainPhysicsLOD,
    /// Collider component
    pub collider: TerrainCollider,
}

impl Default for TerrainChunkBundle {
    fn default() -> Self {
        Self {
            transform: Transform::default(),
            mesh: Handle::default(),
            material: Handle::default(),
            visibility: Visibility::default(),
            computed_visibility: ComputedVisibility::default(),
            lod: lod::TerrainLODConfig::default(),
            physics_lod: physics_lod::TerrainPhysicsLOD::default(),
            collider: TerrainCollider {
                collider: Collider::cuboid(1.0, 1.0, 1.0), // Placeholder
                properties: Default::default(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_terrain_plugin_setup() {
        let mut app = App::new();
        app.add_plugins(TerrainPlugin);

        // Verify resources were added
        assert!(app.world.contains_resource::<HeightmapSettings>());
        assert!(app.world.contains_resource::<PhysicsLODSettings>());
        assert!(app.world.contains_resource::<TerrainHeightmap>());
        assert!(app.world.contains_resource::<TerrainLODManager>());
    }

    #[test]
    fn test_terrain_chunk_bundle() {
        let chunk = TerrainChunkBundle::default();
        
        // Verify components
        assert!(chunk.transform.translation.is_finite());
        assert!(chunk.mesh.id() != Handle::default().id());
        assert!(chunk.material.id() != Handle::default().id());
        assert!(chunk.visibility.is_visible);
        assert!(!chunk.computed_visibility.is_visible());
    }
}

#[derive(Component, Reflect)]
pub struct TerrainConfig {
    pub resolution: UVec2,
    pub size: Vec2,
    pub lod_distances: Vec<f32>,
    pub lod_transition_range: f32,
}

impl Default for TerrainConfig {
    fn default() -> Self {
        Self {
            resolution: UVec2::new(256, 256),
            size: Vec2::new(1000.0, 1000.0),
            lod_distances: vec![50.0, 100.0, 200.0, 400.0],
            lod_transition_range: 20.0,
        }
    }
}

#[derive(Component)]
pub struct TerrainChunk {
    pub position: Vec2,
    pub size: f32,
    pub transition_config: TransitionConfig,
}

#[derive(Default)]
pub struct TransitionConfig {
    pub target_lod: u32,
    pub morph_factor: f32,
}

impl TerrainLODManager {
    pub fn new() -> Self {
        Self {
            chunk_updates: VecDeque::new(),
            max_updates_per_frame: 4,
            distance_thresholds: [50.0, 100.0, 200.0, 400.0, 800.0],
        }
    }

    pub fn determine_lod_level(&self, distance: f32) -> u32 {
        for (i, &threshold) in self.distance_thresholds.iter().enumerate() {
            if distance <= threshold {
                return i as u32;
            }
        }
        4 // Maximum LOD level
    }
}

fn update_terrain_lod(
    mut terrain_lod: ResMut<TerrainLODManager>,
    camera_query: Query<&Transform, With<Camera>>,
    mut chunk_query: Query<(Entity, &Transform, &mut TerrainChunk)>,
) {
    let camera_pos = if let Ok(transform) = camera_query.get_single() {
        transform.translation
    } else {
        return;
    };

    for (entity, transform, mut chunk) in chunk_query.iter_mut() {
        let chunk_center = transform.translation;
        let distance = chunk_center.distance(camera_pos);
        
        let target_lod = terrain_lod.determine_lod_level(distance);
        
        if target_lod != chunk.transition_config.target_lod {
            chunk.transition_config.target_lod = target_lod;
            chunk.transition_config.morph_factor = 0.0;
            terrain_lod.chunk_updates.push_back(entity);
        }
    }
}

fn update_terrain_chunks(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut terrain_lod: ResMut<TerrainLODManager>,
    heightmap: Res<TerrainHeightmap>,
    mut chunk_query: Query<(&mut TerrainChunk, &Handle<Mesh>)>,
) {
    let mut updates_this_frame = 0;
    
    while let Some(entity) = terrain_lod.chunk_updates.pop_front() {
        if updates_this_frame >= terrain_lod.max_updates_per_frame {
            terrain_lod.chunk_updates.push_front(entity);
            break;
        }
        
        if let Ok((chunk, mesh_handle)) = chunk_query.get_mut(entity) {
            let mesh = generate_chunk_mesh(&heightmap, chunk);
            meshes.insert(mesh_handle, mesh);
            updates_this_frame += 1;
        }
    }
}

fn generate_chunk_mesh(heightmap: &TerrainHeightmap, chunk: &TerrainChunk) -> Mesh {
    let lod = chunk.transition_config.target_lod;
    let vertices_per_side = 64 >> lod; // Reduce vertex count based on LOD
    let step = chunk.size / (vertices_per_side - 1) as f32;
    
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut indices = Vec::new();
    
    // Generate vertices
    for z in 0..vertices_per_side {
        for x in 0..vertices_per_side {
            let world_x = chunk.position.x + x as f32 * step;
            let world_z = chunk.position.y + z as f32 * step;
            
            let height = heightmap.sample_height_at_lod(world_x, world_z, lod);
            let normal = heightmap.calculate_normal_at_lod(world_x, world_z, lod);
            
            positions.push([world_x, height, world_z]);
            normals.push(normal);
            uvs.push([x as f32 / (vertices_per_side - 1) as f32, 
                     z as f32 / (vertices_per_side - 1) as f32]);
        }
    }
    
    // Generate indices
    for z in 0..vertices_per_side - 1 {
        for x in 0..vertices_per_side - 1 {
            let top_left = z * vertices_per_side + x;
            let top_right = top_left + 1;
            let bottom_left = (z + 1) * vertices_per_side + x;
            let bottom_right = bottom_left + 1;
            
            indices.extend_from_slice(&[
                top_left as u32,
                bottom_left as u32,
                top_right as u32,
                top_right as u32,
                bottom_left as u32,
                bottom_right as u32,
            ]);
        }
    }
    
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.set_indices(Some(Indices::U32(indices)));
    
    mesh
}

fn sample_height(x: f32, z: f32) -> f32 {
    // Implement your height sampling logic here
    // This could use noise functions, heightmap data, etc.
    0.0
}

fn sample_height_at_lod(x: f32, z: f32, lod: u32) -> f32 {
    // Sample height considering LOD level
    // This might use different noise frequencies or interpolation methods
    sample_height(x, z)
}

fn calculate_normal(x: f32, z: f32) -> [f32; 3] {
    // Calculate surface normal using height samples
    let dx = 1.0;
    let dz = 1.0;
    
    let h_right = sample_height(x + dx, z);
    let h_left = sample_height(x - dx, z);
    let h_up = sample_height(x, z + dz);
    let h_down = sample_height(x, z - dz);
    
    let normal = Vec3::new(
        h_left - h_right,
        2.0,
        h_down - h_up,
    ).normalize();
    
    [normal.x, normal.y, normal.z]
}