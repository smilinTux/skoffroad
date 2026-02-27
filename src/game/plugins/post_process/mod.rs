/// Post-processing module for SandK Offroad
/// 
/// This module implements a flexible post-processing pipeline that handles various visual effects
/// including tone mapping, bloom, ambient occlusion, depth of field, and color grading.
/// 
/// The post-processing system uses a chain of effects that can be enabled/disabled and reordered
/// at runtime. Each effect is implemented as a compute shader for optimal performance.
/// 
/// # Features
/// - HDR tone mapping (ACES, Reinhard, etc.)
/// - Bloom with configurable threshold and intensity
/// - Screen Space Ambient Occlusion (SSAO)
/// - Depth of Field with bokeh simulation
/// - Color grading with LUT support
/// - Temporal Anti-Aliasing (TAA)
/// - Motion blur using velocity vectors
/// 
/// # Example
/// ```rust
/// use bevy::prelude::*;
/// use sandk_offroad::post_process::{PostProcessPlugin, PostProcessSettings};
/// 
/// fn main() {
///     App::new()
///         .add_plugins(DefaultPlugins)
///         .add_plugins(PostProcessPlugin)
///         .insert_resource(PostProcessSettings {
///             tone_mapping: ToneMappingType::ACES,
///             bloom_enabled: true,
///             bloom_threshold: 1.0,
///             bloom_intensity: 0.5,
///             ssao_enabled: true,
///             ssao_radius: 0.5,
///             ssao_bias: 0.025,
///             dof_enabled: true,
///             dof_focal_length: 50.0,
///             dof_aperture: 2.8,
///             // ... other settings
///         })
///         .run();
/// }
/// ```

use bevy::{
    prelude::*,
    render::{
        renderer::RenderDevice,
        RenderApp,
        render_graph::RenderGraph,
    },
};
use bytemuck::{Pod, Zeroable};

mod effects;
mod pipeline;
mod settings;
mod ui;
mod node;
mod test_scene;

// pub use effects::*;
// pub use settings::*;
// pub use ui::PerformanceDisplayPlugin;
use node::PostProcessNode;
use crate::game::plugins::post_process::pipeline::PostProcessPipeline;

/// Post-processing settings that control various visual effects in the rendering pipeline.
/// These settings can be modified in real-time to adjust the visual appearance of the game.
#[derive(Resource, Clone, Debug, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct PostProcessSettings {
    /// The type of tone mapping to apply. Options include:
    /// - "Reinhard": Classic tone mapping, good for HDR scenes
    /// - "ACES": Industry standard cinematic tone mapping
    /// - "AgX": Modern filmic tone mapping with natural response
    pub tone_mapping: u32, // Use a u32 to represent the enum discriminant

    /// Global exposure adjustment. Higher values make the scene brighter.
    /// Range: [0.0, 10.0]
    pub exposure: f32,

    /// Gamma correction value. Standard is 2.2.
    /// Range: [1.0, 3.0]
    pub gamma: f32,

    /// Intensity of the bloom effect. Controls how bright areas bleed into surroundings.
    /// Range: [0.0, 2.0]
    pub bloom_intensity: f32,

    /// Threshold for bloom effect. Pixels brighter than this will contribute to bloom.
    /// Range: [0.0, 10.0]
    pub bloom_threshold: f32,

    /// Color temperature adjustment in Kelvin.
    /// Range: [1000.0 (warm/orange), 15000.0 (cool/blue)]
    pub color_temperature: f32,

    /// Saturation adjustment. 1.0 is normal, 0.0 is grayscale, 2.0 is oversaturated.
    /// Range: [0.0, 2.0]
    pub saturation: f32,

    /// Contrast adjustment. 1.0 is normal.
    /// Range: [0.0, 2.0]
    pub contrast: f32,

    /// Vignette intensity. 0.0 is off, 1.0 is maximum darkening at corners.
    /// Range: [0.0, 1.0]
    pub vignette: f32,

    /// Chromatic aberration strength. 0.0 is off.
    /// Range: [0.0, 1.0]
    pub chromatic_aberration: f32,
}

impl Default for PostProcessSettings {
    fn default() -> Self {
        Self {
            tone_mapping: 0, // Default to ACES
            exposure: 1.0,
            gamma: 2.2,
            bloom_intensity: 0.5,
            bloom_threshold: 1.0,
            color_temperature: 6500.0,
            saturation: 1.0,
            contrast: 1.0,
            vignette: 0.2,
            chromatic_aberration: 0.0,
        }
    }
}

impl PostProcessSettings {
    /// Creates a cinematic preset with strong bloom and contrast
    pub fn cinematic() -> Self {
        Self {
            tone_mapping: 0, // Default to ACES
            exposure: 1.1,
            gamma: 2.2,
            bloom_intensity: 0.8,
            bloom_threshold: 0.9,
            color_temperature: 6000.0,
            saturation: 1.1,
            contrast: 1.2,
            vignette: 0.3,
            chromatic_aberration: 0.1,
        }
    }

    /// Creates a bright, vibrant preset
    pub fn vibrant() -> Self {
        Self {
            tone_mapping: 2, // Default to AgX
            exposure: 1.2,
            gamma: 2.2,
            bloom_intensity: 0.6,
            bloom_threshold: 0.8,
            color_temperature: 7000.0,
            saturation: 1.3,
            contrast: 1.1,
            vignette: 0.1,
            chromatic_aberration: 0.0,
        }
    }

    /// Creates a moody, desaturated preset
    pub fn moody() -> Self {
        Self {
            tone_mapping: 1, // Default to Reinhard
            exposure: 0.9,
            gamma: 2.3,
            bloom_intensity: 0.4,
            bloom_threshold: 1.2,
            color_temperature: 5500.0,
            saturation: 0.8,
            contrast: 1.3,
            vignette: 0.4,
            chromatic_aberration: 0.05,
        }
    }
}

/// Plugin that sets up post-processing effects for the game.
/// This includes effects like:
/// - Tone mapping (ACES, Reinhard, Uncharted2)
/// - Exposure adjustment
/// - Gamma correction
/// - Bloom
/// - Chromatic aberration
/// - Vignette
/// - Color grading (saturation, contrast, brightness)
pub struct PostProcessPlugin;

impl Plugin for PostProcessPlugin {
    fn build(&self, app: &mut App) {
        // Add settings resource
        app.init_resource::<PostProcessSettings>();

        // Add systems to the render app
        let render_app = app.sub_app_mut(RenderApp);
        render_app
            .init_resource::<PostProcessPipeline>()
            .add_systems(Startup, setup_post_process_node);
    }
}

fn setup_post_process_node(
    mut render_graph: ResMut<RenderGraph>,
    device: Res<RenderDevice>,
) {
    // Create and add the post-process node
    let node = PostProcessNode::new(&device);
    render_graph.add_node("post_process", node);

    // Add edge from camera node to post-process node
    render_graph.add_node_edge("camera", "post_process");
}

/// Enum defining available tone mapping operators
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ToneMappingType {
    /// ACES filmic tone mapping
    ACES,
    /// Reinhard tone mapping
    Reinhard,
    /// Uncharted 2 tone mapping
    Uncharted2,
    /// Simple exposure-based tone mapping
    Exposure,
}

/// System that updates post-processing settings
fn update_post_process_settings(settings: Res<PostProcessSettings>) {
    // This will be implemented to handle runtime changes to settings
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::render::renderer::RenderDevice;

    #[test]
    fn test_plugin_setup() {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            bevy::render::RenderPlugin::default(),
            PostProcessPlugin,
        ));

        // Verify settings resource was added
        assert!(app.world.contains_resource::<PostProcessSettings>());

        // Verify render app setup
        let render_app = app.sub_app(RenderApp);
        assert!(render_app.world.contains_resource::<PostProcessPipeline>());
    }
} 