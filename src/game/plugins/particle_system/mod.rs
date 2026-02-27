pub mod buffer;
pub mod compute;
pub mod emitter;
pub mod material;
pub mod particle;
pub mod plugin;
pub mod sorting;

mod animation;
mod presets;
mod texture_gen;
mod gradient;
mod special_effects;
mod basic_particle;
mod examples;

mod prelude {
    // pub use super::buffer::*;
    // pub use super::compute::*;
    // pub use super::emitter::*;
    // pub use super::material::*;
    // pub use super::particle::*;
    // pub use super::plugin::*;
    // pub use super::sorting::*;
}

// pub use prelude::*;
pub use animation::ParticleAnimationPlugin;
pub use compute::ParticleComputePipeline;
// pub use emitter::{BoxEmitter, PointEmitter, SphereEmitter};
pub use material::ParticleMaterial;
pub use particle::ParticleSystem;
pub use texture_gen::ParticleTextureGenPlugin;
// pub use gradient::*;
// pub use special_effects::*;
pub use basic_particle::{
    BasicParticleEffect,
    BasicParticleConfig,
    // BasicParticlePlugin,
    spawn_basic_particle_effect,
};
// pub use examples::basic_particles::BasicParticleExamplePlugin;

use bevy::prelude::*;

/// Plugin that sets up the particle system
pub struct ParticleSystemPlugin;

impl Plugin for ParticleSystemPlugin {
    fn build(&self, app: &mut App) {
        app
            // Add our sub-plugins
            .add_plugins((
                ParticleAnimationPlugin,
                ParticleTextureGenPlugin,
                material::ParticleMaterialPlugin,
            ))
            // Add our resources
            .init_resource::<ParticleComputePipeline>()
            // Add our systems
            .add_systems(Update, (
                particle::update_particle_params,
                compute::dispatch_particle_compute,
            ))
            .add_systems(Startup, (
                presets::spawn_example_effects,
                special_effects::spawn_special_effects_demo,
            ));
    }
} 