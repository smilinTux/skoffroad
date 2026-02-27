use bevy::{
    prelude::*,
    render::{
        render_resource::{Buffer, BufferDescriptor, BufferUsages},
        renderer::RenderDevice,
        RenderApp,
    },
};
use std::collections::HashMap;
use bytemuck::{Pod, Zeroable};
use serde::{Serialize, Deserialize};

/// Tone mapping algorithms available for HDR to LDR conversion
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ToneMapping {
    None,
    ACES,
    Reinhard,
    Uncharted2,
}

impl Default for ToneMapping {
    fn default() -> Self {
        ToneMapping::ACES
    }
}

/// Settings for controlling post-processing effects.
/// These settings can be modified at runtime to adjust the visual appearance.
#[repr(C)]
#[derive(Resource, Component, Clone, Copy, Debug, Serialize, Deserialize, Pod, Zeroable)]
pub struct PostProcessSettings {
    /// Overall exposure adjustment (default: 1.0)
    pub exposure: f32,
    /// Gamma correction value (default: 2.2)
    pub gamma: f32,
    /// Contrast adjustment (default: 1.0)
    pub contrast: f32,
    /// Saturation adjustment (default: 1.0)
    pub saturation: f32,
    /// Brightness adjustment (default: 1.0)
    pub brightness: f32,
    /// Bloom intensity (default: 0.5)
    pub bloom_intensity: f32,
    /// Bloom threshold for bright areas (default: 1.0)
    pub bloom_threshold: f32,
    /// Chromatic aberration strength (default: 0.0)
    pub chromatic_aberration: f32,
    /// Vignette strength (default: 0.0)
    pub vignette_strength: f32,
    /// Vignette radius (default: 0.5)
    pub vignette_radius: f32,
    /// Tone mapping type (0: None, 1: ACES, 2: Reinhard, 3: Uncharted2)
    pub tone_mapping: u32,
}

impl Default for PostProcessSettings {
    fn default() -> Self {
        Self {
            exposure: 1.0,
            gamma: 2.2,
            contrast: 1.0,
            saturation: 1.0,
            brightness: 1.0,
            bloom_intensity: 0.5,
            bloom_threshold: 1.0,
            chromatic_aberration: 0.0,
            vignette_strength: 0.0,
            vignette_radius: 0.5,
            tone_mapping: 1, // ACES by default
        }
    }
}

/// Raw settings data that can be sent to the GPU.
/// This struct must match the layout expected by the shader.
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct PostProcessSettingsRaw {
    pub exposure: f32,
    pub gamma: f32,
    pub contrast: f32,
    pub saturation: f32,
    pub brightness: f32,
    pub bloom_intensity: f32,
    pub bloom_threshold: f32,
    pub chromatic_aberration: f32,
    pub vignette_strength: f32,
    pub vignette_radius: f32,
    pub tone_mapping: u32,
    // Padding to ensure 16-byte alignment
    _padding: [u32; 1],
}

impl From<&PostProcessSettings> for PostProcessSettingsRaw {
    fn from(settings: &PostProcessSettings) -> Self {
        Self {
            exposure: settings.exposure,
            gamma: settings.gamma,
            contrast: settings.contrast,
            saturation: settings.saturation,
            brightness: settings.brightness,
            bloom_intensity: settings.bloom_intensity,
            bloom_threshold: settings.bloom_threshold,
            chromatic_aberration: settings.chromatic_aberration,
            vignette_strength: settings.vignette_strength,
            vignette_radius: settings.vignette_radius,
            tone_mapping: settings.tone_mapping,
            _padding: [0],
        }
    }
}

/// GPU buffer containing post-processing settings
#[derive(Resource)]
pub struct PostProcessSettingsBuffer {
    pub buffer: Buffer,
}

impl FromWorld for PostProcessSettingsBuffer {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        
        // Create uniform buffer for settings
        let buffer = render_device.create_buffer(&BufferDescriptor {
            label: Some("post_process_settings_buffer"),
            size: std::mem::size_of::<PostProcessSettings>() as u64,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self { buffer }
    }
}

impl PostProcessSettings {
    /// Creates settings optimized for HDR rendering
    pub fn hdr() -> Self {
        Self {
            tone_mapping: ToneMapping::ACES as u32,
            exposure: 1.2,
            gamma: 2.2,
            bloom_intensity: 0.7,
            bloom_threshold: 1.2,
            saturation: 1.1,
            contrast: 1.1,
            brightness: 1.0,
            ..Default::default()
        }
    }

    /// Creates settings for a cinematic look
    pub fn cinematic() -> Self {
        Self {
            tone_mapping: ToneMapping::Uncharted2 as u32,
            exposure: 1.1,
            gamma: 2.4,
            bloom_intensity: 0.6,
            bloom_threshold: 0.9,
            saturation: 0.9,
            contrast: 1.2,
            brightness: 0.95,
            vignette_strength: 0.3,
            vignette_radius: 0.8,
            ..Default::default()
        }
    }

    /// Creates settings for a retro/stylized look
    pub fn retro() -> Self {
        Self {
            tone_mapping: ToneMapping::Reinhard as u32,
            exposure: 1.1,
            gamma: 2.0,
            bloom_intensity: 0.4,
            bloom_threshold: 0.8,
            saturation: 1.2,
            contrast: 1.3,
            brightness: 1.05,
            chromatic_aberration: 0.02,
            vignette_strength: 0.4,
            vignette_radius: 0.7,
            ..Default::default()
        }
    }
}

#[derive(Resource, Default)]
pub struct PerformanceStats {
    /// Average frame time in milliseconds
    pub avg_frame_time: f32,
    /// Base frame time without effects in milliseconds
    pub base_frame_time: f32,
    /// Current frame time in milliseconds
    pub current_frame_time: f32,
    /// Map of effect names to their overhead in milliseconds
    pub effect_overhead: HashMap<String, f32>,
}

impl PerformanceStats {
    /// Updates the overhead for a specific effect
    pub fn update_effect_overhead(&mut self, effect: &str, overhead_ms: f32) {
        self.effect_overhead.insert(effect.to_string(), overhead_ms);
    }

    /// Gets the total overhead from all effects
    pub fn total_overhead(&self) -> f32 {
        self.effect_overhead.values().sum()
    }

    /// Updates frame time metrics
    pub fn update_frame_times(&mut self, current: f32) {
        const SMOOTHING: f32 = 0.95; // Exponential moving average factor
        self.current_frame_time = current;
        self.avg_frame_time = self.avg_frame_time * SMOOTHING + current * (1.0 - SMOOTHING);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_settings() {
        let settings = PostProcessSettings::default();
        assert_eq!(settings.exposure, 1.0);
        assert_eq!(settings.tone_mapping, 1);
        assert_eq!(settings.bloom_intensity, 0.5);
    }

    #[test]
    fn test_settings_conversion() {
        let settings = PostProcessSettings {
            exposure: 1.5,
            gamma: 2.4,
            contrast: 1.2,
            saturation: 1.1,
            brightness: 0.9,
            bloom_intensity: 0.6,
            bloom_threshold: 1.2,
            chromatic_aberration: 0.1,
            vignette_strength: 0.3,
            vignette_radius: 0.6,
            tone_mapping: 2,
        };
        
        let raw: PostProcessSettingsRaw = (&settings).into();
        assert_eq!(raw.exposure, 1.5);
        assert_eq!(raw.tone_mapping, 2);
        assert_eq!(raw.vignette_strength, 0.3);
    }

    #[test]
    fn test_settings_buffer_creation() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);

        let render_app = app.sub_app_mut(RenderApp);
        render_app.init_resource::<PostProcessSettingsBuffer>();

        let buffer = render_app.world.resource::<PostProcessSettingsBuffer>();
        // assert!(buffer.buffer.as_hal::<bevy::render::render_resource::hal_types::vulkan::Buffer>().is_some());
    }
} 