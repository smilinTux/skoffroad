// Sprint 80 — Higher-quality audio: layered engine RPM crossfade + enhanced ambient.
//
// This module adds:
//   1. Three-layer engine audio crossfade (Idle/Cruise/Redline) driven by
//      EngineState::rpm.  Each layer has a distinct synthesized timbre:
//        * idle_layer  — low-RPM sub-bass rumble + slow combustion pulse
//        * cruise_layer — mid-RPM growl with 2nd/3rd harmonic content
//        * redline_layer — high-RPM metallic whine + dense harmonic series
//      All three loop continuously; crossfade weights are updated each frame.
//
//   2. Enhanced wind ambient: the existing world_audio.rs wind layer already
//      scales with chassis speed.  We add a dedicated *atmospheric gust* layer
//      that responds only to WindState::speed_mps (not chassis speed) so calm
//      weather feels different from a howling gale even when stationary.
//
// Headless tolerance:
//   All Startup systems guard on Option<Res<Audio>> and Option<ResMut<Assets<..>>>
//   so they silently no-op in the headless drive_test harness (which never adds
//   bevy_kira_audio::AudioPlugin).
//
// Mixer routing:
//   We read SettingsState::master_volume each frame and incorporate it into
//   every volume calculation before writing to kira.  This means the player's
//   volume slider applies immediately to these layers, matching the behaviour
//   of settings.rs::apply_master_volume for the global channel.

use bevy::prelude::*;
use bevy_kira_audio::{Audio, AudioControl, AudioInstance, AudioSource, AudioTween};
use bevy_kira_audio::prelude::{
    Decibels, StaticSoundData, StaticSoundSettings, Frame as KiraFrame,
};
use std::sync::Arc;

use crate::engine_torque::EngineState;
use crate::settings::SettingsState;
use crate::wind::WindState;

// ---------------------------------------------------------------------------
// Public plugin
// ---------------------------------------------------------------------------

pub struct AudioQualityPlugin;

impl Plugin for AudioQualityPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_rpm_layers)
           .add_systems(Startup, spawn_gust_layer)
           .add_systems(Update, crossfade_rpm_layers)
           .add_systems(Update, modulate_gust_layer);
    }
}

// ---------------------------------------------------------------------------
// Resources
// ---------------------------------------------------------------------------

/// Holds the three engine-RPM audio layer instance handles.
#[derive(Resource)]
struct RpmLayers {
    idle:    Handle<AudioInstance>,
    cruise:  Handle<AudioInstance>,
    redline: Handle<AudioInstance>,
}

/// Atmospheric gust layer — responds to WindState, not chassis speed.
#[derive(Resource)]
struct GustLayer {
    instance: Handle<AudioInstance>,
}

// ---------------------------------------------------------------------------
// Synthesis constants
// ---------------------------------------------------------------------------

const SAMPLE_RATE: u32 = 44_100;
const DURATION_S:  f32 = 1.0;

// RPM band centres / edges for crossfade (same as engine_audio_layered.rs HUD).
const IDLE_RPM_FULL:    f32 = 1_200.0;   // below this: idle at 100 %
const IDLE_RPM_ZERO:    f32 = 2_800.0;   // above this: idle at 0 %
const CRUISE_PEAK_RPM:  f32 = 3_500.0;   // gaussian peak
const CRUISE_SIGMA:     f32 = 1_400.0;   // gaussian sigma
const REDLINE_RPM_RISE: f32 = 4_200.0;   // below this: redline at 0 %
const REDLINE_RPM_FULL: f32 = 6_000.0;   // above this: redline at 100 %

// ---------------------------------------------------------------------------
// LCG noise helper
// ---------------------------------------------------------------------------

#[inline]
fn lcg_noise(seed: u32) -> f32 {
    let v = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    (v as f32 / 2_147_483_648.0) - 1.0
}

// ---------------------------------------------------------------------------
// Engine layer synthesis
// ---------------------------------------------------------------------------

/// Idle layer: heavy sub-bass rumble at ~20 Hz with slow 2 Hz combustion
/// envelope.  Sounds like a big engine barely ticking over.
fn idle_sample(t: f32, i: u32) -> f32 {
    use std::f32::consts::PI;

    // Sub-bass at 20 Hz — dominant body shake feeling.
    let sub = (t * 20.0 * 2.0 * PI).sin();

    // Very slow combustion envelope at 2 Hz (idle = ~120 RPM → 2 Hz per cyl).
    let env = (t * 2.0 * 2.0 * PI).sin().powi(2) * 0.5 + 0.5;

    // Tiny noise crackle to keep it from sounding too "pure".
    let crackle = lcg_noise(i) * 0.05;

    let raw = sub * env * 0.7 + crackle;
    let th = 0.75_f32;
    (raw / th).tanh() * th
}

/// Cruise layer: mid-range four-cylinder growl.  Dominant frequency at 80 Hz
/// with 2nd (160 Hz) and 3rd (240 Hz) harmonics, modulated by a combustion
/// cycle envelope at 13 Hz (~780 RPM equivalent for the sample; the Update
/// system pitch-shifts to match actual RPM).
fn cruise_sample(t: f32) -> f32 {
    use std::f32::consts::PI;

    let fund_hz = 80.0_f32;

    let fund  = (t * fund_hz * 2.0 * PI).sin();
    let h2    = (t * fund_hz * 2.0 * 2.0 * PI).sin() * 0.45;
    let h3    = (t * fund_hz * 3.0 * 2.0 * PI).sin() * 0.25;
    let h4    = (t * fund_hz * 4.0 * 2.0 * PI).sin() * 0.12;

    // Combustion envelope at 13 Hz.
    let env = (t * 13.0 * 2.0 * PI).sin().powi(2) * 0.4 + 0.6;

    let raw = (fund + h2 + h3 + h4) * env * 0.55;
    let th = 0.8_f32;
    (raw / th).tanh() * th
}

/// Redline layer: dense upper-harmonic whine with strong odd harmonics for
/// metallic character (engine "screaming").  Dominant at 160 Hz with many
/// upper partials; a fast 30 Hz combustion rate adds gritty texture.
fn redline_sample(t: f32, i: u32) -> f32 {
    use std::f32::consts::PI;

    let fund_hz = 160.0_f32;

    // Dense harmonic stack — odd harmonics louder for a brasher, metallic quality.
    let h1  = (t * fund_hz       * 2.0 * PI).sin() * 1.00;
    let h2  = (t * fund_hz * 2.0 * 2.0 * PI).sin() * 0.40;
    let h3  = (t * fund_hz * 3.0 * 2.0 * PI).sin() * 0.55;
    let h4  = (t * fund_hz * 4.0 * 2.0 * PI).sin() * 0.25;
    let h5  = (t * fund_hz * 5.0 * 2.0 * PI).sin() * 0.35;
    let h6  = (t * fund_hz * 6.0 * 2.0 * PI).sin() * 0.15;
    let h7  = (t * fund_hz * 7.0 * 2.0 * PI).sin() * 0.20;

    // Fast combustion envelope at 30 Hz — dense chuffing.
    let env = (t * 30.0 * 2.0 * PI).sin().powi(2) * 0.3 + 0.7;

    // Thin noise fuzz for metallic grit.
    let fuzz = lcg_noise(i) * 0.08;

    let raw = (h1 + h2 + h3 + h4 + h5 + h6 + h7) * env * 0.35 + fuzz;
    let th = 0.8_f32;
    (raw / th).tanh() * th
}

/// Atmospheric gust: a single wind burst — dense pink-ish noise with a slow
/// 0.8 Hz amplitude swell to simulate a passing gust.
fn gust_sample(t: f32, i: u32) -> f32 {
    use std::f32::consts::PI;

    // Three-octave pink noise approximation.
    let n0 = lcg_noise(i);
    let n1 = lcg_noise(i / 2) * 0.5;
    let n2 = lcg_noise(i / 4) * 0.25;
    let pink = (n0 + n1 + n2) / 1.75;

    // Slow 0.8 Hz swell — the "gust" envelope.
    let swell = (t * 0.8 * 2.0 * PI).sin() * 0.3 + 0.7;

    let raw = pink * 0.6 * swell;
    let th = 0.65_f32;
    (raw / th).tanh() * th
}

// ---------------------------------------------------------------------------
// Helper: build AudioSource from a generator closure
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
// Startup systems
// ---------------------------------------------------------------------------

fn spawn_rpm_layers(
    mut commands: Commands,
    audio_sources: Option<ResMut<Assets<AudioSource>>>,
    audio: Option<Res<Audio>>,
) {
    let (Some(mut audio_sources), Some(audio)) = (audio_sources, audio) else { return };

    let n = (SAMPLE_RATE as f32 * DURATION_S) as usize;

    // --- Idle layer ---
    let idle_handle = build_looped_source(&mut audio_sources, n, |i| {
        let t = i as f32 / SAMPLE_RATE as f32;
        idle_sample(t, i as u32)
    });
    let idle_inst = audio
        .play(idle_handle)
        .looped()
        .with_volume(linear_to_db(0.0001))
        .with_playback_rate(1.0_f64)
        .handle();

    // --- Cruise layer ---
    let cruise_handle = build_looped_source(&mut audio_sources, n, |i| {
        let t = i as f32 / SAMPLE_RATE as f32;
        cruise_sample(t)
    });
    let cruise_inst = audio
        .play(cruise_handle)
        .looped()
        .with_volume(linear_to_db(0.0001))
        .with_playback_rate(1.0_f64)
        .handle();

    // --- Redline layer ---
    let redline_handle = build_looped_source(&mut audio_sources, n, |i| {
        let t = i as f32 / SAMPLE_RATE as f32;
        redline_sample(t, i as u32)
    });
    let redline_inst = audio
        .play(redline_handle)
        .looped()
        .with_volume(linear_to_db(0.0001))
        .with_playback_rate(1.0_f64)
        .handle();

    commands.insert_resource(RpmLayers {
        idle:    idle_inst,
        cruise:  cruise_inst,
        redline: redline_inst,
    });
}

fn spawn_gust_layer(
    mut commands: Commands,
    audio_sources: Option<ResMut<Assets<AudioSource>>>,
    audio: Option<Res<Audio>>,
) {
    let (Some(mut audio_sources), Some(audio)) = (audio_sources, audio) else { return };

    let n = (SAMPLE_RATE as f32 * DURATION_S) as usize;

    let gust_handle = build_looped_source(&mut audio_sources, n, |i| {
        let t = i as f32 / SAMPLE_RATE as f32;
        gust_sample(t, i as u32)
    });

    let gust_inst = audio
        .play(gust_handle)
        .looped()
        .with_volume(linear_to_db(0.0001))
        .with_playback_rate(1.0_f64)
        .handle();

    commands.insert_resource(GustLayer { instance: gust_inst });
}

// ---------------------------------------------------------------------------
// Update systems
// ---------------------------------------------------------------------------

/// Compute RPM-driven blend weights for the three engine layers and write
/// volume + playback rate to each AudioInstance.
///
/// Crossfade model:
///   idle_weight   = clamp(1 - (rpm - IDLE_RPM_FULL) / (IDLE_RPM_ZERO - IDLE_RPM_FULL), 0, 1)
///   cruise_weight = gaussian(rpm, CRUISE_PEAK_RPM, CRUISE_SIGMA)
///   redline_weight = clamp((rpm - REDLINE_RPM_RISE) / (REDLINE_RPM_FULL - REDLINE_RPM_RISE), 0, 1)
///
/// Weights are NOT normalised to sum to 1 — layers blend additively so the
/// overlapping transition regions get both timbres simultaneously (more
/// natural than a hard crossfade).
///
/// Pitch (playback rate): all three layers share the same rate so they stay
/// rhythmically in sync as RPM changes.  Base rate = 1.0 at 700 RPM (idle);
/// rises to ~9.3 at 6500 RPM.  Formula: rpm / IDLE_RPM.
///
/// Master volume from SettingsState is folded in so the player's slider applies.
fn crossfade_rpm_layers(
    rpm_layers: Option<Res<RpmLayers>>,
    mut audio_instances: ResMut<Assets<AudioInstance>>,
    engine: Option<Res<EngineState>>,
    settings: Option<Res<SettingsState>>,
) {
    let Some(rpm_layers) = rpm_layers else { return };

    let rpm = engine.map(|e| e.rpm).unwrap_or(700.0);

    let master = settings.map(|s| s.master_volume).unwrap_or(0.7);

    // Idle weight: full below IDLE_RPM_FULL, linear ramp-down to zero at IDLE_RPM_ZERO.
    let idle_w = {
        let t = ((rpm - IDLE_RPM_FULL) / (IDLE_RPM_ZERO - IDLE_RPM_FULL)).clamp(0.0, 1.0);
        1.0 - t
    };

    // Cruise weight: gaussian centred at CRUISE_PEAK_RPM.
    let cruise_w = {
        let z = (rpm - CRUISE_PEAK_RPM) / CRUISE_SIGMA;
        (-z * z).exp()
    };

    // Redline weight: linear ramp-up from REDLINE_RPM_RISE to REDLINE_RPM_FULL.
    let redline_w = ((rpm - REDLINE_RPM_RISE) / (REDLINE_RPM_FULL - REDLINE_RPM_RISE))
        .clamp(0.0, 1.0);

    // Playback rate — all three layers track the same pitch shift so their
    // timbres change together as RPM changes.  Clamped to avoid extreme speeds.
    // At idle (700 RPM) → 1.0×; at 6500 RPM → ~9.3× (but kira's ceiling is ~8×
    // so we cap gracefully).
    let idle_base_rpm = 700.0_f32;
    let playback_rate = (rpm / idle_base_rpm).clamp(0.5, 8.0) as f64;

    // Volume scalars: max layer volumes (at 100 % weight × master) tune the
    // overall loudness of each layer so they sit at a good blend.
    // idle gets a slightly lower ceiling to avoid masking the cruise growl.
    let idle_vol_max    = 0.30_f32;
    let cruise_vol_max  = 0.45_f32;
    let redline_vol_max = 0.40_f32;

    let tween = AudioTween::linear(std::time::Duration::from_millis(25));

    let layers = [
        (&rpm_layers.idle,    idle_w,    idle_vol_max),
        (&rpm_layers.cruise,  cruise_w,  cruise_vol_max),
        (&rpm_layers.redline, redline_w, redline_vol_max),
    ];

    for (handle, weight, vol_max) in &layers {
        let vol_linear = (weight * vol_max * master).max(1e-6);
        if let Some(inst) = audio_instances.get_mut(*handle) {
            inst.set_decibels(linear_to_db(vol_linear), tween.clone());
            inst.set_playback_rate(playback_rate, AudioTween::default());
        }
    }
}

/// Modulate the atmospheric gust layer based on WindState::speed_mps.
///
/// The gust layer is deliberately distinct from world_audio.rs's wind layer:
///   - world_audio::wind responds to chassis speed (aerodynamic buffeting).
///   - This gust layer responds to environmental wind speed (ambient weather).
///
/// At calm (speed < 2 m/s): nearly silent.
/// At moderate (5 m/s): clearly audible background texture.
/// At strong (6.5 m/s max): ~40 % of master volume.
///
/// Playback rate rises slightly with wind speed for a higher-pitched howl.
fn modulate_gust_layer(
    gust: Option<Res<GustLayer>>,
    mut audio_instances: ResMut<Assets<AudioInstance>>,
    wind: Option<Res<WindState>>,
    settings: Option<Res<SettingsState>>,
) {
    let Some(gust) = gust else { return };

    let wind_speed = wind.map(|w| w.speed_mps).unwrap_or(3.0);
    let master     = settings.map(|s| s.master_volume).unwrap_or(0.7);

    // Volume: 0 at 1 m/s, ramps to 0.4 at 7 m/s.
    let vol_linear = ((wind_speed - 1.0) / 6.0 * 0.40 * master).clamp(0.0, 0.40);

    // Pitch: 0.85 at calm, 1.15 at strong wind — subtle howl shift.
    let rate = (0.85 + wind_speed / 50.0) as f64;

    let tween = AudioTween::linear(std::time::Duration::from_millis(120));

    if let Some(inst) = audio_instances.get_mut(&gust.instance) {
        inst.set_decibels(linear_to_db(vol_linear.max(1e-6)), tween);
        inst.set_playback_rate(rate, AudioTween::default());
    }
}

// ---------------------------------------------------------------------------
// Utility: linear amplitude to Decibels (floor at -60 dB)
// ---------------------------------------------------------------------------

#[inline]
fn linear_to_db(linear: f32) -> Decibels {
    let db = 20.0 * linear.max(1e-6).log10();
    Decibels(db.max(-60.0))
}
