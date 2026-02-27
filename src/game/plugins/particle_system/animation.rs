use bevy::prelude::*;
use rand::Rng;
use super::particle::ParticleSystem;

/// Animation playback pattern
#[derive(Clone, Copy, Debug)]
pub enum AnimationPattern {
    /// Play frames in sequence (0,1,2,3,0,1,...)
    Forward,
    /// Play frames in reverse (3,2,1,0,3,2,...)
    Reverse,
    /// Ping-pong between frames (0,1,2,3,2,1,0,...)
    PingPong,
    /// Random frame each update
    Random,
    /// Play once and stop on last frame
    OneShot,
}

/// Component to track the current frame of a particle system's atlas animation
#[derive(Component)]
pub struct AtlasAnimation {
    /// Current frame index in the atlas
    pub current_frame: u32,
    /// Total number of frames in the atlas
    pub total_frames: u32,
    /// Time accumulated since last frame change
    pub time_accumulated: f32,
    /// Animation playback pattern
    pub pattern: AnimationPattern,
    /// Direction for ping-pong pattern (true = forward)
    pub ping_pong_forward: bool,
    /// Whether the animation has completed (for OneShot)
    pub completed: bool,
}

impl Default for AtlasAnimation {
    fn default() -> Self {
        Self {
            current_frame: 0,
            total_frames: 4, // 2x2 atlas
            time_accumulated: 0.0,
            pattern: AnimationPattern::Forward,
            ping_pong_forward: true,
            completed: false,
        }
    }
}

impl AtlasAnimation {
    /// Create a new animation with a specific pattern
    pub fn new(pattern: AnimationPattern) -> Self {
        Self {
            pattern,
            ..default()
        }
    }

    /// Update the frame based on the animation pattern
    fn advance_frame(&mut self) {
        if self.completed {
            return;
        }

        match self.pattern {
            AnimationPattern::Forward => {
                self.current_frame = (self.current_frame + 1) % self.total_frames;
            }
            AnimationPattern::Reverse => {
                self.current_frame = if self.current_frame == 0 {
                    self.total_frames - 1
                } else {
                    self.current_frame - 1
                };
            }
            AnimationPattern::PingPong => {
                if self.ping_pong_forward {
                    self.current_frame += 1;
                    if self.current_frame >= self.total_frames - 1 {
                        self.ping_pong_forward = false;
                    }
                } else {
                    self.current_frame = self.current_frame.saturating_sub(1);
                    if self.current_frame == 0 {
                        self.ping_pong_forward = true;
                    }
                }
            }
            AnimationPattern::Random => {
                let mut rng = rand::thread_rng();
                self.current_frame = rng.gen_range(0..self.total_frames);
            }
            AnimationPattern::OneShot => {
                if self.current_frame < self.total_frames - 1 {
                    self.current_frame += 1;
                } else {
                    self.completed = true;
                }
            }
        }
    }
}

/// System to update particle atlas animations
pub fn update_particle_animations(
    time: Res<Time>,
    mut query: Query<(&mut AtlasAnimation, &ParticleSystem)>,
) {
    for (mut animation, particle_system) in query.iter_mut() {
        // Skip if no animation is needed
        if particle_system.params.anim_fps <= 0.0 {
            continue;
        }

        // Update time and check if we need to advance frame
        animation.time_accumulated += time.delta_seconds();
        let frame_duration = 1.0 / particle_system.params.anim_fps;
        
        while animation.time_accumulated >= frame_duration {
            animation.time_accumulated -= frame_duration;
            animation.advance_frame();
        }
    }
}

/// Helper function to get UV coordinates for a frame in a 2x2 atlas
pub fn get_atlas_uvs(frame: u32) -> (Vec2, Vec2) {
    let (x, y) = match frame {
        0 => (0, 0), // Top-left
        1 => (1, 0), // Top-right
        2 => (0, 1), // Bottom-left
        3 => (1, 1), // Bottom-right
        _ => (0, 0), // Default to top-left
    };
    
    let min = Vec2::new(x as f32 * 0.5, y as f32 * 0.5);
    let max = min + Vec2::splat(0.5);
    
    (min, max)
}

/// Plugin to add particle animation systems
pub struct ParticleAnimationPlugin;

impl Plugin for ParticleAnimationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, update_particle_animations);
    }
} 