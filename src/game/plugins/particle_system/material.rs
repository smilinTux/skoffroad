/// Particle Material System
/// 
/// This module provides a flexible material system for particle rendering with features including:
/// - Texture atlas support with animation
/// - Multiple blend modes (Alpha, Additive, Premultiplied)
/// - Color tinting and emission
/// - Soft particles with depth testing
/// - Automatic LOD management
/// - Custom shader parameters
/// 
/// # Examples
/// 
/// ```rust
/// // Basic particle material with texture
/// let material = ParticleMaterial::new(texture_handle)
///     .with_blend_mode(BlendMode::Alpha)
///     .with_alpha_threshold(0.01)
///     .with_emission(1.0);
/// 
/// // Animated particle material with color tint
/// let animated_material = ParticleMaterial::new(sprite_sheet_handle)
///     .with_atlas(UVec2::new(4, 4), 16)  // 4x4 grid, 16 frames
///     .with_animation(30.0, true)         // 30 FPS, random start frame
///     .with_color_tint(Vec4::new(1.0, 0.5, 0.2, 1.0))  // Orange tint
///     .with_blend_mode(BlendMode::Additive)
///     .with_emission(2.0);
/// 
/// // Soft particles with LOD
/// let soft_material = ParticleMaterial::new(texture_handle)
///     .with_soft_particles(true, 1.0)
///     .with_lod_settings(LodSettings {
///         fade_start: 30.0,
///         fade_end: 80.0,
///         min_size: 0.3,
///         auto_lod: true,
///     });
/// 
/// // Custom effects using shader parameters
/// let custom_material = ParticleMaterial::new(texture_handle)
///     .with_custom_params(Vec4::new(
///         1.0,  // Size scale
///         0.5,  // Distortion amount
///         0.0,  // Reserved
///         0.0,  // Reserved
///     ));
/// ```
/// 
/// # Blend Modes
/// 
/// - `BlendMode::Alpha`: Standard alpha blending, best for most particle effects
/// - `BlendMode::Additive`: Bright, glowing effects that add to the background
/// - `BlendMode::Premultiplied`: For pre-multiplied alpha textures
/// 
/// # LOD System
/// 
/// The Level of Detail (LOD) system automatically adjusts particle rendering based on camera distance:
/// - Particles smoothly fade out between `fade_start` and `fade_end` distances
/// - Size can be automatically reduced with distance (controlled by `min_size`)
/// - Can be disabled per material by setting `auto_lod` to false
/// 
/// # Custom Parameters
/// 
/// The `custom_params` Vec4 can be used for various effects:
/// - X: Size scale (used by LOD system)
/// - Y: Available for custom effects
/// - Z: Available for custom effects
/// - W: Available for custom effects
/// 
use bevy::{
    prelude::*,
    render::{
        render_resource::{
            BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
            BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource, BindingType,
            BlendComponent, BlendFactor, BlendOperation, BlendState, Buffer, BufferBindingType,
            ColorTargetState, ColorWrites, CompareFunction, DepthBiasState, DepthStencilState,
            FragmentState, FrontFace, MultisampleState, PipelineCache, PolygonMode,
            PrimitiveState, PrimitiveTopology, RenderPipelineDescriptor, SamplerBindingType,
            ShaderStages, ShaderType, TextureFormat, TextureSampleType, TextureViewDimension,
            VertexAttribute, VertexBufferLayout, VertexFormat, VertexState, VertexStepMode,
            BufferInitDescriptor,
        },
        renderer::RenderDevice,
        texture::BevyDefault,
        view::ViewUniform,
    },
};

use super::{
    particle::ParticleSystem,
};

use bevy::reflect::TypePath;

use std::collections::HashMap;
use std::time::{Duration, Instant};

use bevy::math::Vec3;
use bevy::render::primitives::Aabb;

use bevy::render::render_resource::{RenderPipeline, TextureView, Sampler};

use bevy::render::render_asset::RenderAssets;

/// Blend modes for particle rendering
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BlendMode {
    #[default]
    Alpha,
    Additive,
    Premultiplied,
}

/// Material parameters for particle rendering
#[derive(Clone, Copy, ShaderType)]
pub struct MaterialParams {
    /// View-projection matrix
    pub view_proj: Mat4,
    /// Camera position
    pub camera_position: Vec3,
    /// Camera right vector
    pub camera_right: Vec3,
    /// Camera up vector
    pub camera_up: Vec3,
    /// Atlas dimensions (columns, rows)
    pub atlas_size: Vec2,
    /// Total number of frames in the atlas
    pub frame_count: u32,
    /// Current time in the animation
    pub current_time: f32,
    /// Frames per second for UV animation
    pub animation_fps: f32,
    /// Alpha threshold for discard
    pub alpha_threshold: f32,
    /// Emission strength
    pub emission_strength: f32,
    /// Depth fade distance
    pub depth_fade_distance: f32,
    /// Global color tint
    pub color_tint: Vec4,
    /// Custom parameters for shader effects
    pub custom_params: Vec4,
    /// Start/end distances for LOD fading
    pub distance_fade_range: Vec2,
    /// Gradient color start
    pub gradient_color_start: Vec4,
    /// Gradient color end
    pub gradient_color_end: Vec4,
    /// Effect parameters
    pub effect_params: Vec4,
    /// Performance parameters
    pub performance_params: Vec4,
    /// Padding
    pub _padding: Vec2,
}

impl Default for MaterialParams {
    fn default() -> Self {
        Self {
            view_proj: Mat4::IDENTITY,
            camera_position: Vec3::ZERO,
            camera_right: Vec3::X,
            camera_up: Vec3::Y,
            atlas_size: Vec2::new(1.0, 1.0),
            frame_count: 1,
            current_time: 0.0,
            animation_fps: 0.0,
            alpha_threshold: 0.01,
            emission_strength: 1.0,
            depth_fade_distance: 1.0,
            color_tint: Vec4::ONE,
            custom_params: Vec4::ZERO,
            distance_fade_range: Vec2::new(50.0, 100.0),
            gradient_color_start: Vec4::ONE,
            gradient_color_end: Vec4::ONE,
            effect_params: Vec4::new(1.0, 1.0, 0.0, 0.0),
            performance_params: Vec4::new(1000.0, 1.0, 1.0, 0.0),
            _padding: Vec2::ZERO,
        }
    }
}

/// Component for particle material configuration
#[derive(Component, Clone, Default, TypePath)]
pub struct ParticleMaterial {
    /// Texture atlas handle
    pub texture: Handle<Image>,
    /// Atlas dimensions (columns, rows)
    pub atlas_size: UVec2,
    /// Total number of frames in the atlas
    pub frame_count: u32,
    /// Frames per second for UV animation
    pub animation_fps: f32,
    /// Start from random frame
    pub random_initial_frame: bool,
    /// Blend mode
    pub blend_mode: BlendMode,
    /// Alpha threshold for discard
    pub alpha_threshold: f32,
    /// Soft particles flag
    pub soft_particles: bool,
    /// Distance over which particles fade when intersecting geometry
    pub depth_fade_distance: f32,
    /// Emission strength
    pub emission_strength: f32,
    /// Global color tint
    pub color_tint: Vec4,
    /// Custom parameters for shader effects
    pub custom_params: Vec4,
    /// LOD settings
    pub lod_settings: LodSettings,
    /// Gradient color start
    pub gradient_color_start: Vec4,
    /// Gradient color end
    pub gradient_color_end: Vec4,
    /// Noise scale
    pub noise_scale: f32,
    /// Noise speed
    pub noise_speed: f32,
    /// Gradient blend
    pub gradient_blend: f32,
    /// Distortion strength
    pub distortion_strength: f32,
    /// Maximum visible particles
    pub max_visible_particles: f32,
    /// LOD bias
    pub lod_bias: f32,
    /// Quality level
    pub quality_level: f32,
    /// GPU buffer for material parameters
    #[cfg(not(test))]
    pub(crate) params_buffer: Option<Buffer>,
    /// Bind group for material
    pub bind_group: Option<BindGroup>,
}

impl bevy::asset::Asset for ParticleMaterial {}

/// Settings for Level of Detail (LOD) management
#[derive(Clone, Copy, Debug)]
pub struct LodSettings {
    /// Distance at which particles start fading out
    pub fade_start: f32,
    /// Distance at which particles are completely faded out
    pub fade_end: f32,
    /// Minimum particle size multiplier at max distance
    pub min_size: f32,
    /// Whether to enable automatic LOD based on distance
    pub auto_lod: bool,
}

impl Default for LodSettings {
    fn default() -> Self {
        Self {
            fade_start: 50.0,
            fade_end: 100.0,
            min_size: 0.2,
            auto_lod: true,
        }
    }
}

/// Performance settings for particle materials
#[derive(Clone, Copy, Debug)]
pub struct PerformanceSettings {
    /// Maximum number of particles to render at once
    pub max_particles: u32,
    /// Distance at which to start culling particles
    pub cull_distance: f32,
    /// Whether to use frustum culling
    pub use_frustum_culling: bool,
    /// Whether to use occlusion culling
    pub use_occlusion_culling: bool,
    /// Level of detail settings
    pub lod_settings: LodSettings,
}

impl Default for PerformanceSettings {
    fn default() -> Self {
        Self {
            max_particles: 10000,
            cull_distance: 100.0,
            use_frustum_culling: true,
            use_occlusion_culling: true,
            lod_settings: LodSettings::default(),
        }
    }
}

/// Performance metrics for particle materials
#[derive(Debug, Clone, Component)]
pub struct ParticlePerformanceMetrics {
    /// Number of active particles
    pub active_particles: u32,
    /// Number of culled particles
    pub culled_particles: u32,
    /// Number of particles in view frustum
    pub visible_particles: u32,
    /// Current LOD level (0.0 - 1.0)
    pub current_lod: f32,
    /// Average frame time for particle updates (ms)
    pub avg_frame_time: f32,
    /// GPU memory usage for particles (bytes)
    pub gpu_memory_usage: u64,
    /// Number of draw calls per frame
    pub draw_calls: u32,
    /// Number of batches
    pub batch_count: u32,
    /// Time spent in particle updates (ms)
    pub update_time: f32,
    /// Time spent in rendering (ms)
    pub render_time: f32,
}

impl Default for ParticlePerformanceMetrics {
    fn default() -> Self {
        Self {
            active_particles: 0,
            culled_particles: 0,
            visible_particles: 0,
            current_lod: 0.0,
            avg_frame_time: 0.0,
            gpu_memory_usage: 0,
            draw_calls: 0,
            batch_count: 0,
            update_time: 0.0,
            render_time: 0.0,
        }
    }
}

/// Settings for dynamic batch size adjustment
#[derive(Clone, Debug)]
pub struct DynamicBatchSettings {
    /// Target frame time in milliseconds
    pub target_frame_time: f32,
    /// Minimum batch size
    pub min_batch_size: u32,
    /// Maximum batch size
    pub max_batch_size: u32,
    /// How quickly to adjust batch size (0.0 - 1.0)
    pub adjustment_speed: f32,
    /// Frame window for performance averaging
    pub frame_window: u32,
}

impl Default for DynamicBatchSettings {
    fn default() -> Self {
        Self {
            target_frame_time: 16.0,  // Target 60 FPS
            min_batch_size: 100,
            max_batch_size: 10000,
            adjustment_speed: 0.2,
            frame_window: 60,
        }
    }
}

/// Advanced performance settings for fine-tuned control
#[derive(Clone, Debug)]
pub struct AdvancedPerformanceSettings {
    /// Base settings
    pub base: PerformanceSettings,
    /// Batch size for instanced rendering
    pub batch_size: u32,
    /// Maximum number of draw calls per frame
    pub max_draw_calls: u32,
    /// Target frame time in milliseconds
    pub target_frame_time: f32,
    /// Dynamic LOD adjustment speed (0.0 - 1.0)
    pub lod_adjustment_speed: f32,
    /// Minimum particle size before culling
    pub min_particle_size: f32,
    /// Enable dynamic batching
    pub dynamic_batching: bool,
    /// Enable GPU instancing
    pub gpu_instancing: bool,
    /// Quality levels for different effects
    pub quality_settings: QualitySettings,
    /// Memory budget in bytes
    pub memory_budget: u64,
    /// Enable performance metrics collection
    pub collect_metrics: bool,
    /// Settings for dynamic batch size adjustment
    pub dynamic_batch_settings: DynamicBatchSettings,
}

/// Quality settings for different particle effects
#[derive(Clone, Debug)]
pub struct QualitySettings {
    /// Quality level for soft particles (0.0 - 1.0)
    pub soft_particles: f32,
    /// Quality level for distortion effects (0.0 - 1.0)
    pub distortion: f32,
    /// Quality level for lighting (0.0 - 1.0)
    pub lighting: f32,
    /// Quality level for noise effects (0.0 - 1.0)
    pub noise: f32,
    /// Maximum particles per effect type
    pub max_particles_per_type: HashMap<ParticleEffectType, u32>,
}

impl Default for QualitySettings {
    fn default() -> Self {
        let mut max_particles = HashMap::new();
        max_particles.insert(ParticleEffectType::Fire, 1000);
        max_particles.insert(ParticleEffectType::Smoke, 500);
        max_particles.insert(ParticleEffectType::Magic, 2000);
        max_particles.insert(ParticleEffectType::Dust, 300);
        max_particles.insert(ParticleEffectType::Water, 800);

        Self {
            soft_particles: 1.0,
            distortion: 1.0,
            lighting: 1.0,
            noise: 1.0,
            max_particles_per_type: max_particles,
        }
    }
}

impl Default for AdvancedPerformanceSettings {
    fn default() -> Self {
        Self {
            base: PerformanceSettings::default(),
            batch_size: 1000,
            max_draw_calls: 100,
            target_frame_time: 16.0,
            lod_adjustment_speed: 0.5,
            min_particle_size: 0.01,
            dynamic_batching: true,
            gpu_instancing: true,
            quality_settings: QualitySettings::default(),
            memory_budget: 1024 * 1024 * 64, // 64MB
            collect_metrics: true,
            dynamic_batch_settings: DynamicBatchSettings::default(),
        }
    }
}

/// Different types of particle effects for performance tracking
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ParticleEffectType {
    Fire,
    Smoke,
    Magic,
    Dust,
    Water,
    Custom(u32),
}

/// Data for GPU instancing
#[derive(Clone, Copy, Debug, ShaderType)]
pub struct InstanceData {
    /// Instance transform matrix
    pub transform: Mat4,
    /// Instance color and alpha
    pub color: Vec4,
    /// Instance UV offset and scale
    pub uv_data: Vec4,
    /// Instance custom data (time, random seed, etc)
    pub custom_data: Vec4,
}

/// Batch information for particle rendering
#[derive(Component)]
pub struct ParticleBatch {
    /// Number of particles in this batch
    pub particle_count: u32,
    /// Instance buffer for GPU instancing
    pub instance_buffer: Option<Buffer>,
    /// Batch visibility
    pub visible: bool,
    /// Last update frame
    pub last_update: Instant,
}

/// Component for visualizing batch bounds and culling
#[derive(Component)]
pub struct DebugBatchVisualization {
    /// Whether to show batch bounds
    pub show_bounds: bool,
    /// Whether to show culled batches
    pub show_culled: bool,
    /// Color for visible batch bounds
    pub visible_color: Color,
    /// Color for culled batch bounds
    pub culled_color: Color,
    /// Whether to show batch statistics
    pub show_stats: bool,
}

impl Default for DebugBatchVisualization {
    fn default() -> Self {
        Self {
            show_bounds: true,
            show_culled: true,
            visible_color: Color::rgba(0.0, 1.0, 0.0, 0.3), // Green
            culled_color: Color::rgba(1.0, 0.0, 0.0, 0.2),  // Red
            show_stats: true,
        }
    }
}

/// Detailed batching metrics
#[derive(Debug, Clone, Default)]
pub struct BatchingMetrics {
    /// Number of active batches
    pub active_batch_count: u32,
    /// Average number of particles per batch
    pub avg_particles_per_batch: f32,
    /// Maximum particles in any batch
    pub max_batch_size: u32,
    /// Number of visible batches
    pub visible_batch_count: u32,
    /// Time spent preparing batches (ms)
    pub batch_preparation_time: f32,
    /// GPU memory used by instance buffers (bytes)
    pub instance_buffer_memory: u64,
    /// Batch updates per second
    pub batch_update_frequency: f32,
    /// Number of batches culled
    pub culled_batch_count: u32,
    /// Frame-to-frame batch stability (0-1)
    pub batch_stability: f32,
    /// Batch memory fragmentation (0-1)
    pub memory_fragmentation: f32,
}

/// Enhanced performance visualization options
#[derive(Component)]
pub struct PerformanceVisualization {
    /// Whether to show performance overlay
    pub show_overlay: bool,
    /// Whether to show batch metrics
    pub show_batch_metrics: bool,
    /// Whether to show memory metrics
    pub show_memory_metrics: bool,
    /// Whether to show particle metrics
    pub show_particle_metrics: bool,
    /// Whether to show performance graphs
    pub show_graphs: bool,
    /// Position of the overlay
    pub overlay_position: Vec2,
    /// Scale of the overlay
    pub overlay_scale: f32,
    /// Color scheme for the visualization
    pub color_scheme: PerformanceVisualizationColors,
    /// History length for graphs (frames)
    pub graph_history: usize,
}

/// Color scheme for performance visualization
#[derive(Clone)]
pub struct PerformanceVisualizationColors {
    /// Color for good performance indicators
    pub good: Color,
    /// Color for warning performance indicators
    pub warning: Color,
    /// Color for critical performance indicators
    pub critical: Color,
    /// Color for graph background
    pub background: Color,
    /// Color for graph grid
    pub grid: Color,
    /// Color for text
    pub text: Color,
}

impl Default for PerformanceVisualizationColors {
    fn default() -> Self {
        Self {
            good: Color::rgb(0.0, 0.8, 0.0),
            warning: Color::rgb(0.8, 0.8, 0.0),
            critical: Color::rgb(0.8, 0.0, 0.0),
            background: Color::rgba(0.0, 0.0, 0.0, 0.5),
            grid: Color::rgba(1.0, 1.0, 1.0, 0.2),
            text: Color::WHITE,
        }
    }
}

impl Default for PerformanceVisualization {
    fn default() -> Self {
        Self {
            show_overlay: true,
            show_batch_metrics: true,
            show_memory_metrics: true,
            show_particle_metrics: true,
            show_graphs: true,
            overlay_position: Vec2::new(10.0, 10.0),
            overlay_scale: 1.0,
            color_scheme: PerformanceVisualizationColors::default(),
            graph_history: 120, // 2 seconds at 60 FPS
        }
    }
}

/// Debug visualization features for particles
#[derive(Component)]
pub struct ParticleDebugFeatures {
    /// Whether to show wireframe rendering
    pub show_wireframe: bool,
    /// Whether to show particle trails
    pub show_trails: bool,
    /// Maximum trail length in seconds
    pub trail_length: f32,
    /// Trail point spacing in seconds
    pub trail_spacing: f32,
    /// Trail fade out (0-1)
    pub trail_fade: f32,
    /// Trail color
    pub trail_color: Color,
    /// Wireframe color
    pub wireframe_color: Color,
    /// Whether to show velocity vectors
    pub show_velocity: bool,
    /// Velocity vector scale
    pub velocity_scale: f32,
    /// Whether to show particle bounds
    pub show_bounds: bool,
    /// Trail history
    #[cfg(not(test))]
    trail_history: Vec<(Vec3, f32)>, // Position and timestamp
}

impl Default for ParticleDebugFeatures {
    fn default() -> Self {
        Self {
            show_wireframe: false,
            show_trails: false,
            trail_length: 1.0,
            trail_spacing: 0.016,
            trail_fade: 0.8,
            trail_color: Color::rgba(0.0, 1.0, 1.0, 0.5),
            wireframe_color: Color::rgba(1.0, 1.0, 1.0, 0.3),
            show_velocity: false,
            velocity_scale: 1.0,
            show_bounds: false,
            #[cfg(not(test))]
            trail_history: Vec::new(),
        }
    }
}

impl ParticleMaterial {
    pub fn new(texture: Handle<Image>) -> Self {
        Self {
            texture,
            atlas_size: UVec2::new(1, 1),
            frame_count: 1,
            animation_fps: 0.0,
            random_initial_frame: false,
            blend_mode: BlendMode::Alpha,
            alpha_threshold: 0.01,
            soft_particles: true,
            depth_fade_distance: 1.0,
            emission_strength: 1.0,
            color_tint: Vec4::ONE,
            custom_params: Vec4::ZERO,
            lod_settings: LodSettings::default(),
            gradient_color_start: Vec4::ONE,
            gradient_color_end: Vec4::ONE,
            noise_scale: 1.0,
            noise_speed: 1.0,
            gradient_blend: 0.0,
            distortion_strength: 0.0,
            max_visible_particles: 1000.0,
            lod_bias: 1.0,
            quality_level: 1.0,
            params_buffer: None,
            bind_group: None,
        }
    }

    /// Set the texture atlas dimensions
    pub fn with_atlas(mut self, size: UVec2, frames: u32) -> Self {
        self.atlas_size = size;
        self.frame_count = frames;
        self
    }

    /// Set the animation parameters
    pub fn with_animation(mut self, fps: f32, random_start: bool) -> Self {
        self.animation_fps = fps;
        self.random_initial_frame = random_start;
        self
    }

    /// Set the blend mode
    pub fn with_blend_mode(mut self, mode: BlendMode) -> Self {
        self.blend_mode = mode;
        self
    }

    /// Set the soft particles effect strength
    pub fn with_soft_particles(mut self, enabled: bool, fade_distance: f32) -> Self {
        self.soft_particles = enabled;
        self.depth_fade_distance = fade_distance;
        self
    }

    /// Set the emission strength
    pub fn with_emission(mut self, strength: f32) -> Self {
        self.emission_strength = strength;
        self
    }

    /// Set the distance fade range
    pub fn with_distance_fade(mut self, start: f32, end: f32) -> Self {
        self.distance_fade_range = Vec2::new(start, end);
        self
    }

    /// Set the alpha threshold
    pub fn with_alpha_threshold(mut self, threshold: f32) -> Self {
        self.alpha_threshold = threshold;
        self
    }

    /// Set the global color tint
    pub fn with_color_tint(mut self, color: Vec4) -> Self {
        self.color_tint = color;
        self
    }

    /// Set custom shader parameters
    pub fn with_custom_params(mut self, params: Vec4) -> Self {
        self.custom_params = params;
        self
    }

    /// Configure LOD settings
    pub fn with_lod_settings(mut self, settings: LodSettings) -> Self {
        self.lod_settings = settings;
        self
    }

    /// Set gradient colors for the particle effect
    pub fn with_gradient(mut self, start: Vec4, end: Vec4) -> Self {
        self.gradient_color_start = start;
        self.gradient_color_end = end;
        self
    }

    /// Set noise effect parameters
    pub fn with_noise(mut self, scale: f32, speed: f32) -> Self {
        self.noise_scale = scale;
        self.noise_speed = speed;
        self
    }

    /// Set gradient blend factor
    pub fn with_gradient_blend(mut self, blend: f32) -> Self {
        self.gradient_blend = blend;
        self
    }

    /// Set distortion strength
    pub fn with_distortion(mut self, strength: f32) -> Self {
        self.distortion_strength = strength;
        self
    }

    /// Set performance parameters
    pub fn with_performance_settings(mut self, settings: PerformanceSettings) -> Self {
        self.performance_settings = settings;
        self
    }

    /// Configure advanced performance settings
    pub fn with_advanced_performance(mut self, settings: AdvancedPerformanceSettings) -> Self {
        self.advanced_performance_settings = settings;
        self
    }

    /// Update performance metrics
    fn update_performance_metrics(&mut self, frame_stats: &FrameStats, metrics: &mut ParticlePerformanceMetrics) {
        // Update basic metrics
        metrics.active_particles = self.particle_count();
        metrics.culled_particles = self.culled_count;
        metrics.visible_particles = metrics.active_particles - metrics.culled_particles;
        
        // Calculate LOD metrics
        let camera_distance = self.camera_distance();
        metrics.current_lod = self.calculate_lod_factor(camera_distance);
        
        // Update timing metrics with exponential moving average
        let alpha = 0.1; // Smoothing factor
        metrics.avg_frame_time = (1.0 - alpha) * metrics.avg_frame_time + 
                               alpha * frame_stats.frame_time.as_secs_f32() * 1000.0;
        
        // Update GPU metrics
        metrics.gpu_memory_usage = self.calculate_gpu_memory_usage();
        metrics.draw_calls = self.draw_call_count;
        metrics.batch_count = (metrics.visible_particles + self.advanced_performance_settings.batch_size - 1) 
                            / self.advanced_performance_settings.batch_size;
        
        // Update timing breakdown
        metrics.update_time = frame_stats.update_time.as_secs_f32() * 1000.0;
        metrics.render_time = frame_stats.render_time.as_secs_f32() * 1000.0;
        
        // Adjust settings based on metrics
        self.adjust_performance_settings(metrics);
    }

    /// Adjust performance settings based on metrics
    fn adjust_performance_settings(&mut self, metrics: &ParticlePerformanceMetrics) {
        let settings = &mut self.advanced_performance_settings;
        
        // Adjust particle count based on frame time
        if metrics.avg_frame_time > settings.target_frame_time {
            let scale = settings.target_frame_time / metrics.avg_frame_time;
            settings.base.max_particles = 
                (settings.base.max_particles as f32 * scale.powf(settings.lod_adjustment_speed)) as u32;
        }
        
        // Adjust quality settings based on performance
        let performance_factor = settings.target_frame_time / metrics.avg_frame_time;
        settings.quality_settings.soft_particles *= performance_factor.min(1.0);
        settings.quality_settings.distortion *= performance_factor.min(1.0);
        settings.quality_settings.lighting *= performance_factor.min(1.0);
        settings.quality_settings.noise *= performance_factor.min(1.0);
        
        // Adjust batch size based on visible particles
        if settings.dynamic_batching {
            let optimal_batch_size = (metrics.visible_particles as f32 / settings.max_draw_calls as f32).ceil();
            settings.batch_size = optimal_batch_size.max(100.0) as u32;
        }
        
        // Check memory budget
        if metrics.gpu_memory_usage > settings.memory_budget {
            let scale = settings.memory_budget as f32 / metrics.gpu_memory_usage as f32;
            settings.base.max_particles = (settings.base.max_particles as f32 * scale) as u32;
        }
    }

    /// Calculate GPU memory usage
    fn calculate_gpu_memory_usage(&self) -> u64 {
        let particle_size = std::mem::size_of::<ParticleBatch>();
        let instance_size = std::mem::size_of::<InstanceData>();
        let buffer_size = self.advanced_performance_settings.base.max_particles as u64 * 
                         (particle_size + instance_size) as u64;
        
        // Add texture memory
        let texture_size = self.texture_dimensions.x * self.texture_dimensions.y * 4; // RGBA
        
        buffer_size + texture_size
    }

    /// Update performance settings based on current frame stats
    fn update_performance_settings(&mut self, frame_stats: &FrameStats) {
        if frame_stats.frame_time > Duration::from_millis(16) {
            // If frame time is above 16ms (60 FPS), reduce particle count
            self.performance_settings.max_particles = 
                (self.performance_settings.max_particles as f32 * 0.9) as u32;
        } else if frame_stats.frame_time < Duration::from_millis(8) {
            // If frame time is below 8ms, gradually increase particle count
            self.performance_settings.max_particles = 
                (self.performance_settings.max_particles as f32 * 1.1) as u32;
        }
    }

    /// Internal method to create a bind group
    pub(crate) fn create_bind_group(
        &mut self,
        device: &RenderDevice,
        layout: &BindGroupLayout,
        view_uniforms: &ViewUniform,
        time: &Time,
    ) -> BindGroup {
        let params = MaterialParams {
            current_time: time.elapsed_seconds(),
            gradient_color_start: self.gradient_color_start,
            gradient_color_end: self.gradient_color_end,
            effect_params: Vec4::new(
                self.noise_scale,
                self.noise_speed,
                self.gradient_blend,
                self.distortion_strength,
            ),
            performance_params: Vec4::new(
                self.max_visible_particles,
                self.lod_bias,
                self.quality_level,
                0.0,
            ),
            ..Default::default()
        };

        let params_buffer = self.params_buffer.get_or_insert_with(|| {
            device.create_buffer_with_data(&BufferInitDescriptor {
                label: Some("particle_material_params_buffer"),
                contents: bytemuck::cast_slice(&[params]),
                usage: BufferBindingType::UNIFORM | BufferBindingType::COPY_DST,
            })
        });

        // Update buffer if it already exists
        if self.params_buffer.is_some() {
            device.queue().write_buffer(params_buffer, 0, bytemuck::cast_slice(&[params]));
        }

        // Create bind group
        device.create_bind_group(&BindGroupDescriptor {
            label: Some("particle_material_bind_group"),
            layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: params_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(&self.texture.texture_view),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::Sampler(&self.texture.sampler),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: BindingResource::TextureView(&view_uniforms.depth_texture_view),
                },
            ],
        })
    }

    pub(crate) fn get_blend_state(&self) -> BlendState {
        match self.blend_mode {
            BlendMode::Alpha => BlendState::ALPHA_BLENDING,
            BlendMode::Additive => BlendState {
                color: BlendComponent {
                    src_factor: BlendFactor::One,
                    dst_factor: BlendFactor::One,
                    operation: BlendOperation::Add,
                },
                alpha: BlendComponent {
                    src_factor: BlendFactor::One,
                    dst_factor: BlendFactor::One,
                    operation: BlendOperation::Add,
                },
            },
            BlendMode::Premultiplied => BlendState {
                color: BlendComponent {
                    src_factor: BlendFactor::One,
                    dst_factor: BlendFactor::OneMinusSrcAlpha,
                    operation: BlendOperation::Add,
                },
                alpha: BlendComponent {
                    src_factor: BlendFactor::One,
                    dst_factor: BlendFactor::OneMinusSrcAlpha,
                    operation: BlendOperation::Add,
                },
            },
        }
    }

    /// Create a preset for fire/flame effects
    pub fn preset_fire(texture: Handle<Image>) -> Self {
        Self::new(texture)
            .with_blend_mode(BlendMode::Additive)
            .with_color_tint(Vec4::new(1.2, 0.6, 0.2, 1.0))  // Warm orange
            .with_emission(2.0)
            .with_soft_particles(true, 0.5)
            .with_custom_params(Vec4::new(
                1.0,   // Size scale (LOD)
                0.2,   // Heat distortion
                0.8,   // Flicker intensity
                0.0,   // Reserved
            ))
            .with_lod_settings(LodSettings {
                fade_start: 15.0,
                fade_end: 40.0,
                min_size: 0.4,
                auto_lod: true,
            })
    }

    /// Create a preset for smoke effects
    pub fn preset_smoke(texture: Handle<Image>) -> Self {
        Self::new(texture)
            .with_blend_mode(BlendMode::Alpha)
            .with_color_tint(Vec4::new(0.9, 0.9, 0.9, 0.8))  // Light gray
            .with_emission(0.0)
            .with_soft_particles(true, 2.0)
            .with_custom_params(Vec4::new(
                1.0,   // Size scale (LOD)
                0.3,   // Turbulence
                0.0,   // Reserved
                0.0,   // Reserved
            ))
            .with_lod_settings(LodSettings {
                fade_start: 20.0,
                fade_end: 60.0,
                min_size: 0.6,
                auto_lod: true,
            })
    }

    /// Create a preset for magic/spell effects
    pub fn preset_magic(texture: Handle<Image>) -> Self {
        Self::new(texture)
            .with_blend_mode(BlendMode::Additive)
            .with_color_tint(Vec4::new(0.4, 0.8, 1.2, 1.0))  // Bright blue
            .with_emission(3.0)
            .with_soft_particles(true, 1.0)
            .with_custom_params(Vec4::new(
                1.0,   // Size scale (LOD)
                0.5,   // Sparkle intensity
                0.8,   // Pulse frequency
                0.0,   // Reserved
            ))
            .with_lod_settings(LodSettings {
                fade_start: 10.0,
                fade_end: 30.0,
                min_size: 0.3,
                auto_lod: true,
            })
    }

    /// Create a preset for dust/debris effects
    pub fn preset_dust(texture: Handle<Image>) -> Self {
        Self::new(texture)
            .with_blend_mode(BlendMode::Alpha)
            .with_color_tint(Vec4::new(0.8, 0.75, 0.7, 0.6))  // Dusty brown
            .with_emission(0.0)
            .with_soft_particles(true, 1.5)
            .with_custom_params(Vec4::new(
                1.0,   // Size scale (LOD)
                0.2,   // Drift amount
                0.1,   // Rotation speed
                0.0,   // Reserved
            ))
            .with_lod_settings(LodSettings {
                fade_start: 25.0,
                fade_end: 70.0,
                min_size: 0.5,
                auto_lod: true,
            })
    }

    /// Create a preset for water/splash effects
    pub fn preset_water(texture: Handle<Image>) -> Self {
        Self::new(texture)
            .with_blend_mode(BlendMode::Premultiplied)
            .with_color_tint(Vec4::new(0.6, 0.8, 1.0, 0.7))  // Light blue
            .with_emission(0.5)
            .with_soft_particles(true, 1.0)
            .with_custom_params(Vec4::new(
                1.0,   // Size scale (LOD)
                0.3,   // Ripple intensity
                0.5,   // Refraction amount
                0.0,   // Reserved
            ))
            .with_lod_settings(LodSettings {
                fade_start: 15.0,
                fade_end: 45.0,
                min_size: 0.4,
                auto_lod: true,
            })
    }

    /// Create a preset for an energy beam effect with color gradient
    pub fn preset_energy_beam(texture: Handle<Image>) -> Self {
        Self::new(texture)
            .with_blend_mode(BlendMode::Additive)
            .with_gradient(
                Vec4::new(0.2, 0.8, 1.0, 1.0),  // Blue core
                Vec4::new(1.0, 0.4, 0.8, 0.0)   // Pink edges
            )
            .with_emission(2.0)
            .with_noise(2.0, 0.5)
            .with_gradient_blend(0.8)
            .with_distortion(0.2)
            .with_performance_settings(500.0, 1.0, 1.0)
    }

    /// Create a preset for a mystical portal effect
    pub fn preset_portal(texture: Handle<Image>) -> Self {
        Self::new(texture)
            .with_blend_mode(BlendMode::Premultiplied)
            .with_gradient(
                Vec4::new(0.5, 0.1, 0.8, 1.0),  // Purple core
                Vec4::new(0.1, 0.8, 0.6, 0.0)   // Teal edges
            )
            .with_emission(1.5)
            .with_noise(4.0, 0.3)
            .with_gradient_blend(1.0)
            .with_distortion(0.4)
            .with_performance_settings(200.0, 1.0, 0.8)
    }

    /// Create a preset for a void/black hole effect
    pub fn preset_void(texture: Handle<Image>) -> Self {
        Self::new(texture)
            .with_blend_mode(BlendMode::Premultiplied)
            .with_gradient(
                Vec4::new(0.1, 0.0, 0.2, 1.0),  // Dark purple core
                Vec4::new(0.5, 0.0, 1.0, 0.0)   // Bright purple edges
            )
            .with_emission(1.2)
            .with_noise(8.0, 0.2)               // Strong noise distortion
            .with_gradient_blend(0.9)
            .with_distortion(0.8)               // Heavy distortion
            .with_performance_settings(300.0, 1.0, 1.0)
            .with_soft_particles(true, 2.0)
    }

    /// Create a preset for a crystal/diamond effect
    pub fn preset_crystal(texture: Handle<Image>) -> Self {
        Self::new(texture)
            .with_blend_mode(BlendMode::Premultiplied)
            .with_gradient(
                Vec4::new(1.0, 1.0, 1.0, 0.9),  // White core
                Vec4::new(0.7, 0.9, 1.0, 0.0)   // Light blue edges
            )
            .with_emission(3.0)
            .with_noise(1.5, 0.1)               // Subtle noise
            .with_gradient_blend(0.7)
            .with_distortion(0.1)               // Minimal distortion
            .with_performance_settings(150.0, 1.0, 0.9)
            .with_soft_particles(true, 0.5)
    }

    /// Create a preset for a toxic/acid effect
    pub fn preset_toxic(texture: Handle<Image>) -> Self {
        Self::new(texture)
            .with_blend_mode(BlendMode::Additive)
            .with_gradient(
                Vec4::new(0.2, 1.0, 0.0, 1.0),  // Bright green core
                Vec4::new(0.8, 1.0, 0.0, 0.0)   // Yellow-green edges
            )
            .with_emission(1.8)
            .with_noise(3.0, 0.4)               // Moderate noise
            .with_gradient_blend(0.6)
            .with_distortion(0.3)
            .with_performance_settings(400.0, 1.0, 0.9)
            .with_soft_particles(true, 1.0)
    }

    /// Create a preset for a lightning/electric effect
    pub fn preset_lightning(texture: Handle<Image>) -> Self {
        Self::new(texture)
            .with_blend_mode(BlendMode::Additive)
            .with_gradient(
                Vec4::new(0.9, 0.9, 1.0, 1.0),  // White-blue core
                Vec4::new(0.2, 0.4, 1.0, 0.0)   // Deep blue edges
            )
            .with_emission(4.0)                  // Very bright
            .with_noise(6.0, 0.8)               // Fast, strong noise
            .with_gradient_blend(0.5)
            .with_distortion(0.6)
            .with_performance_settings(200.0, 1.0, 1.0)
            .with_soft_particles(true, 0.3)
    }

    /// Create a preset for a hologram/glitch effect
    pub fn preset_hologram(texture: Handle<Image>) -> Self {
        Self::new(texture)
            .with_blend_mode(BlendMode::Additive)
            .with_gradient(
                Vec4::new(0.0, 0.8, 0.8, 0.8),  // Cyan core
                Vec4::new(0.0, 0.4, 0.4, 0.0)   // Dark cyan edges
            )
            .with_emission(1.2)
            .with_noise(10.0, 0.9)              // Very fast noise for glitch effect
            .with_gradient_blend(0.3)
            .with_distortion(0.5)
            .with_performance_settings(250.0, 1.0, 0.95)
            .with_soft_particles(true, 0.8)
    }

    /// Prepare batches for rendering
    pub fn prepare_batches(&mut self, particles: &[Entity], query: &Query<(&Transform, &ParticleInstance)>) {
        if !self.performance_settings.gpu_instancing {
            return;
        }
        
        let batch_size = self.performance_settings.batch_size;
        let num_batches = (particles.len() + batch_size - 1) / batch_size;
        
        // Resize batch vector if needed
        if self.batches.len() != num_batches {
            self.batches.resize_with(num_batches, || ParticleBatch {
                particle_count: 0,
                instance_buffer: None,
                visible: true,
                last_update: Instant::now(),
            });
        }
        
        // Update each batch
        for (batch_idx, batch) in self.batches.iter_mut().enumerate() {
            let start_idx = batch_idx * batch_size;
            let end_idx = (start_idx + batch_size).min(particles.len());
            batch.particle_count = (end_idx - start_idx) as u32;
            
            // Generate instance data
            let mut instance_data = Vec::with_capacity(batch.particle_count as usize);
            for entity in &particles[start_idx..end_idx] {
                if let Ok((transform, instance)) = query.get(*entity) {
                    instance_data.push(InstanceData {
                        transform: transform.compute_matrix(),
                        color: instance.color,
                        uv_data: instance.uv_data,
                        custom_data: Vec4::new(
                            instance.age,
                            instance.size,
                            instance.random_seed,
                            instance.custom_param,
                        ),
                    });
                }
            }
            
            // Create or update instance buffer
            if batch.instance_buffer.is_none() {
                batch.instance_buffer = Some(self.create_instance_buffer(&instance_data));
            } else {
                self.update_instance_buffer(batch.instance_buffer.as_mut().unwrap(), &instance_data);
            }
            
            batch.last_update = Instant::now();
        }
    }
    
    fn create_instance_buffer(&self, instance_data: &[InstanceData]) -> Buffer {
        let mut buffer = Buffer::new(
            instance_data.len() * std::mem::size_of::<InstanceData>(),
            BufferUsage::VERTEX | BufferUsage::COPY_DST,
        );
        buffer.set_data(0, bytemuck::cast_slice(instance_data));
        buffer
    }
    
    fn update_instance_buffer(&self, buffer: &mut Buffer, instance_data: &[InstanceData]) {
        buffer.set_data(0, bytemuck::cast_slice(instance_data));
    }

    /// Update batch visibility based on frustum culling
    pub fn update_batch_visibility(&mut self, camera: &Camera, camera_transform: &GlobalTransform) {
        for batch in &mut self.batches {
            // Skip if batch was recently updated
            if batch.last_update.elapsed() < Duration::from_millis(16) {
                continue;
            }
            
            // Calculate batch bounds
            let bounds = self.calculate_batch_bounds(batch);
            
            // Check if batch is in view frustum
            batch.visible = camera.frustum.intersects_obb(
                &bounds,
                &camera_transform.compute_matrix(),
            );
        }
    }
    
    fn calculate_batch_bounds(&self, batch: &ParticleBatch) -> Aabb {
        let mut min = Vec3::splat(f32::INFINITY);
        let mut max = Vec3::splat(f32::NEG_INFINITY);
        
        // Calculate bounds from instance data
        if let Some(buffer) = &batch.instance_buffer {
            let data = buffer.slice(..);
            let instance_data = data.get::<InstanceData>();
            
            for instance in instance_data {
                let pos = instance.transform.col(3).xyz();
                let size = instance.custom_data.y;
                let half_size = Vec3::splat(size * 0.5);
                
                min = min.min(pos - half_size);
                max = max.max(pos + half_size);
            }
        }
        
        Aabb::from_min_max(min, max)
    }

    fn adjust_batch_size(&mut self, metrics: &ParticlePerformanceMetrics) {
        let settings = &self.advanced_performance_settings.dynamic_batch_settings;
        
        // Skip adjustment if dynamic batching is disabled
        if !self.advanced_performance_settings.dynamic_batching {
            return;
        }
        
        // Calculate performance factors
        let frame_time_factor = settings.target_frame_time / metrics.avg_frame_time;
        let memory_factor = (self.advanced_performance_settings.memory_budget as f32) /
                          (metrics.batching_metrics.instance_buffer_memory as f32);
        let batch_efficiency = metrics.batching_metrics.avg_particles_per_batch /
                             self.advanced_performance_settings.batch_size as f32;
        
        // Calculate optimal batch size
        let mut new_batch_size = self.advanced_performance_settings.batch_size as f32;
        
        // Adjust based on frame time
        if frame_time_factor < 0.9 {
            // Reduce batch size if we're missing performance target
            new_batch_size *= 0.9 + (frame_time_factor - 0.9) * settings.adjustment_speed;
        } else if frame_time_factor > 1.1 && batch_efficiency > 0.8 {
            // Increase batch size if we have performance headroom and good efficiency
            new_batch_size *= 1.1 + (frame_time_factor - 1.1) * settings.adjustment_speed;
        }
        
        // Adjust based on memory usage
        if memory_factor < 1.0 {
            new_batch_size *= memory_factor.powf(settings.adjustment_speed);
        }
        
        // Apply constraints
        new_batch_size = new_batch_size
            .clamp(
                settings.min_batch_size as f32,
                settings.max_batch_size as f32
            );
        
        // Update batch size
        self.advanced_performance_settings.batch_size = new_batch_size as u32;
    }

    /// Enable debug visualization for batch bounds and culling
    pub fn with_batch_debug(self, commands: &mut Commands, entity: Entity) -> Self {
        commands.entity(entity).insert(DebugBatchVisualization::default());
        self
    }

    /// Enable performance visualization
    pub fn with_performance_visualization(self, commands: &mut Commands, entity: Entity) -> Self {
        commands.entity(entity).insert(PerformanceVisualization::default());
        self
    }

    /// Update batching metrics
    fn update_batching_metrics(&self, metrics: &mut BatchingMetrics) {
        let total_batches = self.batches.len() as u32;
        let visible_batches = self.batches.iter().filter(|b| b.visible).count() as u32;
        let total_particles: u32 = self.batches.iter().map(|b| b.particle_count).sum();
        
        metrics.active_batch_count = total_batches;
        metrics.visible_batch_count = visible_batches;
        metrics.culled_batch_count = total_batches - visible_batches;
        metrics.avg_particles_per_batch = if total_batches > 0 {
            total_particles as f32 / total_batches as f32
        } else {
            0.0
        };
        
        metrics.max_batch_size = self.batches.iter()
            .map(|b| b.particle_count)
            .max()
            .unwrap_or(0);
            
        // Calculate memory usage
        metrics.instance_buffer_memory = self.batches.iter()
            .filter_map(|b| b.instance_buffer.as_ref())
            .map(|buffer| buffer.size())
            .sum();
            
        // Calculate batch stability (how much batches change frame-to-frame)
        let avg_update_interval = self.batches.iter()
            .map(|b| b.last_update.elapsed().as_secs_f32())
            .sum::<f32>() / total_batches as f32;
        metrics.batch_stability = 1.0 - (avg_update_interval / 0.016).min(1.0);
        
        // Calculate memory fragmentation
        let used_memory = metrics.instance_buffer_memory;
        let allocated_memory = self.batches.iter()
            .filter_map(|b| b.instance_buffer.as_ref())
            .map(|buffer| buffer.capacity())
            .sum::<u64>();
        metrics.memory_fragmentation = if allocated_memory > 0 {
            1.0 - (used_memory as f32 / allocated_memory as f32)
        } else {
            0.0
        };
    }

    /// Enable debug features
    pub fn with_debug_features(self, commands: &mut Commands, entity: Entity) -> Self {
        commands.entity(entity).insert(ParticleDebugFeatures::default());
        self
    }

    /// Get the wireframe pipeline
    pub(crate) fn get_wireframe_pipeline(
        &self,
        pipeline_cache: &PipelineCache,
    ) -> Option<RenderPipeline> {
        let mut descriptor = self.get_pipeline_descriptor(pipeline_cache)?;
        descriptor.primitive.polygon_mode = PolygonMode::Line;
        Some(pipeline_cache.get_render_pipeline(descriptor))
    }
}

/// Resource for managing the particle material pipeline
#[derive(Resource)]
pub struct ParticleMaterialPipeline {
    /// The render pipeline for particle rendering
    pub material_layout: BindGroupLayout,
    /// Vertex buffer layout for the pipeline
    pub vertex_layout: VertexBufferLayout,
}

impl FromWorld for ParticleMaterialPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

        // Create bind group layout
        let material_layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("particle_material_layout"),
            entries: &[
                // Material params (uniform buffer)
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX_FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: Some(MaterialParams::min_size()),
                    },
                    count: None,
                },
                // Base texture
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                // Texture sampler
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
                // Particle buffer
                BindGroupLayoutEntry {
                    binding: 3,
                    visibility: ShaderStages::VERTEX_FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Particle indices buffer
                BindGroupLayoutEntry {
                    binding: 4,
                    visibility: ShaderStages::VERTEX,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Depth texture
                BindGroupLayoutEntry {
                    binding: 5,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Depth,
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
            ],
        });

        // Define vertex buffer layout
        let vertex_layout = VertexBufferLayout {
            array_stride: 32, // 3 (pos) + 3 (normal) + 2 (uv) = 8 floats * 4 bytes
            step_mode: VertexStepMode::Vertex,
            attributes: vec![
                VertexAttribute {
                    format: VertexFormat::Float32x3,
                    offset: 0,
                    shader_location: 0, // position
                },
                VertexAttribute {
                    format: VertexFormat::Float32x3,
                    offset: 12, // 3 * 4 bytes
                    shader_location: 1, // normal
                },
                VertexAttribute {
                    format: VertexFormat::Float32x2,
                    offset: 24, // (3 + 3) * 4 bytes
                    shader_location: 2, // uv
                },
            ],
        };

        Self {
            material_layout,
            vertex_layout,
        }
    }
}

impl ParticleMaterialPipeline {
    pub fn get_render_pipeline(
        &self,
        pipeline_cache: &PipelineCache,
        blend_mode: u32,
    ) -> Option<bevy::render::render_resource::RenderPipeline> {
        let shader = pipeline_cache.get_shader("shaders/particle_render.wgsl")?;

        let blend_state = match blend_mode {
            0 => Some(BlendState::ALPHA_BLENDING), // Alpha blending
            1 => Some(BlendState { // Additive blending
                color: BlendComponent {
                    src_factor: BlendFactor::One,
                    dst_factor: BlendFactor::One,
                    operation: BlendOperation::Add,
                },
                alpha: BlendComponent {
                    src_factor: BlendFactor::One,
                    dst_factor: BlendFactor::One,
                    operation: BlendOperation::Add,
                },
            }),
            2 => Some(BlendState { // Premultiplied alpha
                color: BlendComponent {
                    src_factor: BlendFactor::One,
                    dst_factor: BlendFactor::OneMinusSrcAlpha,
                    operation: BlendOperation::Add,
                },
                alpha: BlendComponent {
                    src_factor: BlendFactor::One,
                    dst_factor: BlendFactor::OneMinusSrcAlpha,
                    operation: BlendOperation::Add,
                },
            }),
            _ => None,
        }?;

        let pipeline_descriptor = RenderPipelineDescriptor {
            label: Some("particle_pipeline"),
            layout: vec![self.material_layout.clone()],
            vertex: VertexState {
                shader: shader.clone(),
                entry_point: "vertex".into(),
                shader_defs: vec![],
                buffers: vec![self.vertex_layout.clone()],
            },
            fragment: Some(FragmentState {
                shader,
                shader_defs: vec![],
                entry_point: "fragment".into(),
                targets: vec![Some(ColorTargetState {
                    format: TextureFormat::bevy_default(),
                    blend: Some(blend_state),
                    write_mask: ColorWrites::ALL,
                })],
            }),
            primitive: PrimitiveState {
                front_face: FrontFace::Ccw,
                cull_mode: None, // No culling for particles
                unclipped_depth: false,
                polygon_mode: PolygonMode::Fill,
                conservative: false,
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
            },
            depth_stencil: Some(DepthStencilState {
                format: TextureFormat::Depth32Float,
                depth_write_enabled: false, // Disable depth writes for transparency
                depth_compare: CompareFunction::LessEqual,
                stencil: Default::default(),
                bias: DepthBiasState::default(),
            }),
            multisample: MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            push_constant_ranges: vec![],
        };

        Some(pipeline_cache.get_render_pipeline(pipeline_descriptor))
    }

    pub fn create_bind_group(
        &self,
        render_device: &RenderDevice,
        material_buffer: &Buffer,
        texture_view: &TextureView,
        sampler: &Sampler,
        particle_buffer: &Buffer,
        index_buffer: &Buffer,
        depth_texture_view: &TextureView,
    ) -> BindGroup {
        render_device.create_bind_group(&BindGroupDescriptor {
            label: Some("particle_material_bind_group"),
            layout: &self.material_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: material_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(texture_view),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::Sampler(sampler),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: particle_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 4,
                    resource: index_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 5,
                    resource: BindingResource::TextureView(depth_texture_view),
                },
            ],
        })
    }
}

/// Plugin for particle material systems
pub struct ParticleMaterialPlugin;

impl Plugin for ParticleMaterialPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_system(update_material_performance.in_set(ParticleSystemSet::Update))
            .add_system(cull_particles.in_set(ParticleSystemSet::Update))
            .add_system(update_material_params)
            .add_system(create_material_bind_groups)
            .add_system(update_lod_settings)
            .add_system(debug_draw_lod_ranges)
            .add_system(update_performance_metrics.in_set(ParticleSystemSet::Update))
            .add_system(prepare_particle_batches.in_set(ParticleSystemSet::Prepare))
            .add_system(update_batch_visibility.in_set(ParticleSystemSet::Update))
            .add_system(debug_draw_batch_bounds.in_set(ParticleSystemSet::Update))
            .add_system(draw_performance_visualization.in_set(ParticleSystemSet::Update))
            .add_system(update_particle_debug_features.in_set(ParticleSystemSet::Update))
            .add_system(update_trail_settings);
    }
}

/// System to update material parameters
pub fn update_material_params(
    time: Res<Time>,
    mut materials: Query<&mut ParticleMaterial>,
    render_device: Res<RenderDevice>,
) {
    for mut material in materials.iter_mut() {
        if let Some(buffer) = &material.params_buffer {
            let params = MaterialParams {
                current_time: time.elapsed_seconds(),
                ..Default::default() // Other params are updated in create_bind_group
            };
            render_device.queue().write_buffer(buffer, 0, bytemuck::cast_slice(&[params]));
        }
    }
}

/// System to create material bind groups
pub fn create_material_bind_groups(
    mut commands: Commands,
    mut materials: Query<(Entity, &ParticleMaterial, &ParticleSystem), Added<ParticleMaterial>>,
    material_pipeline: Res<ParticleMaterialPipeline>,
    render_device: Res<RenderDevice>,
    images: Res<RenderAssets<Image>>,
) {
    for (entity, material, particle_system) in materials.iter_mut() {
        if let Some(gpu_image) = images.get(&material.texture) {
            let bind_group = material_pipeline.create_bind_group(
                &render_device,
                &material.params_buffer.as_ref().unwrap(),
                &gpu_image.texture_view,
                &gpu_image.sampler,
                &particle_system.particle_buffer,
                &particle_system.index_buffer,
                &particle_system.depth_texture_view,
            );

            commands.entity(entity).insert(bind_group);
        }
    }
}

/// System to update LOD settings based on camera distance
pub fn update_lod_settings(
    mut materials: Query<(&mut ParticleMaterial, &GlobalTransform)>,
    camera: Query<(&Camera, &GlobalTransform)>,
) {
    // Get camera position
    let (_, camera_transform) = camera.single();
    let camera_pos = camera_transform.translation();

    // Update LOD for each material
    for (mut material, transform) in materials.iter_mut() {
        if !material.lod_settings.auto_lod {
            continue;
        }

        let distance = camera_pos.distance(transform.translation());
        let fade_range = material.lod_settings.fade_end - material.lod_settings.fade_start;
        
        // Calculate LOD factor (0.0 = full detail, 1.0 = minimum detail)
        let lod_factor = if distance <= material.lod_settings.fade_start {
            0.0
        } else if distance >= material.lod_settings.fade_end {
            1.0
        } else {
            (distance - material.lod_settings.fade_start) / fade_range
        };

        // Update material parameters based on LOD
        let size_scale = 1.0 - (lod_factor * (1.0 - material.lod_settings.min_size));
        material.custom_params.x = size_scale; // Use x component for size scaling

        // Update distance fade range for shader
        material.distance_fade_range = Vec2::new(
            material.lod_settings.fade_start,
            material.lod_settings.fade_end,
        );
    }
}

/// Component to enable LOD range visualization
#[derive(Component)]
pub struct DebugLodRanges {
    /// Whether to show the LOD range visualization
    pub enabled: bool,
    /// Color for the inner sphere (fade start)
    pub inner_color: Color,
    /// Color for the outer sphere (fade end)
    pub outer_color: Color,
}

impl Default for DebugLodRanges {
    fn default() -> Self {
        Self {
            enabled: true,
            inner_color: Color::rgba(0.0, 1.0, 0.0, 0.2), // Green
            outer_color: Color::rgba(1.0, 0.0, 0.0, 0.1), // Red
        }
    }
}

/// System to visualize LOD ranges for debugging
fn debug_draw_lod_ranges(
    materials: Query<(&ParticleMaterial, &GlobalTransform, &DebugLodRanges)>,
    mut gizmos: Gizmos,
) {
    for (material, transform, debug) in materials.iter() {
        if !debug.enabled {
            continue;
        }

        let position = transform.translation();

        // Draw inner sphere at fade_start distance
        gizmos.sphere(
            position,
            Quat::IDENTITY,
            material.lod_settings.fade_start,
            debug.inner_color,
        );

        // Draw outer sphere at fade_end distance
        gizmos.sphere(
            position,
            Quat::IDENTITY,
            material.lod_settings.fade_end,
            debug.outer_color,
        );

        // Draw connecting line between spheres
        gizmos.line(
            position,
            position + Vec3::new(material.lod_settings.fade_end, 0.0, 0.0),
            Color::WHITE,
        );

        // Draw text labels
        let label_offset = Vec3::new(0.0, material.lod_settings.fade_end * 0.1, 0.0);
        gizmos.text(
            format!("LOD Start: {:.1}", material.lod_settings.fade_start),
            position + label_offset,
            Color::WHITE,
        );
        gizmos.text(
            format!("LOD End: {:.1}", material.lod_settings.fade_end),
            position + label_offset * 2.0,
            Color::WHITE,
        );
    }
}

// Helper function to enable LOD debugging
impl ParticleMaterial {
    /// Enable debug visualization of LOD ranges
    pub fn with_debug_lod(self, commands: &mut Commands, entity: Entity) -> Self {
        commands.entity(entity).insert(DebugLodRanges::default());
        self
    }
}

/// System to update material performance settings
pub fn update_material_performance(
    mut materials: Query<&mut ParticleMaterial>,
    frame_stats: Res<FrameStats>,
) {
    for mut material in materials.iter_mut() {
        material.update_performance_settings(&frame_stats);
    }
}

/// System to cull particles based on distance and frustum
pub fn cull_particles(
    mut particles: Query<(&mut Visibility, &GlobalTransform)>,
    materials: Query<&ParticleMaterial>,
    camera: Query<(&Camera, &GlobalTransform)>,
) {
    let (camera, camera_transform) = camera.single();
    
    for (mut visibility, transform) in particles.iter_mut() {
        let distance = transform.translation().distance(camera_transform.translation());
        
        for material in materials.iter() {
            if distance > material.performance_settings.cull_distance {
                *visibility = Visibility::Hidden;
                continue;
            }
            
            if material.performance_settings.use_frustum_culling {
                let in_frustum = camera.frustum.contains_point(transform.translation());
                if !in_frustum {
                    *visibility = Visibility::Hidden;
                    continue;
                }
            }
            
            *visibility = Visibility::Visible;
        }
    }
}

/// System to collect and update performance metrics
pub fn update_performance_metrics(
    mut materials: Query<(&mut ParticleMaterial, &mut ParticlePerformanceMetrics)>,
    frame_stats: Res<FrameStats>,
) {
    for (mut material, mut metrics) in materials.iter_mut() {
        if material.advanced_performance_settings.collect_metrics {
            material.update_performance_metrics(&frame_stats, &mut metrics);
            material.update_batching_metrics(&mut metrics.batching_metrics);
            material.adjust_batch_size(&metrics);
        }
    }
}

/// System to prepare particle batches for rendering
pub fn prepare_particle_batches(
    mut materials: Query<&mut ParticleMaterial>,
    particles: Query<(Entity, &Transform, &ParticleInstance)>,
) {
    for mut material in materials.iter_mut() {
        let particle_entities: Vec<_> = particles.iter().map(|(e, _, _)| e).collect();
        material.prepare_batches(&particle_entities, &particles);
    }
}

/// System to update batch visibility
pub fn update_batch_visibility(
    mut materials: Query<&mut ParticleMaterial>,
    camera: Query<(&Camera, &GlobalTransform)>,
) {
    if let Ok((camera, camera_transform)) = camera.get_single() {
        for mut material in materials.iter_mut() {
            material.update_batch_visibility(camera, camera_transform);
        }
    }
}

/// System to visualize batch bounds and culling
fn debug_draw_batch_bounds(
    materials: Query<(&ParticleMaterial, &GlobalTransform, &DebugBatchVisualization)>,
    mut gizmos: Gizmos,
) {
    for (material, transform, debug) in materials.iter() {
        if !debug.show_bounds {
            continue;
        }

        for (i, batch) in material.batches.iter().enumerate() {
            let bounds = material.calculate_batch_bounds(batch);
            let color = if batch.visible {
                debug.visible_color
            } else {
                debug.culled_color
            };

            // Only draw culled batches if enabled
            if !batch.visible && !debug.show_culled {
                continue;
            }

            // Draw batch bounds
            let center = (bounds.min + bounds.max) * 0.5;
            let size = bounds.max - bounds.min;
            
            gizmos.cuboid(
                transform.transform_point(center),
                transform.rotation,
                size,
                color,
            );

            // Draw batch statistics if enabled
            if debug.show_stats {
                let stats_pos = transform.transform_point(bounds.max + Vec3::new(0.0, 0.5, 0.0));
                gizmos.text(
                    format!(
                        "Batch {}: {} particles{}",
                        i,
                        batch.particle_count,
                        if !batch.visible { " (culled)" } else { "" }
                    ),
                    stats_pos,
                    Color::WHITE,
                );
            }
        }

        // Draw global statistics
        if debug.show_stats {
            let stats_pos = transform.translation + Vec3::new(0.0, 2.0, 0.0);
            let visible_count = material.batches.iter().filter(|b| b.visible).count();
            let total_particles: u32 = material.batches.iter().map(|b| b.particle_count).sum();
            
            gizmos.text(
                format!(
                    "Total Batches: {} ({} visible)\nTotal Particles: {}",
                    material.batches.len(),
                    visible_count,
                    total_particles
                ),
                stats_pos,
                Color::WHITE,
            );
        }
    }
}

/// System to draw performance visualization
fn draw_performance_visualization(
    materials: Query<(&ParticleMaterial, &ParticlePerformanceMetrics, &PerformanceVisualization)>,
    mut gizmos: Gizmos,
) {
    for (material, metrics, viz) in materials.iter() {
        if !viz.show_overlay {
            continue;
        }

        let mut y_offset = viz.overlay_position.y;
        let x_pos = viz.overlay_position.x;
        let line_height = 20.0 * viz.overlay_scale;

        // Draw background panel
        if viz.show_graphs {
            gizmos.rect(
                Vec2::new(x_pos + 150.0, y_offset + 100.0),
                0.0,
                Vec2::new(300.0, 200.0) * viz.overlay_scale,
                viz.color_scheme.background,
            );
        }

        // Draw batch metrics
        if viz.show_batch_metrics {
            let batch_metrics = &metrics.batching_metrics;
            let batch_text = format!(
                "Batches: {}/{} ({}% culled)\nAvg Particles/Batch: {:.1}\nBatch Stability: {:.1}%\nMemory Fragmentation: {:.1}%",
                batch_metrics.visible_batch_count,
                batch_metrics.active_batch_count,
                (batch_metrics.culled_batch_count as f32 / batch_metrics.active_batch_count as f32 * 100.0) as u32,
                batch_metrics.avg_particles_per_batch,
                batch_metrics.batch_stability * 100.0,
                batch_metrics.memory_fragmentation * 100.0
            );
            gizmos.text_2d(
                Vec2::new(x_pos, y_offset),
                viz.color_scheme.text,
                batch_text,
            );
            y_offset += line_height * 4.0;
        }

        // Draw memory metrics
        if viz.show_memory_metrics {
            let memory_text = format!(
                "GPU Memory: {:.1}MB\nInstance Buffers: {:.1}MB",
                metrics.gpu_memory_usage as f32 / (1024.0 * 1024.0),
                metrics.batching_metrics.instance_buffer_memory as f32 / (1024.0 * 1024.0)
            );
            gizmos.text_2d(
                Vec2::new(x_pos, y_offset),
                viz.color_scheme.text,
                memory_text,
            );
            y_offset += line_height * 2.0;
        }

        // Draw particle metrics
        if viz.show_particle_metrics {
            let particle_text = format!(
                "Active Particles: {}\nVisible: {}\nCulled: {}\nLOD Level: {:.2}",
                metrics.active_particles,
                metrics.visible_particles,
                metrics.culled_particles,
                metrics.current_lod
            );
            gizmos.text_2d(
                Vec2::new(x_pos, y_offset),
                viz.color_scheme.text,
                particle_text,
            );
        }

        // Draw performance graphs if enabled
        if viz.show_graphs {
            draw_performance_graphs(
                &mut gizmos,
                metrics,
                viz,
                Vec2::new(x_pos + 300.0, viz.overlay_position.y),
            );
        }
    }
}

/// Helper function to draw performance graphs
fn draw_performance_graphs(
    gizmos: &mut Gizmos,
    metrics: &ParticlePerformanceMetrics,
    viz: &PerformanceVisualization,
    position: Vec2,
) {
    // Draw frame time graph
    let graph_height = 50.0 * viz.overlay_scale;
    let graph_width = 200.0 * viz.overlay_scale;
    
    // Draw grid
    for i in 0..=4 {
        let y = position.y + (i as f32 * graph_height / 4.0);
        gizmos.line_2d(
            Vec2::new(position.x, y),
            Vec2::new(position.x + graph_width, y),
            viz.color_scheme.grid,
        );
    }

    // Draw frame time line
    let target_frame_time = 16.0; // 60 FPS
    let frame_time_color = if metrics.avg_frame_time > target_frame_time {
        viz.color_scheme.critical
    } else if metrics.avg_frame_time > target_frame_time * 0.8 {
        viz.color_scheme.warning
    } else {
        viz.color_scheme.good
    };

    gizmos.line_2d(
        Vec2::new(position.x, position.y + graph_height),
        Vec2::new(
            position.x + graph_width,
            position.y + graph_height * (1.0 - metrics.avg_frame_time / target_frame_time / 2.0).clamp(0.0, 1.0),
        ),
        frame_time_color,
    );
}

/// System to update and draw particle debug features
fn update_particle_debug_features(
    mut particles: Query<(
        Entity,
        &GlobalTransform,
        &Velocity,
        &mut ParticleDebugFeatures,
    )>,
    time: Res<Time>,
    mut gizmos: Gizmos,
) {
    for (entity, transform, velocity, mut debug) in particles.iter_mut() {
        let position = transform.translation();
        let current_time = time.elapsed_seconds();

        // Update trail history
        if debug.show_trails {
            // Remove old trail points
            debug.trail_history.retain(|(_, t)| {
                current_time - *t <= debug.trail_length
            });

            // Add new trail point if enough time has passed
            if debug.trail_history.is_empty() || 
               current_time - debug.trail_history.last().unwrap().1 >= debug.trail_spacing {
                debug.trail_history.push((position, current_time));
            }

            // Draw trails
            for i in 1..debug.trail_history.len() {
                let (pos1, t1) = debug.trail_history[i - 1];
                let (pos2, t2) = debug.trail_history[i];
                
                let age = (current_time - t2) / debug.trail_length;
                let alpha = (1.0 - age).powf(debug.trail_fade);
                let color = debug.trail_color.with_a(debug.trail_color.a() * alpha);
                
                gizmos.line(pos1, pos2, color);
            }
        }

        // Draw velocity vector
        if debug.show_velocity {
            let velocity_end = position + velocity.0 * debug.velocity_scale;
            gizmos.line(
                position,
                velocity_end,
                Color::YELLOW,
            );
        }

        // Draw particle bounds
        if debug.show_bounds {
            let size = transform.scale() * 0.5;
            gizmos.cuboid(
                transform.translation(),
                transform.rotation(),
                size,
                debug.wireframe_color,
            );
        }
    }
} 

#[derive(Component, Clone, Default)]
pub struct TrailSettings {
    pub color: Vec4,
    pub fade: f32,
    pub length: f32,
    pub spacing: f32,
    pub time: f32,
    pub enabled: bool,
}

impl TrailSettings {
    pub fn new() -> Self {
        Self {
            color: Vec4::new(1.0, 1.0, 1.0, 1.0),
            fade: 0.8,
            length: 1.0,
            spacing: 0.1,
            time: 0.0,
            enabled: false,
        }
    }

    pub fn with_color(mut self, color: Vec4) -> Self {
        self.color = color;
        self
    }

    pub fn with_fade(mut self, fade: f32) -> Self {
        self.fade = fade;
        self
    }

    pub fn with_length(mut self, length: f32) -> Self {
        self.length = length;
        self
    }

    pub fn with_spacing(mut self, spacing: f32) -> Self {
        self.spacing = spacing;
        self
    }
}

impl ParticleMaterial {
    // ... existing code ...

    pub fn with_trail(mut self, trail: TrailSettings) -> Self {
        self.trail = Some(trail);
        self
    }

    pub fn preset_magic_trail() -> Self {
        Self::preset_magic()
            .with_trail(TrailSettings::new()
                .with_color(Vec4::new(0.2, 0.6, 1.0, 1.0))
                .with_fade(0.8)
                .with_length(1.5)
                .with_spacing(0.05))
    }

    pub fn preset_fire_trail() -> Self {
        Self::preset_fire()
            .with_trail(TrailSettings::new()
                .with_color(Vec4::new(1.0, 0.5, 0.1, 1.0))
                .with_fade(0.6)
                .with_length(1.0)
                .with_spacing(0.08))
    }
}

fn update_trail_settings(
    time: Res<Time>,
    mut materials: Query<(&mut ParticleMaterial, &mut TrailSettings)>,
) {
    for (material, mut trail) in materials.iter_mut() {
        if !trail.enabled {
            continue;
        }

        trail.time += time.delta_seconds();

        // Update shader uniforms
        if let Some(pipeline) = &material.pipeline {
            pipeline.set_uniform("trail", UniformData::from(trail.as_ref()));
        }
    }
}

// Temporary placeholder for missing types
#[derive(Resource)]
struct FrameStats;
#[derive(Component)]
struct ParticleInstance;
#[derive(Component)]
struct SimulationParams;
#[derive(Component)]
struct Velocity;

// Correct Bevy 0.12 VisitAssetDependencies implementation
impl bevy::asset::VisitAssetDependencies for ParticleMaterial {
    fn visit_dependencies(&self, _visit: &mut impl FnMut(bevy::asset::UntypedAssetId)) {
        // No dependencies for ParticleMaterial
    }
}