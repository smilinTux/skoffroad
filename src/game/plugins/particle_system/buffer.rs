use bevy::{
    prelude::*,
    render::{
        render_resource::{Buffer, BufferUsages, ShaderType},
        renderer::RenderDevice,
    },
};

// Temporary placeholder for missing Particle type
struct Particle;

/// Manages double buffering for particle data
#[derive(Component)]
pub struct ParticleBufferManager {
    /// Current read buffer
    read_buffer: Buffer,
    /// Current write buffer
    write_buffer: Buffer,
    /// Buffer for indirect draw commands
    draw_commands: Buffer,
    /// Maximum number of particles
    max_particles: u32,
    /// Current frame index (used for buffer swapping)
    frame_index: u32,
}

impl ParticleBufferManager {
    pub fn new(device: &RenderDevice, max_particles: u32) -> Self {
        // Create two buffers for double buffering
        let buffer_size = std::mem::size_of::<Particle>() as u64 * max_particles as u64;
        let buffer_desc = bevy::render::render_resource::BufferDescriptor {
            label: Some("particle_buffer"),
            size: buffer_size,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        };

        let read_buffer = device.create_buffer(&buffer_desc);
        let write_buffer = device.create_buffer(&buffer_desc);

        // Create indirect draw commands buffer
        let draw_commands = device.create_buffer(&bevy::render::render_resource::BufferDescriptor {
            label: Some("particle_draw_commands"),
            size: std::mem::size_of::<u32>() as u64 * 4, // vertex_count, instance_count, first_vertex, first_instance
            usage: BufferUsages::STORAGE | BufferUsages::INDIRECT,
            mapped_at_creation: false,
        });

        Self {
            read_buffer,
            write_buffer,
            draw_commands,
            max_particles,
            frame_index: 0,
        }
    }

    /// Get the current read buffer
    pub fn read_buffer(&self) -> &Buffer {
        &self.read_buffer
    }

    /// Get the current write buffer
    pub fn write_buffer(&self) -> &Buffer {
        &self.write_buffer
    }

    /// Get the indirect draw commands buffer
    pub fn draw_commands(&self) -> &Buffer {
        &self.draw_commands
    }

    /// Get the maximum number of particles
    pub fn max_particles(&self) -> u32 {
        self.max_particles
    }

    /// Swap the read and write buffers
    pub fn swap_buffers(&mut self) {
        std::mem::swap(&mut self.read_buffer, &mut self.write_buffer);
        self.frame_index = self.frame_index.wrapping_add(1);
    }

    /// Get the current frame index
    pub fn frame_index(&self) -> u32 {
        self.frame_index
    }
}

/// Parameters for the particle simulation compute shader
#[derive(Clone, ShaderType)]
pub struct SimulationParams {
    /// Time since last frame
    pub delta_time: f32,
    /// Particles to emit per second
    pub emission_rate: f32,
    /// Minimum initial velocity
    pub initial_velocity_min: f32,
    /// Maximum initial velocity
    pub initial_velocity_max: f32,
    /// Minimum particle size
    pub size_min: f32,
    /// Maximum particle size
    pub size_max: f32,
    /// Minimum rotation in radians
    pub rotation_min: f32,
    /// Maximum rotation in radians
    pub rotation_max: f32,
    /// Minimum lifetime in seconds
    pub lifetime_min: f32,
    /// Maximum lifetime in seconds
    pub lifetime_max: f32,
    /// Starting color
    pub color_start: Vec4,
    /// Ending color
    pub color_end: Vec4,
    /// External forces (gravity, wind, etc.)
    pub forces: Vec3,
    /// Texture atlas configuration (columns, rows)
    pub atlas_config: Vec2,
    /// Animation frames per second
    pub anim_fps: f32,
    /// Current number of active particles
    pub active_particles: u32,
    /// Maximum number of particles
    pub max_particles: u32,
    /// Padding to ensure 16-byte alignment
    pub _padding: Vec2,
}

impl Default for SimulationParams {
    fn default() -> Self {
        Self {
            delta_time: 0.0,
            emission_rate: 100.0,
            initial_velocity_min: 1.0,
            initial_velocity_max: 5.0,
            size_min: 0.1,
            size_max: 0.5,
            rotation_min: 0.0,
            rotation_max: std::f32::consts::PI * 2.0,
            lifetime_min: 1.0,
            lifetime_max: 3.0,
            color_start: Vec4::new(1.0, 1.0, 1.0, 1.0),
            color_end: Vec4::new(1.0, 1.0, 1.0, 0.0),
            forces: Vec3::new(0.0, -9.81, 0.0), // Default gravity
            atlas_config: Vec2::new(1.0, 1.0),  // Single texture by default
            anim_fps: 30.0,
            active_particles: 0,
            max_particles: 10000,
            _padding: Vec2::ZERO,
        }
    }
} 