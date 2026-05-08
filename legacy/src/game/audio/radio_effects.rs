use rand::prelude::*;
use dasp::{Frame, Sample};
use dasp_filter::{BiquadFilter, Type as FilterType};
use dasp_interpolate::linear::Linear;
use dasp_signal::{self as signal, Signal};
use bevy::prelude::*;
use std::f32::consts::PI;

/// Configuration for radio audio effects
#[derive(Debug)]
pub struct RadioEffects {
    /// Low-pass filter cutoff frequency (Hz)
    pub low_pass_cutoff: f32,
    /// High-pass filter cutoff frequency (Hz)
    pub high_pass_cutoff: f32,
    /// Amount of noise to add (0.0 - 1.0)
    pub noise_level: f32,
    /// Amount of distortion to add (0.0 - 1.0)
    pub distortion: f32,
    /// Sample rate for audio processing
    pub sample_rate: u32,
    
    /// Internal state
    low_pass_filter: BiquadFilter<f32>,
    high_pass_filter: BiquadFilter<f32>,
    rng: ThreadRng,
}

impl Default for RadioEffects {
    fn default() -> Self {
        let sample_rate = 44100;
        Self {
            low_pass_cutoff: 3000.0,
            high_pass_cutoff: 300.0,
            noise_level: 0.1,
            distortion: 0.2,
            sample_rate,
            low_pass_filter: BiquadFilter::new(FilterType::LowPass, sample_rate as f32, 3000.0, 0.707),
            high_pass_filter: BiquadFilter::new(FilterType::HighPass, sample_rate as f32, 300.0, 0.707),
            rng: thread_rng(),
        }
    }
}

impl RadioEffects {
    pub fn new(sample_rate: u32) -> Self {
        Self {
            sample_rate,
            low_pass_filter: BiquadFilter::new(FilterType::LowPass, sample_rate as f32, 3000.0, 0.707),
            high_pass_filter: BiquadFilter::new(FilterType::HighPass, sample_rate as f32, 300.0, 0.707),
            ..Default::default()
        }
    }

    /// Process a buffer of audio samples with radio effects
    pub fn process_buffer(&mut self, buffer: &mut [f32]) {
        for sample in buffer.iter_mut() {
            *sample = self.process_sample(*sample);
        }
    }

    /// Process a single audio sample with radio effects
    pub fn process_sample(&mut self, sample: f32) -> f32 {
        // Add noise
        let noise = (self.rng.gen::<f32>() * 2.0 - 1.0) * self.noise_level;
        let mut processed = sample + noise;

        // Apply distortion
        if self.distortion > 0.0 {
            processed = self.apply_distortion(processed);
        }

        // Apply filters
        processed = self.low_pass_filter.run(processed);
        processed = self.high_pass_filter.run(processed);

        processed.clamp(-1.0, 1.0)
    }

    /// Update effects based on signal quality
    pub fn update_effects(&mut self, signal_quality: f32) {
        // Adjust effects based on signal quality (0.0 to 1.0)
        self.noise_level = (1.0 - signal_quality) * 0.5;
        self.distortion = (1.0 - signal_quality) * 0.3;
        
        // Update filter parameters
        let lp_cutoff = signal_quality.mul_add(2000.0, 1000.0); // 1000Hz to 3000Hz
        let hp_cutoff = signal_quality.mul_add(-100.0, 400.0);  // 400Hz to 300Hz
        
        self.low_pass_filter = BiquadFilter::new(
            FilterType::LowPass,
            self.sample_rate as f32,
            lp_cutoff,
            0.707
        );
        
        self.high_pass_filter = BiquadFilter::new(
            FilterType::HighPass,
            self.sample_rate as f32,
            hp_cutoff,
            0.707
        );
    }

    fn apply_distortion(&self, sample: f32) -> f32 {
        // Soft clipping distortion
        let threshold = 1.0 - self.distortion;
        if sample.abs() > threshold {
            if sample > 0.0 {
                threshold + (sample - threshold) / (1.0 + ((sample - threshold) / (1.0 - threshold)).powi(2))
            } else {
                -threshold + (sample + threshold) / (1.0 + ((sample + threshold) / (1.0 - threshold)).powi(2))
            }
        } else {
            sample
        }
    }
}

/// Resamples audio to a different sample rate using linear interpolation
pub fn resample(input: &[f32], input_rate: u32, output_rate: u32) -> Vec<f32> {
    let interpolator = Linear::new([0.0], [0.0]);
    let signal = signal::from_iter(input.iter().cloned());
    let signal = signal.scale_hz(input_rate as f64, output_rate as f64);
    let signal = signal.interpolate(interpolator);
    signal.take(((input.len() as f64 * output_rate as f64) / input_rate as f64) as usize).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_radio_effects_default() {
        let effects = RadioEffects::default();
        assert_eq!(effects.sample_rate, 44100);
        assert_eq!(effects.low_pass_cutoff, 3000.0);
        assert_eq!(effects.high_pass_cutoff, 300.0);
    }

    #[test]
    fn test_process_buffer() {
        let mut effects = RadioEffects::default();
        let mut buffer = vec![0.5; 1000];
        effects.process_buffer(&mut buffer);
        
        // Check that values are within valid range
        for sample in buffer {
            assert!(sample >= -1.0 && sample <= 1.0);
        }
    }

    #[test]
    fn test_signal_quality_update() {
        let mut effects = RadioEffects::default();
        
        // Test with poor signal quality
        effects.update_effects(0.0);
        assert!(effects.noise_level > 0.4);
        assert!(effects.distortion > 0.2);
        
        // Test with good signal quality
        effects.update_effects(1.0);
        assert!(effects.noise_level < 0.1);
        assert!(effects.distortion < 0.1);
    }
}

#[derive(Clone)]
pub struct RadioEffects {
    // Filter chain
    low_pass: BiquadFilterChain,
    high_pass: BiquadFilterChain,
    // Dynamics processing
    noise_gate: NoiseGate,
    compressor: Compressor,
    // Effect parameters
    noise_level: f32,
    distortion: f32,
    sample_rate: f32,
}

#[derive(Clone)]
struct BiquadFilterChain {
    filters: Vec<BiquadFilter>,
    cutoff: f32,
    q: f32,
    filter_type: FilterType,
}

impl BiquadFilterChain {
    fn new(filter_type: FilterType, cutoff: f32, q: f32, stages: usize) -> Self {
        let mut filters = Vec::with_capacity(stages);
        for _ in 0..stages {
            filters.push(BiquadFilter::new(filter_type.clone(), 48000.0, cutoff, q));
        }
        Self {
            filters,
            cutoff,
            q,
            filter_type,
        }
    }

    fn process(&mut self, sample: f32) -> f32 {
        let mut output = sample;
        for filter in &mut self.filters {
            output = filter.process(output);
        }
        output
    }

    fn update_params(&mut self, cutoff: f32, q: f32) {
        if (self.cutoff - cutoff).abs() > 0.01 || (self.q - q).abs() > 0.01 {
            self.cutoff = cutoff;
            self.q = q;
            for filter in &mut self.filters {
                filter.update_coefficients(cutoff, q);
            }
        }
    }
}

#[derive(Clone)]
struct NoiseGate {
    threshold: f32,
    attack_time: f32,
    release_time: f32,
    current_gain: f32,
    sample_rate: f32,
    attack_coef: f32,
    release_coef: f32,
}

impl NoiseGate {
    fn new(threshold_db: f32, attack_ms: f32, release_ms: f32, sample_rate: f32) -> Self {
        let attack_coef = (-1.0 / (attack_ms * 0.001 * sample_rate)).exp();
        let release_coef = (-1.0 / (release_ms * 0.001 * sample_rate)).exp();
        
        Self {
            threshold: 10.0_f32.powf(threshold_db / 20.0),
            attack_time: attack_ms,
            release_time: release_ms,
            current_gain: 1.0,
            sample_rate,
            attack_coef,
            release_coef,
        }
    }

    fn process(&mut self, input: f32) -> f32 {
        let input_level = input.abs();
        let target_gain = if input_level > self.threshold { 1.0 } else { 0.0 };
        
        // Apply attack/release
        if target_gain > self.current_gain {
            self.current_gain = self.attack_coef * (self.current_gain - target_gain) + target_gain;
        } else {
            self.current_gain = self.release_coef * (self.current_gain - target_gain) + target_gain;
        }
        
        input * self.current_gain
    }
}

#[derive(Clone)]
struct Compressor {
    threshold: f32,
    ratio: f32,
    attack_time: f32,
    release_time: f32,
    makeup_gain: f32,
    current_gain: f32,
    sample_rate: f32,
    attack_coef: f32,
    release_coef: f32,
}

impl Compressor {
    fn new(threshold_db: f32, ratio: f32, attack_ms: f32, release_ms: f32, sample_rate: f32) -> Self {
        let attack_coef = (-1.0 / (attack_ms * 0.001 * sample_rate)).exp();
        let release_coef = (-1.0 / (release_ms * 0.001 * sample_rate)).exp();
        
        Self {
            threshold: 10.0_f32.powf(threshold_db / 20.0),
            ratio,
            attack_time: attack_ms,
            release_time: release_ms,
            makeup_gain: 0.0,
            current_gain: 1.0,
            sample_rate,
            attack_coef,
            release_coef,
        }
    }

    fn process(&mut self, input: f32) -> f32 {
        let input_level = input.abs();
        let input_db = 20.0 * input_level.max(1e-6).log10();
        let threshold_db = 20.0 * self.threshold.log10();
        
        let mut gain_reduction = 0.0;
        if input_db > threshold_db {
            gain_reduction = (input_db - threshold_db) * (1.0 - 1.0 / self.ratio);
        }
        
        let target_gain = 10.0_f32.powf(-gain_reduction / 20.0);
        
        // Apply attack/release
        if target_gain < self.current_gain {
            self.current_gain = self.attack_coef * (self.current_gain - target_gain) + target_gain;
        } else {
            self.current_gain = self.release_coef * (self.current_gain - target_gain) + target_gain;
        }
        
        // Apply gain reduction and makeup gain
        input * self.current_gain * 10.0_f32.powf(self.makeup_gain / 20.0)
    }

    fn update_makeup_gain(&mut self) {
        // Calculate makeup gain based on threshold and ratio
        let max_reduction = 20.0;  // dB
        self.makeup_gain = max_reduction * (1.0 - 1.0 / self.ratio);
    }
}

impl Default for RadioEffects {
    fn default() -> Self {
        Self {
            low_pass: BiquadFilterChain::new(FilterType::LowPass, 3400.0, 0.707, 2),
            high_pass: BiquadFilterChain::new(FilterType::HighPass, 300.0, 0.707, 2),
            noise_gate: NoiseGate::new(-50.0, 1.0, 100.0, 48000.0),
            compressor: Compressor::new(-20.0, 4.0, 5.0, 50.0, 48000.0),
            noise_level: 0.01,
            distortion: 0.1,
            sample_rate: 48000.0,
        }
    }
}

impl RadioEffects {
    pub fn process_buffer(&mut self, buffer: &mut [f32]) {
        for sample in buffer.iter_mut() {
            // Apply high-pass first to remove DC and very low frequencies
            *sample = self.high_pass.process(*sample);
            
            // Apply noise gate to reduce background noise
            *sample = self.noise_gate.process(*sample);
            
            // Add noise and distortion
            *sample = self.apply_noise_and_distortion(*sample);
            
            // Apply compression to control dynamics
            *sample = self.compressor.process(*sample);
            
            // Finally apply low-pass to simulate radio bandwidth
            *sample = self.low_pass.process(*sample);
        }
    }

    fn apply_noise_and_distortion(&self, sample: f32) -> f32 {
        let mut rng = rand::thread_rng();
        let noise = (rng.gen::<f32>() * 2.0 - 1.0) * self.noise_level;
        
        // Add noise to signal
        let with_noise = sample + noise;
        
        // Apply soft clipping distortion
        let threshold = 1.0 - self.distortion;
        if with_noise.abs() > threshold {
            let sign = with_noise.signum();
            let excess = (with_noise.abs() - threshold) / (1.0 - threshold);
            let soft_clip = threshold + (1.0 - threshold) * (1.0 - (-3.0 * excess).exp());
            sign * soft_clip
        } else {
            with_noise
        }
    }

    pub fn update_effects(&mut self, quality: &SignalQuality) {
        // Update filter parameters based on signal quality
        let base_q = 0.707;
        let q_mod = 1.0 + (1.0 - quality.clarity) * 0.5;
        
        // Adjust filter cutoffs based on signal quality
        let low_pass_cutoff = 3400.0 * quality.clarity.max(0.3);
        let high_pass_cutoff = 300.0 * (1.0 + (1.0 - quality.clarity) * 0.5);
        
        self.low_pass.update_params(low_pass_cutoff, base_q * q_mod);
        self.high_pass.update_params(high_pass_cutoff, base_q);
        
        // Update noise and distortion levels
        self.noise_level = 0.01 + (1.0 - quality.clarity) * 0.1;
        self.distortion = 0.1 + (1.0 - quality.clarity) * 0.3;
        
        // Update compressor settings
        let comp = &mut self.compressor;
        comp.ratio = 4.0 + (1.0 - quality.clarity) * 4.0;
        comp.update_makeup_gain();
    }
} 