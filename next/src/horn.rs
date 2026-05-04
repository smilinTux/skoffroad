// Procedural two-tone car horn — no audio assets required.
//
// Architecture:
//   Startup : generate 0.4 s of PCM via horn_sample(), build StaticSoundData,
//             register as AudioSource asset, store handle.
//   Update  : check N key each frame; if pressed and cooldown >= 0.15 s, play
//             the horn as a one-shot and reset the cooldown timer.

use bevy::prelude::*;
use bevy_kira_audio::{Audio, AudioControl, AudioSource};
use bevy_kira_audio::prelude::{Decibels, StaticSoundData, StaticSoundSettings, Frame as KiraFrame};
use std::sync::Arc;

// ---------------------------------------------------------------------------
// Public plugin
// ---------------------------------------------------------------------------

pub struct HornPlugin;

impl Plugin for HornPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HornCooldown>()
           .add_systems(Startup, spawn_horn_source)
           .add_systems(Update, play_horn);
    }
}

// ---------------------------------------------------------------------------
// Resources
// ---------------------------------------------------------------------------

#[derive(Resource)]
struct HornSource(Handle<AudioSource>);

// Tracks seconds since the horn last fired; starts at threshold so first press works.
#[derive(Resource, Default)]
struct HornCooldown(f32);

// ---------------------------------------------------------------------------
// Sample synthesis
// ---------------------------------------------------------------------------

const SAMPLE_RATE: u32    = 44_100;
const HORN_DURATION_S: f32 = 0.4;

/// Two-tone car horn: square waves at 440 Hz (A4) and 554 Hz (C#5, a major third).
/// Envelope: 5 ms linear attack, hold, 50 ms linear release from t = 0.35 s.
/// Final soft clip via tanh keeps peaks from clipping the output bus.
fn horn_sample(t: f32) -> f32 {
    use std::f32::consts::PI;

    let v440 = ((t * 440.0 * 2.0 * PI).sin().signum()) * 0.5;
    let v554 = ((t * 554.0 * 2.0 * PI).sin().signum()) * 0.4;
    let mix = v440 + v554;

    let env = if t < 0.005 {
        t / 0.005
    } else if t > 0.35 {
        ((0.4 - t) / 0.05).max(0.0)
    } else {
        1.0
    };

    let raw = mix * env;
    // Soft clip: normalise to 0.7 ceiling via tanh, then rescale back.
    (raw / 0.7).tanh() * 0.7
}

// ---------------------------------------------------------------------------
// Startup system
// ---------------------------------------------------------------------------

fn spawn_horn_source(
    mut commands: Commands,
    mut audio_sources: ResMut<Assets<AudioSource>>,
) {
    let n_frames = (SAMPLE_RATE as f32 * HORN_DURATION_S) as usize;

    let frames: Arc<[KiraFrame]> = (0..n_frames)
        .map(|i| {
            let t = i as f32 / SAMPLE_RATE as f32;
            KiraFrame::from_mono(horn_sample(t))
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
    commands.insert_resource(HornSource(handle));
}

// ---------------------------------------------------------------------------
// Update system: N key triggers one-shot playback
// ---------------------------------------------------------------------------

fn play_horn(
    horn_src: Option<Res<HornSource>>,
    audio: Res<Audio>,
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut cooldown: ResMut<HornCooldown>,
) {
    // Accumulate elapsed time so first-frame presses aren't blocked.
    cooldown.0 += time.delta_secs();

    let Some(horn_src) = horn_src else { return };

    if keys.just_pressed(KeyCode::KeyN) && cooldown.0 >= 0.15 {
        audio.play(horn_src.0.clone()).with_volume(linear_to_db(0.6));
        cooldown.0 = 0.0;
    }
}

// ---------------------------------------------------------------------------
// Utility
// ---------------------------------------------------------------------------

fn linear_to_db(linear: f32) -> Decibels {
    let db = 20.0 * linear.max(1e-6).log10();
    Decibels(db.max(-60.0))
}
