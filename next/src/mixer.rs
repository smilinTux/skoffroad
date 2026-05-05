// Master audio mixer: ducking on hard impacts, master volume scaling.
//
// Architecture:
//   MixerState  — resource tracking ducking amount (0..1) and last computed dB.
//   detect_ducking  — walks EventLog each frame; on new HardImpact raises ducking.
//   apply_master_mix — decays ducking, computes effective volume, calls set_volume.
//
// Interaction with settings.rs:
//   settings.rs::apply_master_volume fires ONLY when SettingsState::is_changed().
//   This mixer runs every frame. During a duck, it sets a lower effective volume;
//   as ducking decays it climbs back to master_volume * 1.0. Settings wins again
//   the next time the user adjusts the slider (is_changed triggers, overwriting us
//   for one frame, after which we take over again). The net result is correct: the
//   duck dips below the user's chosen level and recovers to it.

use bevy::prelude::*;
use bevy_kira_audio::{Audio, AudioControl};
use bevy_kira_audio::prelude::Decibels;

use crate::events::{EventLog, GameEvent};
use crate::settings::SettingsState;

// ---------------------------------------------------------------------------
// Public resource
// ---------------------------------------------------------------------------

/// Tracks the current ducking coefficient and the last applied dB value.
#[derive(Resource, Default)]
pub struct MixerState {
    /// Ducking coefficient 0..1. 0 = no duck; 1 = fully muted.
    pub ducking: f32,
    /// Effective volume in dB last sent to kira (informational).
    pub master_db: f32,
}

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct MixerPlugin;

impl Plugin for MixerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MixerState>()
            .add_systems(Update, (
                detect_ducking,
                apply_master_mix,
            ).chain());
    }
}

// ---------------------------------------------------------------------------
// System: scan EventLog for new HardImpact events
// ---------------------------------------------------------------------------

fn detect_ducking(
    event_log: Option<Res<EventLog>>,
    mut mixer: ResMut<MixerState>,
    mut last_seen: Local<f32>,
) {
    let Some(event_log) = event_log else { return };

    let mut newest_ts = *last_seen;

    for (ts, ev) in &event_log.events {
        if *ts <= *last_seen {
            continue;
        }
        // Track newest event processed this frame.
        if *ts > newest_ts {
            newest_ts = *ts;
        }
        if let GameEvent::HardImpact { .. } = ev {
            // Duck by 60%; clamped so repeated impacts don't stack past 1.0.
            mixer.ducking = (mixer.ducking + 0.6).min(1.0);
        }
    }

    *last_seen = newest_ts;
}

// ---------------------------------------------------------------------------
// System: decay ducking and push effective volume to kira each frame
// ---------------------------------------------------------------------------

fn apply_master_mix(
    mut mixer: ResMut<MixerState>,
    settings: Option<Res<SettingsState>>,
    audio: Option<Res<Audio>>,
    time: Res<Time>,
) {
    // Decay ducking toward 0 — full recovery in ~0.5 s (rate = 2.0 per second).
    let dt = time.delta_secs();
    mixer.ducking = (mixer.ducking * (1.0 - dt * 2.0)).max(0.0);

    // Only push to kira when actively ducking; when ducking == 0 let settings.rs
    // own the channel so we don't fight it every frame at rest.
    if mixer.ducking < 1e-4 {
        return;
    }

    let Some(audio) = audio else { return };

    // Read master volume; fall back to 0.7 (settings default) if unavailable.
    let master_linear = settings
        .as_ref()
        .map(|s| s.master_volume)
        .unwrap_or(0.7);

    let effective_linear = master_linear * (1.0 - mixer.ducking);
    let db = linear_to_db(effective_linear);
    mixer.master_db = db.0;
    audio.set_volume(db);
}

// ---------------------------------------------------------------------------
// Utility
// ---------------------------------------------------------------------------

#[inline]
fn linear_to_db(linear: f32) -> Decibels {
    let db = 20.0 * linear.max(1e-6_f32).log10();
    Decibels(db.max(-60.0))
}
