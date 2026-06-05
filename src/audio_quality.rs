// audio_quality.rs — Sprint 91 audio authenticity refactor.
//
// REMOVED: The three engine RPM crossfade layers (Idle/Cruise/Redline)
// that previously ran alongside audio.rs's engine voice. Six simultaneous
// engine voices (audio.rs, engine_pro.rs, audio_quality.rs ×3, and others)
// caused muddy beating, aliasing crackle, and high CPU cost.
//
// KEPT: The atmospheric gust layer — it responds to WindState::speed_mps
// (not chassis speed or RPM) so it is genuinely a separate ambient effect
// unrelated to the engine voice.
//
// The sole engine voice is now in audio.rs (AuthenticEngineAudio) using a
// band-limited impulse train + resonant biquad filter model.

use bevy::prelude::*;
use bevy_kira_audio::{Audio, AudioControl, AudioInstance, AudioSource, AudioTween};
use bevy_kira_audio::prelude::{
    Decibels, StaticSoundData, StaticSoundSettings, Frame as KiraFrame,
};
use std::sync::Arc;

use crate::graphics_quality::GraphicsQuality;
use crate::settings::SettingsState;
use crate::wind::WindState;

// ---------------------------------------------------------------------------
// Public plugin
// ---------------------------------------------------------------------------

pub struct AudioQualityPlugin;

impl Plugin for AudioQualityPlugin {
    fn build(&self, app: &mut App) {
        // Only the gust layer remains — engine layers removed.
        app.add_systems(Startup, spawn_gust_layer)
           .add_systems(Update, modulate_gust_layer);
    }
}

// ---------------------------------------------------------------------------
// Resource
// ---------------------------------------------------------------------------

/// Atmospheric gust layer — responds to WindState, not chassis speed or RPM.
#[derive(Resource)]
struct GustLayer {
    instance: Handle<AudioInstance>,
}

// ---------------------------------------------------------------------------
// Synthesis constants
// ---------------------------------------------------------------------------

const SAMPLE_RATE: u32 = 44_100;
const GUST_LOOP_N_FRAMES: usize = 44_100; // 1 s at 44100 Hz

// ---------------------------------------------------------------------------
// LCG noise helper
// ---------------------------------------------------------------------------

#[inline]
fn lcg_noise(seed: u32) -> f32 {
    let v = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    (v as f32 / 2_147_483_648.0) - 1.0
}

// ---------------------------------------------------------------------------
// Gust synthesis
// ---------------------------------------------------------------------------

/// Atmospheric gust: a single wind burst — dense pink-ish noise with a slow
/// 0.8 Hz amplitude swell to simulate a passing gust.
fn gust_sample(t: f32, i: u32) -> f32 {
    use std::f32::consts::PI;

    // Three-octave pink noise approximation.
    let n0 = lcg_noise(i);
    let n1 = lcg_noise(i / 2) * 0.5;
    let n2 = lcg_noise(i / 4) * 0.25;
    let pink = (n0 + n1 + n2) / 1.75;

    // Slow 0.8 Hz swell — the "gust" envelope.
    let swell = (t * 0.8 * 2.0 * PI).sin() * 0.3 + 0.7;

    let raw = pink * 0.6 * swell;
    let th = 0.65_f32;
    (raw / th).tanh() * th
}

// ---------------------------------------------------------------------------
// Helper: build AudioSource from a generator closure
// ---------------------------------------------------------------------------

fn build_looped_source(
    audio_sources: &mut Assets<AudioSource>,
    n_frames: usize,
    mut gen: impl FnMut(usize) -> f32,
) -> Handle<AudioSource> {
    let frames: Arc<[KiraFrame]> = (0..n_frames)
        .map(|i| KiraFrame::from_mono(gen(i)))
        .collect::<Vec<_>>()
        .into();

    let sound_data = StaticSoundData {
        sample_rate: SAMPLE_RATE,
        frames,
        settings: StaticSoundSettings::default(),
        slice: None,
    };

    audio_sources.add(AudioSource { sound: sound_data })
}

// ---------------------------------------------------------------------------
// Startup system
// ---------------------------------------------------------------------------

fn spawn_gust_layer(
    mut commands: Commands,
    audio_sources: Option<ResMut<Assets<AudioSource>>>,
    audio: Option<Res<Audio>>,
) {
    let (Some(mut audio_sources), Some(audio)) = (audio_sources, audio) else { return };

    let gust_handle = build_looped_source(&mut audio_sources, GUST_LOOP_N_FRAMES, |i| {
        let t = i as f32 / SAMPLE_RATE as f32;
        gust_sample(t, i as u32)
    });

    let gust_inst = audio
        .play(gust_handle)
        .looped()
        .with_volume(linear_to_db(0.0001))
        .with_playback_rate(1.0_f64)
        .handle();

    commands.insert_resource(GustLayer { instance: gust_inst });
}

// ---------------------------------------------------------------------------
// Update system
// ---------------------------------------------------------------------------

/// Modulate the atmospheric gust layer based on WindState::speed_mps.
///
///   - world_audio.rs wind responds to chassis speed (aerodynamic buffeting).
///   - This gust layer responds to environmental wind (ambient weather).
///
/// At calm (speed < 2 m/s): nearly silent.
/// At moderate (5 m/s): clearly audible background texture.
/// At strong (6.5 m/s max): ~40 % of master volume.
fn modulate_gust_layer(
    gust: Option<Res<GustLayer>>,
    mut audio_instances: ResMut<Assets<AudioInstance>>,
    wind: Option<Res<WindState>>,
    settings: Option<Res<SettingsState>>,
    quality: Option<Res<GraphicsQuality>>,
) {
    let Some(gust) = gust else { return };

    // On Low quality, mute the atmospheric gust layer entirely to save a voice.
    let tier = quality.map(|q| *q).unwrap_or(GraphicsQuality::High);
    if tier == GraphicsQuality::Low {
        let tween = AudioTween::linear(std::time::Duration::from_millis(200));
        if let Some(inst) = audio_instances.get_mut(&gust.instance) {
            inst.set_decibels(linear_to_db(1e-6), tween);
        }
        return;
    }

    let wind_speed = wind.map(|w| w.speed_mps).unwrap_or(3.0);
    let master     = settings.map(|s| s.master_volume).unwrap_or(0.7);

    // Volume: 0 at 1 m/s, ramps to 0.4 at 7 m/s.
    let vol_linear = ((wind_speed - 1.0) / 6.0 * 0.40 * master).clamp(0.0, 0.40);

    // Pitch: 0.85 at calm, 1.15 at strong wind — subtle howl shift.
    let rate = (0.85 + wind_speed / 50.0) as f64;

    let tween = AudioTween::linear(std::time::Duration::from_millis(120));

    if let Some(inst) = audio_instances.get_mut(&gust.instance) {
        inst.set_decibels(linear_to_db(vol_linear.max(1e-6)), tween.clone());
        inst.set_playback_rate(rate, tween);
    }
}

// ---------------------------------------------------------------------------
// Utility: linear amplitude to Decibels (floor at -60 dB)
// ---------------------------------------------------------------------------

#[inline]
fn linear_to_db(linear: f32) -> Decibels {
    let db = 20.0 * linear.max(1e-6).log10();
    Decibels(db.max(-60.0))
}
