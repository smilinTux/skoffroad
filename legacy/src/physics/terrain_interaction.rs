use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use crate::physics::terrain_properties::{PhysicsTerrainType, TerrainProperties};
use crate::physics::wheel::{Wheel, WheelForces};
use crate::terrain::deformation::{TerrainDeformationEvent, DeformationPoint};

/// Component that stores terrain interaction data for a wheel
#[derive(Component, Default, Debug)]
pub struct TerrainInteraction {
    /// Current terrain type the wheel is in contact with
    pub current_terrain: PhysicsTerrainType,
    /// Depth of wheel penetration into terrain
    pub penetration_depth: f32,
    /// Accumulated displacement for terrain deformation
    pub displacement: Vec3,
    /// Contact normal with terrain
    pub contact_normal: Vec3,
    /// Terrain properties at contact point
    pub terrain_properties: TerrainProperties,
}

/// System that updates terrain interaction for wheels
#[allow(clippy::type_complexity)]
pub fn update_terrain_interaction(
    mut wheels: Query<(&mut TerrainInteraction, &Wheel, &mut WheelForces, &GlobalTransform)>,
    rapier_context: Res<RapierContext>,
    mut deformation_events: EventWriter<TerrainDeformationEvent>,
) {
    for (mut interaction, wheel, mut forces, transform) in wheels.iter_mut() {
        // Cast ray downward from wheel center to detect terrain
        let ray_origin = transform.translation();
        let ray_dir = Vec3::NEG_Y;
        let max_distance = wheel.radius * 2.0;
        
        if let Some((collider, intersection)) = rapier_context.cast_ray(
            ray_origin,
            ray_dir,
            max_distance,
            true,
            QueryFilter::default(),
        ) {
            // Get terrain properties from collider user data
            if let Some(terrain_type) = get_terrain_type_from_collider(&rapier_context, collider) {
                interaction.current_terrain = terrain_type;
                interaction.terrain_properties = TerrainProperties::from_terrain_type(terrain_type);
                
                // Update penetration depth
                interaction.penetration_depth = max_distance - intersection.toi;
                interaction.contact_normal = intersection.normal;
                
                // Apply terrain-specific forces
                apply_terrain_forces(&mut forces, &interaction, wheel);
                
                // Update terrain displacement
                update_terrain_displacement(&mut interaction, &forces);

                // Generate deformation event based on penetration
                if interaction.penetration_depth > 0.05 { // Minimum depth threshold
                    deformation_events.send(TerrainDeformationEvent {
                        chunk_pos: transform.translation.floor(),
                        world_pos: transform.translation + transform.up() * intersection.toi,
                        radius: 1.0, // Base radius, could be configurable
                        strength: interaction.penetration_depth * 0.5, // Scale strength with penetration
                    });
                }
            }
        } else {
            // No terrain contact - reset interaction data
            *interaction = TerrainInteraction::default();
        }
    }
}

/// Helper function to get terrain type from collider user data
fn get_terrain_type_from_collider(context: &RapierContext, collider: ColliderHandle) -> Option<PhysicsTerrainType> {
    // TODO: Implement terrain type lookup from collider user data
    // For now, return default terrain type
    Some(PhysicsTerrainType::Dirt)
}

/// Apply forces based on terrain properties
fn apply_terrain_forces(forces: &mut WheelForces, interaction: &TerrainInteraction, wheel: &Wheel) {
    let props = &interaction.terrain_properties;
    
    // Modify friction based on terrain
    forces.friction_coefficient *= props.friction_coefficient;
    
    // Add rolling resistance
    let rolling_resistance = -forces.velocity.normalize_or_zero() * props.rolling_resistance;
    forces.total_force += rolling_resistance;
    
    // Add terrain roughness effect
    let roughness_force = Vec3::new(
        fastrand::f32() * props.surface_roughness,
        0.0,
        fastrand::f32() * props.surface_roughness
    );
    forces.total_force += roughness_force;
}

/// Update terrain displacement (e.g., tire tracks)
fn update_terrain_displacement(interaction: &mut TerrainInteraction, forces: &WheelForces) {
    // Only update displacement if there's significant force
    if forces.total_force.length_squared() > 1.0 {
        let displacement_delta = forces.total_force.normalize_or_zero() * 0.01;
        interaction.displacement += displacement_delta;
    }
}

/// Plugin to register terrain interaction systems
pub struct TerrainInteractionPlugin;

impl Plugin for TerrainInteractionPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<TerrainInteraction>()
           .add_systems(Update, update_terrain_interaction);
    }
} 