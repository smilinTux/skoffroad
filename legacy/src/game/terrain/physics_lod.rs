use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use crate::physics::terrain_properties::{PhysicsTerrainType, TerrainProperties};
use crate::physics::terrain_interaction::TerrainInteraction;
use crate::terrain::deformation::{TerrainDeformationEvent, DeformationPoint};
use super::lod::{TerrainLODManager, TerrainLODLevel};
use super::heightmap::TerrainHeightmap;
use std::collections::{HashMap, HashSet};
use bevy::tasks::{AsyncComputeTaskPool, Task};
use futures_lite::future;
use bevy::utils::Duration;
use bevy::render::primitives::Aabb;
use bevy::render::color::Color;
use bevy::gizmos::gizmos::Gizmos;
use bevy::render::render_resource::PrimitiveTopology;
use bevy::render::mesh::Indices;
use std::time::{Duration, Instant};
use bevy::text::Text2dBundle;
use bevy::sprite::Anchor;

/// Spatial region for efficient collision updates
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
struct CollisionRegion {
    x: i32,
    z: i32,
    lod_level: TerrainLODLevel,
}

impl CollisionRegion {
    fn new(position: Vec3, cell_size: Vec2, lod_level: TerrainLODLevel) -> Self {
        Self {
            x: (position.x / cell_size.x).floor() as i32,
            z: (position.z / cell_size.y).floor() as i32,
            lod_level,
        }
    }

    fn contains(&self, position: Vec3, cell_size: Vec2) -> bool {
        let region_x = (position.x / cell_size.x).floor() as i32;
        let region_z = (position.z / cell_size.y).floor() as i32;
        self.x == region_x && self.z == region_z
    }

    fn min_bounds(&self) -> Vec3 {
        Vec3::new(self.x as f32 * self.cell_size.x, 0.0, self.z as f32 * self.cell_size.y)
    }

    fn max_bounds(&self) -> Vec3 {
        Vec3::new((self.x + 1) as f32 * self.cell_size.x, 0.0, (self.z + 1) as f32 * self.cell_size.y)
    }
}

/// Enhanced component for LOD-aware terrain collision data
#[derive(Component)]
pub struct TerrainPhysicsLOD {
    /// Current LOD level for physics
    pub lod_level: TerrainLODLevel,
    /// Collision cell size at current LOD
    pub cell_size: Vec2,
    /// Cached collision data per region
    pub collision_cache: HashMap<CollisionRegion, Vec<(Vec3, TerrainProperties)>>,
    /// Last update timestamp per region
    pub region_updates: HashMap<CollisionRegion, f64>,
    /// Active deformation regions
    pub dirty_regions: HashSet<CollisionRegion>,
    /// Async collision update task
    pub update_task: Option<Task<(CollisionRegion, Vec<(Vec3, TerrainProperties)>)>>,
    /// Current region being updated
    pub updating_region: Option<CollisionRegion>,
    /// Update priority queue
    pub update_queue: Vec<CollisionRegion>,
}

impl Default for TerrainPhysicsLOD {
    fn default() -> Self {
        Self {
            lod_level: TerrainLODLevel::High,
            cell_size: Vec2::new(1.0, 1.0),
            collision_cache: HashMap::new(),
            region_updates: HashMap::new(),
            dirty_regions: HashSet::new(),
            update_task: None,
            updating_region: None,
            update_queue: Vec::new(),
        }
    }
}

/// Resource for physics LOD settings
#[derive(Resource)]
pub struct PhysicsLODSettings {
    /// Distance thresholds for physics LOD levels
    pub lod_distances: Vec<f32>,
    /// Update frequency for each LOD level (in seconds)
    pub update_frequencies: Vec<f32>,
    /// Maximum collision points per chunk
    pub max_collision_points: usize,
    /// Cache lifetime in seconds
    pub cache_lifetime: f64,
    /// Maximum concurrent update tasks
    pub max_concurrent_updates: usize,
    /// Region size for deformation updates
    pub deformation_region_size: Vec3,
}

impl Default for PhysicsLODSettings {
    fn default() -> Self {
        Self {
            lod_distances: vec![50.0, 100.0, 200.0, 400.0],
            update_frequencies: vec![0.016, 0.032, 0.064, 0.128], // 60Hz, 30Hz, 15Hz, 7.5Hz
            max_collision_points: 1024,
            cache_lifetime: 1.0,
            max_concurrent_updates: 4,
            deformation_region_size: Vec3::new(10.0, 5.0, 10.0),
        }
    }
}

/// Debug visualization settings for physics LOD
#[derive(Resource)]
pub struct PhysicsLODDebug {
    /// Whether to show collision regions
    pub show_regions: bool,
    /// Whether to show dirty regions
    pub show_dirty_regions: bool,
    /// Whether to show LOD levels
    pub show_lod_levels: bool,
    /// Whether to show collision points
    pub show_collision_points: bool,
    /// Whether to show update frequencies
    pub show_update_frequencies: bool,
    /// Whether to show region tooltips
    pub show_tooltips: bool,
    /// Current visualization mode (cycled with Tab)
    pub viz_mode: usize,
    /// Colors for different LOD levels
    pub lod_colors: Vec<Color>,
    /// Mouse position in world space
    pub mouse_world_pos: Vec3,
}

impl Default for PhysicsLODDebug {
    fn default() -> Self {
        Self {
            show_regions: false,
            show_dirty_regions: false,
            show_lod_levels: false,
            show_collision_points: false,
            show_update_frequencies: false,
            show_tooltips: true,
            viz_mode: 0,
            lod_colors: vec![
                Color::GREEN,   // LOD 0
                Color::YELLOW,  // LOD 1
                Color::ORANGE,  // LOD 2
                Color::RED,     // LOD 3
            ],
            mouse_world_pos: Vec3::ZERO,
        }
    }
}

impl PhysicsLODDebug {
    /// Calculate color for a given LOD level
    fn get_lod_color(lod_level: u32) -> Color {
        // Colors transition from green (low LOD) to red (high LOD)
        match lod_level {
            0 => Color::rgba(0.0, 1.0, 0.0, 0.3),  // Green
            1 => Color::rgba(0.5, 1.0, 0.0, 0.3),  // Yellow-green
            2 => Color::rgba(1.0, 1.0, 0.0, 0.3),  // Yellow
            3 => Color::rgba(1.0, 0.5, 0.0, 0.3),  // Orange
            _ => Color::rgba(1.0, 0.0, 0.0, 0.3),  // Red
        }
    }
}

/// Resource to track performance metrics
#[derive(Resource, Default)]
pub struct PhysicsLODMetrics {
    pub update_times: Vec<f32>,
    pub avg_update_time: f32,
    pub active_regions: usize,
    pub dirty_region_count: usize,
    pub collision_point_count: usize,
    pub last_update: f64,
}

impl PhysicsLODMetrics {
    pub fn update_average(&mut self, new_time: f32) {
        self.update_times.push(new_time);
        if self.update_times.len() > 60 { // Keep last 60 frames
            self.update_times.remove(0);
        }
        self.avg_update_time = self.update_times.iter().sum::<f32>() / self.update_times.len() as f32;
    }
}

/// System to update terrain physics LOD based on distance
pub fn update_terrain_physics_lod(
    mut query: Query<(&mut TerrainPhysicsLOD, &GlobalTransform)>,
    camera_query: Query<&GlobalTransform, With<Camera>>,
    settings: Res<PhysicsLODSettings>,
    time: Res<Time>,
) {
    let camera_transform = camera_query.single();
    let camera_pos = camera_transform.translation();

    for (mut physics_lod, transform) in query.iter_mut() {
        let distance = camera_pos.distance(transform.translation());
        
        // Determine LOD level based on distance
        let new_lod = if distance < settings.lod_distances[0] {
            TerrainLODLevel::High
        } else if distance < settings.lod_distances[1] {
            TerrainLODLevel::Medium
        } else if distance < settings.lod_distances[2] {
            TerrainLODLevel::Low
        } else {
            TerrainLODLevel::VeryLow
        };

        // Update LOD level if changed
        if physics_lod.lod_level != new_lod {
            physics_lod.lod_level = new_lod;
            physics_lod.collision_cache.clear(); // Invalidate cache on LOD change
        }

        // Update cell size based on LOD level
        physics_lod.cell_size = match physics_lod.lod_level {
            TerrainLODLevel::High => Vec2::new(1.0, 1.0),
            TerrainLODLevel::Medium => Vec2::new(2.0, 2.0),
            TerrainLODLevel::Low => Vec2::new(4.0, 4.0),
            TerrainLODLevel::VeryLow => Vec2::new(8.0, 8.0),
        };
    }
}

/// System to handle terrain deformation events with spatial partitioning
pub fn handle_terrain_deformation(
    mut deformation_events: EventReader<TerrainDeformationEvent>,
    mut physics_query: Query<(&mut TerrainPhysicsLOD, &GlobalTransform)>,
    settings: Res<PhysicsLODSettings>,
) {
    for event in deformation_events.iter() {
        for (mut physics_lod, transform) in physics_query.iter_mut() {
            let local_point = transform.compute_matrix().inverse().transform_point3(event.position);
            
            // Create region and add to dirty set
            let region = CollisionRegion::new(local_point, physics_lod.cell_size, physics_lod.lod_level);
            physics_lod.dirty_regions.insert(region.clone());
            
            // Add to update queue if not already present
            if !physics_lod.update_queue.contains(&region) {
                physics_lod.update_queue.push(region);
            }
            
            // Also mark neighboring regions as dirty for seamless transitions
            for dx in [-1, 0, 1].iter() {
                for dz in [-1, 0, 1].iter() {
                    if *dx == 0 && *dz == 0 { continue; }
                    
                    let neighbor = CollisionRegion {
                        x: region.x + dx,
                        z: region.z + dz,
                        lod_level: region.lod_level,
                    };
                    
                    physics_lod.dirty_regions.insert(neighbor.clone());
                    if !physics_lod.update_queue.contains(&neighbor) {
                        physics_lod.update_queue.push(neighbor);
                    }
                }
            }
        }
    }
}

/// Optimized system to update terrain collision data
pub fn update_terrain_collision_data(
    mut query: Query<(
        Entity,
        &mut TerrainPhysicsLOD,
        &GlobalTransform,
        &TerrainHeightmap,
    )>,
    settings: Res<PhysicsLODSettings>,
    time: Res<Time>,
) {
    let current_time = time.elapsed_seconds_f64();
    let thread_pool = AsyncComputeTaskPool::get();
    
    for (entity, mut physics_lod, transform, heightmap) in query.iter_mut() {
        // Skip if an update is already in progress
        if physics_lod.update_task.is_some() {
            continue;
        }
        
        // Get next region to update
        let region = if let Some(region) = physics_lod.update_queue.pop() {
            region
        } else {
            // Find oldest region that needs update
            let update_freq = match physics_lod.lod_level {
                TerrainLODLevel::High => settings.update_frequencies[0],
                TerrainLODLevel::Medium => settings.update_frequencies[1],
                TerrainLODLevel::Low => settings.update_frequencies[2],
                TerrainLODLevel::VeryLow => settings.update_frequencies[3],
            };
            
            let oldest_region = physics_lod.region_updates.iter()
                .filter(|(region, &last_update)| {
                    current_time - last_update > update_freq as f64
                })
                .min_by(|a, b| a.1.partial_cmp(b.1).unwrap())
                .map(|(region, _)| region.clone());
                
            if let Some(region) = oldest_region {
                region
            } else {
                continue;
            }
        };

        // Prepare data for async task
        let points_per_side = match physics_lod.lod_level {
            TerrainLODLevel::High => 32,
            TerrainLODLevel::Medium => 24,
            TerrainLODLevel::Low => 16,
            TerrainLODLevel::VeryLow => 8,
        };
        
        let cell_size = physics_lod.cell_size;
        let heightmap = heightmap.clone();
        let transform = *transform;
        let region_clone = region.clone();
        
        // Spawn async task to generate collision data for the region
        let task = thread_pool.spawn(async move {
            let mut collision_points = Vec::new();
            let step = cell_size.x;
            
            let start_x = region_clone.x as f32 * (points_per_side as f32 * step);
            let start_z = region_clone.z as f32 * (points_per_side as f32 * step);
            
            for x in 0..points_per_side {
                for z in 0..points_per_side {
                    let local_pos = Vec3::new(
                        start_x + x as f32 * step,
                        0.0,
                        start_z + z as f32 * step,
                    );
                    
                    let world_pos = transform.transform_point(local_pos);
                    let height = heightmap.sample_height_at_lod(
                        Vec2::new(world_pos.x, world_pos.z),
                        region_clone.lod_level,
                    );
                    
                    world_pos.y = height;

                    let normal = heightmap.calculate_normal_at_lod(
                        Vec2::new(world_pos.x, world_pos.z),
                        region_clone.lod_level,
                    );
                    
                    let slope = normal.dot(Vec3::Y).abs();
                    let terrain_props = determine_terrain_properties(height, slope);

                    collision_points.push((world_pos, terrain_props));
                }
            }
            
            (region_clone, collision_points)
        });
        
        physics_lod.update_task = Some(task);
        physics_lod.updating_region = Some(region);
    }
}

/// System to process completed collision update tasks
pub fn process_collision_updates(
    mut query: Query<(Entity, &mut TerrainPhysicsLOD)>,
    time: Res<Time>,
) {
    for (_, mut physics_lod) in query.iter_mut() {
        if let Some(task) = physics_lod.update_task.take() {
            if let Some((region, new_cache)) = future::block_on(future::poll_once(task)) {
                // Update the cache for this region
                physics_lod.collision_cache.insert(region.clone(), new_cache);
                physics_lod.region_updates.insert(region.clone(), time.elapsed_seconds_f64());
                
                // Remove from dirty regions if present
                physics_lod.dirty_regions.remove(&region);
                physics_lod.updating_region = None;
            } else {
                // Task not complete, put it back
                physics_lod.update_task = Some(task);
            }
        }
    }
}

/// Enhanced terrain properties determination based on multiple factors
fn determine_terrain_properties(height: f32, slope: f32) -> TerrainProperties {
    // Height-based zones
    let base_type = match height {
        h if h > 100.0 => PhysicsTerrainType::Snow, // Mountain peaks
        h if h > 80.0 => PhysicsTerrainType::Rock,  // High elevation
        h if h > 60.0 => PhysicsTerrainType::Gravel, // Upper slopes
        h if h > 40.0 => PhysicsTerrainType::Dirt,   // Mid elevation
        h if h > 20.0 => PhysicsTerrainType::Sand,   // Lower slopes
        h if h > 10.0 => PhysicsTerrainType::Mud,    // Near water level
        _ => PhysicsTerrainType::Water               // Water level and below
    };

    // Slope-based modifications
    let terrain_type = match (base_type, slope) {
        // Very steep slopes become rock regardless of height
        (_, s) if s > 0.8 => PhysicsTerrainType::Rock,
        
        // Moderate slopes modify certain terrain types
        (PhysicsTerrainType::Dirt, s) if s > 0.4 => PhysicsTerrainType::Gravel,
        (PhysicsTerrainType::Sand, s) if s > 0.3 => PhysicsTerrainType::Gravel,
        
        // Shallow slopes can accumulate moisture
        (PhysicsTerrainType::Dirt, s) if s < 0.1 => PhysicsTerrainType::Mud,
        
        // Default to base type if no special cases apply
        (t, _) => t
    };

    // Create properties with terrain-specific parameters
    let mut props = TerrainProperties::new(terrain_type);
    
    // Adjust properties based on slope and type
    props.friction = match terrain_type {
        PhysicsTerrainType::Rock => 0.8 - (slope * 0.3), // Less friction on steeper rock
        PhysicsTerrainType::Gravel => 0.6 - (slope * 0.2),
        PhysicsTerrainType::Dirt => 0.5 - (slope * 0.1),
        PhysicsTerrainType::Sand => 0.4 + (slope * 0.1), // More friction on slopes due to buildup
        PhysicsTerrainType::Mud => 0.3.max(0.1 + slope), // Very slippery when flat
        PhysicsTerrainType::Snow => 0.2 + (slope * 0.2), // More grip on slopes
        PhysicsTerrainType::Water => 0.1,
    };

    // Adjust density based on type and compression
    props.density = match terrain_type {
        PhysicsTerrainType::Rock => 2.5,
        PhysicsTerrainType::Gravel => 1.8,
        PhysicsTerrainType::Dirt => 1.5,
        PhysicsTerrainType::Sand => 1.6,
        PhysicsTerrainType::Mud => 1.7,
        PhysicsTerrainType::Snow => 0.9,
        PhysicsTerrainType::Water => 1.0,
    };

    // Adjust deformability
    props.deformability = match terrain_type {
        PhysicsTerrainType::Rock => 0.1,
        PhysicsTerrainType::Gravel => 0.4,
        PhysicsTerrainType::Dirt => 0.7,
        PhysicsTerrainType::Sand => 0.9,
        PhysicsTerrainType::Mud => 1.0,
        PhysicsTerrainType::Snow => 0.8,
        PhysicsTerrainType::Water => 0.0,
    };

    props
}

/// Component for region tooltip
#[derive(Component)]
struct RegionTooltip;

/// System to update mouse position
pub fn update_mouse_position(
    mut debug_settings: ResMut<PhysicsLODDebug>,
    windows: Query<&Window>,
    camera: Query<(&Camera, &GlobalTransform)>,
) {
    let (camera, camera_transform) = camera.single();
    let window = windows.single();
    
    if let Some(world_pos) = window.cursor_position()
        .and_then(|cursor| camera.viewport_to_world(camera_transform, cursor))
        .map(|ray| ray.origin + ray.direction * (ray.origin.y / ray.direction.y).abs())
    {
        debug_settings.mouse_world_pos = world_pos;
    }
}

/// System to handle keyboard input for debug visualization
pub fn handle_debug_input(
    keyboard: Res<Input<KeyCode>>,
    mut debug_settings: ResMut<PhysicsLODDebug>,
) {
    // Existing toggles
    if keyboard.just_pressed(KeyCode::F1) {
        debug_settings.show_regions = !debug_settings.show_regions;
    }
    if keyboard.just_pressed(KeyCode::F2) {
        debug_settings.show_dirty_regions = !debug_settings.show_dirty_regions;
    }
    if keyboard.just_pressed(KeyCode::F3) {
        debug_settings.show_lod_levels = !debug_settings.show_lod_levels;
    }
    
    // New toggles
    if keyboard.just_pressed(KeyCode::F4) {
        debug_settings.show_collision_points = !debug_settings.show_collision_points;
    }
    if keyboard.just_pressed(KeyCode::F5) {
        debug_settings.show_update_frequencies = !debug_settings.show_update_frequencies;
    }
    if keyboard.just_pressed(KeyCode::F6) {
        debug_settings.show_tooltips = !debug_settings.show_tooltips;
    }
    
    // Cycle visualization modes with Tab
    if keyboard.just_pressed(KeyCode::Tab) {
        debug_settings.viz_mode = (debug_settings.viz_mode + 1) % 4;
        match debug_settings.viz_mode {
            0 => { // Regions only
                debug_settings.show_regions = true;
                debug_settings.show_dirty_regions = false;
                debug_settings.show_lod_levels = false;
                debug_settings.show_collision_points = false;
            }
            1 => { // LOD levels
                debug_settings.show_regions = false;
                debug_settings.show_dirty_regions = false;
                debug_settings.show_lod_levels = true;
                debug_settings.show_collision_points = false;
            }
            2 => { // Collision points
                debug_settings.show_regions = false;
                debug_settings.show_dirty_regions = false;
                debug_settings.show_lod_levels = false;
                debug_settings.show_collision_points = true;
            }
            3 => { // All
                debug_settings.show_regions = true;
                debug_settings.show_dirty_regions = true;
                debug_settings.show_lod_levels = true;
                debug_settings.show_collision_points = true;
            }
            _ => {}
        }
    }
}

/// System to render debug visualization for physics LOD
pub fn render_physics_lod_debug(
    mut gizmos: Gizmos,
    debug_settings: Res<PhysicsLODDebug>,
    terrain_physics: Query<&TerrainPhysicsLOD>,
    mut commands: Commands,
    time: Res<Time>,
    tooltip_query: Query<Entity, With<RegionTooltip>>,
) {
    // Remove existing tooltips
    for entity in tooltip_query.iter() {
        commands.entity(entity).despawn();
    }

    for physics_lod in terrain_physics.iter() {
        // Show collision regions
        if debug_settings.show_regions {
            for region in physics_lod.collision_cache.keys() {
                let min = region.min_bounds();
                let max = region.max_bounds();
                let center = min + (max - min) * 0.5;
                
                gizmos.cuboid(
                    Transform::from_translation(center)
                        .with_scale(Vec3::new(max.x - min.x, 10.0, max.z - min.z)),
                    Color::rgba(0.0, 1.0, 0.0, 0.2),
                );

                // Show update frequency if enabled
                if debug_settings.show_update_frequencies {
                    if let Some(&last_update) = physics_lod.region_updates.get(region) {
                        let time_since_update = time.elapsed_seconds_f64() - last_update;
                        let color = if time_since_update < 0.016 {
                            Color::GREEN
                        } else if time_since_update < 0.033 {
                            Color::YELLOW
                        } else {
                            Color::RED
                        };
                        
                        gizmos.line(
                            center,
                            center + Vec3::new(0.0, time_since_update as f32 * 5.0, 0.0),
                            color,
                        );
                    }
                }
            }
        }

        // Show collision points
        if debug_settings.show_collision_points {
            for points in physics_lod.collision_cache.values() {
                for (pos, props) in points {
                    let color = match props.terrain_type {
                        PhysicsTerrainType::Rock => Color::GRAY,
                        PhysicsTerrainType::Gravel => Color::BEIGE,
                        PhysicsTerrainType::Dirt => Color::BROWN,
                        PhysicsTerrainType::Sand => Color::YELLOW,
                        PhysicsTerrainType::Mud => Color::rgba(0.5, 0.3, 0.0, 1.0),
                        PhysicsTerrainType::Snow => Color::WHITE,
                        PhysicsTerrainType::Water => Color::BLUE,
                    };
                    
                    gizmos.sphere(*pos, Quat::IDENTITY, 0.2, color);
                }
            }
        }

        // Show tooltips
        if debug_settings.show_tooltips {
            for (region, points) in &physics_lod.collision_cache {
                let min = region.min_bounds();
                let max = region.max_bounds();
                let center = min + (max - min) * 0.5;
                
                if debug_settings.mouse_world_pos.distance(center) < (max.x - min.x) * 0.5 {
                    let mut tooltip = format!(
                        "Region ({}, {})\nLOD: {:?}\nPoints: {}\n",
                        region.x, region.z, region.lod_level, points.len()
                    );
                    
                    if let Some(&last_update) = physics_lod.region_updates.get(region) {
                        tooltip.push_str(&format!(
                            "Last Update: {:.1}ms ago\n",
                            (time.elapsed_seconds_f64() - last_update) * 1000.0
                        ));
                    }
                    
                    commands.spawn((
                        Text2dBundle {
                            text: Text::from_section(
                                tooltip,
                                TextStyle {
                                    font_size: 16.0,
                                    color: Color::WHITE,
                                    ..default()
                                },
                            ),
                            transform: Transform::from_translation(center + Vec3::new(0.0, 5.0, 0.0)),
                            text_anchor: Anchor::BottomCenter,
                            ..default()
                        },
                        RegionTooltip,
                    ));
                }
            }
        }

        // Show dirty regions
        if debug_settings.show_dirty_regions {
            for region in &physics_lod.dirty_regions {
                gizmos.cuboid(
                    Transform::from_translation(region.min_bounds() + (region.max_bounds() - region.min_bounds()) * 0.5)
                        .with_scale(Vec3::new(region.max_bounds().x - region.min_bounds().x, region.max_bounds().z - region.min_bounds().z, 15.0)),
                    Color::rgba(1.0, 0.0, 0.0, 0.3),
                );
            }
        }

        // Show LOD levels
        if debug_settings.show_lod_levels {
            for (region, data) in &physics_lod.collision_cache {
                let color = PhysicsLODDebug::get_lod_color(data.lod_level as u32);
                gizmos.cuboid(
                    Transform::from_translation(region.min_bounds() + (region.max_bounds() - region.min_bounds()) * 0.5)
                        .with_scale(Vec3::new(region.max_bounds().x - region.min_bounds().x, region.max_bounds().z - region.min_bounds().z, 5.0)),
                    color,
                );
            }
        }
    }
}

/// System to update performance metrics
pub fn update_physics_metrics(
    time: Res<Time>,
    mut metrics: ResMut<PhysicsLODMetrics>,
    query: Query<&TerrainPhysicsLOD>,
) {
    let current_time = time.elapsed_seconds_f64();
    let frame_time = (current_time - metrics.last_update) as f32;
    metrics.update_average(frame_time);
    metrics.last_update = current_time;

    // Update region counts
    let mut active_regions = 0;
    let mut collision_points = 0;
    let mut dirty_regions = 0;

    for physics_lod in query.iter() {
        active_regions += physics_lod.collision_cache.len();
        dirty_regions += physics_lod.dirty_regions.len();
        collision_points += physics_lod.collision_cache.values()
            .map(|points| points.len())
            .sum::<usize>();
    }

    metrics.active_regions = active_regions;
    metrics.dirty_region_count = dirty_regions;
    metrics.collision_point_count = collision_points;
}

/// System to update the debug status text
pub fn update_debug_text(
    mut commands: Commands,
    debug_settings: Res<PhysicsLODDebug>,
    metrics: Res<PhysicsLODMetrics>,
    query: Query<Entity, With<DebugText>>,
) {
    // Remove existing debug text
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }

    // Create status text
    let mut status = String::new();
    
    // Visualization toggles
    status.push_str(&format!("F1: Collision Regions [{}]\n", if debug_settings.show_regions { "ON" } else { "OFF" }));
    status.push_str(&format!("F2: Dirty Regions [{}]\n", if debug_settings.show_dirty_regions { "ON" } else { "OFF" }));
    status.push_str(&format!("F3: LOD Levels [{}]\n", if debug_settings.show_lod_levels { "ON" } else { "OFF" }));
    status.push_str("\n");
    
    // Performance metrics
    status.push_str(&format!("Update Time: {:.2}ms (avg: {:.2}ms)\n", 
        metrics.update_times.last().unwrap_or(&0.0) * 1000.0,
        metrics.avg_update_time * 1000.0));
    status.push_str(&format!("Active Regions: {}\n", metrics.active_regions));
    status.push_str(&format!("Dirty Regions: {}\n", metrics.dirty_region_count));
    status.push_str(&format!("Collision Points: {}\n", metrics.collision_point_count));

    // Spawn new text
    commands.spawn((
        Text2dBundle {
            text: Text::from_sections(vec![
                TextSection {
                    value: status,
                    style: TextStyle {
                        font_size: 20.0,
                        color: Color::WHITE,
                        ..default()
                    },
                },
            ]),
            transform: Transform::from_xyz(-580.0, 320.0, 100.0),
            text_anchor: Anchor::TopLeft,
            ..default()
        },
        DebugText,
    ));
}

/// Plugin to handle LOD-aware terrain physics
pub struct TerrainPhysicsLODPlugin;

impl Plugin for TerrainPhysicsLODPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PhysicsLODSettings>()
            .init_resource::<PhysicsLODDebug>()
            .init_resource::<PhysicsLODMetrics>()
            .add_systems(Update, (
                update_terrain_physics_lod,
                handle_terrain_deformation,
                update_terrain_collision_data,
                process_collision_updates,
                update_physics_metrics,
                update_mouse_position,
                render_physics_lod_debug,
                handle_debug_input,
                update_debug_text,
            ).chain());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_physics_lod_transitions() {
        let mut physics_lod = TerrainPhysicsLOD::default();
        assert_eq!(physics_lod.lod_level, TerrainLODLevel::High);

        // Test cell size updates
        physics_lod.lod_level = TerrainLODLevel::Medium;
        assert_eq!(physics_lod.cell_size, Vec2::new(1.0, 1.0)); // Initial size

        // Test cache invalidation
        physics_lod.collision_cache.insert(CollisionRegion::new(Vec3::ZERO, Vec2::new(1.0, 1.0), TerrainLODLevel::High), Vec::new());
        physics_lod.lod_level = TerrainLODLevel::Low;
        assert!(physics_lod.collision_cache.is_empty());
    }

    #[test]
    fn test_terrain_properties_determination() {
        // Test rock at high elevation
        let props = determine_terrain_properties(85.0, 0.5);
        assert!(matches!(props.terrain_type, PhysicsTerrainType::Rock));

        // Test mud at low elevation
        let props = determine_terrain_properties(15.0, 0.3);
        assert!(matches!(props.terrain_type, PhysicsTerrainType::Mud));

        // Test rock on steep slope
        let props = determine_terrain_properties(50.0, 0.9);
        assert!(matches!(props.terrain_type, PhysicsTerrainType::Rock));
    }
} 