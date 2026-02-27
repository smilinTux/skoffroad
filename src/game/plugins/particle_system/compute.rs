use bevy::{
    prelude::*,
    render::{
        render_resource::{
            BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
            BindGroupLayoutDescriptor, BindGroupLayoutEntry, BufferBindingType, ComputePipeline,
            PipelineCache, ShaderStages,
        },
        renderer::RenderDevice,
    },
};

use super::{
    buffer::ParticleBufferManager,
    particle::ParticleSystem,
    presets::SimulationParams,
};

/// Resource for managing the particle compute pipeline
#[derive(Resource)]
pub struct ParticleComputePipeline {
    pub pipeline: ComputePipeline,
    pub bind_group_layout: BindGroupLayout,
}

impl FromWorld for ParticleComputePipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let pipeline_cache = world.resource::<PipelineCache>();

        // Create bind group layout
        let bind_group_layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("particle_compute_bind_group_layout"),
            entries: &[
                // Particle buffer A
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::COMPUTE,
                    ty: bevy::render::render_resource::BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Particle buffer B
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::COMPUTE,
                    ty: bevy::render::render_resource::BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Simulation parameters
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::COMPUTE,
                    ty: bevy::render::render_resource::BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        // Create compute pipeline
        let pipeline = pipeline_cache.get_compute_pipeline(
            "particle_compute_pipeline",
            &bind_group_layout,
            include_str!("shaders/particle.wgsl"),
        );

        Self {
            pipeline,
            bind_group_layout,
        }
    }
}

/// System to dispatch the particle compute shader
pub fn dispatch_particle_compute(
    mut particles: Query<(&mut ParticleSystem, &mut ParticleBufferManager)>,
    compute_pipeline: Res<ParticleComputePipeline>,
    render_device: Res<RenderDevice>,
) {
    for (particle_system, mut buffer_manager) in particles.iter_mut() {
        // Create bind group for compute shader
        let bind_group = render_device.create_bind_group(&BindGroupDescriptor {
            label: Some("particle_compute_bind_group"),
            layout: &compute_pipeline.bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: buffer_manager.read_buffer().as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: buffer_manager.write_buffer().as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: particle_system.params.as_entire_binding(),
                },
            ],
        });

        // Create compute pass
        let mut compute_pass = render_device.create_command_encoder("particle_compute_pass");
        {
            let mut pass = compute_pass.begin_compute_pass();
            pass.set_pipeline(&compute_pipeline.pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            
            // Dispatch workgroups based on particle count
            let workgroup_count = (particle_system.particle_count + 63) / 64;
            pass.dispatch_workgroups(workgroup_count, 1, 1);
        }

        // Submit compute pass
        render_device.queue().submit(Some(compute_pass.finish()));

        // Swap buffers
        buffer_manager.swap_buffers();
    }
}

/// System to update particle simulation parameters
pub fn update_particle_params(
    mut particles: Query<(&mut ParticleSystem, &SimulationParams)>,
    time: Res<Time>,
) {
    for (mut particle_system, params) in particles.iter_mut() {
        particle_system.update_simulation_params(params, time.delta_seconds());
    }
} 