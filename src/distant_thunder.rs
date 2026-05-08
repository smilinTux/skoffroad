// Distant thunder: synthesized low rumble that plays each time storm.rs's
// lightning flashes. No audio asset files required.
//
// Architecture:
//   Startup : synthesize 2.5 s of PCM rumble via thunder_sample(), build
//             StaticSoundData, register as AudioSource asset, store handle in
//             ThunderAudio resource.
//   Update  : detect_flash_play_thunder — watches StormState::flash_alpha for
//             a rising edge (prev < 0.5 → cur > 0.7) and plays the rumble
//             one-shot on each flash.
//
// Public API:
//   DistantThunderPlugin

use bevy::prelude::*;
use bevy_kira_audio::{Audio, AudioControl, AudioSource};
use bevy_kira_audio::prelude::{StaticSoundData, StaticSoundSettings, Frame as KiraFrame};
use std::sync::Arc;

use crate::storm::StormState;

// ---------------------------------------------------------------------------
// Public plugin
// ---------------------------------------------------------------------------

pub struct DistantThunderPlugin;

impl Plugin for DistantThunderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, synthesize_thunder)
           .add_systems(Update, detect_flash_play_thunder);
    }
}

// ---------------------------------------------------------------------------
// Internal resource
// ---------------------------------------------------------------------------

/// Holds the pre-synthesized thunder source handle for one-shot playback.
#[derive(Resource)]
pub struct ThunderAudio {
    pub handle: Handle<AudioSource>,
}

// ---------------------------------------------------------------------------
// Sample synthesis constants
// ---------------------------------------------------------------------------

const SAMPLE_RATE: u32  = 44_100;
const DURATION_S:  f32  = 2.5;

// ---------------------------------------------------------------------------
// Thunder sample generator
// ---------------------------------------------------------------------------

/// Generate one mono sample for the thunder rumble at sample index `i`.
///
/// Three layers mixed together:
///   low_noise  : decimated LCG noise (every 4th sample) — heavy sub-bass roar
///   mid        : decimated LCG noise (every 2nd sample) — mid-frequency body
///   boom       : 40 Hz sine — subwoofer thump
///
/// Envelope:
///   0 .. 0.1 s  : linear ramp 0 → 1
///   0.1 .. 0.6 s: sustain at 1
///   0.6 .. 2.5 s: exponential decay  exp(-2 * (t - 0.6))
fn thunder_sample(i: usize) -> f32 {
    use std::f32::consts::PI;

    let t = i as f32 / SAMPLE_RATE as f32;

    // --- Noise layers ---
    let low_noise = lcg_noise(i as u32 / 4);
    let mid       = lcg_noise(i as u32 / 2);

    // --- 40 Hz subwoofer sine ---
    let boom = (t * 40.0 * 2.0 * PI).sin() * 0.3;

    // --- Amplitude envelope ---
    let envelope = if t < 0.1 {
        t / 0.1                              // 0→1 ramp over first 100 ms
    } else if t < 0.6 {
        1.0                                  // full sustain
    } else {
        (-2.0 * (t - 0.6)).exp()             // exponential decay tail
    };

    // --- Mix & master ---
    (low_noise * 0.6 + mid * 0.3 + boom * 0.3) * envelope * 0.4
}

// ---------------------------------------------------------------------------
// LCG noise helper (no external deps)
// ---------------------------------------------------------------------------

/// Deterministic white noise via a classic 32-bit LCG.
/// Returns a value in [-1, 1] for a given unsigned seed.
#[inline]
fn lcg_noise(seed: u32) -> f32 {
    let v = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    (v as f32 / 2_147_483_648.0) - 1.0
}

// ---------------------------------------------------------------------------
// Startup: synthesize PCM and register asset
// ---------------------------------------------------------------------------

fn synthesize_thunder(
    mut commands:      Commands,
    mut audio_sources: ResMut<Assets<AudioSource>>,
) {
    let n_frames = (SAMPLE_RATE as f32 * DURATION_S) as usize;

    let frames: Arc<[KiraFrame]> = (0..n_frames)
        .map(|i| KiraFrame::from_mono(thunder_sample(i)))
        .collect::<Vec<_>>()
        .into();

    let sound_data = StaticSoundData {
        sample_rate: SAMPLE_RATE,
        frames,
        settings: StaticSoundSettings::default(),
        slice: None,
    };

    let handle = audio_sources.add(AudioSource { sound: sound_data });
    commands.insert_resource(ThunderAudio { handle });
}

// ---------------------------------------------------------------------------
// Update: detect flash rising edge and play one-shot thunder
// ---------------------------------------------------------------------------

/// Watch `StormState::flash_alpha` for a rising edge:
///   previous frame < 0.5  AND  current frame > 0.7  → new lightning flash.
///
/// `StormState` is read as `Option<Res<StormState>>` so this system is safe
/// to run even when the StormPlugin is not loaded.
fn detect_flash_play_thunder(
    storm:       Option<Res<StormState>>,
    thunder:     Option<Res<ThunderAudio>>,
    audio:       Res<Audio>,
    mut last_alpha: Local<f32>,
) {
    let (Some(storm), Some(thunder)) = (storm, thunder) else {
        return;
    };

    let current = storm.flash_alpha;
    let previous = *last_alpha;

    if previous < 0.5 && current > 0.7 {
        // Rising edge: a new lightning flash just fired — play the rumble.
        audio.play(thunder.handle.clone());
    }

    *last_alpha = current;
}
