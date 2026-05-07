// Fog horn: distant low-frequency lighthouse horn, plays one-shot every
// 30 seconds. Synthesized procedurally from low-frequency sine waves.
//
// Architecture:
//   Startup : synthesize 2.0 s of PCM (100 Hz fundamental + 50 Hz sub-octave
//             + 200 Hz overtone) with attack/sustain/decay envelope,
//             register as an AudioSource asset, store handle in FogHornAudio.
//   Update  : tick_fog_horn — Local<f32> timer accumulates delta-time;
//             first fire at t = 10 s (timer initialized to 20.0); thereafter
//             every 30 s.
//
// Public API:
//   FogHornPlugin

use bevy::prelude::*;
use bevy_kira_audio::{Audio, AudioControl, AudioSource};
use bevy_kira_audio::prelude::{Decibels, StaticSoundData, StaticSoundSettings, Frame as KiraFrame};
use std::sync::Arc;

// ---------------------------------------------------------------------------
// Public plugin
// ---------------------------------------------------------------------------

pub struct FogHornPlugin;

impl Plugin for FogHornPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_fog_horn)
           .add_systems(Update, tick_fog_horn);
    }
}

// ---------------------------------------------------------------------------
// Resource
// ---------------------------------------------------------------------------

#[derive(Resource)]
pub struct FogHornAudio {
    pub source: Handle<AudioSource>,
}

// ---------------------------------------------------------------------------
// Sample synthesis constants
// ---------------------------------------------------------------------------

const SAMPLE_RATE: u32     = 44_100;
const HORN_DURATION_S: f32 = 2.0;

// ---------------------------------------------------------------------------
// Horn sample synthesis
//
// Layers:
//   low_sine   — 100 Hz fundamental
//   low_octave — 50 Hz sub-octave (× 0.5)
//   sub        — 200 Hz overtone  (× 0.3)
//
// Envelope:
//   attack  : 0.00 – 0.15 s  linear ramp 0 → 1
//   sustain : 0.15 – 1.60 s  = 1
//   decay   : 1.60 – 2.00 s  linear ramp 1 → 0
// ---------------------------------------------------------------------------

fn fog_horn_sample(t: f32) -> f32 {
    use std::f32::consts::PI;

    let low_sine   = (t * 100.0 * 2.0 * PI).sin();
    let low_octave = (t * 50.0  * 2.0 * PI).sin() * 0.5;
    let sub        = (t * 200.0 * 2.0 * PI).sin() * 0.3;

    let envelope = if t < 0.15 {
        t / 0.15
    } else if t < 1.6 {
        1.0
    } else {
        // decay: 1.6 → 2.0 s
        ((HORN_DURATION_S - t) / (HORN_DURATION_S - 1.6)).max(0.0)
    };

    (low_sine * 0.5 + low_octave * 0.5 + sub * 0.3) * envelope * 0.5
}

// ---------------------------------------------------------------------------
// Startup: synthesize PCM and store handle
// ---------------------------------------------------------------------------

fn spawn_fog_horn(
    mut commands: Commands,
    mut audio_sources: ResMut<Assets<AudioSource>>,
) {
    let n_frames = (SAMPLE_RATE as f32 * HORN_DURATION_S) as usize;

    let frames: Arc<[KiraFrame]> = (0..n_frames)
        .map(|i| {
            let t = i as f32 / SAMPLE_RATE as f32;
            KiraFrame::from_mono(fog_horn_sample(t))
        })
        .collect::<Vec<_>>()
        .into();

    let sound_data = StaticSoundData {
        sample_rate: SAMPLE_RATE,
        frames,
        settings: StaticSoundSettings::default(),
        slice: None,
    };

    let handle = audio_sources.add(AudioSource { sound: sound_data });
    commands.insert_resource(FogHornAudio { source: handle });
}

// ---------------------------------------------------------------------------
// Update: timer-based one-shot playback
//
// Local<f32>  — elapsed seconds since last fire.
// Local<bool> — whether the initial-offset has been applied.
//
// The timer is initialized to 20.0 so the first horn fires at t = 10 s
// of real play-time (20 + 10 = 30).  Subsequent fires happen every 30 s.
// ---------------------------------------------------------------------------

fn tick_fog_horn(
    fog_horn: Option<Res<FogHornAudio>>,
    audio: Res<Audio>,
    time: Res<Time>,
    mut timer: Local<f32>,
    mut initialized: Local<bool>,
) {
    // On the very first frame, pre-load the timer so the first horn fires
    // at ~10 s into the session instead of waiting a full 30 s.
    if !*initialized {
        *timer = 20.0;
        *initialized = true;
    }

    *timer += time.delta_secs();

    if *timer >= 30.0 {
        if let Some(fg) = fog_horn {
            audio.play(fg.source.clone()).with_volume(linear_to_db(0.4));
        }
        *timer = 0.0;
    }
}

// ---------------------------------------------------------------------------
// Utility: linear amplitude (0..1) → Decibels
// ---------------------------------------------------------------------------

#[inline]
fn linear_to_db(linear: f32) -> Decibels {
    let db = 20.0 * linear.max(1e-6).log10();
    Decibels(db.max(-60.0))
}
