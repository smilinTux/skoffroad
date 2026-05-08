// 4-cylinder firing-pulse engine audio layer.
//
// Architecture:
//   Startup : generate 1 s of PCM via engine_pro_sample() at idle RPM,
//             build StaticSoundData, register as AudioSource asset, play looped,
//             store instance handle.
//   Update  : read DriveInput + chassis LinearVelocity, recompute pitch/volume,
//             write to AudioInstance each frame (more aggressive mapping than
//             audio.rs to give a "race-feel" layer on top of the base engine).
//
// Headless note: like audio.rs, nothing here runs in headless mode because
// the kira AudioPlugin is only added by main.rs.

use bevy::prelude::*;
use bevy_kira_audio::{Audio, AudioControl, AudioInstance, AudioSource, AudioTween};
use bevy_kira_audio::prelude::{
    Decibels, StaticSoundData, StaticSoundSettings, Frame as KiraFrame,
};
use avian3d::prelude::LinearVelocity;
use std::sync::Arc;

use crate::vehicle::{Chassis, DriveInput};

// ---------------------------------------------------------------------------
// Public plugin
// ---------------------------------------------------------------------------

pub struct EngineProPlugin;

impl Plugin for EngineProPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_engine_pro_audio)
           .add_systems(Update, modulate_engine_pro_audio);
    }
}

// ---------------------------------------------------------------------------
// Resource
// ---------------------------------------------------------------------------

#[derive(Resource)]
struct EnginePro {
    instance: Handle<AudioInstance>,
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const SAMPLE_RATE: u32  = 44_100;
const DURATION_S: f32   = 1.0;

/// Idle RPM converted to Hz: 800 RPM / 60 = 13.33 Hz fundamental.
const IDLE_RPM_HZ: f32  = 800.0 / 60.0;

// ---------------------------------------------------------------------------
// 4-cylinder firing-pulse synthesis
// ---------------------------------------------------------------------------

/// Generate one audio sample at time `t` seconds for a 4-cylinder engine
/// spinning at `rpm_hz` Hz (revolutions per second).
///
/// A 4-stroke 4-cylinder fires 2 times per revolution (each cylinder fires
/// once every 2 revolutions; 4 / 2 = 2 firings/rev).  The per-firing
/// waveform has:
///   - 0..5 % of period  : sharp linear attack (transient "chuff")
///   - 5..35 %           : combustion body — 60 Hz sine shaped by a half-sine
///                         amplitude envelope (the actual cylinder pressure)
///   - 35..85 %          : exponential decay of the resonating exhaust pulse
///   - 85..100 %         : sub-bass ring between firings (exhaust pipe resonance)
///
/// Everything is soft-clipped via tanh to keep peaks inside ±0.7.
fn engine_pro_sample(t: f32, rpm_hz: f32) -> f32 {
    use std::f32::consts::PI;

    // 2 firings per rev for a 4-cyl engine.
    let firings_per_sec = rpm_hz * 2.0;
    let phase_in_firing = (t * firings_per_sec).fract();

    let pulse = if phase_in_firing < 0.05 {
        // Attack: sharp linear ramp up to 1.0 over 5 % of the firing period.
        phase_in_firing / 0.05
    } else if phase_in_firing < 0.35 {
        // Body: 60 Hz sine, amplitude shaped by a half-sine envelope over the
        // 30 % body window so it rises and falls smoothly (no click at edges).
        let body_t = (phase_in_firing - 0.05) / 0.30;
        let env = (body_t * PI).sin();
        (t * 60.0 * 2.0 * PI).sin() * 0.6 * env
    } else if phase_in_firing < 0.85 {
        // Decay: quadratic fall from 0.4 to 0 over 50 % of the firing period.
        let decay_t = (phase_in_firing - 0.35) / 0.50;
        (1.0 - decay_t).powi(2) * 0.4
    } else {
        // Sub-bass ring between pulses: exhaust pipe resonance at 30 Hz.
        (t * 30.0 * 2.0 * PI).sin() * 0.15
    };

    // Soft clip: tanh(x / 0.7) * 0.7 keeps the signal inside ±0.7 with gentle
    // saturation rather than hard limiting.
    (pulse / 0.7).tanh() * 0.7
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build an `AudioSource` asset from a per-sample generator closure.
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

/// Linear amplitude (0..1) to decibels, floored at -60 dB (kira silence).
fn linear_to_db(linear: f32) -> Decibels {
    let db = 20.0 * linear.max(1e-6_f32).log10();
    Decibels(db.max(-60.0))
}

// ---------------------------------------------------------------------------
// Startup system
// ---------------------------------------------------------------------------

fn spawn_engine_pro_audio(
    mut commands: Commands,
    mut audio_sources: ResMut<Assets<AudioSource>>,
    audio: Res<Audio>,
) {
    let n_frames = (SAMPLE_RATE as f32 * DURATION_S) as usize;

    // Generate 1 second of PCM at idle RPM; the Update system pitch-shifts it
    // to match actual speed every frame, so only the waveform shape matters here.
    let source_handle = build_looped_source(&mut audio_sources, n_frames, |i| {
        let t = i as f32 / SAMPLE_RATE as f32;
        engine_pro_sample(t, IDLE_RPM_HZ)
    });

    // Start looped at a low volume; modulate_engine_pro_audio adjusts it live.
    let instance = audio
        .play(source_handle)
        .looped()
        .with_volume(linear_to_db(0.05))
        .with_playback_rate(1.0_f64)
        .handle();

    commands.insert_resource(EnginePro { instance });
}

// ---------------------------------------------------------------------------
// Per-frame modulation
// ---------------------------------------------------------------------------

/// Adjust playback rate (pitch) and volume of the engine-pro layer each frame.
///
/// Pitch: more aggressive than audio.rs (/6.0 vs /8.0) — the firing-pulse
/// layer should feel "racier" and tracks RPM changes faster.
///
/// Volume: quieter at idle (0.05) so it sits beneath the base engine layer and
/// blends in as a distinct chuffing texture rather than dominating.
fn modulate_engine_pro_audio(
    engine_pro: Option<Res<EnginePro>>,
    mut audio_instances: ResMut<Assets<AudioInstance>>,
    drive: Res<DriveInput>,
    chassis_q: Query<&LinearVelocity, With<Chassis>>,
) {
    let Some(engine_pro) = engine_pro else { return };

    // Ground-speed from chassis linear velocity (m/s).
    let speed_mps = chassis_q
        .single()
        .map(|lv| Vec3::new(lv.x, lv.y, lv.z).length())
        .unwrap_or(0.0);

    // Playback rate: 1× at idle, up to ~5× at 24 m/s — steeper curve gives
    // the firing-pulse layer a more responsive feel as the vehicle accelerates.
    let playback_rate = (1.0 + speed_mps / 6.0) as f64;

    // Volume: quieter idle than the base layer; rises sharply with throttle.
    let volume_linear = (0.05 + 0.35 * drive.drive.abs()).clamp(0.0, 1.0);

    if let Some(instance) = audio_instances.get_mut(&engine_pro.instance) {
        instance.set_playback_rate(playback_rate, AudioTween::default());
        instance.set_decibels(linear_to_db(volume_linear), AudioTween::default());
    }
}
