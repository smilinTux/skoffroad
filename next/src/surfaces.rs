// Surface-aware tire audio — one looped track per terrain type (grass/dirt/rock).
//
// Architecture:
//   Startup : synthesise three 1-second PCM loops, register as AudioSource assets,
//             start all three playing at volume 0, store instance handles.
//   Update  : for each grounded wheel compute slope via finite-difference of
//             terrain_height_at(), bucket into grass / dirt / rock, normalise to
//             blend weights, scale by vehicle speed, write volumes + playback rates.
//
// Headless: AudioPlugin is never added in the headless harness, so Audio / Assets
// <AudioSource> are absent. All systems guard with Option<Res<…>> and return early.

use bevy::prelude::*;
use bevy_kira_audio::{Audio, AudioControl, AudioInstance, AudioSource, AudioTween};
use bevy_kira_audio::prelude::{Decibels, StaticSoundData, StaticSoundSettings, Frame as KiraFrame};
use avian3d::prelude::LinearVelocity;
use std::sync::Arc;

use crate::terrain::terrain_height_at;
use crate::vehicle::{Chassis, Wheel, VehicleRoot};

// ---------------------------------------------------------------------------
// Public plugin
// ---------------------------------------------------------------------------

pub struct SurfacesPlugin;

impl Plugin for SurfacesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_surface_loops)
           .add_systems(Update, modulate_surface_audio);
    }
}

// ---------------------------------------------------------------------------
// Resource
// ---------------------------------------------------------------------------

#[derive(Resource)]
struct SurfaceAudio {
    grass: Handle<AudioInstance>,
    dirt:  Handle<AudioInstance>,
    rock:  Handle<AudioInstance>,
}

// ---------------------------------------------------------------------------
// Sample synthesis
// ---------------------------------------------------------------------------

const SAMPLE_RATE: u32 = 44_100;
const DURATION_S:  f32 = 1.0;

/// Deterministic LCG white noise — identical algorithm to audio.rs so the two
/// modules are consistent.  `seed` is the sample index; each call with the same
/// index always returns the same value (stable buffer generation).
#[inline]
fn lcg_noise(seed: u32) -> f32 {
    let v = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    (v as f32 / 2_147_483_648.0) - 1.0
}

/// Grass — hissy high-passed noise: swish / crunch of blades against the tire.
fn grass_sample(i: u32) -> f32 {
    // High-pass approximation: subtract a fraction of a slightly earlier sample.
    // With integer indices the "0.0001 s" offset is about 4 samples at 44 100 Hz.
    let noise      = lcg_noise(i);
    let high_passed = noise - 0.2 * lcg_noise(i.wrapping_sub(4));
    high_passed * 0.5
}

/// Dirt — low-frequency rumble mixed with a 80 Hz band tone.
fn dirt_sample(i: u32, t: f32) -> f32 {
    // Decimate the index to get lower-frequency noise (every 3rd sample value).
    let low_noise = lcg_noise(i / 3);
    let band = ((t * 80.0 * 2.0 * std::f32::consts::PI).sin()) * 0.3;
    (low_noise * 0.6 + band * 0.4) * 0.5
}

/// Rock — pebbly clack: sparse clicks at 40 Hz rate layered over broadband noise.
fn rock_sample(i: u32, t: f32) -> f32 {
    let click_phase = (t * 40.0).fract();
    let click = if click_phase < 0.05 { 1.0_f32 } else { 0.0_f32 };
    let noise = lcg_noise(i);
    (click * 0.6 + noise * 0.3) * 0.4
}

// ---------------------------------------------------------------------------
// Helper: build a looped AudioSource asset from a frame iterator
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

fn spawn_surface_loops(
    mut commands: Commands,
    audio_sources: Option<ResMut<Assets<AudioSource>>>,
    audio: Option<Res<Audio>>,
) {
    // Guard: headless harness never adds bevy_kira_audio::AudioPlugin.
    let (Some(mut audio_sources), Some(audio)) = (audio_sources, audio) else { return };

    let n_frames = (SAMPLE_RATE as f32 * DURATION_S) as usize;

    let grass_handle = build_looped_source(&mut audio_sources, n_frames, |i| {
        grass_sample(i as u32)
    });

    let dirt_handle = build_looped_source(&mut audio_sources, n_frames, |i| {
        let t = i as f32 / SAMPLE_RATE as f32;
        dirt_sample(i as u32, t)
    });

    let rock_handle = build_looped_source(&mut audio_sources, n_frames, |i| {
        let t = i as f32 / SAMPLE_RATE as f32;
        rock_sample(i as u32, t)
    });

    // All three tracks start silent; modulate_surface_audio raises volume each frame.
    let silence = linear_to_db(0.0001);

    let grass_inst = audio
        .play(grass_handle)
        .looped()
        .with_volume(silence)
        .with_playback_rate(1.0_f64)
        .handle();

    let dirt_inst = audio
        .play(dirt_handle)
        .looped()
        .with_volume(silence)
        .with_playback_rate(1.0_f64)
        .handle();

    let rock_inst = audio
        .play(rock_handle)
        .looped()
        .with_volume(silence)
        .with_playback_rate(1.0_f64)
        .handle();

    commands.insert_resource(SurfaceAudio {
        grass: grass_inst,
        dirt:  dirt_inst,
        rock:  rock_inst,
    });
}

// ---------------------------------------------------------------------------
// Per-frame modulation
// ---------------------------------------------------------------------------

/// Finite-difference step used for slope estimation (metres).
const FD_STEP: f32 = 0.5;

/// Compute slope metric at world-space (x, z) from the terrain heightmap.
/// Returns a value in [0, 1]: 0 = perfectly flat, 1 = vertical.
fn slope_at(x: f32, z: f32) -> f32 {
    let h   = terrain_height_at(x, z);
    let hx  = terrain_height_at(x + FD_STEP, z);
    let hz  = terrain_height_at(x, z + FD_STEP);
    // Reconstruct surface normal from two finite-difference tangents.
    let tx  = Vec3::new(FD_STEP, hx - h, 0.0).normalize();
    let tz  = Vec3::new(0.0, hz - h, FD_STEP).normalize();
    let n   = tx.cross(tz).normalize();
    // slope = 0 on flat (normal == Y), 1 on vertical.
    1.0 - n.y.abs().clamp(0.0, 1.0)
}

/// Classify slope into surface bucket.
/// Thresholds mirror terrain.rs vertex-colour blending:
///   slope < 0.15  → grass
///   0.15..0.45    → dirt
///   >= 0.45       → rock
#[derive(Clone, Copy)]
enum Surface { Grass, Dirt, Rock }

fn classify(slope: f32) -> Surface {
    if slope < 0.15 {
        Surface::Grass
    } else if slope < 0.45 {
        Surface::Dirt
    } else {
        Surface::Rock
    }
}

fn modulate_surface_audio(
    surface_audio: Option<Res<SurfaceAudio>>,
    mut audio_instances: ResMut<Assets<AudioInstance>>,
    vehicle: Option<Res<VehicleRoot>>,
    chassis_q: Query<(&Transform, &LinearVelocity), With<Chassis>>,
    wheel_q: Query<(&Transform, &Wheel)>,
) {
    let Some(surface_audio) = surface_audio else { return };
    let Some(vehicle) = vehicle else { return };

    // Chassis world position and speed.
    let Ok((chassis_tf, lin_vel)) = chassis_q.get(vehicle.chassis) else { return };

    let speed_mps = Vec3::new(lin_vel.x, lin_vel.y, lin_vel.z).length();

    // Count grounded wheels per surface bucket.
    let mut counts = [0u32; 3]; // [grass, dirt, rock]
    let mut grounded = 0u32;

    for (wheel_tf, wheel) in wheel_q.iter() {
        if !wheel.is_grounded { continue; }
        grounded += 1;

        // Wheel world position = chassis transform applied to wheel local transform.
        let world_pos = chassis_tf.transform_point(wheel_tf.translation);
        let slope = slope_at(world_pos.x, world_pos.z);
        match classify(slope) {
            Surface::Grass => counts[0] += 1,
            Surface::Dirt  => counts[1] += 1,
            Surface::Rock  => counts[2] += 1,
        }
    }

    // Normalise bucket counts to blend weights [0, 1].
    let total = grounded.max(1) as f32;
    let weights = [
        counts[0] as f32 / total,
        counts[1] as f32 / total,
        counts[2] as f32 / total,
    ];

    // Speed factor: mute at rest, full volume at 10 m/s.
    let vol_factor = (speed_mps / 10.0).clamp(0.0, 1.0);

    // Playback rate: 0.7 at rest, rises with speed.
    let playback_rate = (0.7 + speed_mps / 12.0) as f64;

    let tween = AudioTween::linear(std::time::Duration::from_millis(30));

    let handles = [
        (&surface_audio.grass, weights[0]),
        (&surface_audio.dirt,  weights[1]),
        (&surface_audio.rock,  weights[2]),
    ];

    for (handle, weight) in &handles {
        let vol_linear = (weight * vol_factor * 0.5).max(0.0001);
        if let Some(inst) = audio_instances.get_mut(*handle) {
            inst.set_decibels(linear_to_db(vol_linear), tween.clone());
            inst.set_playback_rate(playback_rate, AudioTween::default());
        }
    }
}

// ---------------------------------------------------------------------------
// Utility
// ---------------------------------------------------------------------------

fn linear_to_db(linear: f32) -> Decibels {
    let db = 20.0 * linear.max(1e-6).log10();
    Decibels(db.max(-60.0))
}
