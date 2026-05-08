use bevy::prelude::*;
use bevy::audio::*;
use bevy::math::{Vec3, Quat};
use std::collections::HashMap;

/// Priority levels for audio sources
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioPriority {
    Low,
    Medium,
    High,
    Critical,
}

impl AudioPriority {
    /// Get the volume multiplier for this priority level
    pub fn volume_multiplier(&self) -> f32 {
        match self {
            AudioPriority::Low => 0.5,
            AudioPriority::Medium => 0.75,
            AudioPriority::High => 1.0,
            AudioPriority::Critical => 1.0,
        }
    }
}

/// Component for configuring spatial audio behavior
#[derive(Component)]
pub struct SpatialAudioSource {
    /// Base volume before distance attenuation
    pub base_volume: f32,
    /// Maximum distance at which the sound can be heard
    pub max_distance: f32,
    /// Distance at which attenuation begins
    pub reference_distance: f32,
    /// How quickly volume drops with distance
    pub rolloff_factor: f32,
    /// Doppler effect strength
    pub doppler_factor: f32,
    /// Whether the sound should loop
    pub looping: bool,
    /// Priority level for volume scaling
    pub priority: AudioPriority,
    /// Current occlusion factor (0.0 = fully occluded, 1.0 = not occluded)
    pub occlusion: Option<f32>,
}

impl Default for SpatialAudioSource {
    fn default() -> Self {
        Self {
            base_volume: 1.0,
            max_distance: 50.0,
            reference_distance: 5.0,
            rolloff_factor: 1.0,
            doppler_factor: 1.0,
            looping: false,
            priority: AudioPriority::Medium,
            occlusion: None,
        }
    }
}

/// Resource for storing the listener position and velocity
#[derive(Resource)]
pub struct AudioListener {
    pub position: Vec3,
    pub velocity: Vec3,
    pub forward: Vec3,
    pub up: Vec3,
}

impl Default for AudioListener {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            velocity: Vec3::ZERO,
            forward: Vec3::Z,
            up: Vec3::Y,
        }
    }
}

/// Resource for managing 3D audio settings and state
#[derive(Resource)]
pub struct SpatialAudioManager {
    /// Active sound sources
    active_sources: HashMap<Entity, AudioSourceData>,
    /// Maximum concurrent sounds
    max_concurrent_sounds: usize,
    /// Global 3D audio settings
    settings: SpatialAudioSettings,
    /// Audio listener data
    listener: Option<AudioListenerData>,
}

#[derive(Debug, Clone)]
pub struct AudioSourceData {
    pub entity: Entity,
    pub position: Vec3,
    pub velocity: Vec3,
    pub volume: f32,
    pub pitch: f32,
}

#[derive(Debug, Clone)]
pub struct AudioListenerData {
    pub position: Vec3,
    pub forward: Vec3,
    pub up: Vec3,
    pub velocity: Vec3,
}

#[derive(Debug, Clone)]
pub struct SpatialAudioSettings {
    pub doppler_scale: f32,
    pub distance_scale: f32,
    pub rolloff_scale: f32,
    pub air_absorption: f32,
    pub occlusion_scale: f32,
}

impl Default for SpatialAudioManager {
    fn default() -> Self {
        Self {
            active_sources: HashMap::new(),
            max_concurrent_sounds: 32,
            settings: SpatialAudioSettings {
                doppler_scale: 1.0,
                distance_scale: 1.0,
                rolloff_scale: 1.0,
                air_absorption: 0.5,
                occlusion_scale: 1.0,
            },
            listener: None,
        }
    }
}

/// Plugin to handle spatial audio
pub struct SpatialAudioPlugin;

impl Plugin for SpatialAudioPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AudioListener>()
           .add_systems(Update, (
               update_listener_position,
               update_spatial_audio,
               update_audio_occlusion,
           ).chain());
    }
}

/// System to update the audio listener position based on camera
fn update_listener_position(
    mut listener: ResMut<AudioListener>,
    camera_query: Query<(&Transform, &GlobalTransform), With<Camera>>,
    time: Res<Time>,
) {
    if let Ok((transform, global_transform)) in camera_query.get_single() {
        let new_position = global_transform.translation();
        listener.velocity = (new_position - listener.position) / time.delta_seconds();
        listener.position = new_position;
        listener.forward = transform.forward();
        listener.up = transform.up();
    }
}

/// System to update spatial audio parameters
fn update_spatial_audio(
    mut audio_sources: Query<(&mut AudioSink, &SpatialAudioSource, &GlobalTransform)>,
    listener: Res<AudioListener>,
) {
    for (mut sink, source, transform) in audio_sources.iter_mut() {
        let source_pos = transform.translation();
        let distance = source_pos.distance(listener.position);

        // Calculate distance attenuation
        let attenuation = if distance <= source.reference_distance {
            1.0
        } else if distance >= source.max_distance {
            0.0
        } else {
            let t = (distance - source.reference_distance) / 
                   (source.max_distance - source.reference_distance);
            (1.0 - t).powf(source.rolloff_factor)
        };

        // Calculate doppler effect
        let source_to_listener = (listener.position - source_pos).normalize();
        let relative_velocity = listener.velocity.dot(source_to_listener);
        let doppler_pitch = 1.0 / (1.0 + source.doppler_factor * relative_velocity / 343.0); // Speed of sound

        // Apply final volume and pitch
        let final_volume = source.base_volume * attenuation * 
                          source.priority.volume_multiplier() *
                          source.occlusion.unwrap_or(1.0);
        
        sink.set_volume(final_volume);
        sink.set_pitch(doppler_pitch);

        // Update panning based on position
        let right = listener.forward.cross(listener.up).normalize();
        let forward = listener.forward;
        
        let to_source = (source_pos - listener.position).normalize();
        let right_dot = to_source.dot(right);
        let forward_dot = to_source.dot(forward);
        
        let pan = right_dot.clamp(-1.0, 1.0);
        sink.set_panning(pan);
    }
}

/// System to update audio occlusion
fn update_audio_occlusion(
    mut audio_sources: Query<(&mut SpatialAudioSource, &GlobalTransform)>,
    listener: Res<AudioListener>,
    // TODO: Add raycast query for actual occlusion testing
) {
    for (mut source, transform) in audio_sources.iter_mut() {
        if source.occlusion.is_some() {
            let source_pos = transform.translation();
            
            // For now, just do a simple distance-based test
            // TODO: Replace with actual raycast occlusion testing
            let distance = source_pos.distance(listener.position);
            let occlusion = (1.0 - (distance / source.max_distance).clamp(0.0, 1.0)).powf(0.5);
            source.occlusion = Some(occlusion);
        }
    }
} 