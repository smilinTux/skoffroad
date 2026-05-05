// Procedural ambient music — no audio assets required.
//
// A 30-second loop is synthesized at Startup from three layers:
//   Pad    — triadic root/3rd/5th sine waves (A2/C#3/E3 = 110/138/165 Hz)
//   Arpeggio — melodic line stepping through the same triad at 1 note/s
//   Noise   — very low-amplitude pink-ish wash for texture
// The loop plays continuously; pitch/volume are modulated by MusicState.
//
// Toggle music on/off with the U key.

use bevy::prelude::*;
use bevy_kira_audio::{Audio, AudioControl, AudioInstance, AudioSource, AudioTween};
use bevy_kira_audio::prelude::{
    Decibels, StaticSoundData, StaticSoundSettings, Frame as KiraFrame,
};
use std::sync::Arc;

use crate::menu::MenuState;
use crate::course::CourseState;

// ---------------------------------------------------------------------------
// Public plugin
// ---------------------------------------------------------------------------

pub struct MusicPlugin;

impl Plugin for MusicPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MusicState>()
           .init_resource::<MusicEnabled>()
           .add_systems(Startup, spawn_music_track)
           .add_systems(Update, (update_music_state, toggle_music));
    }
}

// ---------------------------------------------------------------------------
// Public resources
// ---------------------------------------------------------------------------

#[derive(Resource, Default, PartialEq, Eq, Clone, Copy)]
pub enum MusicState {
    #[default]
    Title,        // title screen up
    FreeRoam,     // playing, no race active
    RaceActive,   // course timer running
    RaceComplete, // course completed
}

#[derive(Resource)]
pub struct MusicEnabled(pub bool);

impl Default for MusicEnabled {
    fn default() -> Self {
        Self(true)
    }
}

// ---------------------------------------------------------------------------
// Internal resource
// ---------------------------------------------------------------------------

#[derive(Resource)]
struct MusicTrack {
    instance: Handle<AudioInstance>,
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const SAMPLE_RATE: u32 = 44_100;
const LOOP_SECS: f32   = 30.0;

// Triad: A2, C#3, E3
const ROOT_HZ: f32  = 110.0;
const THIRD_HZ: f32 = 138.59; // C#3
const FIFTH_HZ: f32 = 164.81; // E3

const ARP_NOTES: [f32; 3] = [ROOT_HZ, THIRD_HZ, FIFTH_HZ];

// ---------------------------------------------------------------------------
// Sample synthesis
// ---------------------------------------------------------------------------

/// Deterministic LCG noise — identical to the pattern in audio.rs.
#[inline]
fn lcg_noise(seed: u32) -> f32 {
    let v = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    (v as f32 / 2_147_483_648.0) - 1.0
}

/// Soft-clip via tanh.
#[inline]
fn soft_clip(x: f32, threshold: f32) -> f32 {
    (x / threshold).tanh() * threshold
}

/// Pad layer: root + major-3rd + perfect-5th sine waves with phase offsets
/// to avoid exact cancellation and to widen the stereo image in mono.
fn pad_sample(t: f32) -> f32 {
    use std::f32::consts::TAU;
    let r  = (t * ROOT_HZ  * TAU).sin();
    let th = (t * THIRD_HZ * TAU + 0.3).sin(); // phase offset
    let fi = (t * FIFTH_HZ * TAU + 0.7).sin(); // phase offset
    (r + th + fi) / 3.0 * 0.3 // mix weight 0.3 per note, then pad gain
}

/// Arpeggio layer: one note per second, cycling through ROOT/THIRD/FIFTH.
/// Each note has a short ADSR: attack 30 ms, hold 200 ms, decay 800 ms.
fn arp_sample(t: f32) -> f32 {
    use std::f32::consts::TAU;

    let note_idx = (t as usize) % ARP_NOTES.len();
    let hz = ARP_NOTES[note_idx];

    // Time within the current 1-second window.
    let local_t = t - t.floor();

    // Envelope: attack 30 ms → hold 200 ms → decay 800 ms → silence
    let attack_end = 0.030_f32;
    let hold_end   = 0.230_f32; // attack + hold
    let decay_end  = 1.030_f32; // attack + hold + decay (clamped to 1 s window)

    let env = if local_t < attack_end {
        local_t / attack_end
    } else if local_t < hold_end {
        1.0
    } else if local_t < decay_end {
        1.0 - (local_t - hold_end) / (decay_end - hold_end)
    } else {
        0.0
    };

    (t * hz * TAU).sin() * env * 0.4
}

/// Noise wash: pink-ish (three octave-spaced generators) at very low amplitude.
fn noise_sample(idx: u32) -> f32 {
    let n0 = lcg_noise(idx);
    let n1 = lcg_noise(idx / 2) * 0.5;
    let n2 = lcg_noise(idx / 4) * 0.25;
    (n0 + n1 + n2) / 1.75 * 0.05
}

/// Full music sample: mix pad + arp + noise, then soft-clip.
fn music_sample(t: f32, idx: u32) -> f32 {
    let mix = pad_sample(t) + arp_sample(t) + noise_sample(idx);
    soft_clip(mix, 0.8)
}

// ---------------------------------------------------------------------------
// Utility
// ---------------------------------------------------------------------------

fn linear_to_db(linear: f32) -> Decibels {
    let db = 20.0 * linear.max(1e-6).log10();
    Decibels(db.max(-60.0))
}

// ---------------------------------------------------------------------------
// Startup: generate PCM and begin looped playback
// ---------------------------------------------------------------------------

fn spawn_music_track(
    mut commands: Commands,
    mut audio_sources: ResMut<Assets<AudioSource>>,
    audio: Res<Audio>,
) {
    let n_frames = (SAMPLE_RATE as f32 * LOOP_SECS) as usize;

    let frames: Arc<[KiraFrame]> = (0..n_frames)
        .map(|i| {
            let t = i as f32 / SAMPLE_RATE as f32;
            KiraFrame::from_mono(music_sample(t, i as u32))
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

    // Start at Title defaults: volume 0.6, rate 0.85.
    let instance = audio
        .play(source_handle)
        .looped()
        .with_volume(linear_to_db(0.6))
        .with_playback_rate(0.85_f64)
        .handle();

    commands.insert_resource(MusicTrack { instance });
}

// ---------------------------------------------------------------------------
// State machine: derive MusicState from menu + course each frame
// ---------------------------------------------------------------------------

fn update_music_state(
    menu: Option<Res<MenuState>>,
    course: Option<Res<CourseState>>,
    mut music_state: ResMut<MusicState>,
    music_enabled: Res<MusicEnabled>,
    track: Option<Res<MusicTrack>>,
    mut audio_instances: ResMut<Assets<AudioInstance>>,
) {
    let Some(track) = track else { return };

    // Derive target state.
    let dismissed = menu.map_or(false, |m| m.dismissed);
    let target = if !dismissed {
        MusicState::Title
    } else if let Some(cs) = course {
        if cs.completed {
            MusicState::RaceComplete
        } else if cs.current_index >= 1 && !cs.completed {
            MusicState::RaceActive
        } else {
            MusicState::FreeRoam
        }
    } else {
        MusicState::FreeRoam
    };

    if target == *music_state {
        return;
    }

    *music_state = target;

    // Choose volume + rate for the new state.
    let (vol_linear, rate) = match target {
        MusicState::Title        => (0.6_f32, 0.85_f64),
        MusicState::FreeRoam     => (0.4,     1.00),
        MusicState::RaceActive   => (0.5,     1.15),
        MusicState::RaceComplete => (0.7,     1.05),
    };

    // Respect the mute toggle.
    let effective_vol = if music_enabled.0 { vol_linear } else { 0.0001 };

    let tween = AudioTween::linear(std::time::Duration::from_secs(1));

    if let Some(instance) = audio_instances.get_mut(&track.instance) {
        instance.set_decibels(linear_to_db(effective_vol), tween.clone());
        instance.set_playback_rate(rate, tween);
    }
}

// ---------------------------------------------------------------------------
// Toggle: U key mutes / unmutes music
// ---------------------------------------------------------------------------

fn toggle_music(
    keys: Res<ButtonInput<KeyCode>>,
    music_state: Res<MusicState>,
    mut music_enabled: ResMut<MusicEnabled>,
    track: Option<Res<MusicTrack>>,
    mut audio_instances: ResMut<Assets<AudioInstance>>,
) {
    if !keys.just_pressed(KeyCode::KeyU) {
        return;
    }

    let Some(track) = track else { return };

    music_enabled.0 = !music_enabled.0;

    let vol_linear = if music_enabled.0 {
        match *music_state {
            MusicState::Title        => 0.6,
            MusicState::FreeRoam     => 0.4,
            MusicState::RaceActive   => 0.5,
            MusicState::RaceComplete => 0.7,
        }
    } else {
        0.0001
    };

    let tween = AudioTween::linear(std::time::Duration::from_millis(300));

    if let Some(instance) = audio_instances.get_mut(&track.instance) {
        instance.set_decibels(linear_to_db(vol_linear), tween);
    }
}
