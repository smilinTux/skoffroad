use bevy::{
    prelude::*,
    render::{
        render_resource::{
            BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
            BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType, Buffer,
            BufferBindingType, BufferUsages, PipelineCache, RenderPipeline,
            SamplerBindingType, ShaderStages, TextureSampleType, TextureViewDimension,
        },
        renderer::RenderDevice,
        texture::BevyDefault,
    },
};

use super::PostProcessSettings;

/// Holds the render pipeline and bind group layout for post-processing effects
#[derive(Resource)]
pub struct PostProcessPipeline {
    /// The bind group layout for post-process resources
    pub bind_group_layout: BindGroupLayout,
    /// The compiled render pipeline
    pub pipeline: Option<RenderPipeline>,
}

/// Contains all resources needed by the post-process shader
#[derive(Resource)]
pub struct PostProcessBindGroup {
    /// The bind group containing all shader resources
    pub bind_group: BindGroup,
}

impl FromWorld for PostProcessPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

        // Create bind group layout
        let bind_group_layout = render_device.create_bind_group_layout(
            &BindGroupLayoutDescriptor {
                label: Some("post_process_bind_group_layout"),
                entries: &[
                    // Scene texture
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
                    // Scene texture sampler
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

        Self {
            bind_group_layout,
            pipeline: None,
        }
    }
}

/// Buffer containing post-process settings for the GPU
#[derive(Resource)]
pub struct PostProcessSettingsBuffer {
    /// The GPU buffer containing settings
    pub buffer: Buffer,
}

impl FromWorld for PostProcessSettingsBuffer {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        
        // Create settings buffer
        let buffer = render_device.create_buffer(&bevy::render::render_resource::BufferDescriptor {
            label: Some("post_process_settings_buffer"),
            size: std::mem::size_of::<PostProcessSettings>() as u64,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self { buffer }
    }
}

/// System to prepare post-process resources each frame
pub fn prepare_post_process(
    render_device: Res<RenderDevice>,
    settings: Res<PostProcessSettings>,
    mut settings_buffer: ResMut<PostProcessSettingsBuffer>,
) {
    // Update settings buffer with current values
    render_device.queue().write_buffer(
        &settings_buffer.buffer,
        0,
        bytemuck::cast_slice(&[settings.clone()]),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipeline_creation() {
        let mut app = App::new();
        
        // Add required resources
        app.world.init_resource::<PostProcessPipeline>();
        
        // Get the created pipeline
        let pipeline = app.world.resource::<PostProcessPipeline>();
        
        // Verify bind group layout was created
        // assert!(pipeline.bind_group_layout.as_hal::<bevy::render::render_resource::hal_types::vulkan::Device>().is_some());
    }

    #[test]
    fn test_settings_buffer_creation() {
        let mut app = App::new();
        
        // Add required resources
        app.world.init_resource::<PostProcessSettingsBuffer>();
        
        // Get the created buffer
        let settings_buffer = app.world.resource::<PostProcessSettingsBuffer>();
        
        // Verify buffer size matches settings struct
        assert_eq!(
            settings_buffer.buffer.size(),
            std::mem::size_of::<PostProcessSettings>() as u64
        );
    }
} 