use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// Quality settings for ray tracing features
#[derive(Resource, Clone, Debug, Serialize, Deserialize)]
pub struct RayTracingSettings {
    /// Maximum number of ray bounces for reflections/refractions
    pub max_bounces: u32,
    /// Number of samples per pixel for anti-aliasing
    pub samples_per_pixel: u32,
    /// Maximum ray distance
    pub max_ray_distance: f32,
    /// Enable ambient occlusion
    pub ambient_occlusion: bool,
    /// Number of AO samples
    pub ao_samples: u32,
    /// AO radius
    pub ao_radius: f32,
    /// Enable caustics
    pub caustics: bool,
    /// Number of caustic photons
    pub caustic_photons: u32,
    /// Enable volumetric lighting
    pub volumetric_lighting: bool,
    /// Resolution of volumetric lighting grid
    pub volumetric_resolution: Vec3,
    /// Quality preset
    pub quality_preset: RayTracingQuality,
    /// Enable subsurface scattering
    pub subsurface_scattering: bool,
    /// Number of subsurface samples
    pub subsurface_samples: u32,
    /// Subsurface radius
    pub subsurface_radius: f32,
    /// Enable dispersion
    pub dispersion: bool,
    /// Number of spectral samples for dispersion
    pub dispersion_samples: u32,
    /// Index of refraction range for dispersion
    pub dispersion_range: Vec2,
    /// Enable dynamic quality adjustment
    pub dynamic_quality: bool,
    /// Target frame time in milliseconds
    pub target_frame_time: f32,
    /// Frame time tolerance before quality adjustment
    pub frame_time_tolerance: f32,
    /// Minimum acceptable quality preset
    pub min_quality_preset: RayTracingQuality,
    /// Quality adjustment cooldown in frames
    pub quality_adjust_cooldown: u32,
}

/// Predefined quality presets
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum RayTracingQuality {
    Low,
    Medium,
    High,
    Ultra,
    Custom,
}

impl Default for RayTracingSettings {
    fn default() -> Self {
        Self::from_preset(RayTracingQuality::Medium)
    }
}

impl RayTracingSettings {
    /// Create settings from a quality preset
    pub fn from_preset(quality: RayTracingQuality) -> Self {
        match quality {
            RayTracingQuality::Low => Self {
                max_bounces: 2,
                samples_per_pixel: 1,
                max_ray_distance: 100.0,
                ambient_occlusion: true,
                ao_samples: 4,
                ao_radius: 0.5,
                caustics: false,
                caustic_photons: 0,
                volumetric_lighting: false,
                volumetric_resolution: Vec3::new(64.0, 32.0, 64.0),
                quality_preset: RayTracingQuality::Low,
                subsurface_scattering: false,
                subsurface_samples: 0,
                subsurface_radius: 0.0,
                dispersion: false,
                dispersion_samples: 0,
                dispersion_range: Vec2::new(1.0, 1.0),
                dynamic_quality: true,
                target_frame_time: 16.6, // 60 FPS
                frame_time_tolerance: 2.0,
                min_quality_preset: RayTracingQuality::Low,
                quality_adjust_cooldown: 60,
            },
            RayTracingQuality::Medium => Self {
                max_bounces: 4,
                samples_per_pixel: 2,
                max_ray_distance: 500.0,
                ambient_occlusion: true,
                ao_samples: 8,
                ao_radius: 1.0,
                caustics: true,
                caustic_photons: 10000,
                volumetric_lighting: true,
                volumetric_resolution: Vec3::new(128.0, 64.0, 128.0),
                quality_preset: RayTracingQuality::Medium,
                subsurface_scattering: true,
                subsurface_samples: 8,
                subsurface_radius: 1.0,
                dispersion: true,
                dispersion_samples: 4,
                dispersion_range: Vec2::new(1.45, 1.75),
                dynamic_quality: true,
                target_frame_time: 16.6,
                frame_time_tolerance: 2.0,
                min_quality_preset: RayTracingQuality::Low,
                quality_adjust_cooldown: 60,
            },
            RayTracingQuality::High => Self {
                max_bounces: 8,
                samples_per_pixel: 4,
                max_ray_distance: 1000.0,
                ambient_occlusion: true,
                ao_samples: 16,
                ao_radius: 2.0,
                caustics: true,
                caustic_photons: 50000,
                volumetric_lighting: true,
                volumetric_resolution: Vec3::new(256.0, 128.0, 256.0),
                quality_preset: RayTracingQuality::High,
                subsurface_scattering: true,
                subsurface_samples: 16,
                subsurface_radius: 2.0,
                dispersion: true,
                dispersion_samples: 8,
                dispersion_range: Vec2::new(1.45, 1.75),
                dynamic_quality: true,
                target_frame_time: 16.6,
                frame_time_tolerance: 2.0,
                min_quality_preset: RayTracingQuality::Medium,
                quality_adjust_cooldown: 60,
            },
            RayTracingQuality::Ultra => Self {
                max_bounces: 16,
                samples_per_pixel: 8,
                max_ray_distance: 2000.0,
                ambient_occlusion: true,
                ao_samples: 32,
                ao_radius: 3.0,
                caustics: true,
                caustic_photons: 100000,
                volumetric_lighting: true,
                volumetric_resolution: Vec3::new(512.0, 256.0, 512.0),
                quality_preset: RayTracingQuality::Ultra,
                subsurface_scattering: true,
                subsurface_samples: 32,
                subsurface_radius: 3.0,
                dispersion: true,
                dispersion_samples: 16,
                dispersion_range: Vec2::new(1.45, 1.75),
                dynamic_quality: true,
                target_frame_time: 16.6,
                frame_time_tolerance: 2.0,
                min_quality_preset: RayTracingQuality::High,
                quality_adjust_cooldown: 60,
            },
            RayTracingQuality::Custom => Self {
                max_bounces: 4,
                samples_per_pixel: 2,
                max_ray_distance: 500.0,
                ambient_occlusion: true,
                ao_samples: 8,
                ao_radius: 1.0,
                caustics: true,
                caustic_photons: 10000,
                volumetric_lighting: true,
                volumetric_resolution: Vec3::new(128.0, 64.0, 128.0),
                quality_preset: RayTracingQuality::Custom,
                subsurface_scattering: true,
                subsurface_samples: 16,
                subsurface_radius: 2.0,
                dispersion: true,
                dispersion_samples: 8,
                dispersion_range: Vec2::new(1.45, 1.75),
                dynamic_quality: true,
                target_frame_time: 16.6,
                frame_time_tolerance: 2.0,
                min_quality_preset: RayTracingQuality::Low,
                quality_adjust_cooldown: 60,
            },
        }
    }

    /// Apply settings to the ray tracing pipeline
    pub fn apply_to_pipeline(&self, pipeline: &mut RayTracingPipeline) {
        pipeline.set_max_bounces(self.max_bounces);
        pipeline.set_samples_per_pixel(self.samples_per_pixel);
        pipeline.set_max_ray_distance(self.max_ray_distance);
        
        if self.ambient_occlusion {
            pipeline.enable_ambient_occlusion(self.ao_samples, self.ao_radius);
        } else {
            pipeline.disable_ambient_occlusion();
        }

        if self.caustics {
            pipeline.enable_caustics(self.caustic_photons);
        } else {
            pipeline.disable_caustics();
        }

        if self.volumetric_lighting {
            pipeline.enable_volumetric_lighting(self.volumetric_resolution);
        } else {
            pipeline.disable_volumetric_lighting();
        }
    }
} 