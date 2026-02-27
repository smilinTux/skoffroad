use bevy::prelude::*;
use dasp::{Frame, Sample};
use dasp_filter::{BiquadFilter, Type as FilterType};
use dasp_signal::{self as signal, Signal};
use super::cb_radio::{SignalQuality, AudioEffects};

/// Configuration for CB radio filter effects
#[derive(Debug, Clone)]
pub struct CBFilterConfig {
    /// Sample rate for audio processing
    pub sample_rate: u32,
    /// Base frequency for carrier tone
    pub carrier_freq: f32,
    /// Squelch threshold in dB
    pub squelch_threshold: f32,
    /// Noise floor baseline in dB
    pub noise_floor: f32,
    /// Maximum bandwidth in Hz
    pub max_bandwidth: f32,
}

impl Default for CBFilterConfig {
    fn default() -> Self {
        Self {
            sample_rate: 48000,
            carrier_freq: 27000.0, // 27 MHz
            squelch_threshold: -60.0,
            noise_floor: -90.0,
            max_bandwidth: 4000.0,
        }
    }
}

/// Dynamic filter chain for CB radio effects
#[derive(Debug)]
pub struct CBFilterChain {
    /// Pre-emphasis filter
    pre_emphasis: BiquadFilter<f32>,
    /// High-pass filter
    high_pass: BiquadFilter<f32>,
    /// Low-pass filter
    low_pass: BiquadFilter<f32>,
    /// De-emphasis filter
    de_emphasis: BiquadFilter<f32>,
    /// Noise gate state
    noise_gate_state: NoiseGateState,
    /// Current signal quality
    signal_quality: SignalQuality,
    /// Configuration
    config: CBFilterConfig,
}

#[derive(Debug)]
struct NoiseGateState {
    threshold: f32,
    attack_time: f32,
    release_time: f32,
    current_gain: f32,
    hold_counter: usize,
    hold_samples: usize,
}

impl CBFilterChain {
    pub fn new(config: CBFilterConfig) -> Self {
        let sample_rate = config.sample_rate as f32;
        
        Self {
            pre_emphasis: BiquadFilter::new(
                FilterType::HighShelf,
                sample_rate,
                2000.0,
                0.707
            ),
            high_pass: BiquadFilter::new(
                FilterType::HighPass,
                sample_rate,
                300.0,
                0.707
            ),
            low_pass: BiquadFilter::new(
                FilterType::LowPass,
                sample_rate,
                3000.0,
                0.707
            ),
            de_emphasis: BiquadFilter::new(
                FilterType::LowShelf,
                sample_rate,
                1000.0,
                0.707
            ),
            noise_gate_state: NoiseGateState {
                threshold: config.squelch_threshold,
                attack_time: 0.002, // 2ms attack
                release_time: 0.1,  // 100ms release
                current_gain: 0.0,
                hold_counter: 0,
                hold_samples: (0.05 * sample_rate) as usize, // 50ms hold
            },
            signal_quality: SignalQuality::default(),
            config,
        }
    }

    /// Update filter parameters based on signal quality
    pub fn update_parameters(&mut self, signal_quality: SignalQuality) {
        self.signal_quality = signal_quality;
        let sample_rate = self.config.sample_rate as f32;

        // Calculate filter parameters based on signal quality
        let clarity = signal_quality.clarity.powf(1.5); // Emphasize clarity changes
        let strength = signal_quality.strength.powf(2.0); // Emphasize strength changes
        
        // Adjust bandwidth based on signal quality
        let bandwidth = self.config.max_bandwidth * clarity;
        let high_pass_freq = 200.0 + (1.0 - strength) * 300.0;
        let low_pass_freq = bandwidth.min(4000.0);

        // Update filter coefficients
        self.high_pass = BiquadFilter::new(
            FilterType::HighPass,
            sample_rate,
            high_pass_freq,
            0.707
        );
        
        self.low_pass = BiquadFilter::new(
            FilterType::LowPass,
            sample_rate,
            low_pass_freq,
            0.707
        );

        // Adjust pre/de-emphasis based on interference
        let emphasis_gain = 1.0 + signal_quality.interference_level * 3.0; // Up to 4x emphasis
        self.pre_emphasis = BiquadFilter::new(
            FilterType::HighShelf,
            sample_rate,
            2000.0,
            emphasis_gain
        );
        
        self.de_emphasis = BiquadFilter::new(
            FilterType::LowShelf,
            sample_rate,
            1000.0,
            1.0 / emphasis_gain
        );

        // Update noise gate threshold based on noise floor
        self.noise_gate_state.threshold = self.config.squelch_threshold + 
            (signal_quality.noise_floor * 20.0);
    }

    /// Process a buffer of audio samples
    pub fn process_buffer(&mut self, buffer: &mut [f32]) {
        for sample in buffer.iter_mut() {
            *sample = self.process_sample(*sample);
        }
    }

    /// Process a single audio sample
    pub fn process_sample(&mut self, mut sample: f32) -> f32 {
        // Apply pre-emphasis
        sample = self.pre_emphasis.run(sample);

        // Apply main filters
        sample = self.high_pass.run(sample);
        sample = self.low_pass.run(sample);

        // Apply noise gate
        sample = self.apply_noise_gate(sample);

        // Add noise based on interference
        sample = self.add_interference_noise(sample);

        // Apply de-emphasis
        sample = self.de_emphasis.run(sample);

        sample.clamp(-1.0, 1.0)
    }

    fn apply_noise_gate(&mut self, sample: f32) -> f32 {
        let level = sample.abs();
        let ng = &mut self.noise_gate_state;

        // Calculate target gain
        let target_gain = if level > ng.threshold.exp10() {
            1.0
        } else {
            0.0
        };

        // Update hold counter
        if target_gain > ng.current_gain {
            ng.hold_counter = ng.hold_samples;
        } else if ng.hold_counter > 0 {
            ng.hold_counter -= 1;
        }

        // Calculate coefficient for smoothing
        let coef = if target_gain > ng.current_gain {
            1.0 - (-1.0 / (self.config.sample_rate as f32 * ng.attack_time)).exp()
        } else if ng.hold_counter == 0 {
            1.0 - (-1.0 / (self.config.sample_rate as f32 * ng.release_time)).exp()
        } else {
            0.0
        };

        // Smooth gain changes
        ng.current_gain += (target_gain - ng.current_gain) * coef;

        sample * ng.current_gain
    }

    fn add_interference_noise(&self, sample: f32) -> f32 {
        let interference = self.signal_quality.interference_level;
        if interference > 0.0 {
            let noise = (fastrand::f32() * 2.0 - 1.0) * interference * 0.3;
            (sample + noise).clamp(-1.0, 1.0)
        } else {
            sample
        }
    }

    /// Get current audio effects parameters
    pub fn get_audio_effects(&self) -> AudioEffects {
        AudioEffects {
            volume: self.signal_quality.strength,
            high_pass: 200.0 + (1.0 - self.signal_quality.clarity) * 300.0,
            low_pass: self.config.max_bandwidth * self.signal_quality.clarity,
            noise_gain: self.signal_quality.interference_level * 0.5,
            distortion: (1.0 - self.signal_quality.clarity) * 0.3,
        }
    }
}

/// Plugin for CB radio filter effects
pub struct CBFilterPlugin;

impl Plugin for CBFilterPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CBFilterConfig>()
           .add_systems(Update, update_cb_filters);
    }
}

/// System to update CB radio filters
fn update_cb_filters(
    mut query: Query<(&mut CBFilterChain, &SignalQuality)>,
) {
    for (mut filter_chain, signal_quality) in query.iter_mut() {
        filter_chain.update_parameters(*signal_quality);
    }
} 