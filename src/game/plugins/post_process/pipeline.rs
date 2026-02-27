use bevy::{
    prelude::*,
    render::{
        render_resource::{
            BindGroup, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry,
            BindGroupEntry, BufferBindingType, Buffer, BufferDescriptor, BufferUsages,
            BindingResource, RenderPipeline, ShaderStages, BindingType, TextureSampleType, TextureViewDimension, SamplerBindingType,
        },
        renderer::{RenderDevice, RenderQueue},
    },
};

use super::settings::PostProcessSettings;
// use crate::game::plugins::post_process::bind_group::PostProcessBindGroup; // TODO: Fix or implement bind_group module

/// Plugin that sets up the post-processing render pipeline
pub struct PostProcessPipelinePlugin;

impl Plugin for PostProcessPipelinePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PostProcessPipeline>();
    }
}

/// Pipeline for post-processing effects including tone mapping, bloom, and other visual enhancements.
/// 
/// This pipeline manages the render state and resources needed for post-processing effects:
/// - Screen texture and sampler for the main render target
/// - Uniform buffer for post-process settings
/// - Render pipeline for applying effects
/// 
/// # Example Usage
/// ```rust
/// use bevy::prelude::*;
/// use crate::game::plugins::post_process::{PostProcessPipeline, PostProcessSettings};
/// 
/// fn setup(mut commands: Commands) {
///     commands.insert_resource(PostProcessSettings {
///         exposure: 1.0,
///         bloom_intensity: 0.5,
///         chromatic_aberration: 0.02,
///         vignette_strength: 0.3,
///         ..Default::default()
///     });
/// }
/// ```
#[derive(Resource)]
pub struct PostProcessPipeline {
    bind_group_layout: BindGroupLayout,
    settings_buffer: Buffer,
    pipeline: Option<RenderPipeline>,
}

/// Contains all resources needed by the post-process shader
#[derive(Resource)]
pub struct PostProcessBindGroup {
    bind_group: BindGroup,
}

/// Buffer containing post-process settings
#[derive(Resource)]
pub struct PostProcessSettingsBuffer {
    buffer: Buffer,
}

impl FromWorld for PostProcessPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        
        // Create bind group layout
        let bind_group_layout = render_device.create_bind_group_layout(
            &BindGroupLayoutDescriptor {
                label: Some("post_process_bind_group_layout"),
                entries: &[
                    // Screen texture
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Float { filterable: true },
                            view_dimension: TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    // Screen sampler
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Sampler(SamplerBindingType::Filtering),
                        count: None,
                    },
                    // Settings uniform buffer
                    BindGroupLayoutEntry {
                        binding: 2,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            }
        );

        // Create settings buffer
        let settings_buffer = render_device.create_buffer(&BufferDescriptor {
            label: Some("post_process_settings_buffer"),
            size: std::mem::size_of::<PostProcessSettings>() as u64,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            bind_group_layout,
            settings_buffer,
            pipeline: None,
        }
    }
}

impl PostProcessPipeline {
    /// Updates the settings buffer with new post-processing parameters
    pub fn update_settings(&self, render_queue: &bevy::render::renderer::RenderQueue, settings: &PostProcessSettings) {
        render_queue.write_buffer(
            &self.settings_buffer,
            0,
            bytemuck::cast_slice(&[*settings]),
        );
    }

    /// Creates a bind group for the post-processing pipeline
    pub fn create_bind_group(
        &self,
        render_device: &RenderDevice,
        texture_view: &bevy::render::render_resource::TextureView,
        sampler: &bevy::render::render_resource::Sampler,
    ) -> BindGroup {
        render_device.create_bind_group(
            Some("post_process_bind_group"),
            &self.bind_group_layout,
            &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(texture_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(sampler),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: self.settings_buffer.as_entire_binding(),
                },
            ],
        )
    }

    /// Public getter for bind_group_layout
    pub fn bind_group_layout(&self) -> &BindGroupLayout {
        &self.bind_group_layout
    }

    /// Public getter for pipeline
    pub fn pipeline(&self) -> Option<&RenderPipeline> {
        self.pipeline.as_ref()
    }
}

/// Updates the post-process settings buffer with current settings
pub fn prepare_post_process(
    settings: Res<PostProcessSettings>,
    settings_buffer: Res<PostProcessSettingsBuffer>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
) {
    render_queue.write_buffer(
        &settings_buffer.buffer,
        0,
        bytemuck::cast_slice(&[*settings]),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipeline_creation() {
        let mut world = World::new();
        world.init_resource::<RenderDevice>();
        
        let pipeline = PostProcessPipeline::from_world(&mut world);
        assert!(pipeline.pipeline.is_none());
        assert!(!pipeline.bind_group_layout.is_empty());
    }

    #[test]
    fn test_settings_update() {
        let mut world = World::new();
        world.init_resource::<RenderDevice>();
        
        let pipeline = PostProcessPipeline::from_world(&mut world);
        let render_device = world.resource::<RenderDevice>();
        
        let settings = PostProcessSettings::default();
        pipeline.update_settings(world.resource::<RenderQueue>(), &settings);
        // Buffer update successful if no panic occurs
    }
} 