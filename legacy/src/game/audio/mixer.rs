use bevy::prelude::*;
use std::collections::HashMap;
use super::cb_radio::CBRadio;

/// Audio bus types for routing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AudioBus {
    Master,
    Radio,
    VoiceChat,
    Effects,
    Music,
    Ambient,
}

/// Audio routing configuration
#[derive(Debug, Clone)]
pub struct AudioRouting {
    /// Volume for each bus
    bus_volumes: HashMap<AudioBus, f32>,
    /// Mute state for each bus
    bus_mutes: HashMap<AudioBus, bool>,
    /// Sends between buses (source -> (destination, gain))
    bus_sends: HashMap<AudioBus, Vec<(AudioBus, f32)>>,
}

impl Default for AudioRouting {
    fn default() -> Self {
        let mut routing = Self {
            bus_volumes: HashMap::new(),
            bus_mutes: HashMap::new(),
            bus_sends: HashMap::new(),
        };
        
        // Set up default routing
        routing.set_volume(AudioBus::Master, 1.0);
        routing.set_volume(AudioBus::Radio, 0.8);
        routing.set_volume(AudioBus::VoiceChat, 1.0);
        routing.set_volume(AudioBus::Effects, 0.7);
        routing.set_volume(AudioBus::Music, 0.5);
        routing.set_volume(AudioBus::Ambient, 0.4);
        
        // Set up default sends
        routing.add_send(AudioBus::Radio, AudioBus::Master, 1.0);
        routing.add_send(AudioBus::VoiceChat, AudioBus::Radio, 1.0);
        routing.add_send(AudioBus::Effects, AudioBus::Master, 1.0);
        routing.add_send(AudioBus::Music, AudioBus::Master, 1.0);
        routing.add_send(AudioBus::Ambient, AudioBus::Master, 1.0);
        
        routing
    }
}

impl AudioRouting {
    /// Set volume for a bus
    pub fn set_volume(&mut self, bus: AudioBus, volume: f32) {
        self.bus_volumes.insert(bus, volume.clamp(0.0, 1.0));
    }
    
    /// Get volume for a bus
    pub fn get_volume(&self, bus: AudioBus) -> f32 {
        *self.bus_volumes.get(&bus).unwrap_or(&1.0)
    }
    
    /// Set mute state for a bus
    pub fn set_mute(&mut self, bus: AudioBus, muted: bool) {
        self.bus_mutes.insert(bus, muted);
    }
    
    /// Get mute state for a bus
    pub fn is_muted(&self, bus: AudioBus) -> bool {
        *self.bus_mutes.get(&bus).unwrap_or(&false)
    }
    
    /// Add a send from one bus to another
    pub fn add_send(&mut self, from: AudioBus, to: AudioBus, gain: f32) {
        self.bus_sends.entry(from)
            .or_insert_with(Vec::new)
            .push((to, gain.clamp(0.0, 1.0)));
    }
    
    /// Remove a send
    pub fn remove_send(&mut self, from: AudioBus, to: AudioBus) {
        if let Some(sends) = self.bus_sends.get_mut(&from) {
            sends.retain(|(dest, _)| *dest != to);
        }
    }
    
    /// Get the final gain for a signal path
    pub fn get_path_gain(&self, from: AudioBus, to: AudioBus) -> f32 {
        let mut gain = 1.0;
        let mut current = from;
        
        // Follow the signal path
        while current != to {
            if let Some(sends) = self.bus_sends.get(&current) {
                if let Some((next_bus, send_gain)) = sends.iter()
                    .find(|(dest, _)| *dest == to || self.has_path_to(*dest, to)) {
                    gain *= send_gain * self.get_volume(current);
                    if self.is_muted(current) {
                        return 0.0;
                    }
                    current = *next_bus;
                } else {
                    return 0.0; // No path found
                }
            } else {
                return 0.0; // Dead end
            }
        }
        
        gain * self.get_volume(to) * if self.is_muted(to) { 0.0 } else { 1.0 }
    }
    
    /// Check if there's a path from one bus to another
    fn has_path_to(&self, from: AudioBus, to: AudioBus) -> bool {
        if from == to {
            return true;
        }
        
        if let Some(sends) = self.bus_sends.get(&from) {
            sends.iter().any(|(dest, _)| *dest == to || self.has_path_to(*dest, to))
        } else {
            false
        }
    }
}

/// Resource for managing audio routing
#[derive(Resource)]
pub struct AudioMixer {
    /// Routing configuration
    routing: AudioRouting,
    /// Current output levels
    levels: HashMap<AudioBus, f32>,
    /// Peak levels
    peak_levels: HashMap<AudioBus, f32>,
    /// Peak hold time
    peak_hold_time: f32,
    /// Last update time
    last_update: f32,
}

impl Default for AudioMixer {
    fn default() -> Self {
        Self {
            routing: AudioRouting::default(),
            levels: HashMap::new(),
            peak_levels: HashMap::new(),
            peak_hold_time: 2.0,
            last_update: 0.0,
        }
    }
}

impl AudioMixer {
    /// Process audio for a specific bus
    pub fn process_audio(&mut self, bus: AudioBus, samples: &mut [f32], time: f32) {
        // Apply bus volume and mute
        let gain = if self.routing.is_muted(bus) {
            0.0
        } else {
            self.routing.get_volume(bus)
        };
        
        // Process samples
        for sample in samples.iter_mut() {
            *sample *= gain;
            
            // Update levels
            let level = sample.abs();
            let current_level = self.levels.entry(bus).or_insert(0.0);
            *current_level = current_level.max(level);
            
            // Update peak levels
            let peak_level = self.peak_levels.entry(bus).or_insert(0.0);
            if level > *peak_level {
                *peak_level = level;
                self.last_update = time;
            }
        }
        
        // Reset peaks after hold time
        if time - self.last_update > self.peak_hold_time {
            self.peak_levels.insert(bus, 0.0);
        }
    }
    
    /// Get the current level for a bus
    pub fn get_level(&self, bus: AudioBus) -> f32 {
        *self.levels.get(&bus).unwrap_or(&0.0)
    }
    
    /// Get the peak level for a bus
    pub fn get_peak_level(&self, bus: AudioBus) -> f32 {
        *self.peak_levels.get(&bus).unwrap_or(&0.0)
    }
    
    /// Get the routing configuration
    pub fn routing(&mut self) -> &mut AudioRouting {
        &mut self.routing
    }
}

/// Plugin for audio mixing and routing
pub struct AudioMixerPlugin;

impl Plugin for AudioMixerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AudioMixer>()
           .add_systems(Update, update_audio_mixer);
    }
}

/// System to update the audio mixer
fn update_audio_mixer(
    mut mixer: ResMut<AudioMixer>,
    time: Res<Time>,
    cb_radios: Query<&CBRadio>,
) {
    // Reset levels
    mixer.levels.clear();
    
    // Process radio audio
    for radio in cb_radios.iter() {
        if radio.powered {
            // Calculate effective volume including signal quality
            let volume = radio.volume * radio.signal_strength;
            mixer.routing.set_volume(AudioBus::Radio, volume);
        } else {
            mixer.routing.set_mute(AudioBus::Radio, true);
        }
    }
    
    // Update time-based effects
    if time.elapsed_seconds() - mixer.last_update > mixer.peak_hold_time {
        mixer.peak_levels.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_routing() {
        let mut routing = AudioRouting::default();
        
        // Test volume control
        routing.set_volume(AudioBus::Master, 0.5);
        assert_eq!(routing.get_volume(AudioBus::Master), 0.5);
        
        // Test muting
        routing.set_mute(AudioBus::Music, true);
        assert!(routing.is_muted(AudioBus::Music));
        
        // Test signal path
        routing.add_send(AudioBus::VoiceChat, AudioBus::Radio, 0.8);
        routing.add_send(AudioBus::Radio, AudioBus::Master, 0.9);
        
        let gain = routing.get_path_gain(AudioBus::VoiceChat, AudioBus::Master);
        assert!(gain > 0.0);
    }

    #[test]
    fn test_audio_mixer() {
        let mut mixer = AudioMixer::default();
        let mut samples = vec![0.5f32; 1000];
        
        // Process some audio
        mixer.process_audio(AudioBus::Music, &mut samples, 0.0);
        
        // Check levels
        assert!(mixer.get_level(AudioBus::Music) > 0.0);
        assert!(mixer.get_peak_level(AudioBus::Music) > 0.0);
    }
} 