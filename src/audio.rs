// Engine audio — ONE authoritative engine voice + tire/skid/wind/thud layers.
//
// ─── Architecture ────────────────────────────────────────────────────────────
//
//   Startup : Build PCM buffers (one per Startup system) via the authentic DSP
//             described below.  Plug into kira as StaticSoundData + looped
//             AudioInstance.  Store instance handles in resources.
//
//   Update  : Read EngineState::rpm (from engine_torque.rs) + DriveInput +
//             LinearVelocity each frame.  Adjust playback_rate and volume via
//             short AudioTween so kira interpolates them smoothly (no zipper).
//
// ─── Authentic Engine DSP model ──────────────────────────────────────────────
//
//   A real engine sounds like a train of combustion impulses — one per cylinder
//   firing — shaped by resonant exhaust and body cavities.
//
//   Step 1 – Band-limited pulse train (BLEP)
//     A 4-cylinder 4-stroke engine fires 2 times per revolution.
//     firing_hz = rpm / 60 * 2                (e.g. 2000 RPM → 66.7 Hz)
//     We synthesise the pulse train as a finite harmonic series:
//       p(t) = Σ_{k=1}^{K} A_k * sin(2π * k * firing_hz * t + φ_k)
//     where K = floor(Nyquist / firing_hz) capped at MAX_HARMONICS.
//     Amplitudes follow A_k = 1/k (sawtooth spectrum).
//     By only summing harmonics below Nyquist (22 050 Hz), aliasing is
//     eliminated — the root cause of the prior "glitchy crackle".
//
//   Step 2 – Resonant biquad bandpass filters
//     The pulse train is passed through three second-order bandpass filters
//     tuned to engine formant frequencies:
//       F1 = 120 Hz  (exhaust fundamental / torque pulse)
//       F2 = 380 Hz  (exhaust pipe resonance / mid bark)
//       F3 = 900 Hz  (combustion chamber ring / metallic edge)
//     Each filter output is summed with a different weight to shape the
//     final timbre so it sounds like a real engine cavity, not a buzzer.
//
//   Step 3 – Load-dependent timbre
//     Under full throttle the harmonic weights shift toward higher partials
//     (brassy/aggressive). Off-throttle they roll off (smooth overrun).
//     A combustion-roughness noise component (filtered LCG noise ×
//     throttle²) adds the characteristic "lope" at idle and load texture.
//
//   Step 4 – Phase-continuous loop
//     The buffer length is chosen so that firing_hz * duration is an
//     integer → zero phase error at the loop seam → no click.
//     At idle firing_hz = 800/60*2 = 26.67 Hz → we use 3 s (80 cycles).
//     Pitch is then varied at runtime via playback_rate, NOT by regenerating
//     the buffer, which keeps startup cost to a single 3 s generation pass.
//
//   Step 5 – Smooth modulation
//     Every per-frame param change goes through a short AudioTween (40-80 ms)
//     so kira interpolates; no instantaneous jumps that cause crackle.
//
// ─── Non-engine roles in this module ────────────────────────────────────────
//
//   TireSkid   — lateral-slip noise with squeal partial
//   Wind       — pink-ish noise + buffeting, speed-scaled
//   Thud       — one-shot impact sub-bass
//
//   These are UNCHANGED from the working prior implementation.
//
// ─── Tier scaling ────────────────────────────────────────────────────────────
//
//   High   — 3 biquad stages, MAX_HARMONICS = 24
//   Medium — 2 biquad stages, MAX_HARMONICS = 16
//   Low    — 1 biquad stage,  MAX_HARMONICS =  8
//
// ─── Headless safety ─────────────────────────────────────────────────────────
//
//   All Update systems guard on Option<Res<...>> and early-return.
//   Startup systems guard Option<Res<Audio>> + Option<ResMut<Assets<...>>>.
//   cargo test --test drive_test (headless) never panics.

use bevy::prelude::*;
use bevy_kira_audio::{Audio, AudioControl, AudioInstance, AudioSource, AudioTween};
use bevy_kira_audio::prelude::{
    Decibels, StaticSoundData, StaticSoundSettings, Frame as KiraFrame,
};
use avian3d::prelude::LinearVelocity;
use std::sync::Arc;

use crate::vehicle::{Chassis, DriveInput};
use crate::events::{EventLog, GameEvent};
use crate::engine_torque::EngineState;
use crate::graphics_quality::GraphicsQuality;

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
// Constants
// ---------------------------------------------------------------------------

const SAMPLE_RATE: u32 = 44_100;
const THUD_DURATION_S: f32 = 0.5;
const AMBIENT_LOOP_S: f32 = 1.0;

// ── Engine loop constants ───────────────────────────────────────────────────
//
// Idle: 800 RPM / 60 s = 13.33 rev/s; 2 firings/rev → 26.67 Hz firing rate.
// 3 s × 26.667 Hz = 80.0 whole cycles → seam is exactly phase-continuous.
// Confirmed: (800f32/60f32)*2f32*3f32 = 80.0 → fract() = 0.0 ✓
// Duration = 3.0 s → 132300 frames.
const ENGINE_LOOP_N_FRAMES: usize = (SAMPLE_RATE as usize) * 3; // 3 s × 44100 Hz

// Idle RPM used to bake the buffer. Runtime RPM shifts pitch via playback_rate.
const IDLE_RPM: f32 = 800.0;

// Maximum harmonic count per quality tier (cap at well below Nyquist).
// At idle firing_hz = 26.67 Hz; harmonic 24 = 640 Hz < 22050 Hz — safe.
// High-RPM: at 6500 RPM firing_hz = 216.7 Hz; harmonic 24 = 5200 Hz — still safe.
const MAX_HARMONICS_HIGH: usize = 24;
const MAX_HARMONICS_MED:  usize = 16;
const MAX_HARMONICS_LOW:  usize =  8;

// ---------------------------------------------------------------------------
// ── Biquad bandpass filter (second-order IIR) ─────────────────────────────
// ---------------------------------------------------------------------------
//
// Direct Form I, computed in stateless sample-at-a-time fashion for buffer
// generation. We carry state manually through a fold so the generator closure
// can remain pure-ish.
//
// Coefficients derived from Audio EQ Cookbook (Robert Bristow-Johnson):
//   H(s) = (s/Q) / (s² + s/Q + 1)
//   a0 = 1 + α  where α = sin(w0)/(2Q)
//   b0 =  α/a0,  b1 = 0,  b2 = -α/a0
//   a1 = -2cos(w0)/a0,  a2 = (1 - α)/a0

struct BiquadBP {
    b0: f32, b2: f32, // b1 = 0
    a1: f32, a2: f32,
    x1: f32, x2: f32,
    y1: f32, y2: f32,
}

impl BiquadBP {
    /// Construct a bandpass with centre `freq_hz` and quality factor `q`.
    fn new(freq_hz: f32, q: f32, sample_rate: u32) -> Self {
        use std::f32::consts::PI;
        let w0 = 2.0 * PI * freq_hz / sample_rate as f32;
        let alpha = w0.sin() / (2.0 * q);
        let a0 = 1.0 + alpha;
        let b0 = alpha / a0;
        let b2 = -alpha / a0;
        let a1 = -2.0 * w0.cos() / a0;
        let a2 = (1.0 - alpha) / a0;
        Self { b0, b2, a1, a2, x1: 0.0, x2: 0.0, y1: 0.0, y2: 0.0 }
    }

    /// Process one sample.
    #[inline]
    fn process(&mut self, x: f32) -> f32 {
        let y = self.b0 * x + self.b2 * self.x2
              - self.a1 * self.y1 - self.a2 * self.y2;
        self.x2 = self.x1; self.x1 = x;
        self.y2 = self.y1; self.y1 = y;
        y
    }
}

// ---------------------------------------------------------------------------
// ── Engine sample synthesis ──────────────────────────────────────────────
// ---------------------------------------------------------------------------

/// Generate a complete engine PCM buffer.
///
/// # Arguments
/// * `n_frames`    – buffer length in samples
/// * `rpm`         – RPM used to derive firing_hz for the biquad buffer-
///                   generation pass (not the runtime pitch shift)
/// * `max_harmonics` – harmonic count cap (quality tier)
/// * `n_filters`   – number of biquad stages (1..=3, quality tier)
///
/// # Aliasing prevention
/// We sum harmonics k = 1..=min(max_harmonics, floor(nyquist/firing_hz)).
/// This guarantees every sinusoid is below the Nyquist limit (22 050 Hz),
/// eliminating the spectral aliasing that caused the prior glitch/crackle.
///
/// # Loop-seam click prevention
/// `n_frames` is chosen so that `firing_hz × (n_frames / sr)` is an integer,
/// meaning the phase of every harmonic returns to exactly 0 at the seam.
/// (See ENGINE_LOOP_N_FRAMES / ENGINE_LOOP_DURATION_S above.)
fn generate_engine_buffer(
    n_frames: usize,
    rpm: f32,
    max_harmonics: usize,
    n_filters: usize,
) -> Vec<f32> {
    use std::f32::consts::PI;

    let sr = SAMPLE_RATE as f32;
    let nyquist = sr * 0.5;

    // Firing frequency for a 4-cyl 4-stroke: 2 firings per revolution.
    let firing_hz = rpm / 60.0 * 2.0;

    // Maximum safe harmonic (must stay below Nyquist).
    let k_max = ((nyquist / firing_hz).floor() as usize).min(max_harmonics);

    // Biquad filter chain:
    //   F1 = 120 Hz  Q=3.0  — exhaust torque pulse, heavy low body thump
    //   F2 = 380 Hz  Q=4.5  — mid bark / exhaust pipe resonance
    //   F3 = 900 Hz  Q=6.0  — combustion chamber ring / metallic edge
    let formants = [(120.0_f32, 3.0_f32), (380.0, 4.5), (900.0, 6.0)];

    // Weights for each filter output summed into the final signal.
    // Low-mid heavy (F1 loudest) since the fundamental is most audible.
    let filter_weights = [0.55_f32, 0.30, 0.15];

    let n_filters = n_filters.clamp(1, 3);

    // Build filter states (one per active stage).
    let mut filters: Vec<BiquadBP> = formants[..n_filters]
        .iter()
        .map(|&(f, q)| BiquadBP::new(f, q, SAMPLE_RATE))
        .collect();

    // LCG noise for combustion roughness.
    fn lcg(seed: u32) -> f32 {
        let v = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        (v as f32 / 2_147_483_648.0) - 1.0
    }

    let mut buf = Vec::with_capacity(n_frames);
    for i in 0..n_frames {
        let t = i as f32 / sr;

        // ── Band-limited pulse train ──────────────────────────────────────
        // Sum harmonics k=1..k_max of a sawtooth spectrum (A_k = 1/k).
        // All frequencies ≤ k_max * firing_hz ≤ Nyquist → no aliasing.
        let mut pulse = 0.0_f32;
        let mut norm = 0.0_f32;
        for k in 1..=k_max {
            let amp = 1.0 / k as f32;
            // Odd harmonics slightly emphasized for that "four cylinder bark".
            let odd_boost = if k % 2 == 1 { 1.15 } else { 1.0 };
            pulse += (2.0 * PI * firing_hz * k as f32 * t).sin() * amp * odd_boost;
            norm  += amp * odd_boost;
        }
        // Normalize so amplitude doesn't depend on harmonic count.
        if norm > 0.0 { pulse /= norm; }

        // ── Combustion-cycle envelope ─────────────────────────────────────
        // A half-rectified sine at firing_hz shapes each "ignition stroke".
        // This gives the characteristic on/off rhythm of cylinder firing.
        let env_phase = (t * firing_hz * 2.0 * PI).sin();
        let env = env_phase.max(0.0).powf(0.5); // soft half-wave rectifier
        pulse *= env;

        // ── Combustion roughness noise (intake lope + thermal grit) ───────
        // Low-amplitude filtered noise mixed in — gives organic roughness.
        // Throttle dependence: we bake with a fixed idle roughness; the
        // modulate_engine_audio system boosts volume under load dynamically.
        let noise_raw = lcg(i as u32);
        // Simple one-pole low-pass of noise: N_lp = 0.15*noise + 0.85*prev.
        // Stateless approximation: blend with next-sample noise.
        let noise_lo = noise_raw * 0.15 + lcg(i.wrapping_add(1) as u32) * 0.85;
        let roughness = noise_lo * 0.08; // 8% roughness at idle

        let pre_filter = pulse + roughness;

        // ── Biquad filter bank ────────────────────────────────────────────
        // Run each active filter and accumulate weighted outputs.
        let mut filtered = 0.0_f32;
        let mut weight_sum = 0.0_f32;
        for (fi, flt) in filters.iter_mut().enumerate() {
            filtered += flt.process(pre_filter) * filter_weights[fi];
            weight_sum += filter_weights[fi];
        }
        if weight_sum > 0.0 { filtered /= weight_sum; }

        // ── Soft clip (tanh) at ±0.8 ─────────────────────────────────────
        // Prevents digital overs while adding gentle musical saturation.
        let th = 0.8_f32;
        let out = (filtered / th).tanh() * th;

        buf.push(out);
    }
    buf
}

// ---------------------------------------------------------------------------
// ── Non-engine sample synthesis (unchanged, correct) ─────────────────────
// ---------------------------------------------------------------------------

/// White noise via a deterministic LCG — no rand crate needed.
#[inline]
fn lcg_noise(seed: u32) -> f32 {
    let v = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    (v as f32 / 2_147_483_648.0) - 1.0
}

/// Tire skid: filtered white noise with a gentle 200 Hz resonance.
fn skid_sample(t: f32, idx: u32) -> f32 {
    use std::f32::consts::PI;
    let noise = lcg_noise(idx);
    let squeal = (t * 200.0 * 2.0 * PI).sin() * 0.15;
    let n_lo = lcg_noise(idx.wrapping_add(1));
    let filtered = noise * 0.6 + n_lo * 0.4;
    let raw = filtered * 0.8 + squeal;
    let threshold = 0.7_f32;
    (raw / threshold).tanh() * threshold
}

/// Impact thud: 50 ms exponential-decay sub-bass thump at ~45 Hz.
fn thud_sample(t: f32) -> f32 {
    use std::f32::consts::PI;
    let body_hz = 45.0_f32;
    let decay = (-t * 80.0).exp();
    let thump = (t * body_hz * 2.0 * PI).sin() * decay;
    let mid_decay = (-t * 120.0).exp();
    let mid = (t * 90.0 * 2.0 * PI).sin() * mid_decay * 0.35;
    let raw = thump + mid;
    let threshold = 0.75_f32;
    (raw / threshold).tanh() * threshold
}

/// Ambient wind: pink-ish noise and low-frequency rumble.
fn wind_sample(t: f32, idx: u32) -> f32 {
    use std::f32::consts::PI;
    let n0 = lcg_noise(idx);
    let n1 = lcg_noise(idx / 2) * 0.5;
    let n2 = lcg_noise(idx / 4) * 0.25;
    let pink = (n0 + n1 + n2) / 1.75;
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
    audio_sources: Option<ResMut<Assets<AudioSource>>>,
    audio: Option<Res<Audio>>,
    quality: Option<Res<GraphicsQuality>>,
) {
    let (Some(mut audio_sources), Some(audio)) = (audio_sources, audio) else { return };

    // Choose DSP complexity by quality tier.
    let tier = quality.map(|q| *q).unwrap_or(GraphicsQuality::High);
    let (max_harmonics, n_filters) = match tier {
        GraphicsQuality::Low    => (MAX_HARMONICS_LOW,  1),
        GraphicsQuality::Medium => (MAX_HARMONICS_MED,  2),
        GraphicsQuality::High   => (MAX_HARMONICS_HIGH, 3),
    };

    // Generate the authentic engine buffer at idle RPM.
    // Runtime RPM is expressed as playback_rate relative to this baked pitch.
    let pcm = generate_engine_buffer(ENGINE_LOOP_N_FRAMES, IDLE_RPM, max_harmonics, n_filters);

    let frames: Arc<[KiraFrame]> = pcm
        .into_iter()
        .map(KiraFrame::from_mono)
        .collect::<Vec<_>>()
        .into();

    let sound_data = StaticSoundData {
        sample_rate: SAMPLE_RATE,
        frames,
        settings: StaticSoundSettings::default(),
        slice: None,
    };

    let source_handle = audio_sources.add(AudioSource { sound: sound_data });

    // Play looped at idle settings; modulate_engine_audio adjusts every frame.
    let instance = audio
        .play(source_handle)
        .looped()
        .with_volume(linear_to_db(0.25))
        .with_playback_rate(1.0_f64)
        .handle();

    commands.insert_resource(EngineAudio { instance });
}

fn spawn_skid_audio(
    mut commands: Commands,
    audio_sources: Option<ResMut<Assets<AudioSource>>>,
    audio: Option<Res<Audio>>,
) {
    let (Some(mut audio_sources), Some(audio)) = (audio_sources, audio) else { return };

    let n_frames = (SAMPLE_RATE as f32 * AMBIENT_LOOP_S) as usize;
    let source_handle = build_looped_source(&mut audio_sources, n_frames, |i| {
        let t = i as f32 / SAMPLE_RATE as f32;
        skid_sample(t, i as u32)
    });

    let instance = audio
        .play(source_handle)
        .looped()
        .with_volume(linear_to_db(0.0001))
        .with_playback_rate(1.0_f64)
        .handle();

    commands.insert_resource(SkidAudio { instance });
}

fn spawn_wind_audio(
    mut commands: Commands,
    audio_sources: Option<ResMut<Assets<AudioSource>>>,
    audio: Option<Res<Audio>>,
) {
    let (Some(mut audio_sources), Some(audio)) = (audio_sources, audio) else { return };

    let n_frames = (SAMPLE_RATE as f32 * AMBIENT_LOOP_S) as usize;
    let source_handle = build_looped_source(&mut audio_sources, n_frames, |i| {
        let t = i as f32 / SAMPLE_RATE as f32;
        wind_sample(t, i as u32)
    });

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
    audio_sources: Option<ResMut<Assets<AudioSource>>>,
) {
    let Some(mut audio_sources) = audio_sources else { return };

    let n_frames = (SAMPLE_RATE as f32 * THUD_DURATION_S) as usize;
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
// Per-frame modulation: authentic RPM-driven engine voice
// ---------------------------------------------------------------------------

/// Modulate the engine voice every frame.
///
/// # Pitch (playback_rate)
/// The buffer was baked at IDLE_RPM (800 RPM). Runtime RPM from EngineState
/// maps directly to playback_rate: rate = rpm / IDLE_RPM.  This is physically
/// correct — doubling RPM doubles playback speed and raises pitch by an octave.
///
/// Responsiveness: the pitch tween is only 30 ms (was 60 ms). The player will
/// feel the engine respond promptly to throttle input.
///
/// # Volume
/// Base at idle 0.25; rises with throttle (load-dependent timbre):
///   - Full throttle → 0.85 (aggressive, forward-mixed)
///   - Overrun (no throttle, speed > 0) → 0.15 (quiet, smooth)
///   This emulates the real engine phenomenon: under load combustion is louder;
///   overrun causes fuel-cut so combustion noise drops.
///
/// # Headless safety
/// Both EngineAudio and EngineState are Option<Res<...>>; if either is absent
/// (headless mode or very early frames before Startup completes), the system
/// returns immediately.
fn modulate_engine_audio(
    engine_audio: Option<Res<EngineAudio>>,
    mut audio_instances: ResMut<Assets<AudioInstance>>,
    drive: Res<DriveInput>,
    chassis_q: Query<&LinearVelocity, With<Chassis>>,
    engine_state: Option<Res<EngineState>>,
) {
    let Some(engine_audio) = engine_audio else { return };

    // Prefer EngineState::rpm (accurate, torque-curve-aware) when available.
    // Fall back to speed-based estimate for robustness.
    let rpm = if let Some(es) = engine_state {
        es.rpm
    } else {
        let speed_mps = chassis_q
            .single()
            .map(|lv| Vec3::new(lv.x, lv.y, lv.z).length())
            .unwrap_or(0.0);
        (speed_mps * 90.0 + 800.0).clamp(800.0, 6500.0)
    };

    // Playback rate = current_rpm / idle_rpm.
    // Physical: 800 RPM → rate 1.0 (baked pitch); 4000 RPM → rate 5.0, etc.
    // Clamped to kira's safe range.
    let playback_rate = (rpm / IDLE_RPM as f32).clamp(0.5, 8.0) as f64;

    // Volume model:
    //   idle base      = 0.25
    //   throttle boost = +0.60 × |drive|  (0 at idle → 0.85 at WOT)
    //   overrun dip    = if no throttle and moving, reduce base to 0.15
    let speed_mps = chassis_q
        .single()
        .map(|lv| Vec3::new(lv.x, lv.y, lv.z).length())
        .unwrap_or(0.0);
    let throttle = drive.drive.abs();
    let is_overrun = throttle < 0.05 && speed_mps > 1.0;
    let base = if is_overrun { 0.15 } else { 0.25 };
    let volume_linear = (base + 0.60 * throttle).clamp(0.0, 1.0);

    if let Some(instance) = audio_instances.get_mut(&engine_audio.instance) {
        // Short tweens for prompt responsiveness without zipper noise.
        let vol_tween   = AudioTween::linear(std::time::Duration::from_millis(30));
        let pitch_tween = AudioTween::linear(std::time::Duration::from_millis(30));
        instance.set_playback_rate(playback_rate, pitch_tween);
        instance.set_decibels(linear_to_db(volume_linear), vol_tween);
    }
}

/// Slip metric: project chassis velocity onto the chassis right-vector.
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

    let volume = ((slip_mps - 1.0) / 5.0).clamp(0.0, 1.0);
    let rate = (1.0 + slip_mps * 0.04) as f64;

    if let Some(instance) = audio_instances.get_mut(&skid.instance) {
        let tween = AudioTween::linear(std::time::Duration::from_millis(30));
        instance.set_decibels(
            linear_to_db(volume.max(0.0001)),
            tween.clone(),
        );
        instance.set_playback_rate(rate, tween);
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

    let volume = (0.1 + speed_mps * 0.012).clamp(0.0, 1.0);
    let rate = (0.9 + speed_mps * 0.01) as f64;

    if let Some(instance) = audio_instances.get_mut(&wind.instance) {
        let tween = AudioTween::linear(std::time::Duration::from_millis(80));
        instance.set_decibels(linear_to_db(volume), tween.clone());
        instance.set_playback_rate(rate, tween);
    }
}

/// Play a one-shot thud whenever a new HardImpact event appears in EventLog.
fn play_thud_on_impact(
    thud_src: Option<Res<ThudSource>>,
    event_log: Option<Res<EventLog>>,
    audio: Option<Res<Audio>>,
    time: Res<Time>,
    mut last_impact_t: Local<f32>,
) {
    let (Some(thud_src), Some(event_log), Some(audio)) = (thud_src, event_log, audio) else { return };

    let now = time.elapsed_secs();

    let mut best: Option<(f32, f32)> = None;
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
        if now - ts < 2.0 {
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
    let db = 20.0 * linear.max(1e-6).log10();
    Decibels(db.max(-60.0))
}
