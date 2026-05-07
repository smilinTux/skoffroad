// Crash audio: synthesized mid-frequency noise burst on each HardImpact event.
//
// Architecture:
//   Startup : synthesize a 0.5 s crash WAV in memory, register as AudioSource
//             asset, store handle in CrashAudio resource.
//   Update  : detect_impact_play_crash — walks EventLog for new HardImpact
//             events and plays the crash one-shot for each, with a slight
//             per-impact pitch variation via lcg_noise for variety.
//
// All audio is fully procedural — no audio asset files required.

use bevy::prelude::*;
use bevy_kira_audio::{Audio, AudioControl, AudioSource};
use bevy_kira_audio::prelude::{StaticSoundData, StaticSoundSettings, Frame as KiraFrame};
use std::sync::Arc;

use crate::events::{EventLog, GameEvent};

// ---------------------------------------------------------------------------
// Public plugin
// ---------------------------------------------------------------------------

pub struct CrashAudioPlugin;

impl Plugin for CrashAudioPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_crash_audio)
           .add_systems(Update, detect_impact_play_crash);
    }
}

// ---------------------------------------------------------------------------
// Resource
// ---------------------------------------------------------------------------

/// Holds the pre-synthesized crash sound source handle for one-shot playback.
#[derive(Resource)]
pub struct CrashAudio {
    pub source: Handle<AudioSource>,
}

// ---------------------------------------------------------------------------
// Synthesis constants
// ---------------------------------------------------------------------------

const SAMPLE_RATE: u32     = 44_100;
const CRASH_DURATION_S: f32 = 0.5;

// ---------------------------------------------------------------------------
// LCG noise — deterministic broadband noise, no rand crate needed.
// Maps a sample index to a float in [-1, 1].
// ---------------------------------------------------------------------------

#[inline]
fn lcg_noise(seed: u32) -> f32 {
    let v = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    (v as f32 / 2_147_483_648.0) - 1.0
}

// ---------------------------------------------------------------------------
// Crash sample synthesis
// ---------------------------------------------------------------------------

/// One crash audio sample at time `t` (seconds) and sample index `i`.
///
/// Mixes broadband LCG noise with a low-mid body sine (~80 Hz), shaped by an
/// exponential decay envelope over 0.5 s. The result is a sharp transient
/// that fades quickly — characteristic of a collision.
fn crash_sample(t: f32, i: u32) -> f32 {
    use std::f32::consts::PI;

    // Broadband noise component.
    let noise = lcg_noise(i);

    // Low-mid body resonance sine — adds a tonal "thud" quality.
    let body = (t * 80.0 * 2.0 * PI).sin();

    // Exponential decay: ~-60 dB by the end of the 0.5 s window.
    let envelope = (-t * 6.0).exp();

    (noise * 0.6 + body * 0.4) * envelope
}

// ---------------------------------------------------------------------------
// Startup: synthesize PCM, register asset, store handle
// ---------------------------------------------------------------------------

fn spawn_crash_audio(
    mut commands: Commands,
    mut audio_sources: ResMut<Assets<AudioSource>>,
) {
    let n_frames = (SAMPLE_RATE as f32 * CRASH_DURATION_S) as usize;

    let frames: Arc<[KiraFrame]> = (0..n_frames)
        .map(|i| {
            let t = i as f32 / SAMPLE_RATE as f32;
            KiraFrame::from_mono(crash_sample(t, i as u32))
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
    commands.insert_resource(CrashAudio { source: handle });
}

// ---------------------------------------------------------------------------
// Update: detect new HardImpact events and play crash one-shot
// ---------------------------------------------------------------------------

/// Play the crash sound once per new HardImpact event.
///
/// Uses a `Local<f32>` timestamp watermark (same pattern as mixer.rs) to track
/// which events have already been handled. Applies a slight per-impact pitch
/// variation (±10 %) using lcg_noise seeded by the fractional part of the
/// event timestamp, so each hit sounds subtly different.
fn detect_impact_play_crash(
    crash_audio: Option<Res<CrashAudio>>,
    event_log: Option<Res<EventLog>>,
    audio: Res<Audio>,
    mut last_seen: Local<f32>,
) {
    let (Some(crash_audio), Some(event_log)) = (crash_audio, event_log) else {
        return;
    };

    let mut newest_ts = *last_seen;

    for (ts, ev) in &event_log.events {
        if *ts <= *last_seen {
            continue;
        }
        // Track the newest timestamp seen this frame regardless of event type.
        if *ts > newest_ts {
            newest_ts = *ts;
        }

        if let GameEvent::HardImpact { .. } = ev {
            // Derive a per-impact seed from the timestamp's sub-second bits.
            // Multiplying by a large prime spreads the fractional part into u32
            // space so different impacts get distinct noise values.
            let seed = (ts.fract() * 1_000_000.0) as u32;
            // Map lcg_noise [-1, 1] → playback rate [0.9, 1.1] for variety.
            let rate = 1.0 + lcg_noise(seed) * 0.1;

            audio
                .play(crash_audio.source.clone())
                .with_playback_rate(rate as f64);
        }
    }

    *last_seen = newest_ts;
}
