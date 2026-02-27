use super::*;
use crate::game::audio::voice_chat::*;
use crate::game::audio::cb_radio::SignalQuality;

#[test]
fn test_voice_chat_filter_chain_integration() {
    let config = VoiceChatConfig {
        sample_rate: 48000.0,
        channels: 1,
        buffer_size: 480,
    };
    let mut voice_chat = VoiceChat::new(config);
    
    // Test with perfect signal quality
    voice_chat.update_signal_quality(SignalQuality {
        strength: 1.0,
        noise: 0.0,
        interference: 0.0,
    });
    
    let input_samples = vec![0.5f32; 480];
    let processed = voice_chat.process_voice_input(&input_samples);
    
    // Verify output length matches input
    assert_eq!(processed.len(), input_samples.len());
    
    // With perfect signal, output should be similar to input but not identical
    // due to filter chain processing
    let avg_difference: f32 = processed.iter()
        .zip(input_samples.iter())
        .map(|(a, b)| (a - b).abs())
        .sum::<f32>() / processed.len() as f32;
    assert!(avg_difference < 0.1);
    
    // Test with poor signal quality
    voice_chat.update_signal_quality(SignalQuality {
        strength: 0.2,
        noise: 0.8,
        interference: 0.6,
    });
    
    let processed_poor = voice_chat.process_voice_input(&input_samples);
    
    // Verify output is more distorted with poor signal
    let avg_difference_poor: f32 = processed_poor.iter()
        .zip(input_samples.iter())
        .map(|(a, b)| (a - b).abs())
        .sum::<f32>() / processed_poor.len() as f32;
    assert!(avg_difference_poor > avg_difference);
}

#[test]
fn test_voice_chat_output_processing() {
    let config = VoiceChatConfig {
        sample_rate: 48000.0,
        channels: 1,
        buffer_size: 480,
    };
    let mut voice_chat = VoiceChat::new(config);
    
    // Create a test voice packet
    let packet = VoicePacket {
        samples: vec![0.5f32; 480],
        timestamp: SystemTime::now(),
        channel: 1,
    };
    
    // Process with good signal
    voice_chat.update_signal_quality(SignalQuality {
        strength: 1.0,
        noise: 0.0,
        interference: 0.0,
    });
    
    let processed = voice_chat.process_voice_output(packet.clone());
    assert_eq!(processed.len(), packet.samples.len());
    
    // Process with poor signal
    voice_chat.update_signal_quality(SignalQuality {
        strength: 0.2,
        noise: 0.8,
        interference: 0.6,
    });
    
    let processed_poor = voice_chat.process_voice_output(packet);
    assert_eq!(processed_poor.len(), packet.samples.len());
    
    // Verify more distortion with poor signal
    let avg_difference: f32 = processed_poor.iter()
        .zip(processed.iter())
        .map(|(a, b)| (a - b).abs())
        .sum::<f32>() / processed_poor.len() as f32;
    assert!(avg_difference > 0.1);
} 