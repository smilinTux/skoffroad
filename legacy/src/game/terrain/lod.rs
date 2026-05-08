use bevy::prelude::*;
use bevy::render::primitives::Aabb;
use bevy::render::view::VisibleEntities;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use bevy::render::render_resource::{FilterMode, TextureDimension, TextureFormat};
use bevy::gizmos::gizmos::Gizmos;
use super::{Heightmap, TerrainMesh, TerrainHeightmap};
use crate::math::*;
use std::collections::{HashMap, BinaryHeap, VecDeque};
use crate::terrain::TerrainChunk;
use super::chunk::TerrainChunkComponent;
use crate::terrain::mesh::spawn_lod_chunk_mesh;

/// Size of each terrain chunk in vertices
pub const CHUNK_SIZE: u32 = 64;

/// Maximum LOD level (0 = highest detail)
pub const MAX_LOD_LEVEL: u32 = 4;

/// Quadtree node for LOD management
#[derive(Debug)]
pub struct QuadTreeNode {
    /// Bounds of this node in world space
    pub bounds: Aabb,
    /// LOD level (0 = highest detail)
    pub level: u32,
    /// Child nodes (if subdivided)
    pub children: Option<Box<[QuadTreeNode; 4]>>,
    /// Entity ID of the chunk for this node (if any)
    pub chunk_entity: Option<Entity>,
}

impl QuadTreeNode {
    pub fn new(bounds: Aabb, level: u32) -> Self {
        Self {
            bounds,
            level,
            children: None,
            chunk_entity: None,
        }
    }

    pub fn subdivide(&mut self) {
        if self.children.is_some() {
            return;
        }

        let center = self.bounds.center;
        let half_extents = self.bounds.half_extents * 0.5;
        let next_level = self.level + 1;

        let children = Box::new([
            // Northwest
            QuadTreeNode::new(
                Aabb {
                    center: Vec3::new(center.x - half_extents.x, center.y, center.z - half_extents.z),
                    half_extents,
                },
                next_level,
            ),
            // Northeast
            QuadTreeNode::new(
                Aabb {
                    center: Vec3::new(center.x + half_extents.x, center.y, center.z - half_extents.z),
                    half_extents,
                },
                next_level,
            ),
            // Southwest
            QuadTreeNode::new(
                Aabb {
                    center: Vec3::new(center.x - half_extents.x, center.y, center.z + half_extents.z),
                    half_extents,
                },
                next_level,
            ),
            // Southeast
            QuadTreeNode::new(
                Aabb {
                    center: Vec3::new(center.x + half_extents.x, center.y, center.z + half_extents.z),
                    half_extents,
                },
                next_level,
            ),
        ]);

        self.children = Some(children);
    }
}

/// Resource for managing terrain LOD
#[derive(Resource)]
pub struct TerrainLODManager {
    /// Root node of the quadtree
    pub root: QuadTreeNode,
    /// Distance thresholds for LOD levels
    pub lod_distances: Vec<f32>,
    /// Transition range for morphing between LOD levels
    pub transition_range: f32,
    /// Maximum LOD level
    pub max_lod_level: u32,
    /// Queue of chunks that need mesh updates
    pub update_queue: VecDeque<(Entity, u32)>, // (chunk_entity, lod_level)
    /// Maximum number of chunk updates per frame
    pub max_updates_per_frame: usize,
}

impl Default for TerrainLODManager {
    fn default() -> Self {
        let bounds = Aabb {
            center: Vec3::ZERO,
            half_extents: Vec3::new(500.0, 100.0, 500.0),
        };

        Self {
            root: QuadTreeNode::new(bounds, 0),
            lod_distances: vec![50.0, 100.0, 200.0, 400.0],
            transition_range: 20.0,
            max_lod_level: MAX_LOD_LEVEL,
            update_queue: VecDeque::new(),
            max_updates_per_frame: 4,
        }
    }
}

impl TerrainLODManager {
    pub fn determine_lod_level(&self, camera: &Camera, camera_transform: &GlobalTransform, node: &QuadTreeNode) -> Option<u32> {
        let node_center = node.bounds.center;
        let camera_pos = camera_transform.translation();
        let distance = node_center.distance(camera_pos);

        // Check if node is in view frustum
        if !camera.frustum.intersects_obb(
            node_center,
            node.bounds.half_extents,
            camera_transform.compute_matrix(),
        ) {
            return None;
        }

        // Determine LOD level based on distance
        for (level, &threshold) in self.lod_distances.iter().enumerate() {
            if distance <= threshold {
                return Some(level as u32);
            }
        }

        Some(self.max_lod_level)
    }

    pub fn queue_chunk_update(&mut self, chunk_entity: Entity, lod_level: u32) {
        self.update_queue.push_back((chunk_entity, lod_level));
    }
}

/// Component for LOD transition state
#[derive(Component)]
pub struct LodTransition {
    /// Current morph factor (0.0 to 1.0)
    pub morph_factor: f32,
    /// Target LOD level
    pub target_level: u32,
    /// Transition speed
    pub transition_speed: f32,
    pub blend_weights: Vec<f32>,
}

impl Default for LodTransition {
    fn default() -> Self {
        Self {
            morph_factor: 0.0,
            target_level: 0,
            transition_speed: 2.0,
            blend_weights: Vec::new(),
        }
    }
}

/// System to update LOD based on camera position
pub fn update_terrain_lod(
    mut commands: Commands,
    mut lod_manager: ResMut<TerrainLODManager>,
    camera_query: Query<(&Camera, &GlobalTransform)>,
    time: Res<Time>,
) {
    let (camera, camera_transform) = match camera_query.get_single() {
        Ok(result) => result,
        Err(_) => return,
    };

    update_quadtree(
        &mut lod_manager.root,
        camera,
        camera_transform,
        &mut lod_manager,
        &mut commands,
    );
}

fn update_quadtree(
    node: &mut QuadTreeNode,
    camera: &Camera,
    camera_transform: &GlobalTransform,
    lod_manager: &mut TerrainLODManager,
    commands: &mut Commands,
) {
    // Determine LOD level for this node
    let target_lod = match lod_manager.determine_lod_level(camera, camera_transform, node) {
        Some(level) => level,
        None => return, // Node not visible
    };

    // Subdivide or merge based on target LOD
    if target_lod < node.level {
        // Need higher detail - subdivide
        node.subdivide();
        if let Some(children) = &mut node.children {
            for child in children.iter_mut() {
                update_quadtree(child, camera, camera_transform, lod_manager, commands);
            }
        }
    } else {
        // Current detail sufficient or too high - remove children
        if let Some(children) = &node.children {
            for child in children.iter() {
                if let Some(entity) = child.chunk_entity {
                    commands.entity(entity).despawn();
                }
            }
        }
        node.children = None;

        // Spawn or update chunk for this node if needed
        if node.chunk_entity.is_none() {
            let chunk_entity = spawn_lod_chunk_mesh(commands, &mut meshes, &mut materials, &chunk_data, target_lod, None);
            node.chunk_entity = Some(chunk_entity);
            lod_manager.queue_chunk_update(chunk_entity, target_lod);
        }
    }
}

/// System to update LOD transitions
pub fn update_lod_transitions(
    mut transitions: Query<(&mut LodTransition, &TerrainChunkComponent)>,
    time: Res<Time>,
) {
    for (mut transition, chunk_comp) in transitions.iter_mut() {
        // Update blend weights based on target LOD
        transition.blend_weights = calculate_blend_weights(chunk_comp.chunk.lod_level, transition.target_level);
    }
}

/// System to process chunk updates
pub fn process_chunk_updates(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut lod_manager: ResMut<TerrainLODManager>,
    heightmap: Res<TerrainHeightmap>,
    chunks: Query<&TerrainChunkComponent>,
) {
    let mut updates_this_frame = 0;

    while updates_this_frame < lod_manager.max_updates_per_frame {
        if let Some((chunk_entity, lod_level)) = lod_manager.update_queue.pop_front() {
            if let Ok(chunk_comp) = chunks.get(chunk_entity) {
                // Generate new mesh for the chunk at the target LOD level
                let mesh = generate_chunk_mesh(&heightmap, &chunk_comp.chunk, lod_level);
                
                // Update the chunk's mesh
                commands.entity(chunk_entity).insert(meshes.add(mesh));
            }
            updates_this_frame += 1;
        } else {
            break;
        }
    }
}

fn generate_chunk_mesh(heightmap: &TerrainHeightmap, chunk: &TerrainChunk, lod_level: u32) -> Mesh {
    let vertices_per_side = CHUNK_SIZE >> lod_level;
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut indices = Vec::new();

    let step = chunk.bounds.half_extents.x * 2.0 / (vertices_per_side - 1) as f32;
    let start_x = chunk.bounds.center.x - chunk.bounds.half_extents.x;
    let start_z = chunk.bounds.center.z - chunk.bounds.half_extents.z;

    // Generate vertices
    for z in 0..vertices_per_side {
        for x in 0..vertices_per_side {
            let world_x = start_x + x as f32 * step;
            let world_z = start_z + z as f32 * step;
            
            // Sample height and normal at the current LOD level
            let height = heightmap.sample_height_at_lod(world_x, world_z, lod_level);
            let normal = heightmap.calculate_normal_at_lod(world_x, world_z, lod_level);

            positions.push([world_x, height, world_z]);
            normals.push(normal.into());
            uvs.push([
                x as f32 / (vertices_per_side - 1) as f32,
                z as f32 / (vertices_per_side - 1) as f32,
            ]);
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
                top_left,
                bottom_left,
                top_right,
                top_right,
                bottom_left,
                bottom_right,
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

/// System to update texture preloading
pub fn update_texture_preloading(
    mut pool_manager: ResMut<TexturePoolManager>,
    mut texture_lod: ResMut<TerrainTextureLOD>,
    mut loading_state: ResMut<ChunkLoadingState>,
    camera: Query<(&Camera, &GlobalTransform, &Velocity)>,
    chunks: Query<(&TerrainChunkComponent, &ChunkPriority)>,
    settings: Res<TerrainLODSettings>,
    mut textures: ResMut<Assets<Image>>,
    heightmap: Res<Heightmap>,
    feature_map: Res<TerrainFeatureMap>,
) {
    let (camera, camera_transform, velocity) = camera.single();
    
    // Update predictions
    pool_manager.predict_lod_changes(
        camera,
        camera_transform,
        velocity.linear,
        &chunks,
        &settings,
    );
    
    // Update texture pool
    pool_manager.update_texture_pool(
        &mut texture_lod,
        &mut loading_state,
        &mut textures,
        &heightmap,
        &feature_map,
    );
}

/// Resource for managing texture blending between LOD levels
#[derive(Resource)]
pub struct TextureBlendManager {
    pub blend_distances: Vec<f32>,         // Distance ranges for blending
    pub blend_weights: HashMap<(u32, u32), f32>, // Weights for LOD pairs
    pub transition_textures: HashMap<(UVec2, u32, u32), Handle<Image>>, // Blended transition textures
    pub max_cached_transitions: usize,     // Maximum number of cached transition textures
    pub blend_pattern: BlendPattern,       // Pattern used for blending
}

#[derive(Debug, Clone)]
pub enum BlendPattern {
    Radial,
    Checkerboard,
    Gradient,
    Custom(Box<dyn Fn(f32, f32) -> f32 + Send + Sync>),
}

impl Default for TextureBlendManager {
    fn default() -> Self {
        Self {
            blend_distances: vec![50.0, 100.0, 200.0, 400.0],
            blend_weights: HashMap::new(),
            transition_textures: HashMap::new(),
            max_cached_transitions: 16,
            blend_pattern: BlendPattern::Radial,
        }
    }
}

impl TextureBlendManager {
    pub fn calculate_blend_weights(&mut self, chunk_comp: &TerrainChunkComponent, camera_distance: f32) {
        let chunk = &chunk_comp.chunk;
        for i in 0..self.blend_distances.len() {
            let current_lod = chunk.lod_level;
            let next_lod = current_lod + 1;
            
            let dist_factor = (camera_distance - self.blend_distances[i]) / 
                            (self.blend_distances[i + 1] - self.blend_distances[i]);
            let weight = (dist_factor.clamp(0.0, 1.0) * std::f32::consts::PI).sin();
            
            self.blend_weights.insert((current_lod, next_lod), weight);
        }
    }

    pub fn get_blend_pattern_value(&self, uv: Vec2, pattern: &BlendPattern) -> f32 {
        match pattern {
            BlendPattern::Radial => {
                let center = Vec2::new(0.5, 0.5);
                let dist = uv.distance(center) * 2.0;
                (1.0 - dist).max(0.0)
            }
            BlendPattern::Checkerboard => {
                let size = 8.0;
                let x = (uv.x * size).floor() as i32;
                let y = (uv.y * size).floor() as i32;
                if (x + y) % 2 == 0 { 1.0 } else { 0.0 }
            }
            BlendPattern::Gradient => {
                (uv.x + uv.y) * 0.5
            }
            BlendPattern::Custom(f) => f(uv.x, uv.y),
        }
    }
}

/// Represents a detected terrain feature
#[derive(Debug, Clone)]
pub struct TerrainFeature {
    pub feature_type: TerrainFeatureType,
    pub position: Vec2,
    pub radius: f32,
    pub intensity: f32,
    pub elevation: f32,
    pub slope: f32,
    pub roughness: f32,
}

/// Types of terrain features that can be detected
#[derive(Debug, Clone, PartialEq)]
pub enum TerrainFeatureType {
    Peak,
    Valley,
    Ridge,
    Cliff,
    Plateau,
    Slope,
    Basin,
    Custom(String),
}

/// Configuration for terrain feature detection
#[derive(Debug, Clone)]
pub struct TerrainFeatureDetectorConfig {
    pub min_feature_size: f32,
    pub max_feature_size: f32,
    pub slope_threshold: f32,
    pub height_threshold: f32,
    pub roughness_threshold: f32,
    pub detection_radius: f32,
    pub feature_merge_distance: f32,
}

impl Default for TerrainFeatureDetectorConfig {
    fn default() -> Self {
        Self {
            min_feature_size: 5.0,
            max_feature_size: 50.0,
            slope_threshold: 0.7,
            height_threshold: 10.0,
            roughness_threshold: 0.5,
            detection_radius: 15.0,
            feature_merge_distance: 10.0,
        }
    }
}

/// System for detecting terrain features
pub fn detect_terrain_features(
    chunk: &TerrainChunk,
    heightmap: &Heightmap,
    config: &TerrainFeatureDetectorConfig,
) -> Vec<TerrainFeature> {
    let mut features = Vec::new();
    let step = (CHUNK_SIZE >> chunk.current_lod) as usize;
    
    // Scan the heightmap for potential features
    for x in (0..CHUNK_SIZE as usize).step_by(step) {
        for y in (0..CHUNK_SIZE as usize).step_by(step) {
            let pos = Vec2::new(x as f32, y as f32);
            
            // Calculate local terrain properties
            let (slope, roughness) = calculate_terrain_metrics(chunk, heightmap, pos, step as f32);
            let elevation = heightmap.get_height(pos);
            
            // Check surrounding area for significant height differences
            let (min_height, max_height) = get_height_range(heightmap, pos, config.detection_radius);
            let height_diff = max_height - min_height;
            
            // Detect features based on terrain metrics
            if let Some(feature_type) = classify_feature(slope, roughness, height_diff, config) {
                let feature = TerrainFeature {
                    feature_type,
                    position: pos,
                    radius: calculate_feature_radius(heightmap, pos, config),
                    intensity: calculate_feature_intensity(slope, roughness, height_diff),
                    elevation,
                    slope,
                    roughness,
                };
                
                // Only add if not too close to existing features
                if !is_near_existing_feature(&features, &feature, config.feature_merge_distance) {
                    features.push(feature);
                }
            }
        }
    }
    
    // Merge nearby similar features
    merge_nearby_features(&mut features, config.feature_merge_distance);
    features
}

/// Calculate slope and roughness at a given position
fn calculate_terrain_metrics(
    chunk: &TerrainChunk,
    heightmap: &Heightmap,
    pos: Vec2,
    step: f32,
) -> (f32, f32) {
    let center = heightmap.get_height(pos);
    let mut slopes = Vec::new();
    let mut heights = Vec::new();
    
    // Sample points in a circle
    for angle in (0..8).map(|i| i as f32 * std::f32::consts::PI / 4.0) {
        let sample_pos = pos + Vec2::new(angle.cos(), angle.sin()) * step;
        let height = heightmap.get_height(sample_pos);
        
        let slope = ((height - center) / step).abs();
        slopes.push(slope);
        heights.push(height);
    }
    
    // Calculate metrics
    let avg_slope = slopes.iter().sum::<f32>() / slopes.len() as f32;
    let roughness = calculate_roughness(&heights);
    
    (avg_slope, roughness)
}

/// Calculate roughness from a set of height samples
fn calculate_roughness(heights: &[f32]) -> f32 {
    let mean = heights.iter().sum::<f32>() / heights.len() as f32;
    let variance = heights.iter()
        .map(|h| (h - mean).powi(2))
        .sum::<f32>() / heights.len() as f32;
    variance.sqrt()
}

/// Classify terrain feature based on metrics
fn classify_feature(
    slope: f32,
    roughness: f32,
    height_diff: f32,
    config: &TerrainFeatureDetectorConfig,
) -> Option<TerrainFeatureType> {
    if height_diff < config.height_threshold {
        return None;
    }
    
    match (slope > config.slope_threshold, roughness > config.roughness_threshold) {
        (true, true) => Some(TerrainFeatureType::Cliff),
        (true, false) => Some(TerrainFeatureType::Slope),
        (false, true) => Some(TerrainFeatureType::Ridge),
        (false, false) => {
            if height_diff > config.height_threshold * 2.0 {
                Some(TerrainFeatureType::Peak)
            } else {
                Some(TerrainFeatureType::Plateau)
            }
        }
    }
}

/// System to update texture blending
pub fn update_texture_blending(
    mut blend_manager: ResMut<TextureBlendManager>,
    mut textures: ResMut<Assets<Image>>,
    chunks: Query<(&TerrainChunkComponent, &ChunkPriority)>,
    heightmap: Res<Heightmap>,
    feature_map: Res<TerrainFeatureMap>,
) {
    // Update blend weights based on distance
    for (chunk_comp, _) in chunks.iter() {
        let current_lod = chunk_comp.chunk.lod_level;
        let target_lod = chunk_comp.chunk.target_lod_level;
        
        if current_lod != target_lod {
            let key = (chunk_comp.chunk.chunk_pos, current_lod, target_lod);
            
            // Generate or update transition texture if needed
            if !blend_manager.transition_textures.contains_key(&key) {
                // Clean up old textures if needed
                if blend_manager.transition_textures.len() >= blend_manager.max_cached_transitions {
                    // Remove oldest texture
                    if let Some(old_key) = blend_manager.transition_textures.keys().next().cloned() {
                        blend_manager.transition_textures.remove(&old_key);
                    }
                }
                
                // Generate new blended texture
                let blended = generate_blended_texture(
                    &chunk_comp.chunk,
                    current_lod,
                    target_lod,
                    &textures,
                    &heightmap,
                    &feature_map,
                    &blend_manager.blend_pattern,
                );
                
                let texture_handle = textures.add(blended);
                blend_manager.transition_textures.insert(key, texture_handle);
            }
        }
    }
}

/// Resource for managing debug visualization settings
#[derive(Resource)]
pub struct LODDebugVisualization {
    pub enabled: bool,
    pub show_lod_levels: bool,
    pub show_chunk_bounds: bool,
    pub show_feature_overlays: bool,
    pub show_performance_metrics: bool,
    pub show_memory_usage: bool,
    pub show_transition_states: bool,
    pub show_texture_quality: bool,
    pub color_scheme: LODColorScheme,
    pub text_scale: f32,
    pub overlay_opacity: f32,
}

impl Default for LODDebugVisualization {
    fn default() -> Self {
        Self {
            enabled: false,
            show_lod_levels: true,
            show_chunk_bounds: true,
            show_feature_overlays: true,
            show_performance_metrics: true,
            show_memory_usage: true,
            show_transition_states: true,
            show_texture_quality: true,
            color_scheme: LODColorScheme::default(),
            text_scale: 1.0,
            overlay_opacity: 0.5,
        }
    }
}

#[derive(Clone)]
pub struct LODColorScheme {
    pub lod_colors: Vec<Color>,
    pub feature_colors: HashMap<TerrainFeatureType, Color>,
    pub transition_color: Color,
    pub chunk_bound_color: Color,
    pub text_color: Color,
}

impl Default for LODColorScheme {
    fn default() -> Self {
        let mut feature_colors = HashMap::new();
        feature_colors.insert(TerrainFeatureType::Peak, Color::RED);
        feature_colors.insert(TerrainFeatureType::Valley, Color::GREEN);
        feature_colors.insert(TerrainFeatureType::Ridge, Color::BLUE);
        feature_colors.insert(TerrainFeatureType::Cliff, Color::YELLOW);
        feature_colors.insert(TerrainFeatureType::Plateau, Color::PURPLE);
        feature_colors.insert(TerrainFeatureType::Slope, Color::ORANGE);
        feature_colors.insert(TerrainFeatureType::Basin, Color::CYAN);

        Self {
            lod_colors: vec![
                Color::rgba(0.0, 1.0, 0.0, 0.5), // LOD 0 (highest)
                Color::rgba(1.0, 1.0, 0.0, 0.5),
                Color::rgba(1.0, 0.5, 0.0, 0.5),
                Color::rgba(1.0, 0.0, 0.0, 0.5), // LOD 3 (lowest)
            ],
            feature_colors,
            transition_color: Color::rgba(1.0, 1.0, 1.0, 0.3),
            chunk_bound_color: Color::rgba(0.5, 0.5, 0.5, 0.8),
            text_color: Color::WHITE,
        }
    }
}

/// System to render debug visualization
pub fn render_debug_visualization(
    mut gizmos: Gizmos,
    debug_settings: Res<LODDebugVisualization>,
    chunks: Query<(&TerrainChunkComponent, &ChunkPriority)>,
    performance: Res<PerformanceMetrics>,
    feature_detector: Res<TerrainFeatureDetector>,
    camera: Query<(&Camera, &GlobalTransform)>,
) {
    if !debug_settings.enabled {
        return;
    }

    let (camera, camera_transform) = camera.single();
    let camera_pos = camera_transform.translation();

    // Draw chunk bounds and LOD levels
    for (chunk_comp, priority) in chunks.iter() {
        if debug_settings.show_chunk_bounds {
            let min = chunk_comp.chunk.bounds.min;
            let max = chunk_comp.chunk.bounds.max;
            gizmos.line(
                Vec3::new(min.x, min.y, min.z),
                Vec3::new(max.x, min.y, min.z),
                debug_settings.color_scheme.chunk_bound_color,
            );
            // Draw remaining lines for chunk bounds...
        }

        if debug_settings.show_lod_levels {
            let color = debug_settings.color_scheme.lod_colors[chunk_comp.chunk.lod_level as usize % 4];
            let center = chunk_comp.chunk.bounds.center();
            let text = format!("LOD {}", chunk_comp.chunk.lod_level);
            gizmos.text(text, center, debug_settings.color_scheme.text_color);
        }

        if debug_settings.show_transition_states && chunk_comp.chunk.transition_progress > 0.0 {
            let center = chunk_comp.chunk.bounds.center();
            let progress = format!("{:.1}%", chunk_comp.chunk.transition_progress * 100.0);
            gizmos.text(progress, center + Vec3::new(0.0, 2.0, 0.0), debug_settings.color_scheme.transition_color);
        }
    }

    // Draw performance metrics
    if debug_settings.show_performance_metrics {
        let screen_pos = Vec3::new(-0.9, 0.9, 0.0);
        let metrics = format!(
            "FPS: {:.1}\nActive Chunks: {}\nTexture Updates/s: {:.1}",
            1.0 / performance.frame_times.iter().sum::<f32>() * performance.frame_times.len() as f32,
            performance.active_chunks,
            performance.texture_updates_per_second
        );
        gizmos.text(metrics, screen_pos, debug_settings.color_scheme.text_color);
    }

    // Draw memory usage
    if debug_settings.show_memory_usage {
        let screen_pos = Vec3::new(-0.9, 0.7, 0.0);
        let memory = format!(
            "Memory: {:.1} MB / {:.1} MB",
            performance.texture_memory_usage as f32 / (1024.0 * 1024.0),
            512.0 // Max memory budget in MB
        );
        gizmos.text(memory, screen_pos, debug_settings.color_scheme.text_color);
    }

    // Draw feature overlays
    if debug_settings.show_feature_overlays {
        for (chunk_comp, _) in chunks.iter() {
            if let Some(features) = feature_detector.get_features(chunk_comp.chunk.chunk_entity.unwrap()) {
                for feature in features {
                    let color = debug_settings.color_scheme.feature_colors
                        .get(&feature.feature_type)
                        .copied()
                        .unwrap_or(Color::WHITE);
                    
                    let world_pos = Vec3::new(
                        feature.position.x,
                        feature.elevation,
                        feature.position.y
                    );
                    
                    // Draw feature marker
                    gizmos.circle(
                        world_pos,
                        Vec3::Y,
                        feature.radius,
                        color.with_alpha(debug_settings.overlay_opacity),
                    );

                    // Draw feature label if close to camera
                    let distance = camera_pos.distance(world_pos);
                    if distance < 50.0 {
                        let label = format!(
                            "{:?}\nSlope: {:.1}°\nRough: {:.2}",
                            feature.feature_type,
                            feature.slope.to_degrees(),
                            feature.roughness
                        );
                        gizmos.text(label, world_pos + Vec3::Y * 2.0, color);
                    }
                }
            }
        }
    }
}

/// Helper function to calculate terrain metrics for feature detection
fn calculate_terrain_metrics(
    heightmap: &Heightmap,
    chunk_pos: UVec2,
    radius: f32,
) -> (f32, f32, f32) {
    let mut slopes = Vec::new();
    let mut heights = Vec::new();
    
    // Sample points in the chunk
    let sample_count = 16;
    let step = radius / sample_count as f32;
    
    for x in 0..sample_count {
        for z in 0..sample_count {
            let pos = Vec2::new(
                chunk_pos.x as f32 * radius + x as f32 * step,
                chunk_pos.y as f32 * radius + z as f32 * step,
            );
            
            let height = heightmap.get_height(pos);
            heights.push(height);
            
            // Calculate slope using neighboring points
            if x > 0 && z > 0 {
                let dx = heightmap.get_height(pos + Vec2::new(-step, 0.0)) - height;
                let dz = heightmap.get_height(pos + Vec2::new(0.0, -step)) - height;
                let slope = (dx * dx + dz * dz).sqrt() / step;
                slopes.push(slope);
            }
        }
    }
    
    // Calculate metrics
    let avg_slope = slopes.iter().sum::<f32>() / slopes.len() as f32;
    let avg_height = heights.iter().sum::<f32>() / heights.len() as f32;
    let height_var = heights.iter()
        .map(|h| (h - avg_height).powi(2))
        .sum::<f32>()
        .sqrt() / heights.len() as f32;
    
    let roughness = slopes.iter()
        .map(|s| (s - avg_slope).powi(2))
        .sum::<f32>()
        .sqrt() / slopes.len() as f32;
    
    (avg_slope, roughness, height_var)
}

/// Helper function to determine terrain features
fn determine_features(
    slope: f32,
    roughness: f32,
    height_var: f32,
    settings: &FeatureDetectionSettings,
) -> (TerrainFeatureType, TerrainFeatureType, f32) {
    let mut features = vec![
        (TerrainFeatureType::Rock, slope * 2.0 + roughness),
        (TerrainFeatureType::Grass, 1.0 - slope),
        (TerrainFeatureType::Sand, 1.0 - height_var),
        (TerrainFeatureType::Snow, height_var * 2.0),
    ];
    
    features.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    
    let primary = features[0].0.clone();
    let secondary = features[1].0.clone();
    let blend_factor = (features[0].1 - features[1].1).abs();
    
    (primary, secondary, blend_factor)
}

/// Generate a blended texture between two LOD levels
fn generate_blended_texture(
    chunk: &TerrainChunk,
    current_lod: u32,
    target_lod: u32,
    textures: &Assets<Image>,
    heightmap: &Heightmap,
    feature_map: &TerrainFeatureMap,
    blend_pattern: &BlendPattern,
) -> Image {
    let size = CHUNK_SIZE >> current_lod;
    let mut blended = Image::new(
        Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        vec![0; (size * size * 4) as usize],
        TextureFormat::Rgba8UnormSrgb,
    );

    let pixels = blended.data.chunks_exact_mut(4);
    for (i, pixel) in pixels.enumerate() {
        let x = (i % size as usize) as f32 / size as f32;
        let y = (i / size as usize) as f32 / size as f32;
        
        // Calculate blend factor based on pattern
        let blend = match blend_pattern {
            BlendPattern::Radial => {
                let center = Vec2::new(0.5, 0.5);
                let dist = Vec2::new(x, y).distance(center);
                (dist * 2.0).clamp(0.0, 1.0)
            },
            BlendPattern::Checkerboard => {
                let checker_size = 8;
                let cx = (x * size as f32 / checker_size as f32) as i32;
                let cy = (y * size as f32 / checker_size as f32) as i32;
                if (cx + cy) % 2 == 0 { 0.0 } else { 1.0 }
            },
            BlendPattern::Gradient => x,
            BlendPattern::Custom(func) => func(x, y),
        };

        // Sample colors from both LOD levels
        let current_color = sample_texture_color(
            chunk,
            current_lod,
            Vec2::new(x, y),
            heightmap,
            feature_map,
        );
        let target_color = sample_texture_color(
            chunk,
            target_lod,
            Vec2::new(x, y),
            heightmap,
            feature_map,
        );

        // Blend colors
        for i in 0..4 {
            pixel[i] = lerp(current_color[i], target_color[i], blend) as u8;
        }
    }

    // Configure texture parameters
    blended.sampler.min_filter = FilterMode::Linear;
    blended.sampler.mag_filter = FilterMode::Linear;
    blended.sampler.mipmap_filter = FilterMode::Linear;

    blended
}

/// Sample color from a specific LOD level
fn sample_texture_color(
    chunk: &TerrainChunk,
    lod: u32,
    uv: Vec2,
    heightmap: &Heightmap,
    feature_map: &TerrainFeatureMap,
) -> [f32; 4] {
    let world_pos = Vec2::new(
        chunk.chunk_pos.x as f32 * CHUNK_SIZE as f32 + uv.x * CHUNK_SIZE as f32,
        chunk.chunk_pos.y as f32 * CHUNK_SIZE as f32 + uv.y * CHUNK_SIZE as f32,
    );

    let features = feature_map.features.get(&chunk.chunk_pos);
    let mut color = [0.5, 0.5, 0.5, 1.0]; // Default gray

    if let Some(features) = features {
        let mut total_influence = 0.0;
        let mut weighted_color = [0.0; 4];

        for feature in features {
            let distance = world_pos.distance(feature.position);
            if distance <= feature.radius {
                let influence = (1.0 - distance / feature.radius) * feature.intensity;
                total_influence += influence;

                let feature_color = get_feature_color(&feature.feature_type);
                for i in 0..4 {
                    weighted_color[i] += feature_color[i] as f32 * influence;
                }
            }
        }

        if total_influence > 0.0 {
            for i in 0..4 {
                color[i] = weighted_color[i] / total_influence / 255.0;
            }
        }
    }

    color
}

#[derive(Resource)]
pub struct TerrainFeatureDetector {
    config: TerrainFeatureDetectorConfig,
    features: HashMap<Entity, Vec<TerrainFeature>>,
    feature_cache: HashMap<Vec2, TerrainFeature>,
}

impl TerrainFeatureDetector {
    pub fn new(config: TerrainFeatureDetectorConfig) -> Self {
        Self {
            config,
            features: HashMap::new(),
            feature_cache: HashMap::new(),
        }
    }

    pub fn analyze_terrain_features(&mut self, chunk_entity: Entity, heightmap: &[f32], chunk_size: usize, world_position: Vec3, scale: Vec3) {
        let mut features = Vec::new();
        let step = chunk_size / 8; // Analysis grid size
        
        for x in (0..chunk_size).step_by(step) {
            for z in (0..chunk_size).step_by(step) {
                let pos = world_position + Vec3::new(
                    x as f32 * scale.x,
                    0.0,
                    z as f32 * scale.z
                );
                
                // Calculate local terrain metrics
                let (slope, roughness, curvature) = self.calculate_local_metrics(heightmap, chunk_size, x, z);
                
                // Detect primary and secondary features
                let (primary, secondary) = self.classify_terrain_features(slope, roughness, curvature);
                
                if let Some(feature_type) = primary {
                    features.push(TerrainFeature {
                        feature_type,
                        position: Vec2::new(pos.x, pos.z),
                        radius: (step as f32) * scale.x,
                        intensity: slope.max(roughness),
                        elevation: heightmap[z * chunk_size + x],
                        slope,
                        roughness,
                    });
                }
                
                if let Some(feature_type) = secondary {
                    features.push(TerrainFeature {
                        feature_type,
                        position: Vec2::new(pos.x, pos.z),
                        radius: (step as f32) * scale.x * 0.5,
                        intensity: roughness,
                        elevation: heightmap[z * chunk_size + x],
                        slope,
                        roughness,
                    });
                }
            }
        }
        
        // Merge nearby similar features
        self.merge_similar_features(&mut features);
        
        // Store the detected features
        self.features.insert(chunk_entity, features);
    }
    
    fn calculate_local_metrics(&self, heightmap: &[f32], size: usize, x: usize, z: usize) -> (f32, f32, f32) {
        let radius = 2;
        let mut heights = Vec::new();
        let mut slopes = Vec::new();
        
        for dx in -radius..=radius {
            for dz in -radius..=radius {
                let nx = (x as i32 + dx).clamp(0, size as i32 - 1) as usize;
                let nz = (z as i32 + dz).clamp(0, size as i32 - 1) as usize;
                let height = heightmap[nz * size + nx];
                heights.push(height);
                
                if dx != 0 || dz != 0 {
                    let slope = (height - heightmap[z * size + x]).abs() / 
                              ((dx * dx + dz * dz) as f32).sqrt();
                    slopes.push(slope);
                }
            }
        }
        
        let avg_height = heights.iter().sum::<f32>() / heights.len() as f32;
        let height_var = heights.iter()
            .map(|h| (h - avg_height).powi(2))
            .sum::<f32>() / heights.len() as f32;
            
        let avg_slope = slopes.iter().sum::<f32>() / slopes.len() as f32;
        let roughness = slopes.iter()
            .map(|s| (s - avg_slope).powi(2))
            .sum::<f32>() / slopes.len() as f32;
            
        let curvature = height_var / (avg_slope + 0.001);
        
        (avg_slope, roughness, curvature)
    }
    
    fn classify_terrain_features(&self, slope: f32, roughness: f32, curvature: f32) -> (Option<TerrainFeatureType>, Option<TerrainFeatureType>) {
        let mut primary = None;
        let mut secondary = None;
        
        // Primary feature classification
        if slope > self.config.slope_threshold * 1.5 {
            primary = Some(TerrainFeatureType::Cliff);
        } else if slope > self.config.slope_threshold {
            primary = Some(TerrainFeatureType::Slope);
        } else if curvature > self.config.height_threshold {
            if roughness > self.config.roughness_threshold {
                primary = Some(TerrainFeatureType::Peak);
            } else {
                primary = Some(TerrainFeatureType::Plateau);
            }
        } else if curvature < -self.config.height_threshold {
            if roughness > self.config.roughness_threshold {
                primary = Some(TerrainFeatureType::Valley);
            } else {
                primary = Some(TerrainFeatureType::Basin);
            }
        }
        
        // Secondary feature classification
        if roughness > self.config.roughness_threshold * 1.5 {
            secondary = Some(TerrainFeatureType::Ridge);
        }
        
        (primary, secondary)
    }
    
    fn merge_similar_features(&self, features: &mut Vec<TerrainFeature>) {
        features.sort_by(|a, b| b.intensity.partial_cmp(&a.intensity).unwrap());
        
        let mut i = 0;
        while i < features.len() {
            let mut j = i + 1;
            while j < features.len() {
                let dist = features[i].position.distance(features[j].position);
                if dist < self.config.feature_merge_distance &&
                   features[i].feature_type == features[j].feature_type {
                    // Merge features
                    features[i].radius = (features[i].radius + features[j].radius) * 0.5;
                    features[i].intensity = features[i].intensity.max(features[j].intensity);
                    features.remove(j);
                } else {
                    j += 1;
                }
            }
            i += 1;
        }
    }

    pub fn get_features(&self, chunk_entity: Entity) -> Option<&Vec<TerrainFeature>> {
        self.features.get(&chunk_entity)
    }

    pub fn clear_cache(&mut self) {
        self.feature_cache.clear();
    }
}

// System to update terrain features
pub fn update_terrain_features(
    mut feature_detector: ResMut<TerrainFeatureDetector>,
    terrain_chunks: Query<(Entity, &TerrainChunkComponent)>,
) {
    for (entity, chunk_comp) in terrain_chunks.iter() {
        if chunk_comp.chunk.needs_feature_update {
            feature_detector.analyze_terrain_features(
                entity,
                &chunk_comp.chunk.heightmap,
                chunk_comp.chunk.size,
                chunk_comp.chunk.world_position,
                chunk_comp.chunk.scale,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_creation() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<Assets<Mesh>>()
            .init_resource::<ChunkLoadingState>()
            .add_systems(Update, (update_terrain_lod, update_chunk_meshes));

        // Spawn camera with frustum
        let camera_entity = app.world.spawn((
            Camera3dBundle::default(),
            Frustum::default(),
            GlobalTransform::from_xyz(0.0, 100.0, 0.0),
        )).id();

        // Spawn terrain
        let terrain_entity = app.world.spawn((
            Heightmap::new(UVec2::new(256, 256), Vec2::new(1000.0, 1000.0)),
            TerrainLOD::default(),
            GlobalTransform::from_xyz(0.0, 0.0, 0.0),
        )).id();

        // Run systems
        app.update();

        // Verify chunks were created
        let chunks: Vec<_> = app.world.query::<&TerrainChunk>().iter(&app.world).collect();
        assert!(!chunks.is_empty());
        
        // Verify chunk properties
        let chunk = chunks[0];
        assert!(chunk.lod_level < 4); // Should be within our LOD levels
        assert!(chunk.bounds.half_extents.length() > 0.0);
    }

    #[test]
    fn test_chunk_updates() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<Assets<Mesh>>()
            .init_resource::<ChunkLoadingState>();
            
        let mut loading_state = ChunkLoadingState::default();
        loading_state.max_updates_per_frame = 2;
        app.insert_resource(loading_state);
        
        // Add test updates
        app.world.resource_mut::<ChunkLoadingState>().update_queue = vec![
            (Entity::from_raw(1), UVec2::new(0, 0), 0),
            (Entity::from_raw(2), UVec2::new(1, 0), 1),
            (Entity::from_raw(3), UVec2::new(0, 1), 1),
        ];
        
        // Run update system
        app.add_systems(Update, update_chunk_meshes);
        app.update();
        
        // Verify only max_updates_per_frame were processed
        assert_eq!(app.world.resource::<ChunkLoadingState>().update_queue.len(), 1);
    }

    #[test]
    fn test_lod_transitions() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<Assets<Mesh>>()
            .add_systems(Update, update_lod_transitions);
            
        // Create test meshes
        let mut meshes = app.world.resource_mut::<Assets<Mesh>>();
        let mesh1 = meshes.add(Mesh::from(shape::Cube { size: 1.0 }));
        let mesh2 = meshes.add(Mesh::from(shape::Cube { size: 2.0 }));
        
        // Spawn chunk with transition
        let chunk_entity = app.world.spawn((
            TerrainChunk {
                chunk_pos: UVec2::ZERO,
                lod_level: 0,
                target_lod_level: 1,
                transition_progress: 0.0,
                morph_factor: 0.0,
                camera_distance: 0.0,
                bounds: Aabb::default(),
                meshes: Some((mesh1.clone(), mesh2.clone())),
            },
            TerrainLOD::default(),
        )).id();
        
        // Run system multiple times
        for _ in 0..60 { // Simulate 1 second at 60fps
            app.update();
        }
        
        // Verify transition completed
        let chunk = app.world.entity(chunk_entity).get::<TerrainChunk>().unwrap();
        assert_eq!(chunk.lod_level, chunk.target_lod_level);
        assert_eq!(chunk.transition_progress, 0.0);
    }

    #[test]
    fn test_geomorphing() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<Assets<Mesh>>()
            .add_systems(Update, update_lod_transitions);
            
        // Create test meshes with different resolutions
        let mut meshes = app.world.resource_mut::<Assets<Mesh>>();
        let mesh1 = meshes.add(create_test_mesh(2)); // 2x2 grid
        let mesh2 = meshes.add(create_test_mesh(4)); // 4x4 grid
        
        // Spawn chunk with initial state
        let chunk_entity = app.world.spawn((
            TerrainChunk {
                chunk_pos: UVec2::ZERO,
                lod_level: 0,
                target_lod_level: 1,
                transition_progress: 0.0,
                morph_factor: 0.0,
                camera_distance: 0.0,
                bounds: Aabb::default(),
                meshes: Some((mesh1.clone(), mesh2.clone())),
            },
            TerrainLOD::default(),
            Transform::from_xyz(0.0, 0.0, 0.0),
            GlobalTransform::default(),
        )).id();
        
        // Spawn camera
        app.world.spawn((
            Camera3dBundle::default(),
            GlobalTransform::from_xyz(0.0, 0.0, 150.0), // Position for ~50% morphing
        ));
        
        // Run system
        app.update();
        
        // Verify morph factor was calculated
        let chunk = app.world.entity(chunk_entity).get::<TerrainChunk>().unwrap();
        assert!(chunk.morph_factor > 0.0 && chunk.morph_factor < 1.0);
    }
    
    fn create_test_mesh(resolution: u32) -> Mesh {
        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
        let mut positions = Vec::new();
        
        for y in 0..resolution {
            for x in 0..resolution {
                let px = x as f32 / (resolution - 1) as f32;
                let pz = y as f32 / (resolution - 1) as f32;
                positions.push([px, 0.0, pz]);
            }
        }
        
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        mesh
    }

    #[test]
    fn test_lod_transition() {
        let mut transition = LodTransition {
            current_lod: 0,
            target_lod: 1,
            transition_progress: 0.5,
            vertex_heights: vec![0.0, 1.0, 2.0, 3.0],
            target_heights: vec![0.5, 1.5, 2.5, 3.5],
            vertex_positions: Vec::new(),
            target_positions: Vec::new(),
        };
        
        // Test interpolation
        let height = get_interpolated_height(&transition, 0, 0);
        assert_eq!(height, 0.25); // Should be halfway between 0.0 and 0.5
        
        // Test transition completion
        transition.transition_progress = 1.0;
        let height = get_interpolated_height(&transition, 1, 0);
        assert_eq!(height, 1.5); // Should be at target height
    }
    
    #[test]
    fn test_height_calculation() {
        let heightmap = Heightmap {
            dimensions: UVec2::new(128, 128),
            size: Vec2::new(1000.0, 1000.0),
            heights: vec![1.0; 128 * 128],
        };
        
        let chunk = TerrainChunk {
            chunk_pos: UVec2::new(0, 0),
            lod_level: 0,
            target_lod_level: 0,
            transition_progress: 0.0,
            bounds: Aabb::default(),
            meshes: None,
            morph_factor: 0.0,
            camera_distance: 0.0,
        };
        
        let heights = calculate_vertex_heights(&heightmap, &chunk, 5);
        assert_eq!(heights.len(), 25); // 5x5 vertices
        assert!(heights.iter().all(|&h| h == 1.0));
    }
} 