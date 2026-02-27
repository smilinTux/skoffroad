use bevy::{
    prelude::*,
    render::{
        render_graph::{Node, NodeRunError, RenderGraphContext, InputSlotError},
        renderer::{RenderContext, RenderDevice},
    },
    render::render_resource::BufferVec,
    render::render_resource::BufferUsages,
    render::renderer::RenderQueue,
    render::color::Color,
};

use super::pipeline::PostProcessPipeline;
use super::PostProcessSettings;

/// Node that handles post-processing effects in the render graph.
/// This node applies effects like tone mapping, bloom, and color adjustments
/// to the final rendered image.
pub struct PostProcessNode {
    settings_buffer: BufferVec<PostProcessSettings>,
}

impl PostProcessNode {
    pub fn new(_device: &RenderDevice) -> Self {
        Self {
            settings_buffer: BufferVec::new(BufferUsages::UNIFORM | BufferUsages::COPY_DST),
        }
    }
}

impl Node for PostProcessNode {
    fn update(&mut self, world: &mut World) {
        let settings = world.resource::<PostProcessSettings>();
        self.settings_buffer.clear();
        self.settings_buffer.push(settings.clone());
        self.settings_buffer.write_buffer(
            world.resource::<RenderDevice>(),
            world.resource::<RenderQueue>(),
        );
    }

    fn run(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let pipeline = world.resource::<PostProcessPipeline>();
        let view_target = match graph.get_input("view_target")? {
            bevy::render::render_graph::SlotValue::TextureView(view) => view,
            _ => return Err(NodeRunError::InputSlotError(InputSlotError::InvalidSlot("invalid input slot: view_target".into()))),
        };

        let sampler = render_context.render_device().create_sampler(&bevy::render::render_resource::SamplerDescriptor::default());
        let bind_group = pipeline.create_bind_group(render_context.render_device(), view_target, &sampler);

        // Begin the post-process render pass
        let mut render_pass = render_context.begin_tracked_render_pass(
            bevy::render::render_resource::RenderPassDescriptor {
                label: Some("post_process_pass"),
                color_attachments: &[Some(
                    bevy::render::render_resource::RenderPassColorAttachment {
                        view: view_target,
                        resolve_target: None,
                        ops: bevy::render::render_resource::Operations {
                            load: bevy::render::render_resource::LoadOp::Clear(Color::BLACK.into()),
                            store: true,
                        },
                    },
                )],
                depth_stencil_attachment: None,
            },
        );

        // Set pipeline and bind group
        if let Some(ref render_pipeline) = pipeline.pipeline() {
            render_pass.set_render_pipeline(render_pipeline);
        } else {
            return Err(NodeRunError::InputSlotError(InputSlotError::InvalidSlot("missing post-process pipeline".into())));
        }
        render_pass.set_bind_group(0, &bind_group, &[]);

        // Draw fullscreen quad
        render_pass.draw(0..3, 0..1);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // Skipping RenderDevice creation as wgpu_create_test_device is not available in Bevy 0.12+
    // This test can be expanded with a proper RenderDevice mock or integration test if needed.
} 