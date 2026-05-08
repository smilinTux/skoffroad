use bevy::prelude::*;
use bevy::asset::{AssetLoader, LoadContext, LoadedAsset};
use bevy::reflect::TypeUuid;
use bevy::utils::BoxedFuture;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Engine audio configuration asset
#[derive(Debug, Deserialize, TypeUuid)]
#[uuid = "f9e6db71-4388-4a3b-a88f-3793e8e1d992"]
pub struct EngineAudioConfig {
    pub name: String,
    pub description: String,
    pub base_samples: HashMap<String, String>,
    pub crossfade_points: CrossfadePoints,
    pub volume_curve: VolumeCurve,
    pub pitch_curve: PitchCurve,
    pub effects: AudioEffects,
}

#[derive(Debug, Deserialize)]
pub struct CrossfadePoints {
    pub idle_to_low: u32,
    pub low_to_mid: u32,
    pub mid_to_high: u32,
}

#[derive(Debug, Deserialize)]
pub struct VolumeCurve {
    pub idle: f32,
    pub low: f32,
    pub mid: f32,
    pub high: f32,
}

#[derive(Debug, Deserialize)]
pub struct PitchCurve {
    pub min_rpm: u32,
    pub max_rpm: u32,
    pub min_pitch: f32,
    pub max_pitch: f32,
}

#[derive(Debug, Deserialize)]
pub struct AudioEffects {
    pub reverb: ReverbEffect,
    pub doppler: DopplerEffect,
}

#[derive(Debug, Deserialize)]
pub struct ReverbEffect {
    pub enabled: bool,
    pub room_size: f32,
    pub damping: f32,
    pub wet_level: f32,
    pub dry_level: f32,
}

#[derive(Debug, Deserialize)]
pub struct DopplerEffect {
    pub enabled: bool,
    pub factor: f32,
}

#[derive(Default)]
pub struct EngineAudioConfigLoader;

impl AssetLoader for EngineAudioConfigLoader {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<(), bevy::asset::Error>> {
        Box::pin(async move {
            let config: EngineAudioConfig = serde_json::from_slice(bytes)?;
            load_context.set_default_asset(LoadedAsset::new(config));
            Ok(())
        })
    }

    fn extensions(&self) -> &[&str] {
        &["audio.json"]
    }
}

/// Component for managing engine audio state
#[derive(Component)]
pub struct EngineAudioState {
    pub current_rpm: f32,
    pub target_rpm: f32,
    pub volume: f32,
    pub pitch: f32,
}

impl Default for EngineAudioState {
    fn default() -> Self {
        Self {
            current_rpm: 0.0,
            target_rpm: 0.0,
            volume: 0.0,
            pitch: 1.0,
        }
    }
}

/// System to update engine audio based on RPM
pub fn update_engine_audio(
    mut query: Query<(&mut EngineAudioState, &Handle<AudioSource>)>,
    audio: Res<Audio>,
    engine_configs: Res<Assets<EngineAudioConfig>>,
    time: Res<Time>,
) {
    for (mut state, audio_handle) in query.iter_mut() {
        // Update RPM smoothly
        let rpm_delta = (state.target_rpm - state.current_rpm) * time.delta_seconds() * 5.0;
        state.current_rpm += rpm_delta;

        // Calculate volume and pitch based on RPM
        if let Some(config) = engine_configs.get(audio_handle) {
            state.volume = calculate_volume(&config.volume_curve, state.current_rpm);
            state.pitch = calculate_pitch(&config.pitch_curve, state.current_rpm);
            
            // Apply audio updates
            audio.set_volume(state.volume);
            audio.set_playback_rate(state.pitch);
        }
    }
}

fn calculate_volume(curve: &VolumeCurve, rpm: f32) -> f32 {
    // Simple linear interpolation between volume points
    if rpm < 1000.0 {
        curve.idle
    } else if rpm < 3000.0 {
        lerp(curve.idle, curve.low, (rpm - 1000.0) / 2000.0)
    } else if rpm < 5000.0 {
        lerp(curve.low, curve.mid, (rpm - 3000.0) / 2000.0)
    } else {
        lerp(curve.mid, curve.high, (rpm - 5000.0) / 2000.0)
    }
}

fn calculate_pitch(curve: &PitchCurve, rpm: f32) -> f32 {
    let rpm_factor = (rpm - curve.min_rpm as f32) / (curve.max_rpm - curve.min_rpm) as f32;
    lerp(curve.min_pitch, curve.max_pitch, rpm_factor.clamp(0.0, 1.0))
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
} 