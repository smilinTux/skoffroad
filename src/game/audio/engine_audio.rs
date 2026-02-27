use bevy::prelude::*;
use bevy::audio::*;
use crate::game::vehicle::Vehicle;
use super::{AudioAssets, SpatialAudioSource, AudioPriority};

/// Component for configuring engine audio behavior
#[derive(Component)]
pub struct EngineAudioConfig {
    /// Base pitch for idle RPM
    pub idle_pitch: f32,
    /// Maximum pitch at redline
    pub max_pitch: f32,
    /// RPM where engine starts to rev
    pub rev_start_rpm: f32,
    /// RPM at redline
    pub redline_rpm: f32,
    /// How quickly pitch changes with RPM
    pub pitch_smoothing: f32,
    /// Volume increase under load
    pub load_volume_factor: f32,
    /// Pitch variation under load
    pub load_pitch_factor: f32,
}

impl Default for EngineAudioConfig {
    fn default() -> Self {
        Self {
            idle_pitch: 1.0,
            max_pitch: 2.0,
            rev_start_rpm: 1000.0,
            redline_rpm: 7000.0,
            pitch_smoothing: 0.1,
            load_volume_factor: 0.2,
            load_pitch_factor: 0.1,
        }
    }
}

/// Plugin to handle engine audio
pub struct EngineAudioPlugin;

impl Plugin for EngineAudioPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (
            setup_engine_audio,
            update_engine_audio,
        ).chain());
    }
}

/// System to set up engine audio for vehicles
fn setup_engine_audio(
    mut commands: Commands,
    audio_assets: Res<AudioAssets>,
    vehicles: Query<(Entity, &Transform), (With<Vehicle>, Without<AudioSink>)>,
) {
    for (entity, transform) in vehicles.iter() {
        // Add idle sound
        commands.entity(entity).insert((
            AudioBundle {
                source: audio_assets.engine_sounds[0].clone(),
                settings: PlaybackSettings::LOOP.with_spatial(true),
            },
            SpatialAudioSource {
                base_volume: 0.8,
                max_distance: 100.0,
                reference_distance: 10.0,
                rolloff_factor: 1.0,
                doppler_factor: 0.5,
                looping: true,
                priority: AudioPriority::High,
                occlusion: None,
            },
            EngineAudioConfig::default(),
        ));

        // Add rev sound
        commands.spawn((
            AudioBundle {
                source: audio_assets.engine_sounds[1].clone(),
                settings: PlaybackSettings::LOOP.with_spatial(true),
            },
            SpatialAudioSource {
                base_volume: 0.0, // Starts silent
                max_distance: 100.0,
                reference_distance: 10.0,
                rolloff_factor: 1.0,
                doppler_factor: 0.5,
                looping: true,
                priority: AudioPriority::High,
                occlusion: None,
            },
            Parent(entity),
        ));

        // Add high RPM sound
        commands.spawn((
            AudioBundle {
                source: audio_assets.engine_sounds[2].clone(),
                settings: PlaybackSettings::LOOP.with_spatial(true),
            },
            SpatialAudioSource {
                base_volume: 0.0, // Starts silent
                max_distance: 100.0,
                reference_distance: 10.0,
                rolloff_factor: 1.0,
                doppler_factor: 0.5,
                looping: true,
                priority: AudioPriority::High,
                occlusion: None,
            },
            Parent(entity),
        ));
    }
}

/// System to update engine audio based on vehicle state
fn update_engine_audio(
    mut audio_sources: Query<(&mut AudioSink, &mut SpatialAudioSource, &Parent)>,
    vehicles: Query<(&Vehicle, &EngineAudioConfig)>,
    time: Res<Time>,
) {
    for (mut sink, mut source, parent) in audio_sources.iter_mut() {
        if let Ok((vehicle, config)) in vehicles.get(parent.get()) {
            // Calculate RPM ratio (0.0 to 1.0)
            let rpm_ratio = (vehicle.engine_rpm - config.rev_start_rpm) / 
                          (config.redline_rpm - config.rev_start_rpm);
            let rpm_ratio = rpm_ratio.clamp(0.0, 1.0);

            // Calculate load factor (0.0 to 1.0)
            let load_factor = vehicle.throttle * vehicle.engine_load;

            // Update pitch based on RPM and load
            let target_pitch = config.idle_pitch + 
                             (config.max_pitch - config.idle_pitch) * rpm_ratio +
                             config.load_pitch_factor * load_factor;

            let current_pitch = sink.pitch();
            let new_pitch = lerp(current_pitch, target_pitch, config.pitch_smoothing * time.delta_seconds());
            sink.set_pitch(new_pitch);

            // Update volume based on RPM and load
            let base_volume = source.base_volume;
            let target_volume = base_volume * (1.0 + config.load_volume_factor * load_factor);
            source.base_volume = lerp(source.base_volume, target_volume, config.pitch_smoothing * time.delta_seconds());

            // Crossfade between idle, rev, and high RPM sounds
            if rpm_ratio < 0.3 {
                // Mostly idle sound
                source.base_volume = lerp(base_volume, 0.8, 0.1);
            } else if rpm_ratio < 0.7 {
                // Blend with rev sound
                source.base_volume = lerp(base_volume, 0.4, 0.1);
            } else {
                // Blend with high RPM sound
                source.base_volume = lerp(base_volume, 0.2, 0.1);
            }
        }
    }
}

/// Linear interpolation helper
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
} 