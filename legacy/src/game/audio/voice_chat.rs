use bevy::prelude::*;
use bevy_kira_audio::{AudioControl, AudioInstance, AudioSource};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use ringbuf::{HeapRb, Producer, Consumer};
use dasp::{Sample, Signal};
use dasp_signal::rate::Converter;
use dasp_ring_buffer::Fixed;
use dasp_interpolate::linear::Linear;
use super::cb_radio::{CBRadio, SignalQuality, AudioEffectControl};
use std::collections::{VecDeque, HashMap};
use std::time::{Duration, SystemTime};
use crate::game::audio::{RadioEffects};
use super::cb_filter_chain::{CBFilterChain, CBFilterChainConfig};
use super::voice_detection::VoiceActivityDetector;

const SAMPLE_RATE: f32 = 48000.0;
const CHANNELS: u16 = 1;
const BUFFER_SIZE: usize = 4800; // 100ms of audio
const PACKET_SIZE: usize = 128;
const MAX_JITTER_BUFFER_SIZE: usize = 1024;
const TRANSMISSION_START_FREQ: f32 = 1200.0;
const TRANSMISSION_END_FREQ: f32 = 800.0;
const TONE_DURATION: f32 = 0.1;
const FRAME_SIZE: usize = 480; // 10ms at 48kHz

// Network-related constants
const MAX_SEQUENCE_NUMBER: u16 = 65535;
const DEFAULT_JITTER_BUFFER_MS: u32 = 100;
const MIN_JITTER_BUFFER_MS: u32 = 50;
const MAX_JITTER_BUFFER_MS: u32 = 200;
const COMFORT_NOISE_THRESHOLD: f32 = -60.0;
const RTT_SMOOTHING_FACTOR: f32 = 0.1;

/// Component for voice chat functionality
#[derive(Component)]
pub struct VoiceChat {
    /// Audio input stream
    input_stream: Option<cpal::Stream>,
    /// Audio output stream
    output_stream: Option<cpal::Stream>,
    /// Ring buffer for audio samples
    sample_producer: Option<Producer<f32>>,
    sample_consumer: Option<Consumer<f32>>,
    /// Voice activity detection state
    is_speaking: bool,
    /// Audio processing state
    voice_filter: VoiceFilter,
    /// Last processed frame timestamp
    last_frame: f64,
    /// Network voice data buffer
    voice_buffer: Vec<f32>,
    /// Network state
    network_state: NetworkState,
    /// Jitter buffer for received packets
    jitter_buffer: Vec<(u32, Vec<f32>)>,
    /// Packet loss count
    packet_loss_count: u32,
    /// Playback buffer for voice output
    playback_buffer: Vec<f32>,
    /// Push-to-talk state
    pub push_to_talk: bool,
    /// Currently selected CB radio channel
    pub current_channel: u8,
    /// Last signal quality for effects
    pub last_signal_quality: SignalQuality,
    /// Voice activity detection
    vad: VoiceActivityDetector,
    /// Adaptive jitter buffer
    jitter_config: AdaptiveJitterBuffer,
    /// Network synchronization
    network_sync: NetworkSync,
    /// Audio buffers
    input_buffer: HeapRb<f32>,
    output_buffer: HeapRb<f32>,
    /// Radio effects
    radio_effects: RadioEffects,
    /// Processing buffers
    temp_buffer: Vec<f32>,
    /// New fields for network synchronization
    network_stats: NetworkStats,
    sequence_counter: u16,
    last_packet_time: SystemTime,
    comfort_noise_generator: ComfortNoiseGenerator,
    /// CB radio filter chain for voice effects
    filter_chain: CBFilterChain,
    /// Current signal quality affecting the filter chain
    signal_quality: SignalQuality,
    /// Whether voice activity is currently detected
    is_voice_active: bool,
    /// Time since last VAD update
    last_vad_update: f32,
}

#[derive(Default)]
struct NetworkState {
    sequence_number: u32,
    last_received: u32,
    jitter_buffer: Vec<(u32, Vec<f32>)>,
    packet_loss_count: u32,
    current_time: f64,
    connection: Option<NetworkConnection>,
}

/// Voice filter configuration
#[derive(Clone)]
pub struct VoiceFilter {
    /// Noise gate parameters
    gate_threshold: f32,
    gate_attack_ms: f32,
    gate_release_ms: f32,
    /// Compression parameters
    comp_threshold: f32,
    comp_ratio: f32,
    comp_attack_ms: f32,
    comp_release_ms: f32,
    /// Filter parameters
    pre_filter_cutoff: f32,
    post_filter_cutoff: f32,
}

impl Default for VoiceFilter {
    fn default() -> Self {
        Self {
            gate_threshold: -50.0,  // dB
            gate_attack_ms: 1.0,
            gate_release_ms: 100.0,
            comp_threshold: -20.0,  // dB
            comp_ratio: 2.0,
            comp_attack_ms: 5.0,
            comp_release_ms: 50.0,
            pre_filter_cutoff: 8000.0,
            post_filter_cutoff: 3400.0,
        }
    }
}

/// Voice activity detection configuration
#[derive(Clone)]
pub struct VoiceActivityDetection {
    /// Energy threshold for speech detection
    energy_threshold: f32,
    /// Minimum duration for speech detection (seconds)
    min_speech_duration: f32,
    /// Hold time after speech ends (seconds)
    hold_time: f32,
    /// Last time speech was detected
    last_speech_time: f32,
    /// Running average of background noise
    noise_floor: f32,
    /// Adaptation rate for noise floor
    noise_adaptation_rate: f32,
}

impl Default for VoiceActivityDetection {
    fn default() -> Self {
        Self {
            energy_threshold: -40.0,  // dB
            min_speech_duration: 0.1,
            hold_time: 0.2,
            last_speech_time: 0.0,
            noise_floor: -60.0,
            noise_adaptation_rate: 0.1,
        }
    }
}

impl VoiceActivityDetection {
    /// Update voice activity detection state
    fn update(&mut self, frame_energy: f32, time: f32) -> bool {
        // Convert frame energy to dB
        let energy_db = 20.0 * frame_energy.max(1e-10).log10();
        
        // Update noise floor estimate during silence
        if energy_db < self.noise_floor + 10.0 {
            self.noise_floor += (energy_db - self.noise_floor) * self.noise_adaptation_rate;
        }
        
        // Check if energy exceeds threshold
        let is_speech = energy_db > self.noise_floor + self.energy_threshold;
        
        if is_speech {
            self.last_speech_time = time;
        }
        
        // Apply minimum duration and hold time
        time < self.last_speech_time + self.hold_time
    }
}

/// Adaptive jitter buffer configuration
#[derive(Clone)]
pub struct AdaptiveJitterBuffer {
    /// Target buffer size in milliseconds
    target_size_ms: f32,
    /// Maximum buffer size in milliseconds
    max_size_ms: f32,
    /// Minimum buffer size in milliseconds
    min_size_ms: f32,
    /// Current buffer size in milliseconds
    current_size_ms: f32,
    /// Network jitter estimate
    jitter_estimate: f32,
    /// Adaptation rate
    adaptation_rate: f32,
}

impl Default for AdaptiveJitterBuffer {
    fn default() -> Self {
        Self {
            target_size_ms: 60.0,
            max_size_ms: 200.0,
            min_size_ms: 20.0,
            current_size_ms: 60.0,
            jitter_estimate: 0.0,
            adaptation_rate: 0.1,
        }
    }
}

impl AdaptiveJitterBuffer {
    /// Update jitter buffer size based on network conditions
    fn update(&mut self, packet_delay: f32) {
        // Update jitter estimate
        let delay_diff = (packet_delay - self.jitter_estimate).abs();
        self.jitter_estimate += (delay_diff - self.jitter_estimate) * self.adaptation_rate;
        
        // Calculate target buffer size based on jitter
        let target = (self.jitter_estimate * 4.0).clamp(
            self.min_size_ms,
            self.max_size_ms
        );
        
        // Smoothly adapt current size
        self.current_size_ms += (target - self.current_size_ms) * self.adaptation_rate;
    }
    
    /// Get current buffer size in samples
    fn get_buffer_size(&self, sample_rate: u32) -> usize {
        (self.current_size_ms * 0.001 * sample_rate as f32) as usize
    }
}

/// Network synchronization configuration
#[derive(Clone)]
pub struct NetworkSync {
    /// Maximum allowed packet delay (ms)
    max_packet_delay: f32,
    /// Minimum required packets for playback
    min_buffer_packets: usize,
    /// Network statistics
    stats: NetworkStats,
    /// Packet reordering window
    reorder_window: VecDeque<VoicePacket>,
    /// Last processed sequence number
    last_processed_seq: u32,
}

#[derive(Clone, Default)]
struct NetworkStats {
    /// Average packet delay
    avg_delay: f32,
    /// Packet loss rate
    loss_rate: f32,
    /// Jitter (delay variation)
    jitter: f32,
    /// Number of packets processed
    packet_count: u32,
    /// Number of packets lost
    lost_packets: u32,
}

impl Default for NetworkSync {
    fn default() -> Self {
        Self {
            max_packet_delay: 200.0,  // 200ms maximum delay
            min_buffer_packets: 2,     // Minimum 2 packets for smooth playback
            stats: NetworkStats::default(),
            reorder_window: VecDeque::with_capacity(32),
            last_processed_seq: 0,
        }
    }
}

impl NetworkSync {
    /// Update network statistics
    fn update_stats(&mut self, packet: &VoicePacket, current_time: f64) {
        let delay = (current_time - packet.timestamp) * 1000.0;  // Convert to ms
        
        // Update average delay using exponential moving average
        self.stats.avg_delay = self.stats.avg_delay * 0.95 + delay * 0.05;
        
        // Update jitter
        let delay_diff = (delay - self.stats.avg_delay).abs();
        self.stats.jitter = self.stats.jitter * 0.95 + delay_diff * 0.05;
        
        // Update packet counts
        self.stats.packet_count += 1;
        
        // Check for packet loss
        let expected_seq = self.last_processed_seq.wrapping_add(1);
        if packet.sequence > expected_seq {
            let lost = packet.sequence - expected_seq;
            self.stats.lost_packets += lost;
            self.stats.loss_rate = self.stats.lost_packets as f32 / self.stats.packet_count as f32;
        }
    }
    
    /// Process incoming packet
    fn process_packet(&mut self, packet: VoicePacket, current_time: f64) -> Option<Vec<f32>> {
        // Update statistics
        self.update_stats(&packet, current_time);
        
        // Check if packet is too old
        let delay = (current_time - packet.timestamp) * 1000.0;
        if delay > self.max_packet_delay {
            return None;
        }
        
        // Add to reorder window
        let insert_idx = self.reorder_window
            .binary_search_by_key(&packet.sequence, |p| p.sequence)
            .unwrap_or_else(|i| i);
        self.reorder_window.insert(insert_idx, packet);
        
        // Process packets in order if we have enough buffered
        if self.reorder_window.len() >= self.min_buffer_packets {
            if let Some(next_packet) = self.reorder_window.pop_front() {
                self.last_processed_seq = next_packet.sequence;
                return Some(next_packet.samples);
            }
        }
        
        None
    }
}

impl Default for VoiceChat {
    fn default() -> Self {
        let filter_config = CBFilterChainConfig {
            sample_rate: SAMPLE_RATE,
            ..Default::default()
        };
        
        Self {
            input_stream: None,
            output_stream: None,
            sample_producer: None,
            sample_consumer: None,
            is_speaking: false,
            voice_filter: VoiceFilter::default(),
            last_frame: 0.0,
            voice_buffer: Vec::with_capacity(BUFFER_SIZE),
            network_state: NetworkState::default(),
            jitter_buffer: Vec::new(),
            packet_loss_count: 0,
            playback_buffer: Vec::new(),
            push_to_talk: false,
            current_channel: 0,
            last_signal_quality: SignalQuality::default(),
            vad: VoiceActivityDetector::new(SAMPLE_RATE),
            is_voice_active: false,
            last_vad_update: 0.0,
            jitter_config: AdaptiveJitterBuffer::default(),
            network_sync: NetworkSync::default(),
            input_buffer: HeapRb::new(BUFFER_SIZE),
            output_buffer: HeapRb::new(BUFFER_SIZE),
            radio_effects: RadioEffects::default(),
            temp_buffer: vec![0.0; FRAME_SIZE],
            network_stats: NetworkStats::default(),
            sequence_counter: 0,
            last_packet_time: SystemTime::now(),
            comfort_noise_generator: ComfortNoiseGenerator::default(),
            filter_chain: CBFilterChain::new(filter_config),
            signal_quality: SignalQuality::default(),
        }
    }
}

/// Plugin for voice chat functionality
pub struct VoiceChatPlugin {
    audio_host: cpal::Host,
    input_device: Option<cpal::Device>,
    output_device: Option<cpal::Device>,
    transmission_start: Handle<AudioSource>,
    transmission_end: Handle<AudioSource>,
}

impl Plugin for VoiceChatPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_voice_chat)
           .add_systems(Update, (
               process_voice_input,
               handle_push_to_talk,
               process_voice_output,
               update_voice_effects,
               apply_radio_effects,
               handle_network_voice,
               update_voice_output,
               handle_voice_packets,
               handle_network_packets,
               cleanup_disconnected_voices
           ).chain());
    }
}

impl VoiceChatPlugin {
    pub fn new() -> Self {
        let audio_host = cpal::default_host();
        let input_device = audio_host.default_input_device();
        let output_device = audio_host.default_output_device();
        
        // Generate transmission tones
        let start_tone = generate_tone(TRANSMISSION_START_FREQ, TONE_DURATION);
        let end_tone = generate_tone(TRANSMISSION_END_FREQ, TONE_DURATION);
        
        Self {
            audio_host,
            input_device,
            output_device,
            transmission_start: Handle::default(), // Load in setup
            transmission_end: Handle::default(),   // Load in setup
        }
    }
}

/// Initialize voice chat for an entity
fn setup_voice_chat(mut commands: Commands) {
    let host = cpal::default_host();
    
    // Set up input device
    if let Ok(input_device) = host.default_input_device() {
        let input_config = input_device.default_input_config().unwrap();
        
        // Create ring buffer
        let ring_buffer = HeapRb::new(BUFFER_SIZE * 2);
        let (producer, consumer) = ring_buffer.split();
        
        // Create input stream
        let input_stream = input_device.build_input_stream(
            &input_config.into(),
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                // Send input data to ring buffer
                for &sample in data {
                    let _ = producer.push(sample);
                }
            },
            |err| error!("Input stream error: {}", err),
        ).ok();
        
        commands.spawn((
            VoiceChat {
                input_stream,
                sample_consumer: Some(consumer),
                ..default()
            },
        ));
    }
}

/// Process incoming voice data with CB radio effects
fn process_voice_input(
    mut voice_chat: Query<&mut VoiceChat>,
    cb_radios: Query<(&CBRadio, &SignalQuality)>,
    time: Res<Time>,
) {
    for mut chat in voice_chat.iter_mut() {
        if let Ok((radio, signal_quality)) = cb_radios.get_single() {
            if !radio.powered || !chat.push_to_talk {
                continue;
            }

            if let Some(consumer) = &mut chat.sample_consumer {
                let mut frame_buffer = Vec::new();
                
                // Read available samples
                while let Some(sample) = consumer.pop() {
                    frame_buffer.push(sample);
                }
                
                if !frame_buffer.is_empty() {
                    // Calculate frame energy
                    let frame_energy = calculate_rms(&frame_buffer);
                    
                    // Update VAD state
                    chat.is_speaking = chat.vad.process_frame(frame_buffer.as_slice(), time.elapsed_seconds_f32());
                    
                    if chat.is_speaking {
                        // Update filter parameters based on signal quality
                        update_voice_filter(&mut chat.voice_filter, signal_quality);
                        
                        // Apply CB radio effects
                        apply_cb_radio_effects(&mut frame_buffer, signal_quality);
                        
                        // Apply filters
                        apply_filters(&mut frame_buffer, &chat.voice_filter);
                        
                        // Store processed audio for network transmission
                        chat.voice_buffer.extend_from_slice(&frame_buffer);
                        
                        // Send voice data if buffer is full
                        if chat.voice_buffer.len() >= PACKET_SIZE {
                            chat.send_voice_packet();
                        }
                    }
                }
            }
            
            chat.last_frame = time.elapsed_seconds_f64();
            chat.last_signal_quality = signal_quality.clone();
        }
    }
}

/// Update voice filter parameters based on signal quality
fn update_voice_filter(filter: &mut VoiceFilter, quality: &SignalQuality) {
    // Adjust filter parameters based on signal quality
    let clarity_factor = quality.clarity.powf(2.0);
    let strength_factor = quality.strength.powf(1.5);
    
    filter.pre_filter_cutoff = (3400.0 * clarity_factor).max(1200.0);
    filter.post_filter_cutoff = 300.0 + (1.0 - clarity_factor) * 400.0;
    filter.gate_threshold = -50.0 + (1.0 - strength_factor) * 20.0;
    filter.comp_threshold = -20.0 + (1.0 - clarity_factor) * 20.0;
    filter.comp_ratio = 2.0 + (1.0 - clarity_factor) * 2.0;
    filter.gate_attack_ms = 1.0;
    filter.gate_release_ms = 100.0;
    filter.comp_attack_ms = 5.0;
    filter.comp_release_ms = 50.0;
}

/// Apply CB radio effects to voice data
fn apply_cb_radio_effects(samples: &mut [f32], quality: &SignalQuality) {
    let interference_factor = 1.0 - quality.interference_level;
    let noise_amount = (1.0 - interference_factor) * 0.3;
    
    // Add static noise based on interference
    if interference_factor < 0.8 {
        for sample in samples.iter_mut() {
            let noise = (fastrand::f32() * 2.0 - 1.0) * noise_amount;
            *sample = (*sample + noise).clamp(-1.0, 1.0);
        }
    }
    
    // Add signal fading based on strength
    let strength_factor = quality.strength.powf(1.5);
    for sample in samples.iter_mut() {
        *sample *= strength_factor;
    }
}

/// Audio buffer for voice playback
struct AudioBuffer {
    samples: Vec<f32>,
    channels: u16,
    sample_rate: u32,
}

impl AudioBuffer {
    fn from_samples(samples: &[f32], channels: u16, sample_rate: u32) -> Self {
        Self {
            samples: samples.to_vec(),
            channels,
            sample_rate,
        }
    }
}

impl AudioSource for AudioBuffer {
    fn duration(&self) -> f64 {
        self.samples.len() as f64 / (self.sample_rate as f64 * self.channels as f64)
    }
    
    fn write_samples(&self, buffer: &mut [f32]) -> usize {
        let mut written = 0;
        for (out, &sample) in buffer.iter_mut().zip(self.samples.iter()) {
            *out = sample;
            written += 1;
        }
        written
    }
}

/// Apply radio effects to voice based on signal quality
fn apply_radio_effects(
    mut voice_chat: Query<&mut VoiceChat>,
    radios: Query<(&CBRadio, &SignalQuality)>,
) {
    for mut chat in voice_chat.iter_mut() {
        if let Ok((radio, quality)) = radios.get_single() {
            if radio.powered && radio.transmitting {
                // Calculate dynamic filter parameters based on signal quality
                let clarity_factor = quality.clarity.powf(2.0); // Exponential scaling
                let strength_factor = quality.strength.powf(1.5);
                let interference_factor = 1.0 - (quality.interference_level * 0.8);
                
                // Update filter parameters based on signal quality
                chat.voice_filter = VoiceFilter {
                    // Reduce bandwidth as signal degrades
                    pre_filter_cutoff: (3400.0 * clarity_factor).max(1200.0),
                    // Increase high-pass to simulate poor bass response
                    post_filter_cutoff: 300.0 + (1.0 - clarity_factor) * 400.0,
                    // Raise noise gate threshold with interference
                    gate_threshold: -50.0 + (1.0 - strength_factor) * 20.0,
                    // Increase compression with poor signal
                    comp_threshold: -20.0 + (1.0 - clarity_factor) * 20.0,
                    comp_ratio: 2.0 + (1.0 - clarity_factor) * 2.0,
                    gate_attack_ms: 1.0,
                    gate_release_ms: 100.0,
                    comp_attack_ms: 5.0,
                    comp_release_ms: 50.0,
                };
                
                // Add noise based on interference level
                if interference_factor < 0.8 {
                    let noise_amount = (1.0 - interference_factor) * 0.3;
                    for sample in chat.voice_buffer.iter_mut() {
                        let noise = (fastrand::f32() * 2.0 - 1.0) * noise_amount;
                        *sample = (*sample + noise).clamp(-1.0, 1.0);
                    }
                }
            }
        }
    }
}

/// Handle incoming network voice packets
fn handle_network_voice(
    mut voice_chat: Query<&mut VoiceChat>,
    audio: Res<Audio>,
) {
    for mut chat in voice_chat.iter_mut() {
        // Process packets in jitter buffer
        chat.jitter_buffer.sort_by_key(|(seq, _)| *seq);
        
        while let Some((seq, samples)) = chat.jitter_buffer.first().cloned() {
            if seq <= chat.network_state.last_received {
                // Discard old packets
                chat.jitter_buffer.remove(0);
            } else if seq == chat.network_state.last_received + 1 {
                // Process next packet in sequence
                play_voice_packet(&samples, &audio);
                chat.network_state.last_received = seq;
                chat.jitter_buffer.remove(0);
            } else {
                // Wait for missing packets
                break;
            }
        }
        
        // Clear old packets from jitter buffer
        chat.jitter_buffer.retain(|(seq, _)| 
            *seq <= chat.network_state.last_received + 32
        );
    }
}

/// Update voice output processing
fn update_voice_output(
    mut voice_chat: Query<&mut VoiceChat>,
    audio: Res<Audio>,
    time: Res<Time>,
) {
    for mut chat in voice_chat.iter_mut() {
        // Handle output stream configuration if needed
        if chat.output_stream.is_none() {
            if let Ok(output_device) = cpal::default_host().default_output_device() {
                let output_config = output_device.default_output_config().unwrap();
                
                let output_stream = output_device.build_output_stream(
                    &output_config.into(),
                    move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                        // Fill output buffer with silence if no voice data
                        for sample in data.iter_mut() {
                            *sample = 0.0;
                        }
                    },
                    |err| error!("Output stream error: {}", err),
                ).ok();
                
                chat.output_stream = output_stream;
            }
        }
    }
}

// Helper functions

fn calculate_rms(samples: &[f32]) -> f32 {
    let sum_squares: f32 = samples.iter()
        .map(|&s| s * s)
        .sum();
    (sum_squares / samples.len() as f32).sqrt()
}

fn apply_filters(samples: &mut [f32], filter: &VoiceFilter) {
    // Apply high-pass filter
    let mut high_pass = BiquadFilter::new(
        FilterType::HighPass,
        SAMPLE_RATE as f32,
        filter.post_filter_cutoff,
        0.707
    );
    
    // Apply low-pass filter
    let mut low_pass = BiquadFilter::new(
        FilterType::LowPass,
        SAMPLE_RATE as f32,
        filter.pre_filter_cutoff,
        0.707
    );
    
    for sample in samples.iter_mut() {
        // Apply filters
        *sample = high_pass.process(*sample);
        *sample = low_pass.process(*sample);
        
        // Apply compression
        if sample.abs() > filter.gate_threshold.exp10() {
            let gain_reduction = (sample.abs() / filter.gate_threshold.exp10())
                .powf(1.0 / filter.comp_ratio - 1.0);
            *sample *= gain_reduction;
        }
    }
}

/// Send voice packet over the network
fn send_voice_packet(
    mut voice_chat: Query<&mut VoiceChat>,
    mut net_state: ResMut<NetworkState>,
) {
    for mut chat in voice_chat.iter_mut() {
        if chat.is_speaking && chat.voice_buffer.len() >= PACKET_SIZE {
            // Create packet from buffer
            let mut packet_data = vec![0.0; PACKET_SIZE];
            for i in 0..PACKET_SIZE {
                packet_data[i] = chat.voice_buffer[i];
            }
            
            let packet = VoicePacket {
                timestamp: net_state.current_time,
                sequence: net_state.sequence_number,
                samples: packet_data,
                sample_rate: chat.sample_rate,
                channels: chat.channels,
            };
            
            // Send packet using reliable ordered channel
            if let Some(connection) = net_state.connection.as_mut() {
                connection.send_message(
                    MessageChannel::ReliableOrdered,
                    bincode::serialize(&packet).unwrap()
                );
            }
            
            // Update sequence number
            net_state.sequence_number = net_state.sequence_number.wrapping_add(1);
            
            // Remove sent samples from buffer
            chat.voice_buffer.drain(0..PACKET_SIZE);
        }
    }
}

fn play_voice_packet(samples: &[f32], audio: &Audio) {
    // Create audio buffer from samples
    let buffer = AudioBuffer::from_samples(samples, CHANNELS, SAMPLE_RATE as u32);
    
    // Play through audio system
    if let Ok(instance) = audio.play_buffer(buffer) {
        // Apply radio effects
        audio.set_volume(instance, 1.0);
    }
}

#[derive(Clone)]
struct VoicePacket {
    sequence: u32,
    timestamp: f64,
    samples: Vec<f32>,
}

// Biquad filter implementation for audio processing
struct BiquadFilter {
    a1: f32,
    a2: f32,
    b0: f32,
    b1: f32,
    b2: f32,
    x1: f32,
    x2: f32,
    y1: f32,
    y2: f32,
}

enum FilterType {
    LowPass,
    HighPass,
}

impl BiquadFilter {
    fn new(filter_type: FilterType, sample_rate: f32, cutoff: f32, q: f32) -> Self {
        let w0 = 2.0 * std::f32::consts::PI * cutoff / sample_rate;
        let cos_w0 = w0.cos();
        let alpha = w0.sin() / (2.0 * q);
        
        let (b0, b1, b2, a0, a1, a2) = match filter_type {
            FilterType::LowPass => {
                let b1 = 1.0 - cos_w0;
                let b0 = b1 / 2.0;
                let b2 = b0;
                let a0 = 1.0 + alpha;
                let a1 = -2.0 * cos_w0;
                let a2 = 1.0 - alpha;
                (b0, b1, b2, a0, a1, a2)
            }
            FilterType::HighPass => {
                let b1 = -(1.0 + cos_w0);
                let b0 = (1.0 + cos_w0) / 2.0;
                let b2 = b0;
                let a0 = 1.0 + alpha;
                let a1 = -2.0 * cos_w0;
                let a2 = 1.0 - alpha;
                (b0, b1, b2, a0, a1, a2)
            }
        };
        
        Self {
            a1: a1 / a0,
            a2: a2 / a0,
            b0: b0 / a0,
            b1: b1 / a0,
            b2: b2 / a0,
            x1: 0.0,
            x2: 0.0,
            y1: 0.0,
            y2: 0.0,
        }
    }
    
    fn process(&mut self, input: f32) -> f32 {
        let output = self.b0 * input + self.b1 * self.x1 + self.b2 * self.x2
                    - self.a1 * self.y1 - self.a2 * self.y2;
        
        self.x2 = self.x1;
        self.x1 = input;
        self.y2 = self.y1;
        self.y1 = output;
        
        output
    }
}

/// Handle received voice packets and manage jitter buffer
fn handle_voice_packets(
    mut voice_chat: Query<&mut VoiceChat>,
    time: Res<Time>,
) {
    for mut chat in voice_chat.iter_mut() {
        while let Some(packet_data) = receive_voice_packet() {
            if let Ok(packet) = bincode::deserialize::<VoicePacket>(&packet_data) {
                // Process packet through network sync
                if let Some(samples) = chat.network_sync.process_packet(
                    packet,
                    time.elapsed_seconds_f64()
                ) {
                    // Update jitter buffer size based on network conditions
                    chat.jitter_config.update(
                        chat.network_sync.stats.jitter
                    );
                    
                    // Get target buffer size
                    let target_size = chat.jitter_config.get_buffer_size(SAMPLE_RATE as u32);
                    
                    // Add processed samples to playback buffer
                    chat.playback_buffer.extend(samples);
                    
                    // Trim buffer if too large
                    while chat.playback_buffer.len() > target_size {
                        chat.playback_buffer.drain(0..PACKET_SIZE);
                    }
                }
            }
        }
        
        // Handle packet loss compensation
        if chat.playback_buffer.is_empty() && chat.network_sync.stats.loss_rate > 0.1 {
            // Insert comfort noise during high packet loss
            let noise_level = chat.network_sync.stats.loss_rate * 0.1;
            let comfort_noise = generate_comfort_noise(PACKET_SIZE, noise_level);
            chat.playback_buffer.extend(comfort_noise);
        }
    }
}

/// Handle push-to-talk state changes
fn handle_push_to_talk(
    mut voice_chat: Query<&mut VoiceChat>,
    mut audio: ResMut<Audio>,
    plugin: Res<VoiceChatPlugin>,
    input: Res<Input<KeyCode>>,
) {
    for mut chat in voice_chat.iter_mut() {
        let was_transmitting = chat.push_to_talk;
        
        // Update push-to-talk state (Space bar for testing)
        chat.push_to_talk = input.pressed(KeyCode::Space);
        
        // Play transmission tones on state change
        if !was_transmitting && chat.push_to_talk {
            // Start transmission
            audio.play(plugin.transmission_start.clone());
        } else if was_transmitting && !chat.push_to_talk {
            // End transmission
            audio.play(plugin.transmission_end.clone());
            
            // Clear any remaining voice data
            chat.voice_buffer.clear();
            chat.is_speaking = false;
        }
    }
}

/// Update voice effects based on signal quality changes
fn update_voice_effects(
    mut voice_chat: Query<&mut VoiceChat>,
    cb_radios: Query<&SignalQuality>,
) {
    if let Ok(signal_quality) = cb_radios.get_single() {
        for mut chat in voice_chat.iter_mut() {
            // Only update if signal quality has changed significantly
            if signal_quality_changed(&chat.last_signal_quality, signal_quality) {
                update_voice_filter(&mut chat.voice_filter, signal_quality);
                chat.last_signal_quality = signal_quality.clone();
            }
        }
    }
}

/// Check if signal quality has changed significantly
fn signal_quality_changed(last: &SignalQuality, current: &SignalQuality) -> bool {
    let threshold = 0.1;
    (last.clarity - current.clarity).abs() > threshold ||
    (last.strength - current.strength).abs() > threshold ||
    (last.interference_level - current.interference_level).abs() > threshold
}

/// Generate a sine wave tone at the specified frequency
fn generate_tone(frequency: f32, duration: f32) -> Vec<f32> {
    let sample_count = (SAMPLE_RATE as f32 * duration) as usize;
    let mut samples = Vec::with_capacity(sample_count);
    
    for i in 0..sample_count {
        let t = i as f32 / SAMPLE_RATE as f32;
        let sample = (2.0 * std::f32::consts::PI * frequency * t).sin();
        samples.push(sample * 0.5); // 50% amplitude
    }
    
    samples
}

/// Generate comfort noise to fill gaps during packet loss
fn generate_comfort_noise(size: usize, level: f32) -> Vec<f32> {
    let mut noise = Vec::with_capacity(size);
    for _ in 0..size {
        let sample = (fastrand::f32() * 2.0 - 1.0) * level;
        noise.push(sample);
    }
    noise
}

fn handle_network_packets(
    mut voice_query: Query<&mut VoiceChat>,
    time: Res<Time>,
    mut net: ResMut<NetworkResource>,
) {
    // Process incoming voice packets
    while let Some(packet) = net.receive_voice_packet() {
        if let Ok(mut voice) = voice_query.get_mut(packet.entity) {
            // Update network statistics
            voice.update_network_stats(NetworkStats {
                latency: packet.latency,
                jitter: packet.jitter,
                packet_loss: packet.packet_loss,
            });
            
            // Process voice data through effects chain
            let mut samples = packet.samples.clone();
            voice.process_audio_frame(&mut samples, &voice.last_signal_quality);
            
            // Add to jitter buffer
            voice.jitter_buffer.add_packet(packet.sequence, samples);
        }
    }
    
    // Update all voice chat instances
    for mut voice in voice_query.iter_mut() {
        // Get playback samples from jitter buffer
        if let Some(samples) = voice.jitter_buffer.get_samples(time.delta_seconds()) {
            // Add to output buffer
            for sample in samples {
                let _ = voice.output_buffer.push(sample);
            }
        }
    }
}

fn cleanup_disconnected_voices(
    mut commands: Commands,
    voice_query: Query<(Entity, &VoiceChat)>,
    net: Res<NetworkResource>,
) {
    for (entity, voice) in voice_query.iter() {
        if !net.is_connected(entity) {
            commands.entity(entity).despawn();
        }
    }
}

// Helper struct for network statistics
#[derive(Default, Clone, Copy)]
pub struct NetworkStats {
    latency: f32,
    jitter: f32,
    packet_loss: f32,
}

// Voice activity detection
pub struct VoiceActivityDetection {
    energy_threshold: f32,
    frame_energy: f32,
    hold_frames: u32,
    current_hold: u32,
    sample_rate: f32,
}

impl VoiceActivityDetection {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            energy_threshold: 0.002, // -54 dB
            frame_energy: 0.0,
            hold_frames: (0.2 * sample_rate / FRAME_SIZE as f32) as u32, // 200ms hold time
            current_hold: 0,
            sample_rate,
        }
    }
    
    pub fn is_voice_active(&mut self, samples: &[f32]) -> bool {
        // Calculate frame energy
        self.frame_energy = samples.iter()
            .map(|&s| s * s)
            .sum::<f32>() / samples.len() as f32;
        
        if self.frame_energy > self.energy_threshold {
            self.current_hold = self.hold_frames;
            true
        } else if self.current_hold > 0 {
            self.current_hold -= 1;
            true
        } else {
            false
        }
    }
}

// Adaptive jitter buffer
pub struct AdaptiveJitterBuffer {
    min_delay_ms: f32,
    max_delay_ms: f32,
    current_delay_ms: f32,
    buffer: VecDeque<(u64, Vec<f32>)>,
    next_sequence: u64,
    stats: NetworkStats,
}

impl AdaptiveJitterBuffer {
    pub fn new(min_delay_ms: f32, max_delay_ms: f32) -> Self {
        Self {
            min_delay_ms,
            max_delay_ms,
            current_delay_ms: min_delay_ms,
            buffer: VecDeque::new(),
            next_sequence: 0,
            stats: NetworkStats::default(),
        }
    }
    
    pub fn update_network_stats(&mut self, stats: &NetworkStats) {
        self.stats = *stats;
        
        // Adjust buffer delay based on network conditions
        let target_delay = (self.min_delay_ms + self.stats.jitter * 2.0)
            .min(self.max_delay_ms);
        
        self.current_delay_ms = self.current_delay_ms * 0.95 + target_delay * 0.05;
    }
    
    pub fn add_packet(&mut self, sequence: u64, samples: Vec<f32>) {
        // Insert packet in order
        let pos = self.buffer.partition_point(|&(seq, _)| seq < sequence);
        self.buffer.insert(pos, (sequence, samples));
        
        // Trim buffer if too large
        while self.buffer.len() as f32 * FRAME_SIZE as f32 / SAMPLE_RATE > self.max_delay_ms / 1000.0 {
            self.buffer.pop_front();
        }
    }
    
    pub fn get_samples(&mut self, dt: f32) -> Option<Vec<f32>> {
        if self.buffer.is_empty() {
            return None;
        }
        
        // Check if enough samples are buffered
        let buffered_ms = self.buffer.len() as f32 * FRAME_SIZE as f32 / SAMPLE_RATE * 1000.0;
        if buffered_ms < self.current_delay_ms {
            return None;
        }
        
        // Get next packet
        if let Some((sequence, samples)) = self.buffer.pop_front() {
            if sequence >= self.next_sequence {
                self.next_sequence = sequence + 1;
                Some(samples)
            } else {
                None
            }
        } else {
            None
        }
    }
}

struct ComfortNoiseGenerator {
    noise_floor: f32,
    last_update: SystemTime,
}

impl Default for ComfortNoiseGenerator {
    fn default() -> Self {
        Self {
            noise_floor: COMFORT_NOISE_THRESHOLD,
            last_update: SystemTime::now(),
        }
    }
}

impl ComfortNoiseGenerator {
    fn generate(&mut self, num_samples: usize) -> Vec<f32> {
        let mut rng = rand::thread_rng();
        let amplitude = (10.0f32.powf(self.noise_floor / 20.0) * 0.1).max(0.001);
        (0..num_samples)
            .map(|_| (rng.gen::<f32>() * 2.0 - 1.0) * amplitude)
            .collect()
    }

    fn update_noise_floor(&mut self, signal_quality: &SignalQuality) {
        self.noise_floor = signal_quality.noise_floor;
        self.last_update = SystemTime::now();
    }
}

/// System to handle voice chat network synchronization
#[allow(clippy::too_many_arguments)]
fn handle_voice_network(
    mut voice_chat: Query<&mut VoiceChat>,
    cb_radios: Query<(&CBRadio, &SignalQuality)>,
    time: Res<Time>,
    mut net_events: EventReader<NetworkVoiceEvent>,
) {
    for mut chat in voice_chat.iter_mut() {
        if let Ok((radio, signal_quality)) = cb_radios.get_single() {
            // Process incoming voice packets
            for event in net_events.iter() {
                match event {
                    NetworkVoiceEvent::VoiceData(packet) => {
                        // Update network statistics
                        update_network_stats(&mut chat.network_stats, packet);
                        
                        // Insert packet into jitter buffer
                        chat.jitter_buffer.insert(packet.clone());
                    }
                    NetworkVoiceEvent::NetworkStats(rtt, loss_rate) => {
                        // Update RTT with exponential smoothing
                        chat.network_stats.rtt = chat.network_stats.rtt * (1.0 - RTT_SMOOTHING_FACTOR) +
                            rtt * RTT_SMOOTHING_FACTOR;
                        chat.network_stats.packet_loss_rate = loss_rate;
                    }
                }
            }

            // Process outgoing voice data
            if chat.is_speaking && radio.powered && radio.transmitting {
                let current_time = SystemTime::now();
                
                // Create voice packet
                if !chat.voice_buffer.is_empty() && 
                   chat.voice_buffer.len() >= PACKET_SIZE {
                    let packet = VoicePacket {
                        sequence: chat.sequence_counter,
                        timestamp: current_time,
                        samples: chat.voice_buffer.drain(..PACKET_SIZE).collect(),
                    };
                    
                    // Send packet (you'll need to implement the actual network send)
                    send_voice_packet(&packet);
                    
                    // Update sequence counter
                    chat.sequence_counter = chat.sequence_counter.wrapping_add(1);
                }
                
                chat.last_packet_time = current_time;
            }

            // Adjust jitter buffer size based on network conditions
            chat.jitter_buffer.adjust_size(&chat.network_stats);
            
            // Update comfort noise generator
            chat.comfort_noise_generator.update_noise_floor(signal_quality);
        }
    }
}

fn update_network_stats(stats: &mut NetworkStats, packet: &VoicePacket) {
    // Calculate packet loss
    let expected_sequence = stats.last_sequence.wrapping_add(1);
    if packet.sequence > expected_sequence {
        let lost = packet.sequence.wrapping_sub(expected_sequence) as u64;
        stats.packets_lost += lost;
    }
    stats.packets_received += 1;
    
    // Update packet loss rate
    let total_packets = stats.packets_received + stats.packets_lost;
    if total_packets > 0 {
        stats.packet_loss_rate = stats.packets_lost as f32 / total_packets as f32;
    }
    
    // Update sequence tracking
    stats.last_sequence = packet.sequence;
}

#[derive(Debug, Clone)]
pub enum NetworkVoiceEvent {
    VoiceData(VoicePacket),
    NetworkStats(f32, f32), // RTT, loss rate
}

// Function to send voice packet over network
fn send_voice_packet(packet: &VoicePacket) {
    // TODO: Implement actual network send
    // This will depend on your networking implementation (e.g., bevy_networking_turbulence)
}

// Update the process_voice_output system to use the jitter buffer
fn process_voice_output(
    mut voice_chat: Query<&mut VoiceChat>,
    cb_radios: Query<(&CBRadio, &SignalQuality)>,
    audio: Res<Audio>,
    time: Res<Time>,
) {
    for mut chat in voice_chat.iter_mut() {
        if let Ok((radio, signal_quality)) = cb_radios.get_single() {
            if !radio.powered {
                continue;
            }

            let current_time = SystemTime::now();
            
            // Get next packet from jitter buffer
            if let Some(samples) = chat.jitter_buffer.get_next_packet(current_time) {
                // Apply radio effects
                let mut processed_samples = samples;
                apply_cb_radio_effects(&mut processed_samples, signal_quality);
                
                // Play the processed audio
                if let Some(buffer) = create_audio_buffer(&processed_samples) {
                    audio.play(buffer)
                        .with_volume(radio.volume);
                }
            } else {
                // Generate comfort noise during packet loss
                let comfort_noise = chat.comfort_noise_generator
                    .generate(FRAME_SIZE);
                
                if let Some(buffer) = create_audio_buffer(&comfort_noise) {
                    audio.play(buffer)
                        .with_volume(radio.volume * 0.3); // Lower volume for comfort noise
                }
            }
        }
    }
}

// Helper function to create audio buffer
fn create_audio_buffer(samples: &[f32]) -> Option<AudioBuffer> {
    Some(AudioBuffer::from_samples(
        samples,
        CHANNELS,
        SAMPLE_RATE as u32,
    ))
}

impl VoiceChat {
    pub fn new(config: VoiceChatConfig) -> Self {
        let filter_config = CBFilterChainConfig {
            sample_rate: config.sample_rate,
            ..Default::default()
        };
        
        Self {
            // ... existing field initialization ...
            
            filter_chain: CBFilterChain::new(filter_config),
            signal_quality: SignalQuality::default(),
        }
    }
    
    /// Process incoming audio data with CB radio effects
    pub fn process_voice_input(&mut self, input_buffer: &[f32]) -> Vec<f32> {
        let mut output_buffer = Vec::with_capacity(input_buffer.len());
        
        // Update filter parameters based on current signal quality
        self.filter_chain.update_parameters(self.signal_quality);
        
        // Process each sample through the filter chain
        for &sample in input_buffer {
            let processed = self.filter_chain.process(sample);
            output_buffer.push(processed);
        }
        
        output_buffer
    }
    
    /// Update the signal quality affecting voice effects
    pub fn update_signal_quality(&mut self, quality: SignalQuality) {
        self.signal_quality = quality;
    }
    
    /// Process voice output with network jitter compensation and effects
    pub fn process_voice_output(&mut self, packet: VoicePacket) -> Vec<f32> {
        // First apply network jitter compensation
        let mut output_buffer = if let Some(samples) = self.jitter_buffer.get_next_packet() {
            samples
        } else {
            // Generate comfort noise during packet loss
            self.comfort_noise_generator.generate_samples(packet.samples.len())
        };
        
        // Then apply CB radio effects
        output_buffer = self.process_voice_input(&output_buffer);
        
        output_buffer
    }

    /// Process incoming audio with voice activity detection
    pub fn process_audio_input(&mut self, input_buffer: &[f32], time: f32) -> bool {
        // Update VAD
        self.is_voice_active = self.vad.process_frame(input_buffer, time);
        self.last_vad_update = time;

        // If voice is active, process the audio
        if self.is_voice_active {
            let processed = self.process_voice_input(input_buffer);
            self.voice_buffer.extend(processed);
            
            // Send voice packet if buffer is full
            if self.voice_buffer.len() >= PACKET_SIZE {
                self.send_voice_packet();
            }
        }

        self.is_voice_active
    }

    /// Send a voice packet if conditions are met
    fn send_voice_packet(&mut self) {
        if self.voice_buffer.len() >= PACKET_SIZE {
            let packet = VoicePacket {
                sequence: self.sequence_counter,
                timestamp: SystemTime::now(),
                samples: self.voice_buffer.drain(..PACKET_SIZE).collect(),
            };
            
            // Send packet (you'll need to implement the actual network send)
            send_voice_packet(&packet);
            
            // Update sequence counter
            self.sequence_counter = self.sequence_counter.wrapping_add(1);
        }
    }

    /// Get current voice activity state
    pub fn is_voice_active(&self) -> bool {
        self.is_voice_active
    }

    /// Get current energy level in dB
    pub fn current_energy(&self) -> f32 {
        self.vad.current_energy()
    }

    /// Get current VAD threshold in dB
    pub fn vad_threshold(&self) -> f32 {
        self.vad.threshold()
    }
}