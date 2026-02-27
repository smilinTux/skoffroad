use bevy::prelude::*;
use bevy::render::RenderSet;
use crate::game::plugins::particle_system::{ParticleSystem, ParticleMaterial};

use super::{
    compute::{ParticleComputePipeline, dispatch_particle_compute, update_particle_params},
    emitter::update_emitter_transforms,
    material::{ParticleMaterialPipeline, update_material_params, create_material_bind_groups},
    sorting::{ParticleSortPipeline, dispatch_particle_sort, init_particle_indices},
};

/// Plugin for managing particle systems
pub struct ParticleSystemPlugin;

impl Plugin for ParticleSystemPlugin {
    fn build(&self, app: &mut App) {
        // Initialize resources
        // app.init_resource::<ParticleSystemSettings>()
        app.init_resource::<ParticleComputePipeline>()
            .init_resource::<ParticleSortPipeline>()
            .init_resource::<ParticleMaterialPipeline>();

        // Add systems
        app.add_systems(Update, (
            update_emitter_transforms,
            update_particle_params,
            init_particle_indices,
            update_material_params,
            create_material_bind_groups,
        ));

        // Add render systems
        app.add_systems(RenderSet::Render, (
            dispatch_particle_compute,
            dispatch_particle_sort,
            dispatch_particle_render,
        ));
    }
}

/// System to dispatch particle render
fn dispatch_particle_render(
    particles: Query<(&ParticleSystem, &ParticleMaterial)>,
    material_pipeline: Res<ParticleMaterialPipeline>,
) {
    for (particle_system, material) in particles.iter() {
        // Dispatch render pipeline for particle rendering
        // This is a placeholder - implement actual render dispatch
    }
} 