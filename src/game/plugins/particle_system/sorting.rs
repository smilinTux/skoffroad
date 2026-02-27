use bevy::{
    prelude::*,
    render::{
        render_resource::*,
        renderer::{RenderDevice, RenderQueue},
    },
};
use std::borrow::Cow;

use super::{
    buffer::ParticleBufferManager,
    particle::ParticleSystem,
};

/// Parameters for the particle sorting compute shader
#[derive(Clone, Copy, ShaderType)]
pub struct SortParams {
    /// Camera position for depth calculation
    pub camera_pos: Vec3,
    /// Number of active particles
    pub particle_count: u32,
    /// Current sort stage
    pub k: u32,
    /// Current sort step
    pub j: u32,
    /// Whether to sort ascending (0) or descending (1)
    pub sort_descending: u32,
    /// Padding to ensure 16-byte alignment
    pub _padding: Vec2,
}

/// Resource for managing the particle sorting pipeline
#[derive(Resource)]
pub struct ParticleSortPipeline {
    /// The compute pipeline for particle sorting
    pipeline: ComputePipeline,
    /// Bind group layout for the pipeline
    bind_group_layout: BindGroupLayout,
    /// Buffer for sort parameters
    params_buffer: Buffer,
    /// Whether sorting is enabled
    pub enabled: bool,
}

impl FromWorld for ParticleSortPipeline {
    fn from_world(world: &mut World) -> Self {
        let device = world.resource::<RenderDevice>();

        // Create bind group layout
        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("particle_sort_bind_group_layout"),
            entries: &[
                // Particle buffer
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: BufferSize::new(0),
                    },
                    count: None,
                },
                // Index buffer
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: BufferSize::new(0),
                    },
                    count: None,
                },
                // Sort parameters
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: BufferSize::new(0),
                    },
                    count: None,
                },
            ],
        });

        // Create parameters buffer
        let params_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("particle_sort_params_buffer"),
            size: std::mem::size_of::<SortParams>() as u64,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Load compute shader
        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("particle_sort_shader"),
            source: ShaderSource::Wgsl(include_str!("shaders/particle_sort.wgsl").into()),
        });

        // Create compute pipeline
        let pipeline = device.create_compute_pipeline(&ComputePipelineDescriptor {
            label: Some("particle_sort_pipeline"),
            layout: Some(&device.create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("particle_sort_pipeline_layout"),
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            })),
            module: &shader,
            entry_point: Cow::Borrowed("sort"),
        });

        Self {
            pipeline,
            bind_group_layout,
            params_buffer,
            enabled: true,
        }
    }
}

/// System to dispatch particle sorting compute shader
pub fn dispatch_particle_sort(
    sort_pipeline: Res<ParticleSortPipeline>,
    particle_systems: Query<(&ParticleSystem, &ParticleBufferManager)>,
    camera: Query<&GlobalTransform, With<Camera>>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
) {
    if !sort_pipeline.enabled {
        return;
    }

    let camera_transform = camera.single();
    let camera_pos = camera_transform.translation();

    let mut encoder = render_device.create_command_encoder(&CommandEncoderDescriptor {
        label: Some("particle_sort_encoder"),
    });

    for (particle_system, buffer_manager) in particle_systems.iter() {
        let n = particle_system.active_particles;
        if n <= 1 {
            continue;
        }

        // Calculate number of stages for bitonic sort
        let num_stages = 32 - n.leading_zeros();

        // Update sort parameters
        for k in 0..num_stages {
            for j in (0..=k).rev() {
                let params = SortParams {
                    camera_pos,
                    particle_count: n,
                    k: 1 << k,
                    j: 1 << j,
                    sort_descending: 1, // Sort back-to-front for alpha blending
                    _padding: Vec2::ZERO,
                };

                // Update parameters buffer
                render_queue.write_buffer(&sort_pipeline.params_buffer, 0, bytemuck::cast_slice(&[params]));

                // Create bind group
                let bind_group = render_device.create_bind_group(&BindGroupDescriptor {
                    label: Some("particle_sort_bind_group"),
                    layout: &sort_pipeline.bind_group_layout,
                    entries: &[
                        BindGroupEntry {
                            binding: 0,
                            resource: buffer_manager.read_buffer().as_entire_binding(),
                        },
                        BindGroupEntry {
                            binding: 1,
                            resource: buffer_manager.index_buffer().as_entire_binding(),
                        },
                        BindGroupEntry {
                            binding: 2,
                            resource: sort_pipeline.params_buffer.as_entire_binding(),
                        },
                    ],
                });

                // Dispatch compute shader
                {
                    let mut compute_pass = encoder.begin_compute_pass(&ComputePassDescriptor {
                        label: Some("particle_sort_pass"),
                    });
                    compute_pass.set_pipeline(&sort_pipeline.pipeline);
                    compute_pass.set_bind_group(0, &bind_group, &[]);

                    let workgroup_count = (n + 255) / 256;
                    compute_pass.dispatch_workgroups(workgroup_count, 1, 1);
                }
            }
        }
    }

    // Submit compute work
    render_queue.submit(std::iter::once(encoder.finish()));
}

/// System to initialize particle indices
pub fn init_particle_indices(
    mut particle_systems: Query<(&ParticleSystem, &ParticleBufferManager), Added<ParticleSystem>>,
    render_queue: Res<RenderQueue>,
) {
    for (particle_system, buffer_manager) in particle_systems.iter_mut() {
        let indices: Vec<u32> = (0..particle_system.params.max_particles).collect();
        render_queue.write_buffer(
            buffer_manager.index_buffer(),
            0,
            bytemuck::cast_slice(&indices),
        );
    }
} 