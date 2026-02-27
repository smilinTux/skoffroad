use bevy::prelude::*;
use std::collections::VecDeque;

const FRAME_SIZE: usize = 480; // 10ms at 48kHz
const HISTORY_SIZE: usize = 50; // 500ms history
const ENERGY_THRESHOLD: f32 = -45.0; // dB
const HOLD_TIME: f32 = 0.3; // seconds
const ATTACK_TIME: f32 = 0.01; // seconds
const RELEASE_TIME: f32 = 0.15; // seconds

/// Voice Activity Detector for automatic voice detection
#[derive(Debug)]
pub struct VoiceActivityDetector {
    /// Energy history for adaptive thresholding
    energy_history: VecDeque<f32>,
    /// Current energy threshold
    threshold: f32,
    /// Last detection state
    last_state: bool,
    /// Time since last state change
    last_change: f32,
    /// Current smoothed energy
    current_energy: f32,
    /// Sample rate
    sample_rate: f32,
    /// Attack coefficient for smoothing
    attack_coeff: f32,
    /// Release coefficient for smoothing
    release_coeff: f32,
}

impl VoiceActivityDetector {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            energy_history: VecDeque::with_capacity(HISTORY_SIZE),
            threshold: ENERGY_THRESHOLD,
            last_state: false,
            last_change: 0.0,
            current_energy: -90.0,
            sample_rate,
            attack_coeff: 1.0 - (-1.0 / (sample_rate * ATTACK_TIME)).exp(),
            release_coeff: 1.0 - (-1.0 / (sample_rate * RELEASE_TIME)).exp(),
        }
    }

    /// Process a frame of audio and detect voice activity
    pub fn process_frame(&mut self, frame: &[f32], time: f32) -> bool {
        // Calculate frame energy in dB
        let energy = calculate_frame_energy(frame);
        
        // Update energy history
        self.energy_history.push_back(energy);
        if self.energy_history.len() > HISTORY_SIZE {
            self.energy_history.pop_front();
        }
        
        // Update smoothed energy with attack/release
        let coeff = if energy > self.current_energy {
            self.attack_coeff
        } else {
            self.release_coeff
        };
        self.current_energy += coeff * (energy - self.current_energy);
        
        // Update adaptive threshold
        if !self.energy_history.is_empty() {
            let noise_floor = self.energy_history.iter()
                .fold(f32::INFINITY, |a, &b| a.min(b));
            self.threshold = noise_floor + 15.0; // 15dB above noise floor
        }
        
        // Detect voice activity with hysteresis
        let is_active = self.current_energy > self.threshold;
        
        // Apply hold time
        if is_active != self.last_state {
            let time_since_change = time - self.last_change;
            if time_since_change < HOLD_TIME {
                return self.last_state;
            }
            self.last_change = time;
            self.last_state = is_active;
        }
        
        is_active
    }

    /// Get the current energy level in dB
    pub fn current_energy(&self) -> f32 {
        self.current_energy
    }

    /// Get the current threshold in dB
    pub fn threshold(&self) -> f32 {
        self.threshold
    }

    /// Reset the detector state
    pub fn reset(&mut self) {
        self.energy_history.clear();
        self.current_energy = -90.0;
        self.threshold = ENERGY_THRESHOLD;
        self.last_state = false;
        self.last_change = 0.0;
    }
}

/// Calculate the RMS energy of a frame in dB
fn calculate_frame_energy(frame: &[f32]) -> f32 {
    if frame.is_empty() {
        return -90.0;
    }
    
    let sum_squared: f32 = frame.iter()
        .map(|&x| x * x)
        .sum();
    
    let rms = (sum_squared / frame.len() as f32).sqrt();
    
    // Convert to dB, with -90dB floor
    20.0 * rms.max(1e-4).log10()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vad_silence() {
        let mut vad = VoiceActivityDetector::new(48000.0);
        let silence = vec![0.0f32; FRAME_SIZE];
        assert!(!vad.process_frame(&silence, 0.0));
    }

    #[test]
    fn test_vad_loud_signal() {
        let mut vad = VoiceActivityDetector::new(48000.0);
        let loud_signal = vec![0.5f32; FRAME_SIZE];
        assert!(vad.process_frame(&loud_signal, 0.0));
    }

    #[test]
    fn test_vad_hold_time() {
        let mut vad = VoiceActivityDetector::new(48000.0);
        let loud_signal = vec![0.5f32; FRAME_SIZE];
        let silence = vec![0.0f32; FRAME_SIZE];
        
        // Trigger voice detection
        assert!(vad.process_frame(&loud_signal, 0.0));
        
        // Should stay active during hold time
        assert!(vad.process_frame(&silence, 0.1));
        
        // Should deactivate after hold time
        assert!(!vad.process_frame(&silence, 0.5));
    }

    #[test]
    fn test_vad_adaptive_threshold() {
        let mut vad = VoiceActivityDetector::new(48000.0);
        let initial_threshold = vad.threshold();
        
        // Feed some background noise
        for i in 0..100 {
            let noise = vec![0.01f32; FRAME_SIZE];
            vad.process_frame(&noise, i as f32 * 0.01);
        }
        
        // Threshold should adapt
        assert!(vad.threshold() != initial_threshold);
    }
} 