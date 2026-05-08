use bevy::prelude::*;
use bevy_kira_audio::{AudioControl, AudioInstance, AudioSource, AudioTween};
use std::collections::HashMap;
use super::{AudioAssets, AudioSettings};
use fastrand;
use super::cb_filter::{CBFilterChain, CBFilterConfig};
use std::ops::RangeInclusive;
use dasp_filter::window::Windowed;
use dasp_frame::Frame;
use dasp_sample::Sample;
use dasp_window::Window;
use rand::Rng;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TerrainType {
    Mountain,
    Urban,
    Forest,
    Tunnel,
    Water,
    Desert,
    Plains,
    Hill,
}

#[derive(Debug, Clone)]
pub struct InterferenceSource {
    pub position: Vec3,
    pub radius: f32,
    pub intensity: f32,
    pub source_type: InterferenceSourceType,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InterferenceSourceType {
    PowerLine,
    RadioTower,
    IndustrialEquipment,
    ElectronicDevice,
}

#[derive(Debug, Clone)]
pub struct WorldState {
    pub current_weather: WeatherType,
    pub time_of_day: f32, // 0-24 hour format
    pub vehicle_positions: Vec<Vec3>,
    pub interference_sources: Vec<InterferenceSource>,
    pub terrain_features: Vec<TerrainFeature>,
    pub temperature: f32,
    pub humidity: f32,
    pub pressure: f32,
}

impl WorldState {
    pub fn get_terrain_type_at(&self, position: Vec3) -> TerrainType {
        // This would normally query the game world's terrain system
        // For now return a default
        TerrainType::Plains
    }
}

/// Component for CB radio functionality
#[derive(Component, Debug)]
pub struct CBRadio {
    /// Current channel (1-40)
    pub channel: u8,
    /// Radio volume (0.0 - 1.0)
    pub volume: f32,
    /// Whether the radio is powered on
    pub powered: bool,
    /// Whether squelch is enabled
    pub squelch_enabled: bool,
    /// Current transmission state
    pub transmitting: bool,
    /// Signal strength for current reception (0.0 - 1.0)
    pub signal_strength: f32,
    /// Audio instance for static noise
    pub static_instance: Option<Handle<AudioInstance>>,
    /// Audio instance for received transmission
    pub transmission_instance: Option<Handle<AudioInstance>>,
    /// Last squelch trigger time
    pub last_squelch: f32,
    /// Emergency channel monitoring
    pub monitor_emergency: bool,
    /// Current signal quality metrics
    pub signal_quality: SignalQuality,
    /// Transmitter position for signal calculations
    pub transmitter_position: Option<Vec3>,
    /// Receiver position for signal calculations 
    pub receiver_position: Option<Vec3>,
    /// Filter chain for signal processing
    pub filter_chain: CBFilterChain,
}

impl Default for CBRadio {
    fn default() -> Self {
        Self {
            channel: 19, // Default to trucker's channel
            volume: 0.8,
            powered: true,
            squelch_enabled: true,
            transmitting: false,
            signal_strength: 0.0,
            static_instance: None,
            transmission_instance: None,
            last_squelch: 0.0,
            monitor_emergency: false,
            signal_quality: SignalQuality::default(),
            transmitter_position: None,
            receiver_position: None,
            filter_chain: CBFilterChain::new(CBFilterConfig::default()),
        }
    }
}

/// Resource for managing CB radio state
#[derive(Resource)]
pub struct CBRadioManager {
    /// Active transmissions per channel
    pub active_transmissions: Vec<Entity>,
    /// Emergency broadcasts
    pub emergency_broadcast: Option<Entity>,
    /// AI chatter cooldown
    ai_chatter_timer: Timer,
    /// Last update timestamp
    last_update: f32,
    /// Filter configuration
    pub config: CBFilterConfig,
}

impl Default for CBRadioManager {
    fn default() -> Self {
        Self {
            active_transmissions: Vec::new(),
            emergency_broadcast: None,
            ai_chatter_timer: Timer::from_seconds(30.0, TimerMode::Repeating),
            last_update: 0.0,
            config: CBFilterConfig::default(),
        }
    }
}

/// Configuration for CB radio effects
#[derive(Resource)]
pub struct CBRadioConfig {
    /// Maximum transmission range
    pub max_range: f32,
    /// Static noise intensity based on distance
    pub static_curve: f32,
    /// Squelch threshold for signal strength
    pub squelch_threshold: f32,
    /// Base static volume
    pub static_volume: f32,
    /// Transmission effect volume
    pub effect_volume: f32,
    /// AI chatter frequency (minutes)
    pub ai_chatter_frequency: f32,
}

impl Default for CBRadioConfig {
    fn default() -> Self {
        Self {
            max_range: 5000.0, // 5km range
            static_curve: 1.5,
            squelch_threshold: 0.2,
            static_volume: 0.3,
            effect_volume: 0.4,
            ai_chatter_frequency: 5.0,
        }
    }
}

/// Grid cell size in meters
const GRID_CELL_SIZE: f32 = 500.0;

/// A spatial partitioning grid for efficient interference source lookup
#[derive(Default, Resource)]
pub struct SpatialGrid {
    /// Maps grid cell coordinates to interference sources within that cell
    cells: HashMap<IVec2, Vec<Entity>>,
    /// Cache of entity positions for quick position checks without querying Transform
    entity_positions: HashMap<Entity, Vec2>,
}

impl SpatialGrid {
    /// Convert a world position to grid cell coordinates
    fn get_cell_coords(position: Vec2) -> IVec2 {
        IVec2::new(
            (position.x / GRID_CELL_SIZE).floor() as i32,
            (position.y / GRID_CELL_SIZE).floor() as i32,
        )
    }

    /// Update the position of an interference source in the grid
    pub fn update_position(&mut self, entity: Entity, new_pos: Vec2) {
        // Remove from old position if it exists
        if let Some(old_pos) = self.entity_positions.get(&entity) {
            let old_cell = Self::get_cell_coords(*old_pos);
            if let Some(sources) = self.cells.get_mut(&old_cell) {
                sources.retain(|&e| e != entity);
            }
        }

        // Add to new position
        let new_cell = Self::get_cell_coords(new_pos);
        self.cells.entry(new_cell).or_default().push(entity);
        self.entity_positions.insert(entity, new_pos);
    }

    /// Remove an interference source from the grid
    pub fn remove(&mut self, entity: Entity) {
        if let Some(pos) = self.entity_positions.remove(&entity) {
            let cell = Self::get_cell_coords(pos);
            if let Some(sources) = self.cells.get_mut(&cell) {
                sources.retain(|&e| e != entity);
            }
        }
    }

    /// Get all interference sources within range of a position
    pub fn get_sources_in_range(&self, position: Vec2, range: f32) -> Vec<Entity> {
        let mut result = Vec::new();
        let cell_range = (range / GRID_CELL_SIZE).ceil() as i32;
        let center_cell = Self::get_cell_coords(position);

        // Check all cells that could contain sources within range
        for dx in -cell_range..=cell_range {
            for dy in -cell_range..=cell_range {
                let cell = center_cell + IVec2::new(dx, dy);
                if let Some(sources) = self.cells.get(&cell) {
                    for &entity in sources {
                        if let Some(source_pos) = self.entity_positions.get(&entity) {
                            if (source_pos - position).length() <= range {
                                result.push(entity);
                            }
                        }
                    }
                }
            }
        }
        result
    }

    /// Clean up any empty cells to prevent memory growth
    pub fn cleanup_empty_cells(&mut self) {
        self.cells.retain(|_, sources| !sources.is_empty());
    }
}

/// Plugin for CB radio functionality
pub struct CBRadioPlugin;

impl Plugin for CBRadioPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CBRadioManager>()
           .init_resource::<CBRadioConfig>()
           .add_systems(Update, (
               update_cb_radio_state,
               handle_transmissions,
               update_signal_effects,
               trigger_ai_chatter,
               update_spatial_grid,
               update_signal_quality,
           ).chain());
    }
}

/// System to update CB radio state
fn update_cb_radio_state(
    mut radios: Query<(Entity, &mut CBRadio, &GlobalTransform)>,
    mut manager: ResMut<CBRadioManager>,
    config: Res<CBRadioConfig>,
    time: Res<Time>,
    world_state: Res<WorldState>,
) {
    // Clear old transmissions
    manager.active_transmissions.clear();
    manager.emergency_broadcast = None;

    // Update radio states
    for (entity, mut radio, transform) in radios.iter_mut() {
        if !radio.powered {
            radio.signal_strength = 0.0;
            continue;
        }

        // Update receiver position
        radio.receiver_position = Some(transform.translation());

        // Add active transmissions to manager
        if radio.transmitting {
            manager.active_transmissions.push(entity);

            // Track emergency broadcasts
            if radio.channel == 9 {
                manager.emergency_broadcast = Some(entity);
            }
        }

        // Calculate signal quality from nearby transmissions
        let pos = transform.translation();
        let mut max_signal = 0.0f32;

        for &transmitter in &manager.active_transmissions {
            if let Ok((other_radio, other_transform)) = radios.get(transmitter) {
                if other_radio.channel == radio.channel || (radio.monitor_emergency && other_radio.channel == 9) {
                    let tx_pos = other_transform.translation();
                    radio.transmitter_position = Some(tx_pos);
                    
                    // Calculate detailed signal quality
                    let signal = calculate_signal_quality(pos.distance(tx_pos));
                    
                    // Use signal strength from quality calculation
                    let signal_strength = signal.strength;
                    max_signal = max_signal.max(signal_strength);
                }
            }
        }

        radio.signal_strength = max_signal;
    }

    manager.last_update = time.elapsed_seconds();
}

/// System to handle radio transmissions
fn handle_transmissions(
    mut commands: Commands,
    mut radios: Query<(Entity, &mut CBRadio)>,
    audio_assets: Res<AudioAssets>,
    audio: Res<Audio>,
    time: Res<Time>,
) {
    for (entity, mut radio) in radios.iter_mut() {
        if !radio.powered {
            continue;
        }

        // Handle squelch effects
        if radio.squelch_enabled && radio.signal_strength > 0.0 {
            if time.elapsed_seconds() - radio.last_squelch > 0.1 {
                // Play squelch sound
                if let Some(squelch) = &audio_assets.cb_squelch {
                    audio.play(squelch.clone())
                        .with_volume(radio.volume * 0.5);
                }
                radio.last_squelch = time.elapsed_seconds();
            }
        }

        // Update static noise
        match (&radio.static_instance, radio.signal_strength) {
            (None, strength) if strength < 0.8 => {
                // Start static
                if let Some(static_sound) = &audio_assets.cb_static {
                    let instance = audio.play(static_sound.clone())
                        .looped()
                        .with_volume(radio.volume * (1.0 - strength))
                        .handle();
                    commands.entity(entity).insert(CBRadio {
                        static_instance: Some(instance),
                        ..(*radio)
                    });
                }
            }
            (Some(instance), strength) => {
                // Adjust static volume
                audio.set_volume(*instance, radio.volume * (1.0 - strength));
            }
            _ => {}
        }
    }
}

/// System to update signal effects
fn update_signal_effects(
    mut radios: Query<&mut CBRadio>,
    config: Res<CBRadioConfig>,
    audio: Res<Audio>,
) {
    for mut radio in radios.iter_mut() {
        if !radio.powered {
            continue;
        }

        // Get audio effects from signal quality
        let effects = radio.signal_quality.get_audio_effects();

        // Update transmission effects
        if let Some(instance) = radio.transmission_instance {
            let effect_volume = if radio.signal_strength > config.squelch_threshold {
                effects.volume * radio.volume
            } else {
                0.0
            };
            
            // Apply audio effects
            audio.set_volume(instance, effect_volume);
            
            // Apply filters and effects
            audio.set_high_pass(instance, effects.high_pass);
            audio.set_low_pass(instance, effects.low_pass);
            audio.set_distortion(instance, effects.distortion);
        }

        // Update static noise
        if let Some(static_instance) = radio.static_instance {
            audio.set_volume(static_instance, effects.noise_gain * radio.volume);
        }
    }
}

/// System to trigger AI radio chatter
fn trigger_ai_chatter(
    mut manager: ResMut<CBRadioManager>,
    mut commands: Commands,
    audio_assets: Res<AudioAssets>,
    config: Res<CBRadioConfig>,
    time: Res<Time>,
) {
    manager.ai_chatter_timer.tick(time.delta());

    if manager.ai_chatter_timer.just_finished() {
        // Random chance to trigger AI chatter
        if fastrand::f32() < 0.3 {
            // Spawn AI chatter entity
            commands.spawn((
                CBRadio {
                    channel: 19, // Trucker's channel
                    powered: true,
                    transmitting: true,
                    ..default()
                },
                // Add transform at random location
                Transform::from_xyz(
                    fastrand::f32() * 1000.0 - 500.0,
                    0.0,
                    fastrand::f32() * 1000.0 - 500.0,
                ),
                GlobalTransform::default(),
            ));

            // Set next interval
            let next_interval = fastrand::f32() * 
                (config.ai_chatter_frequency * 120.0) + 60.0; // 1-7 minutes
            manager.ai_chatter_timer.set_duration(
                std::time::Duration::from_secs_f32(next_interval)
            );
        }
    }
}

/// Helper function to spawn a CB radio entity
pub fn spawn_cb_radio(
    commands: &mut Commands,
    transform: Transform,
    channel: u8,
) -> Entity {
    commands.spawn((
        CBRadio {
            channel,
            powered: false,
            ..default()
        },
        transform,
        GlobalTransform::default(),
    )).id()
}

/// Helper function to calculate signal strength between two points
pub fn calculate_signal_strength(
    distance: f32,
    config: &CBRadioConfig,
) -> f32 {
    (1.0 - (distance / config.max_range).powf(config.static_curve))
        .clamp(0.0, 1.0)
}

#[derive(Debug, Clone, Copy)]
pub struct SignalQuality {
    pub strength: f32,
    pub clarity: f32,
    pub interference_level: f32,
    pub noise_floor: f32,
}

impl SignalQuality {
    pub fn new() -> Self {
        Self {
            strength: 1.0,
            clarity: 1.0,
            interference_level: 0.0,
            noise_floor: -90.0,
        }
    }

    pub fn calculate(&mut self, transmitter_pos: Vec3, receiver_pos: Vec3, world_state: &WorldState) {
        // Calculate base signal strength using inverse square law
        let distance = transmitter_pos.distance(receiver_pos);
        self.strength = 1.0 / (1.0 + distance * 0.01);

        // Apply atmospheric effects
        let atmospheric_attenuation = self.calculate_atmospheric_effects(&world_state);
        self.strength *= 1.0 - atmospheric_attenuation;

        // Apply terrain interference
        let terrain_interference = self.calculate_terrain_interference(&world_state);
        self.clarity *= 1.0 - terrain_interference;

        // Apply dynamic interference
        let dynamic_interference = self.calculate_dynamic_interference(&world_state);
        self.interference_level = dynamic_interference;

        // Adjust noise floor based on interference
        self.noise_floor = -90.0 + (40.0 * self.interference_level);

        // Clamp values
        self.strength = self.strength.clamp(0.0, 1.0);
        self.clarity = self.clarity.clamp(0.0, 1.0);
        self.interference_level = self.interference_level.clamp(0.0, 1.0);
    }

    fn calculate_atmospheric_effects(&self, world_state: &WorldState) -> f32 {
        let mut attenuation = 0.0;
        
        // Time of day effects (ionospheric propagation)
        let hour = world_state.time_of_day;
        if hour < 6.0 || hour > 18.0 {
            // Better propagation at night
            attenuation -= 0.1;
        }
        
        // Temperature effects
        let temp = world_state.temperature;
        if temp > 30.0 {
            // Hot air can cause signal ducting
            attenuation += (temp - 30.0) * 0.01;
        } else if temp < 0.0 {
            // Cold air tends to be more stable
            attenuation -= temp.abs() * 0.005;
        }
        
        // Humidity effects
        let humidity = world_state.humidity;
        attenuation += humidity * 0.2; // Higher humidity = more attenuation
        
        // Atmospheric pressure effects
        let pressure = world_state.pressure;
        let standard_pressure = 1013.25; // hPa
        let pressure_diff = (pressure - standard_pressure).abs() / 100.0;
        attenuation += pressure_diff * 0.1;
        
        attenuation.clamp(0.0, 1.0)
    }

    fn calculate_terrain_interference(&self, world_state: &WorldState) -> f32 {
        let mut interference = 0.0;
        
        // Process terrain features between transmitter and receiver
        for feature in &world_state.terrain_features {
            match feature.terrain_type {
                TerrainType::Mountain => {
                    // Mountains cause significant interference
                    interference += feature.height * 0.002;
                }
                TerrainType::Hill => {
                    // Hills cause moderate interference
                    interference += feature.height * 0.001;
                }
                TerrainType::Forest => {
                    // Dense foliage causes some interference
                    interference += feature.density * 0.3;
                }
                TerrainType::Urban => {
                    // Buildings cause significant interference
                    interference += feature.density * 0.5;
                }
                TerrainType::Water => {
                    // Water can actually improve propagation
                    interference -= feature.size * 0.001;
                }
            }
        }

        // Line of sight check
        if let (Some(tx_pos), Some(rx_pos)) = (world_state.transmitter_position, world_state.receiver_position) {
            let distance = tx_pos.distance(rx_pos);
            let height_diff = (tx_pos.y - rx_pos.y).abs();
            
            // Calculate approximate earth curvature effect
            let earth_radius = 6371000.0; // meters
            let curvature_height = (distance.powi(2)) / (2.0 * earth_radius);
            
            if height_diff < curvature_height {
                // Signal is affected by earth's curvature
                interference += (curvature_height - height_diff) * 0.0001;
            }
        }
        
        interference.clamp(0.0, 1.0)
    }

    fn calculate_dynamic_interference(&self, world_state: &WorldState) -> f32 {
        let mut interference = 0.0;

        // Process each interference source
        for source in &world_state.interference_sources {
            let distance = source.position.distance(Vec3::ZERO);
            if distance <= source.radius {
                let factor = 1.0 - (distance / source.radius);
                interference += source.intensity * factor;

                // Add source-specific effects
                match source.source_type {
                    InterferenceSourceType::PowerLine => {
                        interference += 0.2 * fastrand::f32();
                    }
                    InterferenceSourceType::RadioTower => {
                        interference += 0.3 * fastrand::f32();
                    }
                    InterferenceSourceType::IndustrialEquipment => {
                        interference += 0.25 * fastrand::f32();
                    }
                    InterferenceSourceType::ElectronicDevice => {
                        interference += 0.15 * fastrand::f32();
                    }
                }
            }
        }

        // Add random noise bursts
        if fastrand::f32() < 0.05 {
            interference += fastrand::f32() * 0.3;
        }

        interference.clamp(0.0, 1.0)
    }

    pub fn get_audio_effects(&self) -> AudioEffects {
        AudioEffects {
            volume: self.strength,
            high_pass: 20.0 + (1000.0 * (1.0 - self.clarity)),
            low_pass: 10000.0 * self.clarity,
            noise_gain: self.interference_level * 0.5,
            distortion: (1.0 - self.clarity) * 0.3,
        }
    }

    fn calculate_path_loss(&mut self, distance: f32) {
        // Free space path loss calculation
        // FSPL = 20 * log10(d) + 20 * log10(f) + 32.44
        // where d is distance in km and f is frequency in MHz
        
        let frequency = 27.0; // CB radio frequency in MHz
        let distance_km = distance / 1000.0;
        
        let fspl = 20.0 * distance_km.log10() + 
                  20.0 * frequency.log10() + 
                  32.44;
                  
        // Convert path loss to signal strength (0-1)
        let max_loss = 120.0; // Maximum expected path loss in dB
        self.strength = (1.0 - (fspl / max_loss)).clamp(0.0, 1.0);
    }
}

#[derive(Debug, Clone)]
pub struct AudioEffects {
    pub volume: f32,
    pub high_pass: f32,
    pub low_pass: f32,
    pub noise_gain: f32,
    pub distortion: f32,
}

/// Audio effect control trait implementations
pub trait AudioEffectControl {
    fn set_high_pass(&self, instance: Handle<AudioInstance>, frequency: f32);
    fn set_low_pass(&self, instance: Handle<AudioInstance>, frequency: f32);
    fn set_distortion(&self, instance: Handle<AudioInstance>, amount: f32);
}

impl AudioEffectControl for Audio {
    fn set_high_pass(&self, instance: Handle<AudioInstance>, frequency: f32) {
        self.set_filter(instance, AudioFilter::HighPass(frequency));
    }

    fn set_low_pass(&self, instance: Handle<AudioInstance>, frequency: f32) {
        self.set_filter(instance, AudioFilter::LowPass(frequency));
    }

    fn set_distortion(&self, instance: Handle<AudioInstance>, amount: f32) {
        self.set_effect(instance, AudioEffect::Distortion(amount));
    }
}

#[derive(Debug, Clone)]
pub enum AudioFilter {
    LowPass(f32),
    HighPass(f32),
    BandPass(f32, f32),
}

#[derive(Debug, Clone)]
pub enum AudioEffect {
    Distortion(f32),
    Delay(f32),
    Echo(f32),
}

impl CBRadio {
    pub fn update_signal_effects(&mut self, audio: &Audio, world_state: &WorldState) {
        // Calculate signal quality based on current conditions
        let signal_quality = SignalQuality::calculate(
            self.transmitter_position,
            self.receiver_position,
            world_state,
        );

        // Update stored signal quality
        self.signal_quality = signal_quality;

        // Apply audio effects based on signal quality
        if let Some(instance) = self.transmission_instance {
            // High-pass filter based on signal strength
            let high_pass_freq = 200.0 + (1.0 - signal_quality.strength) * 800.0;
            audio.set_high_pass(instance, high_pass_freq);

            // Low-pass filter based on interference
            let low_pass_freq = 8000.0 - signal_quality.interference_level * 4000.0;
            audio.set_low_pass(instance, low_pass_freq);

            // Distortion based on noise floor
            let distortion = signal_quality.noise_floor * 0.5;
            audio.set_distortion(instance, distortion);

            // Volume adjustment based on signal strength
            let volume = signal_quality.strength.max(0.1);
            audio.set_volume(instance, volume);
        }
    }

    pub fn update_signal_quality(&mut self, world_state: &WorldState) {
        // Update signal quality calculations
        let signal_quality = SignalQuality::calculate(
            self.transmitter_position,
            self.receiver_position,
            world_state,
        );

        // Store the updated signal quality
        self.signal_quality = signal_quality;

        // Emit events based on significant signal quality changes
        if signal_quality.strength < 0.2 {
            // Emit weak signal event
            self.emit_event(CBRadioEvent::WeakSignal);
        } else if signal_quality.interference_level > 0.8 {
            // Emit high interference event
            self.emit_event(CBRadioEvent::HighInterference);
        }
    }

    fn emit_event(&self, event: CBRadioEvent) {
        // Event emission logic here
        println!("CB Radio Event: {:?}", event);
    }
}

#[derive(Debug)]
pub enum CBRadioEvent {
    WeakSignal,
    HighInterference,
    SignalLost,
    SignalRestored,
}

// Add system to update spatial grid
pub fn update_spatial_grid(
    mut grid: ResMut<SpatialGrid>,
    query: Query<(Entity, &Transform), With<InterferenceSource>>,
) {
    for (entity, transform) in query.iter() {
        let pos = Vec2::new(transform.translation.x, transform.translation.z);
        grid.update_position(entity, pos);
    }
    grid.cleanup_empty_cells();
}

// Update the signal quality calculation to use the spatial grid
impl SignalQuality {
    pub fn calculate_interference(
        &self,
        position: Vec2,
        grid: &SpatialGrid,
        query: &Query<&InterferenceSource>,
    ) -> f32 {
        let mut total_interference = 0.0;
        
        // Get sources within reasonable range (2000m)
        for entity in grid.get_sources_in_range(position, 2000.0) {
            if let Ok(source) = query.get(entity) {
                let source_pos = Vec2::new(source.position.x, source.position.z);
                let distance = (source_pos - position).length();
                
                if distance <= source.radius {
                    // Calculate interference based on distance and source properties
                    let falloff = 1.0 - (distance / source.radius).clamp(0.0, 1.0);
                    total_interference += source.intensity * falloff;
                }
            }
        }
        
        total_interference.clamp(0.0, 1.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WeatherType {
    Clear,
    Rain { intensity: f32 },
    Storm { intensity: f32, lightning: bool },
    Fog { density: f32 },
    Snow { intensity: f32 },
}

impl WeatherType {
    /// Get the signal attenuation factor for this weather type
    pub fn get_attenuation(&self) -> f32 {
        match self {
            WeatherType::Clear => 0.0,
            WeatherType::Rain { intensity } => {
                // Rain causes moderate signal attenuation
                intensity * 0.3
            }
            WeatherType::Storm { intensity, lightning } => {
                // Storms cause significant attenuation, especially with lightning
                let base_attenuation = intensity * 0.5;
                if *lightning {
                    // Lightning can cause brief but intense interference
                    base_attenuation + 0.3
                } else {
                    base_attenuation
                }
            }
            WeatherType::Fog { density } => {
                // Fog causes mild signal attenuation
                density * 0.2
            }
            WeatherType::Snow { intensity } => {
                // Snow causes moderate to high attenuation
                intensity * 0.4
            }
        }
    }

    /// Get additional noise floor increase due to weather
    pub fn get_noise_floor_increase(&self) -> f32 {
        match self {
            WeatherType::Clear => 0.0,
            WeatherType::Rain { intensity } => intensity * 0.2,
            WeatherType::Storm { intensity, lightning } => {
                let base_noise = intensity * 0.4;
                if *lightning {
                    base_noise + 0.4
                } else {
                    base_noise
                }
            }
            WeatherType::Fog { density } => density * 0.1,
            WeatherType::Snow { intensity } => intensity * 0.25,
        }
    }
}

impl SignalQuality {
    pub fn calculate_weather_effects(&mut self, weather: WeatherType) {
        // Apply weather-based attenuation to signal strength
        let attenuation = weather.get_attenuation();
        self.strength *= 1.0 - attenuation;

        // Increase noise floor based on weather conditions
        let noise_increase = weather.get_noise_floor_increase();
        self.noise_floor += noise_increase;

        // Weather can also affect signal clarity
        self.clarity *= 1.0 - (attenuation * 0.5);
    }

    pub fn update(&mut self, world_state: &WorldState, grid: &SpatialGrid, interference_query: &Query<&InterferenceSource>) {
        // Calculate base path loss
        self.calculate_path_loss(world_state.distance);
        
        // Apply weather effects
        self.calculate_weather_effects(world_state.current_weather);
        
        // Calculate terrain interference
        let terrain_interference = self.calculate_terrain_interference(world_state);
        self.clarity *= 1.0 - terrain_interference;
        
        // Calculate dynamic interference using spatial grid
        let dynamic_interference = self.calculate_interference(
            Vec2::new(world_state.position.x, world_state.position.z),
            grid,
            interference_query
        );
        self.interference_level = dynamic_interference;
        
        // Ensure values stay within valid ranges
        self.clamp_values();
    }

    fn clamp_values(&mut self) {
        self.strength = self.strength.clamp(0.0, 1.0);
        self.clarity = self.clarity.clamp(0.0, 1.0);
        self.noise_floor = self.noise_floor.clamp(0.0, 1.0);
        self.interference_level = self.interference_level.clamp(0.0, 1.0);
    }
}

pub fn update_signal_quality(
    mut cb_radios: Query<(&mut CBRadio, &GlobalTransform)>,
    world_state: Res<WorldState>,
    spatial_grid: Res<SpatialGrid>,
    interference_sources: Query<&InterferenceSource>,
    time: Res<Time>,
) {
    for (mut radio, transform) in cb_radios.iter_mut() {
        // Only update if radio is on
        if !radio.powered {
            continue;
        }

        // Create world state for this radio's position
        let position = transform.translation();
        let distance = position.distance(world_state.listener_position);
        
        let local_state = WorldState {
            position,
            distance,
            current_weather: world_state.current_weather,
            time_of_day: world_state.time_of_day,
            ..*world_state
        };

        // Update signal quality
        radio.signal_quality.update(&local_state, &spatial_grid, &interference_sources);

        // Apply audio effects based on signal quality
        if let Some(instance) = radio.transmission_instance {
            radio.apply_audio_effects(instance);
        }

        // Update static noise volume based on signal quality
        radio.update_static_noise(time.delta_seconds());
    }
}

impl CBRadio {
    fn apply_audio_effects(&self, instance: Handle<AudioInstance>) {
        let sq = &self.signal_quality;
        
        // Calculate filter frequencies based on signal quality
        let low_pass_freq = 1000.0 + (sq.clarity * 4000.0); // Range: 1000Hz - 5000Hz
        let high_pass_freq = 80.0 + ((1.0 - sq.clarity) * 120.0); // Range: 80Hz - 200Hz
        
        // Calculate distortion amount based on interference
        let distortion = sq.interference_level * 0.8; // Range: 0.0 - 0.8
        
        // Apply effects
        Audio::set_low_pass(instance, low_pass_freq);
        Audio::set_high_pass(instance, high_pass_freq);
        Audio::set_distortion(instance, distortion);
    }

    fn update_static_noise(&mut self, dt: f32) {
        let sq = &self.signal_quality;
        
        // Calculate base static volume from signal quality
        let target_volume = (1.0 - sq.strength) * 0.5 + 
                          sq.noise_floor * 0.3 +
                          sq.interference_level * 0.2;
        
        // Smoothly interpolate current volume to target
        let t = (dt * 5.0).min(1.0); // 5 = speed of volume change
        self.static_volume = self.static_volume * (1.0 - t) + target_volume * t;
        
        // Update static noise instance volume if it exists
        if let Some(instance) = self.static_instance {
            Audio::set_volume(instance, self.static_volume);
        }
    }
}

#[derive(Clone, Debug)]
pub struct EmergencyBroadcast {
    incident_id: String,
    status: EmergencyStatus,
    timestamp: f32,
    message: String,
}

#[derive(Clone, Debug, PartialEq)]
pub enum EmergencyStatus {
    Reported,
    UnitsDispatched,
    UnitsOnScene,
    Resolved,
    Cancelled,
}

impl CBRadioManager {
    pub fn update_emergency(&mut self, incident_id: &str, status: EmergencyStatus, time: f32) {
        let message = match status {
            EmergencyStatus::Reported => format!("Emergency reported. Incident ID: {}", incident_id),
            EmergencyStatus::UnitsDispatched => format!("Units dispatched to incident {}", incident_id),
            EmergencyStatus::UnitsOnScene => format!("Units on scene at incident {}", incident_id),
            EmergencyStatus::Resolved => format!("Incident {} has been resolved", incident_id),
            EmergencyStatus::Cancelled => format!("Incident {} has been cancelled", incident_id),
        };

        let broadcast = EmergencyBroadcast {
            incident_id: incident_id.to_string(),
            status,
            timestamp: time,
            message,
        };

        // Broadcast emergency update on channel 9 (emergency channel)
        if let Some(radio) = self.radios.iter_mut().find(|r| r.channel == 9) {
            radio.broadcast_emergency(broadcast);
        }
    }
}

impl CBRadio {
    fn broadcast_emergency(&mut self, broadcast: EmergencyBroadcast) {
        // Only broadcast if radio is on and tuned to channel 9
        if self.power && self.channel == 9 {
            // Apply emergency tone effect
            self.apply_emergency_tone();
            
            // Queue emergency message for transmission
            self.message_queue.push_back(Message {
                content: broadcast.message,
                priority: MessagePriority::Emergency,
                timestamp: broadcast.timestamp,
            });
        }
    }

    fn apply_emergency_tone(&mut self) {
        // Generate attention-getting tone sequence before emergency broadcast
        if let Some(audio_instance) = &self.current_audio {
            // Play emergency alert tones (specific frequencies and durations)
            let tone_sequence = vec![
                (853.0, 0.5), // P1
                (960.0, 0.5), // P2
                (853.0, 0.5), // P3
                (960.0, 0.5), // P4
            ];

            for (frequency, duration) in tone_sequence {
                // Apply tone through audio system
                // This would integrate with the actual audio system implementation
                // to generate the emergency alert tones
                self.play_tone(frequency, duration);
            }
        }
    }
}

// Add terrain feature struct
#[derive(Debug, Clone)]
pub struct TerrainFeature {
    pub terrain_type: TerrainType,
    pub height: f32,
    pub density: f32,
    pub size: f32,
    pub position: Vec3,
}

fn calculate_signal_quality(distance: f32) -> SignalQuality {
    const MAX_RANGE: f32 = 5000.0; // 5km max range
    const CLARITY_FALLOFF: f32 = 0.7; // How quickly clarity degrades with distance
    const INTERFERENCE_BUILDUP: f32 = 0.3; // How quickly interference builds up

    let strength = (1.0 - (distance / MAX_RANGE)).max(0.0);
    let clarity = (1.0 - (distance / MAX_RANGE).powf(CLARITY_FALLOFF)).max(0.0);
    let interference = ((distance / MAX_RANGE) * INTERFERENCE_BUILDUP).min(1.0);

    SignalQuality {
        strength,
        clarity,
        interference_level: interference,
        noise_floor: -90.0 + (interference * 30.0), // Noise floor rises with interference
    }
}

#[derive(Resource)]
pub struct CBRadioState {
    pub current_channel: CBRadioChannel,
    pub is_transmitting: bool,
    pub is_receiving: bool,
    pub signal_strength: f32,
}

impl Default for CBRadioState {
    fn default() -> Self {
        Self {
            current_channel: CBRadioChannel(19), // Channel 19 is the default truckers' channel
            is_transmitting: false,
            is_receiving: false,
            signal_strength: 1.0,
        }
    }
}

#[derive(Resource)]
pub struct CBRadioVolume(pub f32);

impl Default for CBRadioVolume {
    fn default() -> Self {
        Self(0.5) // Default to 50% volume
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CBRadioChannel(pub u8);

// Constants for CB radio configuration
pub const MAX_CHANNEL: u8 = 40;
pub const EMERGENCY_CHANNEL: u8 = 9;
pub const TRUCKER_CHANNEL: u8 = 19;

// Signal degradation configuration
pub const MAX_RADIO_RANGE: f32 = 5000.0; // Maximum range in meters
pub const MIN_SIGNAL_STRENGTH: f32 = 0.1; // Minimum signal strength (10%)

pub const CHANNEL_RANGE: RangeInclusive<u8> = 1..=40;

impl Default for CBRadioChannel {
    fn default() -> Self {
        Self(19) // Default to trucker's channel
    }
}

impl CBRadioChannel {
    pub fn next(&self) -> Self {
        let next_channel = if self.0 >= *CHANNEL_RANGE.end() {
            *CHANNEL_RANGE.start()
        } else {
            self.0 + 1
        };
        Self(next_channel)
    }

    pub fn prev(&self) -> Self {
        let prev_channel = if self.0 <= *CHANNEL_RANGE.start() {
            *CHANNEL_RANGE.end()
        } else {
            self.0 - 1
        };
        Self(prev_channel)
    }

    pub fn is_emergency(&self) -> bool {
        self.0 == EMERGENCY_CHANNEL
    }

    pub fn is_trucker(&self) -> bool {
        self.0 == TRUCKER_CHANNEL
    }
}

pub struct CBRadioPlugin;

impl Plugin for CBRadioPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CBRadioVolume>()
           .init_resource::<CBRadioState>()
           .add_systems(Update, (
               update_radio_state,
               handle_radio_input,
           ));
    }
}

// System to update radio state and signal strength
fn update_radio_state(
    mut radio_state: ResMut<CBRadioState>,
    time: Res<Time>,
) {
    let mut rng = rand::thread_rng();
    
    // Simulate signal strength fluctuations
    if radio_state.signal_strength > 0.0 {
        let noise = (rng.gen::<f32>() - 0.5) * 0.1;
        radio_state.signal_strength = (radio_state.signal_strength + noise)
            .clamp(0.0, 1.0);
    }
}

// System to handle radio input
fn handle_radio_input(
    mut radio_state: ResMut<CBRadioState>,
    keyboard: Res<Input<KeyCode>>,
) {
    // Channel up/down
    if keyboard.just_pressed(KeyCode::BracketRight) {
        radio_state.current_channel = CBRadioChannel::next(radio_state.current_channel);
    }
    if keyboard.just_pressed(KeyCode::BracketLeft) {
        radio_state.current_channel = CBRadioChannel::prev(radio_state.current_channel);
    }

    // Quick channel selection
    if keyboard.just_pressed(KeyCode::Key9) {
        radio_state.current_channel = CBRadioChannel(EMERGENCY_CHANNEL);
    }
    if keyboard.just_pressed(KeyCode::Key0) {
        radio_state.current_channel = CBRadioChannel(TRUCKER_CHANNEL);
    }

    // Push-to-talk
    radio_state.is_transmitting = keyboard.pressed(KeyCode::T);

    // Toggle squelch
    if keyboard.just_pressed(KeyCode::S) {
        radio_state.squelch_enabled = !radio_state.squelch_enabled;
    }
}