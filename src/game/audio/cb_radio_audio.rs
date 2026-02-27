use bevy::prelude::*;
use bevy_kira_audio::{Audio, AudioControl};
use crate::game::audio::cb_radio::{CBRadioState, CBRadioVolume, MAX_RADIO_RANGE, MIN_SIGNAL_STRENGTH};

pub struct CBRadioAudioPlugin;

impl Plugin for CBRadioAudioPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (
            process_radio_audio,
            update_signal_strength,
        ));
    }
}

// Process radio audio including effects and transmission
fn process_radio_audio(
    mut radio_state: ResMut<CBRadioState>,
    volume: Res<CBRadioVolume>,
    audio: Res<Audio>,
    time: Res<Time>,
) {
    if radio_state.is_transmitting {
        apply_transmission_effects(&audio, volume.0);
    } else if radio_state.is_receiving {
        apply_reception_effects(&audio, volume.0, radio_state.signal_strength);
    }

    // Add background static based on signal strength
    let static_volume = ((1.0 - radio_state.signal_strength) * 0.3) * volume.0;
    if static_volume > 0.01 {
        play_static_effect(&audio, static_volume);
    }
}

// Update signal strength based on distance and interference
fn update_signal_strength(
    mut radio_state: ResMut<CBRadioState>,
    // TODO: Add parameters for distance calculation between radio users
) {
    // Placeholder: Calculate signal strength based on distance
    // This will be implemented when the multiplayer system is in place
    let distance = 0.0; // Replace with actual distance calculation
    
    // Linear degradation of signal strength based on distance
    let signal_strength = 1.0 - (distance / MAX_RADIO_RANGE).clamp(0.0, 1.0);
    radio_state.signal_strength = signal_strength.max(MIN_SIGNAL_STRENGTH);
}

// Apply audio effects for radio transmission
fn apply_transmission_effects(audio: &Audio, volume: f32) {
    // Play transmission start tone
    audio.play_sfx("radio_start.ogg").with_volume(volume);
    
    // Apply radio filter effect to voice
    // TODO: Implement voice chat audio processing
}

// Apply audio effects for radio reception
fn apply_reception_effects(audio: &Audio, volume: f32, signal_strength: f32) {
    // Apply signal degradation effects
    let effect_intensity = 1.0 - signal_strength;
    
    // Add noise and interference based on signal strength
    if effect_intensity > 0.2 {
        audio.play_sfx("radio_interference.ogg")
            .with_volume(effect_intensity * volume);
    }
}

// Play radio static background noise
fn play_static_effect(audio: &Audio, volume: f32) {
    audio.play_sfx("radio_static.ogg")
        .with_volume(volume)
        .looped();
} 