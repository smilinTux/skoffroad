//! Sample-based engine audio — VNS-style RPM crossfade.
//!
//! This is the "authentic" engine-audio path: instead of synthesizing the
//! engine note (audio.rs), it plays a small set of *recorded* engine loops at
//! different RPMs and blends between the two nearest with a constant-power
//! crossfade, pitch-shifting each by the RPM ratio. This is the same technique
//! used by the open-source VehicleNoiseSynthesizer (MIT) and by most shipping
//! driving games — it sounds real because the source material is real.
//!
//! Gated behind the `engine_samples` cargo feature so the DEFAULT build is
//! byte-for-byte the synthesized engine from audio.rs (zero regression):
//!   * feature OFF (default): this plugin does nothing; audio.rs synth runs.
//!   * feature ON: this plugin loads assets/audio/engine/{idle,mid,high}.wav,
//!     crossfades them by RPM, and audio.rs mutes its synth engine (it checks
//!     for the `EngineSamplesActive` resource this plugin inserts).
//!
//! Build with samples:   cargo run --features engine_samples
//! (web)  trunk build/serve with the feature, e.g. `--features engine_samples`.
//!
//! Drop your own CC0 loops in to replace the starter WAVs — see docs/ASSETS.md.
//! Filenames are fixed (idle.wav / mid.wav / high.wav); `.ogg` also works once
//! you swap the extension in PATHS below and ship ogg files (ogg is enabled by
//! default in bevy_kira_audio; this feature additionally enables the wav loader).

use bevy::prelude::*;

/// Marker resource: present only when the sample-based engine is active.
/// audio.rs reads `Option<Res<EngineSamplesActive>>` and mutes its synthesized
/// engine voice when this exists, so the two never play at once.
#[derive(Resource)]
pub struct EngineSamplesActive;

pub struct EngineSamplesPlugin;

impl Plugin for EngineSamplesPlugin {
    fn build(&self, app: &mut App) {
        #[cfg(feature = "engine_samples")]
        {
            app.insert_resource(EngineSamplesActive)
                .add_systems(Startup, imp::start_engine_loops)
                .add_systems(Update, imp::modulate_engine_loops);
        }
        #[cfg(not(feature = "engine_samples"))]
        {
            let _ = app; // no-op: synthesized engine in audio.rs stays in charge.
        }
    }
}

#[cfg(feature = "engine_samples")]
mod imp {
    use bevy::prelude::*;
    use bevy_kira_audio::{Audio, AudioControl, AudioInstance, AudioSource, AudioTween};
    use bevy_kira_audio::prelude::Decibels;
    use std::time::Duration;

    use crate::engine_torque::EngineState;

    /// Engine loops, ascending by the RPM they were recorded/baked at.
    const PATHS: [&str; 3] = [
        "audio/engine/idle.wav",
        "audio/engine/mid.wav",
        "audio/engine/high.wav",
    ];
    /// RPM each loop represents. Must match PATHS order and be strictly ascending.
    const ANCHORS: [f32; 3] = [700.0, 2600.0, 5000.0];
    /// Overall engine loudness ceiling (linear).
    const MASTER: f32 = 0.8;
    /// Clamp pitch so a loop is never stretched past sensible bounds.
    const RATE_MIN: f32 = 0.55;
    const RATE_MAX: f32 = 2.2;

    #[derive(Resource)]
    pub struct EngineLoops {
        instances: Vec<Handle<AudioInstance>>,
    }

    fn linear_to_db(linear: f32) -> Decibels {
        let db = 20.0 * linear.max(1e-6).log10();
        Decibels(db.max(-60.0))
    }

    /// Startup: load each loop and begin it muted+looped. Headless-safe: the
    /// `Audio` resource only exists when bevy_kira_audio's backend is present.
    pub fn start_engine_loops(
        mut commands: Commands,
        asset_server: Option<Res<AssetServer>>,
        audio: Option<Res<Audio>>,
    ) {
        let (Some(asset_server), Some(audio)) = (asset_server, audio) else { return };

        let mut instances = Vec::with_capacity(PATHS.len());
        for path in PATHS {
            let source: Handle<AudioSource> = asset_server.load(path);
            let instance = audio
                .play(source)
                .looped()
                .with_volume(linear_to_db(0.0))
                .with_playback_rate(1.0_f64)
                .handle();
            instances.push(instance);
        }
        commands.insert_resource(EngineLoops { instances });
        info!(
            "engine_samples: {} engine loops loaded — sample-based engine active",
            PATHS.len()
        );
    }

    /// Update: constant-power crossfade + per-loop pitch by current RPM.
    pub fn modulate_engine_loops(
        engine: Option<Res<EngineState>>,
        loops: Option<Res<EngineLoops>>,
        mut audio_instances: ResMut<Assets<AudioInstance>>,
    ) {
        let Some(loops) = loops else { return };

        let n = ANCHORS.len();
        let rpm = engine.map(|e| e.rpm).unwrap_or(ANCHORS[0]).clamp(ANCHORS[0], ANCHORS[n - 1]);

        // Find the bracketing pair [lo, hi] for the current RPM.
        let mut hi = n - 1;
        for i in 1..n {
            if rpm <= ANCHORS[i] {
                hi = i;
                break;
            }
        }
        let lo = hi.saturating_sub(1);
        let span = (ANCHORS[hi] - ANCHORS[lo]).max(1.0);
        let f = ((rpm - ANCHORS[lo]) / span).clamp(0.0, 1.0);

        // Constant-power (equal-energy) crossfade: sin^2 + cos^2 = 1.
        let mut weights = [0.0f32; 3];
        if lo == hi {
            weights[lo] = 1.0;
        } else {
            let theta = f * std::f32::consts::FRAC_PI_2;
            weights[lo] = theta.cos();
            weights[hi] = theta.sin();
        }

        let vol_tween = AudioTween::linear(Duration::from_millis(40));
        let pitch_tween = AudioTween::linear(Duration::from_millis(40));

        for i in 0..n {
            if let Some(instance) = audio_instances.get_mut(&loops.instances[i]) {
                let rate = (rpm / ANCHORS[i]).clamp(RATE_MIN, RATE_MAX);
                instance.set_decibels(linear_to_db(weights[i] * MASTER), vol_tween.clone());
                instance.set_playback_rate(rate as f64, pitch_tween.clone());
            }
        }
    }
}
