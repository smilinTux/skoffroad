use bevy::prelude::*;
use dasp::{Frame, Sample};
use dasp_filter::{BiquadFilter, Type as FilterType};
use dasp_signal::{self as signal, Signal};
use super::cb_radio::SignalQuality;

/// Configuration for the CB radio filter chain
#[derive(Debug, Clone)]
pub struct CBFilterChainConfig {
    /// Sample rate in Hz
    pub sample_rate: u32,
    /// Base frequency for carrier simulation
    pub carrier_freq: f32,
    /// Minimum cutoff frequency for bandpass
    pub min_cutoff: f32,
    /// Maximum cutoff frequency for bandpass
    pub max_cutoff: f32,
    /// Q factor for filters
    pub q_factor: f32,
    /// Noise gate threshold in dB
    pub noise_gate_threshold: f32,
    /// Compression ratio
    pub compression_ratio: f32,
    /// Compression threshold in dB
    pub compression_threshold: f32,
}

impl Default for CBFilterChainConfig {
    fn default() -> Self {
        Self {
            sample_rate: 48000,
            carrier_freq: 27000.0,
            min_cutoff: 300.0,
            max_cutoff: 3000.0,
            q_factor: 0.707,
            noise_gate_threshold: -60.0,
            compression_ratio: 4.0,
            compression_threshold: -20.0,
        }
    }
}

/// Complete filter chain for CB radio effects
#[derive(Debug)]
pub struct CBFilterChain {
    /// Pre-emphasis filter to boost high frequencies
    pre_emphasis: BiquadFilter<f32>,
    /// High-pass filter to remove DC and very low frequencies
    high_pass: BiquadFilter<f32>,
    /// Low-pass filter to simulate bandwidth limitation
    low_pass: BiquadFilter<f32>,
    /// Band-pass filter for carrier simulation
    carrier_filter: BiquadFilter<f32>,
    /// De-emphasis filter to restore frequency balance
    de_emphasis: BiquadFilter<f32>,
    /// Configuration
    config: CBFilterChainConfig,
    /// Current signal quality
    signal_quality: SignalQuality,
    /// Compression state
    compressor: Compressor,
    /// Noise gate state
    noise_gate: NoiseGate,
}

impl CBFilterChain {
    pub fn new(config: CBFilterChainConfig) -> Self {
        let sample_rate = config.sample_rate as f32;
        
        Self {
            pre_emphasis: BiquadFilter::new(
                FilterType::HighShelf,
                sample_rate,
                2000.0,
                config.q_factor
            ),
            high_pass: BiquadFilter::new(
                FilterType::HighPass,
                sample_rate,
                config.min_cutoff,
                config.q_factor
            ),
            low_pass: BiquadFilter::new(
                FilterType::LowPass,
                sample_rate,
                config.max_cutoff,
                config.q_factor
            ),
            carrier_filter: BiquadFilter::new(
                FilterType::BandPass,
                sample_rate,
                config.carrier_freq,
                1.0
            ),
            de_emphasis: BiquadFilter::new(
                FilterType::LowShelf,
                sample_rate,
                2000.0,
                config.q_factor
            ),
            config,
            signal_quality: SignalQuality::default(),
            compressor: Compressor::new(
                config.compression_threshold,
                config.compression_ratio,
                sample_rate
            ),
            noise_gate: NoiseGate::new(
                config.noise_gate_threshold,
                sample_rate
            ),
        }
    }

    /// Process a single audio sample through the filter chain
    pub fn process(&mut self, sample: f32) -> f32 {
        // Apply noise gate first
        let gated = self.noise_gate.process(sample);
        
        // Apply filter chain
        let mut processed = gated;
        processed = self.pre_emphasis.run(processed);
        processed = self.high_pass.run(processed);
        processed = self.carrier_filter.run(processed);
        processed = self.low_pass.run(processed);
        processed = self.de_emphasis.run(processed);
        
        // Apply compression last
        self.compressor.process(processed)
    }

    /// Update filter parameters based on signal quality
    pub fn update_parameters(&mut self, quality: SignalQuality) {
        self.signal_quality = quality;
        
        // Calculate filter parameters based on signal quality
        let clarity_factor = quality.clarity.powf(1.5);
        let interference_factor = quality.interference_level;
        
        // Adjust filter cutoffs
        let high_pass_freq = self.config.min_cutoff * (1.0 + interference_factor * 0.5);
        let low_pass_freq = self.config.max_cutoff * clarity_factor.max(0.3);
        
        // Update filter coefficients
        let sample_rate = self.config.sample_rate as f32;
        self.high_pass = BiquadFilter::new(
            FilterType::HighPass,
            sample_rate,
            high_pass_freq,
            self.config.q_factor
        );
        self.low_pass = BiquadFilter::new(
            FilterType::LowPass,
            sample_rate,
            low_pass_freq,
            self.config.q_factor
        );
        
        // Update dynamics processors
        self.noise_gate.threshold = self.config.noise_gate_threshold + 
            (interference_factor * 20.0);
        self.compressor.ratio = self.config.compression_ratio +
            ((1.0 - clarity_factor) * 2.0);
    }
}

/// Compressor for dynamic range control
#[derive(Debug)]
struct Compressor {
    threshold: f32,
    ratio: f32,
    attack_time: f32,
    release_time: f32,
    envelope: f32,
    sample_rate: f32,
}

impl Compressor {
    fn new(threshold_db: f32, ratio: f32, sample_rate: u32) -> Self {
        Self {
            threshold: 10.0f32.powf(threshold_db / 20.0),
            ratio,
            attack_time: 0.005, // 5ms attack
            release_time: 0.050, // 50ms release
            envelope: 0.0,
            sample_rate: sample_rate as f32,
        }
    }

    fn process(&mut self, sample: f32) -> f32 {
        let input_level = sample.abs();
        
        // Envelope follower
        let coeff = if input_level > self.envelope {
            1.0 - (-1.0 / (self.sample_rate * self.attack_time)).exp()
        } else {
            1.0 - (-1.0 / (self.sample_rate * self.release_time)).exp()
        };
        
        self.envelope += coeff * (input_level - self.envelope);
        
        // Apply compression
        if self.envelope > self.threshold {
            let gain_reduction = (self.envelope / self.threshold).powf(1.0 / self.ratio - 1.0);
            sample * gain_reduction
        } else {
            sample
        }
    }
}

/// Noise gate for noise reduction
#[derive(Debug)]
struct NoiseGate {
    threshold: f32,
    envelope: f32,
    sample_rate: f32,
    attack_time: f32,
    release_time: f32,
}

impl NoiseGate {
    fn new(threshold_db: f32, sample_rate: u32) -> Self {
        Self {
            threshold: 10.0f32.powf(threshold_db / 20.0),
            envelope: 0.0,
            sample_rate: sample_rate as f32,
            attack_time: 0.001, // 1ms attack
            release_time: 0.100, // 100ms release
        }
    }

    fn process(&mut self, sample: f32) -> f32 {
        let input_level = sample.abs();
        
        // Envelope follower
        let coeff = if input_level > self.envelope {
            1.0 - (-1.0 / (self.sample_rate * self.attack_time)).exp()
        } else {
            1.0 - (-1.0 / (self.sample_rate * self.release_time)).exp()
        };
        
        self.envelope += coeff * (input_level - self.envelope);
        
        // Apply gate
        if self.envelope > self.threshold {
            sample
        } else {
            0.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_chain_creation() {
        let config = CBFilterChainConfig::default();
        let chain = CBFilterChain::new(config);
        assert!(chain.config.sample_rate > 0);
    }

    #[test]
    fn test_signal_quality_update() {
        let config = CBFilterChainConfig::default();
        let mut chain = CBFilterChain::new(config);
        
        let quality = SignalQuality {
            strength: 0.8,
            clarity: 0.9,
            interference_level: 0.1,
            noise_floor: -90.0,
        };
        
        chain.update_parameters(quality);
        assert_eq!(chain.signal_quality.strength, 0.8);
    }

    #[test]
    fn test_audio_processing() {
        let config = CBFilterChainConfig::default();
        let mut chain = CBFilterChain::new(config);
        
        // Process a simple sine wave
        let frequency = 1000.0;
        let sample_rate = config.sample_rate as f32;
        
        for i in 0..1000 {
            let t = i as f32 / sample_rate;
            let sample = (t * frequency * 2.0 * std::f32::consts::PI).sin();
            let processed = chain.process(sample);
            assert!(processed.abs() <= 1.0);
        }
    }
} 