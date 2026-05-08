use super::super::mixer::{AudioBus, AudioMixer, AudioRouting};
use bevy::prelude::*;

#[test]
fn test_audio_routing_configuration() {
    let mut routing = AudioRouting::default();
    
    // Test default volumes
    assert_eq!(routing.get_volume(AudioBus::Master), 1.0);
    assert_eq!(routing.get_volume(AudioBus::Radio), 0.8);
    assert_eq!(routing.get_volume(AudioBus::VoiceChat), 1.0);
    assert_eq!(routing.get_volume(AudioBus::Effects), 0.7);
    assert_eq!(routing.get_volume(AudioBus::Music), 0.5);
    assert_eq!(routing.get_volume(AudioBus::Ambient), 0.4);
    
    // Test volume clamping
    routing.set_volume(AudioBus::Master, 1.5);
    assert_eq!(routing.get_volume(AudioBus::Master), 1.0);
    
    routing.set_volume(AudioBus::Radio, -0.5);
    assert_eq!(routing.get_volume(AudioBus::Radio), 0.0);
}

#[test]
fn test_audio_routing_muting() {
    let mut routing = AudioRouting::default();
    
    // Test initial mute states
    assert!(!routing.is_muted(AudioBus::Master));
    assert!(!routing.is_muted(AudioBus::Radio));
    
    // Test muting and unmuting
    routing.set_mute(AudioBus::Radio, true);
    assert!(routing.is_muted(AudioBus::Radio));
    
    routing.set_mute(AudioBus::Radio, false);
    assert!(!routing.is_muted(AudioBus::Radio));
}

#[test]
fn test_audio_routing_sends() {
    let mut routing = AudioRouting::default();
    
    // Test adding sends
    routing.add_send(AudioBus::VoiceChat, AudioBus::Radio, 0.8);
    routing.add_send(AudioBus::Radio, AudioBus::Master, 0.9);
    
    // Test path gain calculation
    let gain = routing.get_path_gain(AudioBus::VoiceChat, AudioBus::Master);
    let expected_gain = 0.8 * 0.9 * routing.get_volume(AudioBus::VoiceChat) 
        * routing.get_volume(AudioBus::Radio) 
        * routing.get_volume(AudioBus::Master);
    assert!((gain - expected_gain).abs() < 0.001);
    
    // Test removing sends
    routing.remove_send(AudioBus::VoiceChat, AudioBus::Radio);
    assert_eq!(routing.get_path_gain(AudioBus::VoiceChat, AudioBus::Master), 0.0);
}

#[test]
fn test_audio_mixer_processing() {
    let mut mixer = AudioMixer::default();
    let time = 0.0;
    
    // Test processing with default settings
    let mut samples = vec![0.5f32; 1000];
    let original_samples = samples.clone();
    
    mixer.process_audio(AudioBus::Music, &mut samples, time);
    
    // Check that samples were attenuated by the Music bus volume (0.5)
    for (processed, original) in samples.iter().zip(original_samples.iter()) {
        assert!((processed - original * 0.5).abs() < 0.001);
    }
    
    // Test level metering
    assert!(mixer.get_level(AudioBus::Music) > 0.0);
    assert!(mixer.get_peak_level(AudioBus::Music) > 0.0);
}

#[test]
fn test_audio_mixer_peak_hold() {
    let mut mixer = AudioMixer::default();
    
    // Process some audio at t=0
    let mut samples = vec![0.8f32; 100];
    mixer.process_audio(AudioBus::Effects, &mut samples, 0.0);
    let initial_peak = mixer.get_peak_level(AudioBus::Effects);
    
    // Process quieter audio at t=1.0
    let mut samples = vec![0.2f32; 100];
    mixer.process_audio(AudioBus::Effects, &mut samples, 1.0);
    
    // Peak should still be held
    assert_eq!(mixer.get_peak_level(AudioBus::Effects), initial_peak);
    
    // Process after peak hold time
    mixer.process_audio(AudioBus::Effects, &mut samples, 3.0);
    
    // Peak should be reset
    assert!(mixer.get_peak_level(AudioBus::Effects) < initial_peak);
}

#[test]
fn test_complex_routing_path() {
    let mut routing = AudioRouting::default();
    
    // Set up a more complex routing path
    routing.add_send(AudioBus::VoiceChat, AudioBus::Radio, 0.8);    // Voice -> Radio
    routing.add_send(AudioBus::Radio, AudioBus::Effects, 0.7);      // Radio -> Effects
    routing.add_send(AudioBus::Effects, AudioBus::Master, 0.9);     // Effects -> Master
    
    // Calculate expected gain through the path
    let expected_gain = 0.8 * 0.7 * 0.9 
        * routing.get_volume(AudioBus::VoiceChat)
        * routing.get_volume(AudioBus::Radio)
        * routing.get_volume(AudioBus::Effects)
        * routing.get_volume(AudioBus::Master);
    
    let actual_gain = routing.get_path_gain(AudioBus::VoiceChat, AudioBus::Master);
    assert!((actual_gain - expected_gain).abs() < 0.001);
    
    // Test muting in the middle of the path
    routing.set_mute(AudioBus::Radio, true);
    assert_eq!(routing.get_path_gain(AudioBus::VoiceChat, AudioBus::Master), 0.0);
}

#[test]
fn test_circular_routing_prevention() {
    let mut routing = AudioRouting::default();
    
    // Set up a potential circular route
    routing.add_send(AudioBus::VoiceChat, AudioBus::Radio, 0.8);
    routing.add_send(AudioBus::Radio, AudioBus::Effects, 0.7);
    routing.add_send(AudioBus::Effects, AudioBus::VoiceChat, 0.6);
    
    // Attempting to calculate gain through a circular path should not hang
    let gain = routing.get_path_gain(AudioBus::VoiceChat, AudioBus::Master);
    assert_eq!(gain, 0.0); // Should return 0 for invalid path
}

#[test]
fn test_mixer_level_tracking() {
    let mut mixer = AudioMixer::default();
    
    // Process increasing amplitudes
    for i in 0..5 {
        let amplitude = (i as f32 + 1.0) * 0.2;
        let mut samples = vec![amplitude; 100];
        mixer.process_audio(AudioBus::Music, &mut samples, i as f32);
        
        // Check that level tracking follows the highest amplitude
        assert!((mixer.get_level(AudioBus::Music) - amplitude).abs() < 0.001);
    }
} 