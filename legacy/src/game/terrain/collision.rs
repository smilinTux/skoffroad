use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use crate::physics::terrain_properties::TerrainProperties;
use super::physics_lod::{TerrainPhysicsLOD, PhysicsLODSettings};
use super::{Heightmap, TerrainMesh};
use crate::physics::terrain_properties::PhysicsTerrainType;

/// Surface properties for different terrain types
#[derive(Clone, Copy)]
pub struct SurfaceProperties {
    pub friction_static: f32,
    pub friction_dynamic: f32,
    pub restitution: f32,
    pub damping: f32,
}

impl Default for SurfaceProperties {
    fn default() -> Self {
        Self {
            friction_static: 0.8,
            friction_dynamic: 0.5,
            restitution: 0.3,
            damping: 0.1,
        }
    }
}

/// Component for terrain collision data
#[derive(Component)]
pub struct TerrainCollider {
    /// Collision shape data
    pub collider: Collider,
    /// Current terrain properties
    pub properties: TerrainProperties,
}

/// System to update terrain colliders based on LOD physics data
pub fn update_terrain_colliders(
    mut commands: Commands,
    mut query: Query<(Entity, &TerrainPhysicsLOD, Option<&mut TerrainCollider>)>,
    settings: Res<PhysicsLODSettings>,
) {
    for (entity, physics_lod, collider) in query.iter_mut() {
        // Skip if cache is empty
        if physics_lod.collision_cache.is_empty() {
            continue;
        }

        // Create heightfield points from cache
        let points_per_side = (physics_lod.collision_cache.len() as f32).sqrt() as usize;
        let mut heights = vec![0.0; points_per_side * points_per_side];
        let mut properties = Vec::new();

        for (i, (pos, props)) in physics_lod.collision_cache.iter().enumerate() {
            heights[i] = pos.y;
            properties.push(*props);
        }

        // Create heightfield collider
        let collider = Collider::heightfield(
            heights,
            points_per_side,
            points_per_side,
            Vec3::new(
                physics_lod.cell_size.x * points_per_side as f32,
                1.0,
                physics_lod.cell_size.y * points_per_side as f32,
            ),
        );

        // Update or insert collider component
        match collider {
            Some(mut existing) => {
                *existing.collider = collider;
                // Update properties based on majority terrain type
                let mut type_counts = std::collections::HashMap::new();
                for prop in properties.iter() {
                    *type_counts.entry(prop.terrain_type).or_insert(0) += 1;
                }
                let majority_type = type_counts.iter()
                    .max_by_key(|(_, &count)| count)
                    .map(|(&typ, _)| typ)
                    .unwrap_or(existing.properties.terrain_type);
                existing.properties = TerrainProperties::new(majority_type);
            }
            None => {
                // Calculate average properties for new collider
                let avg_properties = if !properties.is_empty() {
                    let mut type_counts = std::collections::HashMap::new();
                    for prop in properties.iter() {
                        *type_counts.entry(prop.terrain_type).or_insert(0) += 1;
                    }
                    let majority_type = type_counts.iter()
                        .max_by_key(|(_, &count)| count)
                        .map(|(&typ, _)| typ)
                        .unwrap_or_default();
                    TerrainProperties::new(majority_type)
                } else {
                    TerrainProperties::default()
                };

                commands.entity(entity).insert(TerrainCollider {
                    collider,
                    properties: avg_properties,
                });
            }
        }
    }
}

/// Plugin to handle terrain collision
pub struct TerrainCollisionPlugin;

impl Plugin for TerrainCollisionPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, update_terrain_colliders);
    }
}

/// System that generates collision shapes for terrain
pub fn generate_terrain_colliders(
    mut commands: Commands,
    heightmap_query: Query<(Entity, &Heightmap), Added<Heightmap>>,
) {
    for (entity, heightmap) in heightmap_query.iter() {
        let cell_size = Vec2::new(
            heightmap.size.x / heightmap.dimensions.x as f32,
            heightmap.size.y / heightmap.dimensions.y as f32,
        );
        
        let collider = TerrainCollider::new(heightmap, cell_size);
        
        // Create heightfield collider
        let heights: Vec<_> = heightmap.heights.iter().map(|&h| h as f32).collect();
        let collider_shape = Collider::heightfield(
            heights,
            heightmap.dimensions.x as usize,
            heightmap.dimensions.y as usize,
            Vec3::new(heightmap.size.x, 1.0, heightmap.size.y),
        );
        
        commands.entity(entity).insert((
            collider,
            collider_shape,
            RigidBody::Fixed,
            Friction::coefficient(0.7),
            Restitution::coefficient(0.3),
            Damping {
                linear_damping: 0.1,
                angular_damping: 0.1,
            },
        ));
    }
}

/// System that handles terrain-vehicle collision response
pub fn handle_terrain_collisions(
    mut collision_events: EventReader<CollisionEvent>,
    terrain_query: Query<&TerrainCollider>,
    mut vehicle_query: Query<(&mut Transform, &mut Velocity, &mut ExternalForce), Without<TerrainCollider>>,
) {
    for collision_event in collision_events.iter() {
        if let CollisionEvent::Started(e1, e2, _) = collision_event {
            // Check if one entity is terrain and the other is a vehicle
            let (terrain_entity, vehicle_entity) = if terrain_query.contains(*e1) {
                (*e1, *e2)
            } else if terrain_query.contains(*e2) {
                (*e2, *e1)
            } else {
                continue;
            };
            
            // Get terrain and vehicle data
            if let (Ok(terrain), Ok((mut vehicle_transform, mut vehicle_velocity, mut vehicle_force))) = (
                terrain_query.get(terrain_entity),
                vehicle_query.get_mut(vehicle_entity),
            ) {
                // Get terrain height and properties at vehicle position
                let vehicle_pos = vehicle_transform.translation.xz();
                if let Some(terrain_height) = terrain.get_height_at(vehicle_pos.into()) {
                    let surface_props = terrain.get_surface_properties(terrain_height);
                    
                    // Adjust vehicle height to stay above terrain
                    let min_height = terrain_height + 0.1; // Small offset to prevent clipping
                    if vehicle_transform.translation.y < min_height {
                        vehicle_transform.translation.y = min_height;
                        
                        // Calculate normal force
                        let penetration = min_height - vehicle_transform.translation.y;
                        let spring_force = 1000.0 * penetration;
                        let damping_force = surface_props.damping * 1000.0 * -vehicle_velocity.linvel.y;
                        let normal_force = spring_force + damping_force;
                        
                        // Apply friction force
                        let lateral_velocity = Vec2::new(vehicle_velocity.linvel.x, vehicle_velocity.linvel.z);
                        if lateral_velocity.length() > 0.01 {
                            let friction_coef = if lateral_velocity.length() < 0.1 {
                                surface_props.friction_static
                            } else {
                                surface_props.friction_dynamic
                            };
                            
                            let friction_force = -lateral_velocity.normalize() * normal_force * friction_coef;
                            vehicle_force.force.x += friction_force.x;
                            vehicle_force.force.z += friction_force.y;
                        }
                        
                        // Apply normal force
                        vehicle_velocity.linvel.y += normal_force * 0.016; // Assuming 60fps
                        
                        // Apply restitution
                        if vehicle_velocity.linvel.y < 0.0 {
                            vehicle_velocity.linvel.y *= -surface_props.restitution;
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::physics::terrain_properties::PhysicsTerrainType;

    #[test]
    fn test_surface_properties() {
        let heightmap = Heightmap::new(UVec2::new(10, 10), Vec2::new(100.0, 100.0));
        let collider = TerrainCollider::new(&heightmap, Vec2::new(10.0, 10.0));
        
        // Test low terrain properties
        let props = collider.get_surface_properties(5.0);
        assert_eq!(props.friction_static, 0.9);
        
        // Test high terrain properties
        let props = collider.get_surface_properties(50.0);
        assert_eq!(props.friction_static, 0.3);
    }

    #[test]
    fn test_collision_response() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, RapierPhysicsPlugin::<NoUserData>::default()))
            .add_systems(Update, (generate_terrain_colliders, handle_terrain_collisions));
            
        // Create test heightmap
        let heightmap = Heightmap::new(UVec2::new(10, 10), Vec2::new(100.0, 100.0));
        let heightmap_entity = app.world.spawn(heightmap).id();
        
        // Create test vehicle
        let vehicle_entity = app.world.spawn((
            Transform::default(),
            Velocity::default(),
            ExternalForce::default(),
        )).id();
        
        // Run systems
        app.update();
        
        // Verify components
        let terrain = app.world.entity(heightmap_entity);
        assert!(terrain.contains::<TerrainCollider>());
        assert!(terrain.contains::<Friction>());
        assert!(terrain.contains::<Restitution>());
        assert!(terrain.contains::<Damping>());
    }

    #[test]
    fn test_collider_creation() {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            RapierPhysicsPlugin::<NoUserData>::default(),
        ));
        
        // Create test entity with physics LOD
        let entity = app.world.spawn((
            TerrainPhysicsLOD {
                lod_level: super::super::lod::TerrainLODLevel::High,
                cell_size: Vec2::new(1.0, 1.0),
                collision_cache: vec![
                    (Vec3::new(0.0, 1.0, 0.0), TerrainProperties::new(PhysicsTerrainType::Dirt)),
                    (Vec3::new(1.0, 2.0, 0.0), TerrainProperties::new(PhysicsTerrainType::Rock)),
                    (Vec3::new(0.0, 1.5, 1.0), TerrainProperties::new(PhysicsTerrainType::Dirt)),
                    (Vec3::new(1.0, 2.5, 1.0), TerrainProperties::new(PhysicsTerrainType::Rock)),
                ],
                last_update: 0.0,
            },
        )).id();

        // Run systems
        app.update();

        // Verify collider was created
        let collider = app.world.entity(entity).get::<TerrainCollider>();
        assert!(collider.is_some());
    }
} 