// World ambient audio: wind hiss, birds (day), crickets (night).
//
// Architecture:
//   Startup : generate 1 s of PCM for each layer via the respective sample
//             functions, register as AudioSource assets, play looped, store
//             instance handles in WorldAudio resource.
//   Update  : read TimeOfDay + WindState + chassis LinearVelocity, recompute
//             per-layer volumes each frame, write to AudioInstance.
//
// All audio is fully procedural — no asset files required.

use bevy::prelude::*;
use bevy_kira_audio::{Audio, AudioControl, AudioInstance, AudioSource, AudioTween};
use bevy_kira_audio::prelude::{Decibels, StaticSoundData, StaticSoundSettings, Frame as KiraFrame};
use avian3d::prelude::LinearVelocity;
use std::sync::Arc;

use crate::sky::TimeOfDay;
use crate::vehicle::{Chassis, VehicleRoot};
use crate::wind::WindState;

// ---------------------------------------------------------------------------
// Public plugin
// ---------------------------------------------------------------------------

pub struct WorldAudioPlugin;

impl Plugin for WorldAudioPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_world_audio)
           .add_systems(Update, modulate_world_audio);
    }
}

// ---------------------------------------------------------------------------
// Resource
// ---------------------------------------------------------------------------

#[derive(Resource)]
struct WorldAudio {
    wind:     Handle<AudioInstance>,
    birds:    Handle<AudioInstance>,
    crickets: Handle<AudioInstance>,
}

// ---------------------------------------------------------------------------
// Sample synthesis constants
// ---------------------------------------------------------------------------

const SAMPLE_RATE: u32   = 44_100;
const DURATION_S:  f32   = 1.0;

// ---------------------------------------------------------------------------
// LCG noise — deterministic, no rand crate needed.
// Converts a sample index to a float in [-1, 1].
// ---------------------------------------------------------------------------

#[inline]
fn lcg_noise(seed: u32) -> f32 {
    let v = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    (v as f32 / 2_147_483_648.0) - 1.0
}

// ---------------------------------------------------------------------------
// Sample functions (each returns a float in roughly [-1, 1])
// ---------------------------------------------------------------------------

/// Hissy filtered noise; mid-frequency content.
fn wind_sample(t: f32) -> f32 {
    // Convert continuous time into LCG seeds using per-sample indices.
    let idx  = (t * SAMPLE_RATE as f32) as u32;
    let idx2 = ((t * 0.7) * SAMPLE_RATE as f32) as u32;
    let noise = lcg_noise(idx);
    let n2    = lcg_noise(idx2);
    (noise * 0.5 + n2 * 0.4 - 0.2) * 0.5
}

/// Sparse high-frequency chirps every ~0.6 s, phase-windowed sines.
fn birds_sample(t: f32) -> f32 {
    let chirp_phase  = (t * 1.6).fract();
    let chirp_active = if chirp_phase < 0.06 { 1.0_f32 } else { 0.0 };
    let chirp        = (t * 2400.0 * 2.0 * std::f32::consts::PI).sin() * chirp_active * 0.4;

    let chirp2_phase  = ((t + 0.4) * 1.5).fract();
    let chirp2_active = if chirp2_phase < 0.04 { 1.0_f32 } else { 0.0 };
    let chirp2        = (t * 3200.0 * 2.0 * std::f32::consts::PI).sin() * chirp2_active * 0.3;

    chirp + chirp2
}

/// Steady high-frequency stridulation ~4-6 kHz, rapidly gated.
fn crickets_sample(t: f32) -> f32 {
    let gate  = if (t * 50.0).fract() < 0.3 { 1.0_f32 } else { 0.0 };
    let trill = (t * 4500.0 * 2.0 * std::f32::consts::PI).sin() * gate * 0.3;
    let mid   = (t * 5500.0 * 2.0 * std::f32::consts::PI).sin() * gate * 0.2;
    trill + mid
}

// ---------------------------------------------------------------------------
// Helper: build a looped AudioSource from a sample-generating closure
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
// Startup: synthesize PCM and start looped playback
// ---------------------------------------------------------------------------

fn spawn_world_audio(
    mut commands: Commands,
    mut audio_sources: ResMut<Assets<AudioSource>>,
    audio: Res<Audio>,
) {
    let n_frames = (SAMPLE_RATE as f32 * DURATION_S) as usize;

    // Wind layer — starts at a low base volume; modulate_world_audio drives it.
    let wind_handle = build_looped_source(&mut audio_sources, n_frames, |i| {
        let t = i as f32 / SAMPLE_RATE as f32;
        wind_sample(t)
    });
    let wind_instance = audio
        .play(wind_handle)
        .looped()
        .with_volume(linear_to_db(0.05))
        .handle();

    // Birds layer — starts silent; raised during daytime.
    let birds_handle = build_looped_source(&mut audio_sources, n_frames, |i| {
        let t = i as f32 / SAMPLE_RATE as f32;
        birds_sample(t)
    });
    let birds_instance = audio
        .play(birds_handle)
        .looped()
        .with_volume(linear_to_db(0.0001))
        .handle();

    // Crickets layer — starts silent; raised at night.
    let crickets_handle = build_looped_source(&mut audio_sources, n_frames, |i| {
        let t = i as f32 / SAMPLE_RATE as f32;
        crickets_sample(t)
    });
    let crickets_instance = audio
        .play(crickets_handle)
        .looped()
        .with_volume(linear_to_db(0.0001))
        .handle();

    commands.insert_resource(WorldAudio {
        wind:     wind_instance,
        birds:    birds_instance,
        crickets: crickets_instance,
    });
}

// ---------------------------------------------------------------------------
// Per-frame modulation
// ---------------------------------------------------------------------------

fn modulate_world_audio(
    world_audio: Option<Res<WorldAudio>>,
    mut audio_instances: ResMut<Assets<AudioInstance>>,
    tod: Option<Res<TimeOfDay>>,
    wind: Option<Res<WindState>>,
    vehicle: Option<Res<VehicleRoot>>,
    chassis_q: Query<&LinearVelocity, With<Chassis>>,
) {
    let Some(world_audio) = world_audio else { return };

    // --- Day/night factor (1 = noon, 0 = midnight) -------------------------
    let day = if let Some(tod) = tod {
        let raw = ((tod.t - 0.25) * std::f32::consts::TAU).cos() * 0.5 + 0.5;
        raw.clamp(0.0, 1.0)
    } else {
        0.5 // default to daytime feel when resource absent
    };

    // --- Chassis speed (m/s) -----------------------------------------------
    let chassis_speed = if let Some(vehicle) = vehicle {
        chassis_q
            .get(vehicle.chassis)
            .map(|lv| Vec3::new(lv.x, lv.y, lv.z).length())
            .unwrap_or(0.0)
    } else {
        0.0
    };

    // --- Wind speed --------------------------------------------------------
    let wind_speed = wind.map(|w| w.speed_mps).unwrap_or(3.0);

    // --- Wind volume -------------------------------------------------------
    let wind_vol = ((chassis_speed / 25.0) + (wind_speed / 8.0) * 0.4)
        .clamp(0.05, 0.7);

    // --- Bird/cricket volumes: attenuate birds when wind is loud -----------
    // birds are quieter when wind noise would mask them
    let wind_cap    = (1.0 - wind_vol * 0.8).clamp(0.0, 1.0);
    let birds_vol   = day * 0.3 * wind_cap;
    let crickets_vol = (1.0 - day) * 0.3;

    // Apply volumes with a short tween to avoid zipper noise
    let tween = AudioTween::linear(std::time::Duration::from_millis(80));

    if let Some(inst) = audio_instances.get_mut(&world_audio.wind) {
        inst.set_decibels(linear_to_db(wind_vol.max(0.0001)), tween.clone());
    }
    if let Some(inst) = audio_instances.get_mut(&world_audio.birds) {
        inst.set_decibels(linear_to_db(birds_vol.max(0.0001)), tween.clone());
    }
    if let Some(inst) = audio_instances.get_mut(&world_audio.crickets) {
        inst.set_decibels(linear_to_db(crickets_vol.max(0.0001)), tween);
    }
}

// ---------------------------------------------------------------------------
// Utility: linear amplitude (0..1) to Decibels
// ---------------------------------------------------------------------------

#[inline]
fn linear_to_db(linear: f32) -> Decibels {
    let db = 20.0 * linear.max(1e-6).log10();
    Decibels(db.max(-60.0))
}
