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
use crate::events::{EventLog, GameEvent};

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
            .add_systems(Startup, spawn_skid_audio)
            .add_systems(Startup, spawn_wind_audio)
            .add_systems(Startup, spawn_thud_source)
            .add_systems(Update, modulate_engine_audio)
            .add_systems(Update, modulate_skid)
            .add_systems(Update, modulate_wind)
            .add_systems(Update, play_thud_on_impact);
    }
}

// ---------------------------------------------------------------------------
// Internal resources
// ---------------------------------------------------------------------------

#[derive(Resource)]
struct EngineAudio {
    instance: Handle<AudioInstance>,
}

#[derive(Resource)]
struct SkidAudio {
    instance: Handle<AudioInstance>,
}

#[derive(Resource)]
struct WindAudio {
    instance: Handle<AudioInstance>,
}

// Thud is one-shot; we keep the source handle and re-play as needed.
#[derive(Resource)]
struct ThudSource {
    handle: Handle<AudioSource>,
}

// ---------------------------------------------------------------------------
// Sample synthesis
// ---------------------------------------------------------------------------

const SAMPLE_RATE: u32 = 44_100;
const BASE_HZ: f32     = 80.0;  // fundamental frequency at idle RPM
const DURATION_S: f32  = 1.0;
const THUD_DURATION_S: f32 = 0.5;

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

/// White noise via a deterministic LCG — no rand crate needed.
/// `seed` should be the sample index so each call returns the same value for
/// the same index (stable buffer generation).
#[inline]
fn lcg_noise(seed: u32) -> f32 {
    // Classic 32-bit LCG parameters (Numerical Recipes).
    let v = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    // Map unsigned 32-bit to [-1, 1].
    (v as f32 / 2_147_483_648.0) - 1.0
}

/// Tire skid: filtered white noise with a gentle 200 Hz resonance.
///
/// Real skid is broadband friction noise coloured by tire/road resonance.
/// We approximate by mixing raw noise with a sine at a "squeal" partial
/// (200 Hz) gated by the same noise envelope, then soft-clip.
fn skid_sample(t: f32, idx: u32) -> f32 {
    use std::f32::consts::PI;

    let noise = lcg_noise(idx);

    // Gentle squeal partial: 200 Hz sine at low amplitude mixed into the noise.
    // This adds just enough tonal character without sounding like a pure tone.
    let squeal = (t * 200.0 * 2.0 * PI).sin() * 0.15;

    // Simple 1-pole low-pass emulation: blend noise and the delayed-by-one
    // approach is impractical here in a stateless generator, so instead we
    // tilt the spectrum by summing adjacent samples at ±1 index (FIR notch
    // on the Nyquist component, keeps the low-mid energy).
    let n_lo = lcg_noise(idx.wrapping_add(1));
    let filtered = noise * 0.6 + n_lo * 0.4;

    let raw = filtered * 0.8 + squeal;
    let threshold = 0.7_f32;
    (raw / threshold).tanh() * threshold
}

/// Impact thud: 50 ms exponential-decay sub-bass thump at ~45 Hz.
///
/// Models the body resonance of a vehicle hitting terrain: a short low-frequency
/// burst that decays fast. We use a decaying sine and a click transient layered.
fn thud_sample(t: f32) -> f32 {
    use std::f32::consts::PI;

    // Sub-bass fundamental at ~45 Hz.
    let body_hz = 45.0_f32;
    // Tight exponential decay: ~-60 dB in 50 ms.
    let decay = (-t * 80.0).exp();

    let thump = (t * body_hz * 2.0 * PI).sin() * decay;

    // Second partial at 90 Hz, lighter, for the mid punch.
    let mid_decay = (-t * 120.0).exp();
    let mid = (t * 90.0 * 2.0 * PI).sin() * mid_decay * 0.35;

    let raw = thump + mid;
    let threshold = 0.75_f32;
    (raw / threshold).tanh() * threshold
}

/// Ambient wind: pink-ish noise and low-frequency rumble.
///
/// Pink noise is approximated by summing three octave-spaced noise generators
/// with amplitudes 1, 0.5, 0.25 (each octave down = -6 dB, mimicking 1/f).
/// A slow 4 Hz rumble sine adds the "buffeting against bodywork" feel.
fn wind_sample(t: f32, idx: u32) -> f32 {
    use std::f32::consts::PI;

    // Three octaves of noise for pink-ish spectrum.
    let n0 = lcg_noise(idx);
    let n1 = lcg_noise(idx / 2) * 0.5;
    let n2 = lcg_noise(idx / 4) * 0.25;
    let pink = (n0 + n1 + n2) / 1.75; // normalise sum

    // Low-frequency buffeting rumble.
    let rumble = (t * 4.0 * 2.0 * PI).sin() * 0.15;

    let raw = pink * 0.55 + rumble;
    let threshold = 0.6_f32;
    (raw / threshold).tanh() * threshold
}

// ---------------------------------------------------------------------------
// Helpers: build StaticSoundData from a sample function
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
// Startup system: generate PCM and begin looped playback
// ---------------------------------------------------------------------------

fn spawn_engine_audio(
    mut commands: Commands,
    mut audio_sources: ResMut<Assets<AudioSource>>,
    audio: Res<Audio>,
) {
    let n_frames = (SAMPLE_RATE as f32 * DURATION_S) as usize;
    let source_handle = build_looped_source(&mut audio_sources, n_frames, |i| {
        let t = i as f32 / SAMPLE_RATE as f32;
        engine_sample(t, BASE_HZ)
    });

    // Play looped at idle settings; the Update system adjusts pitch/volume each frame.
    let instance = audio
        .play(source_handle)
        .looped()
        .with_volume(linear_to_db(0.2))
        .with_playback_rate(1.0_f64)
        .handle();

    commands.insert_resource(EngineAudio { instance });
}

fn spawn_skid_audio(
    mut commands: Commands,
    mut audio_sources: ResMut<Assets<AudioSource>>,
    audio: Res<Audio>,
) {
    let n_frames = (SAMPLE_RATE as f32 * DURATION_S) as usize;
    let source_handle = build_looped_source(&mut audio_sources, n_frames, |i| {
        let t = i as f32 / SAMPLE_RATE as f32;
        skid_sample(t, i as u32)
    });

    // Start silent; modulate_skid raises volume when slip is detected.
    let instance = audio
        .play(source_handle)
        .looped()
        .with_volume(linear_to_db(0.0001)) // effectively silent
        .with_playback_rate(1.0_f64)
        .handle();

    commands.insert_resource(SkidAudio { instance });
}

fn spawn_wind_audio(
    mut commands: Commands,
    mut audio_sources: ResMut<Assets<AudioSource>>,
    audio: Res<Audio>,
) {
    let n_frames = (SAMPLE_RATE as f32 * DURATION_S) as usize;
    let source_handle = build_looped_source(&mut audio_sources, n_frames, |i| {
        let t = i as f32 / SAMPLE_RATE as f32;
        wind_sample(t, i as u32)
    });

    // Base wind volume is low; speed raises it further.
    let instance = audio
        .play(source_handle)
        .looped()
        .with_volume(linear_to_db(0.1))
        .with_playback_rate(1.0_f64)
        .handle();

    commands.insert_resource(WindAudio { instance });
}

fn spawn_thud_source(
    mut commands: Commands,
    mut audio_sources: ResMut<Assets<AudioSource>>,
) {
    let n_frames = (SAMPLE_RATE as f32 * THUD_DURATION_S) as usize;
    // Thud doesn't loop; we just store the source handle for one-shot playback.
    let frames: Arc<[KiraFrame]> = (0..n_frames)
        .map(|i| {
            let t = i as f32 / SAMPLE_RATE as f32;
            KiraFrame::from_mono(thud_sample(t))
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
    commands.insert_resource(ThudSource { handle });
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

/// Slip metric: project chassis velocity onto the chassis right-vector.
/// When the vehicle drives purely forward the lateral component is zero.
/// Cornering, sliding, or braking at an angle raises it — a good proxy for skid.
fn modulate_skid(
    chassis_q: Query<(&Transform, &LinearVelocity), With<Chassis>>,
    skid: Option<Res<SkidAudio>>,
    mut audio_instances: ResMut<Assets<AudioInstance>>,
) {
    let Some(skid) = skid else { return };

    let slip_mps = chassis_q
        .single()
        .map(|(transform, lv)| {
            let vel = Vec3::new(lv.x, lv.y, lv.z);
            let right = transform.rotation * Vec3::X;
            vel.dot(right).abs()
        })
        .unwrap_or(0.0);

    // Volume ramps from 0 at 1 m/s slip to full at 6 m/s.
    let volume = ((slip_mps - 1.0) / 5.0).clamp(0.0, 1.0);
    // Pitch rises slightly at higher slip — tires squealing harder.
    let rate = (1.0 + slip_mps * 0.04) as f64;

    if let Some(instance) = audio_instances.get_mut(&skid.instance) {
        // Use a small linear tween (20 ms) to avoid zipper noise on volume changes.
        instance.set_decibels(
            linear_to_db(volume.max(0.0001)),
            AudioTween::linear(std::time::Duration::from_millis(20)),
        );
        instance.set_playback_rate(rate, AudioTween::default());
    }
}

fn modulate_wind(
    chassis_q: Query<&LinearVelocity, With<Chassis>>,
    wind: Option<Res<WindAudio>>,
    mut audio_instances: ResMut<Assets<AudioInstance>>,
) {
    let Some(wind) = wind else { return };

    let speed_mps = chassis_q
        .single()
        .map(|lv| Vec3::new(lv.x, lv.y, lv.z).length())
        .unwrap_or(0.0);

    // Base 0.1, rises to ~0.45 at 30 m/s highway speed.
    let volume = (0.1 + speed_mps * 0.012).clamp(0.0, 1.0);
    // Wind pitch rises slightly with speed (Doppler-ish high-frequency edge).
    let rate = (0.9 + speed_mps * 0.01) as f64;

    if let Some(instance) = audio_instances.get_mut(&wind.instance) {
        instance.set_decibels(
            linear_to_db(volume),
            AudioTween::linear(std::time::Duration::from_millis(60)),
        );
        instance.set_playback_rate(rate, AudioTween::default());
    }
}

/// Play a one-shot thud whenever a new HardImpact event appears in EventLog.
///
/// We track the timestamp of the last event we triggered on; since EventLog
/// timestamps come from `time.elapsed_secs()` and are monotonically increasing,
/// any event newer than our last-seen time is fresh.
fn play_thud_on_impact(
    thud_src: Option<Res<ThudSource>>,
    event_log: Option<Res<EventLog>>,
    audio: Res<Audio>,
    time: Res<Time>,
    mut last_impact_t: Local<f32>,
) {
    let (Some(thud_src), Some(event_log)) = (thud_src, event_log) else { return };

    let now = time.elapsed_secs();

    // Find the most recent HardImpact newer than our watermark.
    let mut best: Option<(f32, f32)> = None; // (timestamp, magnitude)
    for (ts, ev) in &event_log.events {
        if let GameEvent::HardImpact { v } = ev {
            if *ts > *last_impact_t {
                if best.map_or(true, |(bt, _)| *ts > bt) {
                    best = Some((*ts, v.abs()));
                }
            }
        }
    }

    if let Some((ts, mag)) = best {
        // Guard: don't replay if somehow we're still within the 500 ms decay window.
        if now - ts < 2.0 {
            // Scale volume by impact magnitude; IMPACT_THRESHOLD is 5.0 m/s.
            // Map [5, 20] m/s → [0.3, 1.0].
            let volume = ((mag - 5.0) / 15.0 * 0.7 + 0.3).clamp(0.3, 1.0);
            audio
                .play(thud_src.handle.clone())
                .with_volume(linear_to_db(volume));
        }
        *last_impact_t = ts;
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
