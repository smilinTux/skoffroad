// Engine audio synthesized at startup — no audio assets required.
//
// Architecture:
//   Startup : generate 1 s of PCM via engine_sample(), build StaticSoundData,
//             register as AudioSource asset, play looped, store instance handle.
//   Update  : read DriveInput + chassis LinearVelocity, recompute pitch/volume,
//             write to AudioInstance each frame.
//
// Headless / no-device environments: the bevy_kira_audio AudioPlugin is added
// only by main.rs (full game). The headless harness never adds our AudioPlugin,
// so none of this code runs in that context. If the audio device fails to open,
// kira silently no-ops rather than panicking.

use bevy::prelude::*;
use bevy_kira_audio::{Audio, AudioControl, AudioInstance, AudioSource, AudioTween};
// StaticSoundData / StaticSoundSettings / Frame / Decibels come from kira,
// which is bevy_kira_audio's public dependency — use its prelude's re-exports.
use bevy_kira_audio::prelude::{
    Decibels, StaticSoundData, StaticSoundSettings, Frame as KiraFrame,
};
use avian3d::prelude::LinearVelocity;
use std::sync::Arc;

use crate::vehicle::{Chassis, DriveInput};

// ---------------------------------------------------------------------------
// Public plugin
// ---------------------------------------------------------------------------

pub struct AudioPlugin;

impl Plugin for AudioPlugin {
    fn build(&self, app: &mut App) {
        // Register the kira audio backend. Our struct shadows the crate-level
        // name, so use the fully qualified path.
        app.add_plugins(bevy_kira_audio::AudioPlugin::default())
            .add_systems(Startup, spawn_engine_audio)
            .add_systems(Update, modulate_engine_audio);
    }
}

// ---------------------------------------------------------------------------
// Internal resource
// ---------------------------------------------------------------------------

#[derive(Resource)]
struct EngineAudio {
    instance: Handle<AudioInstance>,
}

// ---------------------------------------------------------------------------
// Sample synthesis
// ---------------------------------------------------------------------------

const SAMPLE_RATE: u32 = 44_100;
const BASE_HZ: f32     = 80.0;  // fundamental frequency at idle RPM
const DURATION_S: f32  = 1.0;

/// One audio sample at time `t` (seconds) for a given fundamental `rpm_hz`.
/// Mixes sawtooth + octave harmonic + sub-bass, with a combustion-cycle
/// envelope and soft clip.
fn engine_sample(t: f32, rpm_hz: f32) -> f32 {
    use std::f32::consts::PI;

    // Sawtooth at fundamental
    let saw = (t * rpm_hz).fract() * 2.0 - 1.0;

    // One octave up, lower amplitude — adds 2nd harmonic bite
    let harm = ((t * rpm_hz * 2.0).fract() * 2.0 - 1.0) * (1.0 / 3.0);

    // Sub-bass sine — body/rumble below the fundamental
    let sub = (t * rpm_hz * 0.5 * 2.0 * PI).sin() * 0.4;

    // Combustion-cycle envelope: sin^2 window per cycle softens each stroke
    let envelope = (t * rpm_hz * 2.0 * PI).sin().powi(2);

    let raw = (saw + harm + sub) * envelope;

    // Soft clip at ±0.7 via tanh-approximation: tanh(x / 0.7) * 0.7
    let threshold = 0.7_f32;
    (raw / threshold).tanh() * threshold
}

// ---------------------------------------------------------------------------
// Startup system: generate PCM and begin looped playback
// ---------------------------------------------------------------------------

fn spawn_engine_audio(
    mut commands: Commands,
    mut audio_sources: ResMut<Assets<AudioSource>>,
    audio: Res<Audio>,
) {
    let n_frames = (SAMPLE_RATE as f32 * DURATION_S) as usize;
    let frames: Arc<[KiraFrame]> = (0..n_frames)
        .map(|i| {
            let t = i as f32 / SAMPLE_RATE as f32;
            let s = engine_sample(t, BASE_HZ);
            KiraFrame::from_mono(s)
        })
        .collect::<Vec<_>>()
        .into();

    let sound_data = StaticSoundData {
        sample_rate: SAMPLE_RATE,
        frames,
        settings: StaticSoundSettings::default(),
        slice: None,
    };

    let source_handle = audio_sources.add(AudioSource { sound: sound_data });

    // Play looped at idle settings; the Update system adjusts pitch/volume each frame.
    let instance = audio
        .play(source_handle)
        .looped()
        .with_volume(linear_to_db(0.2))
        .with_playback_rate(1.0_f64)
        .handle();

    commands.insert_resource(EngineAudio { instance });
}

// ---------------------------------------------------------------------------
// Per-frame modulation: pitch tracks speed, volume tracks throttle
// ---------------------------------------------------------------------------

fn modulate_engine_audio(
    engine: Option<Res<EngineAudio>>,
    mut audio_instances: ResMut<Assets<AudioInstance>>,
    drive: Res<DriveInput>,
    chassis_q: Query<&LinearVelocity, With<Chassis>>,
) {
    let Some(engine) = engine else { return };

    // Speed from chassis linear velocity (m/s)
    let speed_mps = chassis_q
        .single()
        .map(|lv| Vec3::new(lv.x, lv.y, lv.z).length())
        .unwrap_or(0.0);

    // Pitch: 1× at idle, up to ~4× at 24 m/s
    let playback_rate = (1.0 + speed_mps / 8.0) as f64;

    // Volume: base 0.2 plus throttle contribution, clamped
    let volume_linear = (0.2 + 0.4 * drive.drive.abs()).clamp(0.0, 1.0);

    if let Some(instance) = audio_instances.get_mut(&engine.instance) {
        instance.set_playback_rate(playback_rate, AudioTween::default());
        instance.set_decibels(linear_to_db(volume_linear), AudioTween::default());
    }
}

// ---------------------------------------------------------------------------
// Utility: linear amplitude (0..1) to decibels
// ---------------------------------------------------------------------------

fn linear_to_db(linear: f32) -> Decibels {
    // -60 dB is kira's silence floor; avoid log(0).
    let db = 20.0 * linear.max(1e-6).log10();
    Decibels(db.max(-60.0))
}
