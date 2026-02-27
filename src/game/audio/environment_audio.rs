use std::time::Duration;
use bevy::prelude::*;
use bevy_kira_audio::{AudioControl, AudioInstance, AudioSource, AudioTween};
use rand::prelude::*;
use bevy::gizmos::gizmos::Gizmos;
use std::collections::{HashMap, HashSet};
use super::config::*;

use super::{AudioAssets, AudioSettings, SpatialAudioSource};

/// Component for configuring environmental audio behavior
#[derive(Component)]
pub struct EnvironmentAudioSource {
    /// Current environment type
    pub environment_type: EnvironmentType,
    /// Current weather type
    pub weather_type: WeatherType,
    /// Time of day (0.0 - 24.0)
    pub time_of_day: f32,
    /// Crossfade duration for smooth transitions
    pub crossfade_duration: f32,
    /// Current crossfade progress (0.0 - 1.0)
    pub crossfade_progress: f32,
    /// Previous environment type for crossfading
    pub previous_environment: Option<EnvironmentType>,
    /// Previous weather type for crossfading
    pub previous_weather: Option<WeatherType>,
    pub weather_instance: Option<Handle<AudioInstance>>,
    pub ambient_instances: Vec<Handle<AudioInstance>>,
    pub oneshot_timer: Timer,
    pub current_volume: f32,
    pub target_volume: f32,
    pub volume_smoothing: f32,
    pub current_reverb: ReverbSettings,
    pub target_reverb: ReverbSettings,
    pub reverb_smoothing: f32,
    pub last_distance_check: f32,
    pub active_transition_sounds: Vec<Handle<AudioInstance>>,
}

/// Different types of environments with their own ambient sounds
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EnvironmentType {
    City,
    Highway,
    Forest,
    Desert,
    Mountain,
    Beach,
    Swamp,
}

/// Different weather conditions affecting ambient sounds
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WeatherType {
    Clear,
    Rain,
    Storm,
    Wind,
    Snow,
}

/// Weather transition effects configuration
#[derive(Clone)]
pub struct WeatherTransitionConfig {
    pub fade_duration: f32,
    pub crossfade_curve: TransitionCurve,
    pub intensity: f32,
    pub transition_sounds: Vec<String>,
}

#[derive(Clone, Copy)]
pub enum TransitionCurve {
    Linear,
    EaseIn,
    EaseOut,
    EaseInOut,
}

impl Default for EnvironmentAudioSource {
    fn default() -> Self {
        Self {
            environment_type: EnvironmentType::City,
            weather_type: WeatherType::Clear,
            time_of_day: 12.0,
            crossfade_duration: 2.0,
            crossfade_progress: 0.0,
            previous_environment: None,
            previous_weather: None,
            weather_instance: None,
            ambient_instances: Vec::new(),
            oneshot_timer: Timer::from_seconds(5.0, TimerMode::Repeating),
            current_volume: 0.0,
            target_volume: 0.0,
            volume_smoothing: 0.1,
            current_reverb: ReverbSettings::default(),
            target_reverb: ReverbSettings::default(),
            reverb_smoothing: 0.1,
            last_distance_check: 0.0,
            active_transition_sounds: Vec::new(),
        }
    }
}

/// Resource for managing environment audio state
#[derive(Resource)]
pub struct EnvironmentAudioManager {
    /// Active environment sound sources
    active_sources: HashMap<(EnvironmentType, WeatherType), Entity>,
    /// One-shot sound cooldowns
    one_shot_cooldowns: HashMap<String, f32>,
    /// Random number generator for one-shot sounds
    rng: fastrand::Rng,
}

impl Default for EnvironmentAudioManager {
    fn default() -> Self {
        Self {
            active_sources: HashMap::new(),
            one_shot_cooldowns: HashMap::new(),
            rng: fastrand::Rng::new(),
        }
    }
}

/// Component for visualizing audio zones
#[derive(Component)]
pub struct AudioZoneDebug {
    pub color: Color,
    pub radius: f32,
    pub show_transition: bool,
}

/// Enhanced environment configuration with transition settings
#[derive(Clone)]
pub struct EnvironmentConfig {
    pub base_volume: f32,
    pub transition_radius: f32,
    pub ambient_sounds: Vec<String>,
    pub oneshot_sounds: Vec<String>,
    pub reverb_settings: ReverbSettings,
    pub weather_transitions: HashMap<(WeatherType, WeatherType), WeatherTransitionConfig>,
    pub time_of_day_volumes: [(f32, f32); 24], // (hour, volume multiplier)
    pub distance_attenuation: f32,
    pub occlusion_factor: f32,
}

#[derive(Clone)]
pub struct ReverbSettings {
    pub room_size: f32,
    pub damping: f32,
    pub wet_level: f32,
    pub dry_level: f32,
}

impl Default for ReverbSettings {
    fn default() -> Self {
        Self {
            room_size: 0.5,
            damping: 0.5,
            wet_level: 0.33,
            dry_level: 0.4,
        }
    }
}

impl Default for EnvironmentConfig {
    fn default() -> Self {
        let mut weather_transitions = HashMap::new();
        
        // Clear to Rain transition
        weather_transitions.insert(
            (WeatherType::Clear, WeatherType::Rain),
            WeatherTransitionConfig {
                fade_duration: 4.0,
                crossfade_curve: TransitionCurve::EaseIn,
                intensity: 0.8,
                transition_sounds: vec!["light_rain_start".to_string()],
            },
        );

        // Rain to Storm transition
        weather_transitions.insert(
            (WeatherType::Rain, WeatherType::Storm),
            WeatherTransitionConfig {
                fade_duration: 5.0,
                crossfade_curve: TransitionCurve::EaseInOut,
                intensity: 1.0,
                transition_sounds: vec!["thunder_distant".to_string(), "wind_rising".to_string()],
            },
        );

        // Default hourly volumes (example: quieter at night)
        let time_of_day_volumes = [
            (0.0, 0.3), (1.0, 0.3), (2.0, 0.3), (3.0, 0.3), // Night (0-3)
            (4.0, 0.4), (5.0, 0.5), (6.0, 0.6), (7.0, 0.7), // Dawn (4-7)
            (8.0, 0.8), (9.0, 0.9), (10.0, 1.0), (11.0, 1.0), // Morning (8-11)
            (12.0, 1.0), (13.0, 1.0), (14.0, 1.0), (15.0, 1.0), // Afternoon (12-15)
            (16.0, 1.0), (17.0, 0.9), (18.0, 0.8), (19.0, 0.7), // Evening (16-19)
            (20.0, 0.6), (21.0, 0.5), (22.0, 0.4), (23.0, 0.3), // Night (20-23)
        ];

        Self {
            base_volume: 0.5,
            transition_radius: 10.0,
            ambient_sounds: Vec::new(),
            oneshot_sounds: Vec::new(),
            reverb_settings: ReverbSettings::default(),
            weather_transitions,
            time_of_day_volumes,
            distance_attenuation: 1.0,
            occlusion_factor: 0.5,
        }
    }
}

/// Resource for environment-specific configurations
#[derive(Resource)]
pub struct EnvironmentConfigs {
    configs: HashMap<EnvironmentType, EnvironmentConfig>,
}

impl Default for EnvironmentConfigs {
    fn default() -> Self {
        let mut configs = HashMap::new();
        
        // City environment
        configs.insert(EnvironmentType::City, EnvironmentConfig {
            base_volume: 0.6,
            transition_radius: 15.0,
            ambient_sounds: vec!["city_ambient".to_string(), "traffic".to_string()],
            oneshot_sounds: vec!["car_horn".to_string(), "siren".to_string()],
            reverb_settings: ReverbSettings {
                room_size: 0.8,
                damping: 0.4,
                wet_level: 0.4,
                dry_level: 0.6,
            },
            weather_transitions: HashMap::new(),
            time_of_day_volumes: [(0.0, 0.3), (1.0, 0.3), (2.0, 0.3), (3.0, 0.3), (4.0, 0.4), (5.0, 0.5), (6.0, 0.6), (7.0, 0.7), (8.0, 0.8), (9.0, 0.9), (10.0, 1.0), (11.0, 1.0), (12.0, 1.0), (13.0, 1.0), (14.0, 1.0), (15.0, 1.0), (16.0, 1.0), (17.0, 0.9), (18.0, 0.8), (19.0, 0.7), (20.0, 0.6), (21.0, 0.5), (22.0, 0.4), (23.0, 0.3)],
            distance_attenuation: 1.0,
            occlusion_factor: 0.5,
        });

        // Forest environment
        configs.insert(EnvironmentType::Forest, EnvironmentConfig {
            base_volume: 0.5,
            transition_radius: 20.0,
            ambient_sounds: vec!["forest_ambient".to_string(), "wind_trees".to_string()],
            oneshot_sounds: vec!["bird_call".to_string(), "branch_snap".to_string()],
            reverb_settings: ReverbSettings {
                room_size: 0.9,
                damping: 0.3,
                wet_level: 0.5,
                dry_level: 0.5,
            },
            weather_transitions: HashMap::new(),
            time_of_day_volumes: [(0.0, 0.3), (1.0, 0.3), (2.0, 0.3), (3.0, 0.3), (4.0, 0.4), (5.0, 0.5), (6.0, 0.6), (7.0, 0.7), (8.0, 0.8), (9.0, 0.9), (10.0, 1.0), (11.0, 1.0), (12.0, 1.0), (13.0, 1.0), (14.0, 1.0), (15.0, 1.0), (16.0, 1.0), (17.0, 0.9), (18.0, 0.8), (19.0, 0.7), (20.0, 0.6), (21.0, 0.5), (22.0, 0.4), (23.0, 0.3)],
            distance_attenuation: 1.0,
            occlusion_factor: 0.5,
        });

        // Desert environment
        configs.insert(EnvironmentType::Desert, EnvironmentConfig {
            base_volume: 0.4,
            transition_radius: 25.0,
            ambient_sounds: vec!["desert_wind".to_string(), "sand_drift".to_string()],
            oneshot_sounds: vec!["dust_devil".to_string(), "rock_slide".to_string()],
            reverb_settings: ReverbSettings {
                room_size: 0.95,
                damping: 0.2,
                wet_level: 0.3,
                dry_level: 0.7,
            },
            weather_transitions: {
                let mut transitions = HashMap::new();
                transitions.insert(
                    (WeatherType::Clear, WeatherType::Wind),
                    WeatherTransitionConfig {
                        fade_duration: 6.0,
                        crossfade_curve: TransitionCurve::EaseIn,
                        intensity: 0.9,
                        transition_sounds: vec!["wind_rising".to_string(), "sand_swirl".to_string()],
                    },
                );
                transitions
            },
            time_of_day_volumes: [
                (0.0, 0.2), (1.0, 0.2), (2.0, 0.2), (3.0, 0.2), // Night (very quiet)
                (4.0, 0.3), (5.0, 0.5), (6.0, 0.7), (7.0, 0.9), // Dawn (increasing)
                (8.0, 1.0), (9.0, 1.0), (10.0, 1.0), (11.0, 1.0), // Day (full)
                (12.0, 1.0), (13.0, 1.0), (14.0, 1.0), (15.0, 1.0), // Day (full)
                (16.0, 0.9), (17.0, 0.7), (18.0, 0.5), (19.0, 0.3), // Dusk (decreasing)
                (20.0, 0.2), (21.0, 0.2), (22.0, 0.2), (23.0, 0.2), // Night (very quiet)
            ],
            distance_attenuation: 0.8,
            occlusion_factor: 0.3,
        });

        // Mountain environment
        configs.insert(EnvironmentType::Mountain, EnvironmentConfig {
            base_volume: 0.5,
            transition_radius: 30.0,
            ambient_sounds: vec!["high_wind".to_string(), "distant_avalanche".to_string()],
            oneshot_sounds: vec!["rock_fall".to_string(), "eagle_cry".to_string()],
            reverb_settings: ReverbSettings {
                room_size: 1.0,
                damping: 0.1,
                wet_level: 0.6,
                dry_level: 0.4,
            },
            weather_transitions: {
                let mut transitions = HashMap::new();
                transitions.insert(
                    (WeatherType::Clear, WeatherType::Snow),
                    WeatherTransitionConfig {
                        fade_duration: 8.0,
                        crossfade_curve: TransitionCurve::EaseInOut,
                        intensity: 0.7,
                        transition_sounds: vec!["wind_howl".to_string(), "snow_start".to_string()],
                    },
                );
                transitions
            },
            time_of_day_volumes: [
                (0.0, 0.4), (1.0, 0.4), (2.0, 0.4), (3.0, 0.4), // Night (moderate)
                (4.0, 0.6), (5.0, 0.8), (6.0, 1.0), (7.0, 1.0), // Dawn (increasing)
                (8.0, 1.0), (9.0, 1.0), (10.0, 1.0), (11.0, 1.0), // Day (full)
                (12.0, 1.0), (13.0, 1.0), (14.0, 1.0), (15.0, 1.0), // Day (full)
                (16.0, 0.8), (17.0, 0.6), (18.0, 0.5), (19.0, 0.4), // Dusk (decreasing)
                (20.0, 0.4), (21.0, 0.4), (22.0, 0.4), (23.0, 0.4), // Night (moderate)
            ],
            distance_attenuation: 0.6,
            occlusion_factor: 0.2,
        });

        // Add more environment configurations...
        
        Self { configs }
    }
}

/// Plugin to handle environmental audio
pub struct EnvironmentAudioPlugin;

impl Plugin for EnvironmentAudioPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<EnvironmentAudioManager>()
           .init_resource::<EnvironmentConfigs>()
           .add_systems(Update, (
               update_environment_audio,
               handle_environment_transitions,
               trigger_one_shot_sounds,
               visualize_audio_zones,
           ).chain());
    }
}

/// System to update environmental audio based on location and conditions
fn update_environment_audio(
    mut commands: Commands,
    mut query: Query<(Entity, &mut EnvironmentAudioSource, &GlobalTransform)>,
    audio_assets: Res<AudioAssets>,
    configs: Res<EnvironmentConfigs>,
    time: Res<Time>,
    audio: Res<Audio>,
    game_time: Res<GameTime>, // You'll need to create this resource
) {
    for (entity, mut source, transform) in query.iter_mut() {
        if let Some(config) = configs.configs.get(&source.environment_type) {
            // Update volume based on time of day
            let current_hour = game_time.hour() as usize;
            let next_hour = (current_hour + 1) % 24;
            let hour_progress = game_time.minute() as f32 / 60.0;
            
            let current_volume = config.time_of_day_volumes[current_hour].1;
            let next_volume = config.time_of_day_volumes[next_hour].1;
            let interpolated_volume = lerp(current_volume, next_volume, hour_progress);
            
            source.target_volume = interpolated_volume * config.base_volume;
            
            // Smooth volume transitions
            source.current_volume = lerp(
                source.current_volume,
                source.target_volume,
                source.volume_smoothing,
            );
            
            // Update ambient sounds
            source.oneshot_timer.tick(time.delta());
            if source.oneshot_timer.just_finished() {
                for sound_config in &config.oneshot_sounds {
                    if should_play_sound(sound_config, &source, transform.translation(), &game_time) {
                        if let Some(sound) = audio_assets.get_environment_sound(&sound_config.sound_name) {
                            let pitch = 1.0 + (rand::random::<f32>() - 0.5) * sound_config.pitch_variation;
                            audio.play(sound.clone())
                                .with_volume(sound_config.base_volume * source.current_volume)
                                .with_pitch(pitch);
                        }
                    }
                }
                
                // Set next interval randomly between min and max
                let next_interval = rand::random::<f32>() * 
                    (config.oneshot_sounds[0].max_interval - config.oneshot_sounds[0].min_interval) +
                    config.oneshot_sounds[0].min_interval;
                source.oneshot_timer.set_duration(Duration::from_secs_f32(next_interval));
            }
            
            // Update reverb settings
            source.target_reverb = config.reverb_settings.clone();
            source.current_reverb = ReverbSettings {
                room_size: lerp(
                    source.current_reverb.room_size,
                    source.target_reverb.room_size,
                    source.reverb_smoothing,
                ),
                damping: lerp(
                    source.current_reverb.damping,
                    source.target_reverb.damping,
                    source.reverb_smoothing,
                ),
                dry_level: lerp(
                    source.current_reverb.dry_level,
                    source.target_reverb.dry_level,
                    source.reverb_smoothing,
                ),
                wet_level: lerp(
                    source.current_reverb.wet_level,
                    source.target_reverb.wet_level,
                    source.reverb_smoothing,
                ),
            };
            
            // Apply current settings to all active sounds
            for instance in source.ambient_instances.iter() {
                audio.set_volume(*instance, source.current_volume);
                // Apply reverb settings here using your audio system's reverb interface
            }
            
            if let Some(weather) = source.weather_instance.as_ref() {
                audio.set_volume(*weather, source.current_volume);
                // Apply reverb to weather sounds
            }
        }
    }
}

/// System to handle transitions between environments and weather conditions
fn handle_environment_transitions(
    mut commands: Commands,
    audio_assets: Res<AudioAssets>,
    mut env_sources: Query<(Entity, &mut EnvironmentAudioSource)>,
    mut env_manager: ResMut<EnvironmentAudioManager>,
) {
    for (entity, mut source) in env_sources.iter_mut() {
        // Check if we need to start a transition
        if source.previous_environment.is_none() && source.previous_weather.is_none() {
            let key = (source.environment_type, source.weather_type);
            
            // Create new audio source if needed
            if !env_manager.active_sources.contains_key(&key) {
                let audio_index = match source.environment_type {
                    EnvironmentType::City => 0,
                    EnvironmentType::Highway => 1,
                    EnvironmentType::Forest => 2,
                    EnvironmentType::Desert => 3,
                    EnvironmentType::Mountain => 4,
                    EnvironmentType::Beach => 5,
                    EnvironmentType::Swamp => 6,
                };

                commands.spawn((
                    AudioBundle {
                        source: audio_assets.environment_sounds[audio_index].clone(),
                        settings: PlaybackSettings::LOOP.with_spatial(true),
                    },
                    SpatialAudioSource {
                        base_volume: 0.6,
                        max_distance: 200.0,
                        reference_distance: 20.0,
                        rolloff_factor: 0.5,
                        doppler_factor: 0.0,
                        looping: true,
                        priority: AudioPriority::Medium,
                        occlusion: None,
                    },
                ));

                env_manager.active_sources.insert(key, entity);
            }
        }
    }
}

/// System to trigger one-shot environmental sounds
fn trigger_one_shot_sounds(
    mut commands: Commands,
    audio_assets: Res<AudioAssets>,
    mut env_manager: ResMut<EnvironmentAudioManager>,
    time: Res<Time>,
) {
    // Update cooldowns
    env_manager.one_shot_cooldowns.retain(|_, cooldown| {
        *cooldown -= time.delta_seconds();
        *cooldown > 0.0
    });

    // Chance to trigger new one-shot sounds
    if env_manager.rng.f32() < 0.01 {  // 1% chance per frame
        let sound_key = "ambient_oneshot";
        
        if !env_manager.one_shot_cooldowns.contains_key(sound_key) {
            // Spawn one-shot sound
            commands.spawn((
                AudioBundle {
                    source: audio_assets.environment_sounds[env_manager.rng.usize(..3)].clone(),
                    settings: PlaybackSettings::DESPAWN,
                },
                SpatialAudioSource {
                    base_volume: 0.4,
                    max_distance: 150.0,
                    reference_distance: 15.0,
                    rolloff_factor: 1.0,
                    doppler_factor: 0.0,
                    looping: false,
                    priority: AudioPriority::Low,
                    occlusion: None,
                },
            ));

            // Set cooldown
            env_manager.one_shot_cooldowns.insert(sound_key.to_string(), 10.0);
        }
    }
}

/// Linear interpolation helper
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Resource to configure environment audio behavior
#[derive(Resource)]
pub struct EnvironmentAudioConfig {
    pub base_volume: f32,
    pub weather_volume: f32,
    pub oneshot_volume: f32,
    pub transition_duration: f32,
    pub min_oneshot_interval: f32,
    pub max_oneshot_interval: f32,
}

impl Default for EnvironmentAudioConfig {
    fn default() -> Self {
        Self {
            base_volume: 0.5,
            weather_volume: 0.4,
            oneshot_volume: 0.6,
            transition_duration: 3.0,
            min_oneshot_interval: 10.0,
            max_oneshot_interval: 30.0,
        }
    }
}

/// System to handle weather transitions
fn handle_weather_transitions(
    mut commands: Commands,
    mut env_query: Query<(Entity, &mut EnvironmentAudioSource)>,
    audio_assets: Res<AudioAssets>,
    configs: Res<EnvironmentConfigs>,
    time: Res<Time>,
    audio: Res<Audio>,
) {
    for (entity, mut env_audio) in env_query.iter_mut() {
        if let Some(config) = configs.configs.get(&env_audio.environment_type) {
            // Check for weather transition
            if let Some(prev_weather) = env_audio.previous_weather {
                let transition_key = (prev_weather, env_audio.weather_type);
                if let Some(transition_config) = config.weather_transitions.get(&transition_key) {
                    // Update transition progress
                    env_audio.crossfade_progress += time.delta_seconds() / transition_config.fade_duration;
                    
                    // Apply transition curve
                    let curved_progress = apply_transition_curve(
                        env_audio.crossfade_progress,
                        transition_config.crossfade_curve,
                    );

                    // Update volumes based on transition
                    if let Some(prev_handle) = env_audio.weather_instance.as_ref() {
                        audio.set_volume(
                            prev_handle,
                            (1.0 - curved_progress) * transition_config.intensity,
                        );
                    }

                    // Play transition sounds if we haven't yet
                    if env_audio.crossfade_progress < 0.1 {
                        for sound_name in &transition_config.transition_sounds {
                            if let Some(sound) = audio_assets.get_weather_transition_sound(sound_name) {
                                audio.play(sound.clone())
                                    .with_volume(transition_config.intensity);
                            }
                        }
                    }

                    // Complete transition
                    if env_audio.crossfade_progress >= 1.0 {
                        env_audio.previous_weather = None;
                        env_audio.crossfade_progress = 0.0;
                    }
                }
            }
        }
    }
}

/// System to play random ambient one-shot sounds
fn play_ambient_oneshots(
    mut env_query: Query<&mut EnvironmentAudioSource>,
    time: Res<Time>,
    audio_assets: Res<AudioAssets>,
    config: Res<EnvironmentAudioConfig>,
    audio: Res<bevy_kira_audio::Audio>,
) {
    let mut rng = rand::thread_rng();

    for mut env_audio in env_query.iter_mut() {
        env_audio.last_oneshot += time.delta_seconds();

        if env_audio.last_oneshot >= env_audio.oneshot_cooldown {
            // Select appropriate one-shot sounds based on environment and weather
            let oneshot_pool = match (env_audio.environment_type, env_audio.weather_type) {
                (EnvironmentType::Forest, _) => &audio_assets.environment_sounds[2..5],
                (EnvironmentType::City, _) => &audio_assets.environment_sounds[0..2],
                (EnvironmentType::Beach, _) => &audio_assets.environment_sounds[5..6],
                // Add more combinations as needed
                _ => &[],  // Empty slice for environments without one-shots
            };

            if !oneshot_pool.is_empty() {
                let sound = oneshot_pool.choose(&mut rng).unwrap();
                
                audio.play(sound.clone())
                    .with_volume(config.oneshot_volume)
                    .handle();

                // Set new random cooldown
                env_audio.oneshot_cooldown = rng.gen_range(
                    config.min_oneshot_interval..=config.max_oneshot_interval
                );
                env_audio.last_oneshot = 0.0;
            }
        }
    }
}

/// System to visualize audio zones
fn visualize_audio_zones(
    mut gizmos: Gizmos,
    query: Query<(&Transform, &EnvironmentAudioSource, &AudioZoneDebug)>,
) {
    for (transform, source, debug) in query.iter() {
        // Draw main zone
        gizmos.circle(
            transform.translation,
            transform.up(),
            debug.radius,
            debug.color.with_alpha(0.3),
        );

        // Draw transition zone if enabled
        if debug.show_transition {
            let transition_radius = debug.radius + source.crossfade_duration * 5.0;
            gizmos.circle(
                transform.translation,
                transform.up(),
                transition_radius,
                debug.color.with_alpha(0.1),
            );
        }
    }
}

/// Helper function to spawn environment audio for a location
pub fn spawn_environment_audio(
    commands: &mut Commands,
    env_type: EnvironmentType,
    audio_assets: &AudioAssets,
    audio: &bevy_kira_audio::Audio,
    config: &EnvironmentAudioConfig,
) -> Entity {
    // Get the appropriate environment audio handle
    let env_handle = match env_type {
        EnvironmentType::City => &audio_assets.environment_sounds[0],
        EnvironmentType::Highway => &audio_assets.environment_sounds[1],
        EnvironmentType::Forest => &audio_assets.environment_sounds[2],
        EnvironmentType::Desert => &audio_assets.environment_sounds[3],
        EnvironmentType::Mountain => &audio_assets.environment_sounds[4],
        EnvironmentType::Beach => &audio_assets.environment_sounds[5],
        EnvironmentType::Swamp => &audio_assets.environment_sounds[6],
    };

    // Start playing the environment audio
    let env_instance = audio
        .play(env_handle.clone())
        .looped()
        .with_volume(config.base_volume)
        .handle();

    // Spawn the entity with EnvironmentAudio component
    commands
        .spawn(EnvironmentAudioSource {
            environment_type: env_type,
            weather_type: WeatherType::Clear,
            time_of_day: 12.0,
            crossfade_duration: 2.0,
            crossfade_progress: 0.0,
            previous_environment: None,
            previous_weather: None,
            weather_instance: None,
            ambient_instances: Vec::new(),
            oneshot_timer: Timer::from_seconds(5.0, TimerMode::Repeating),
            current_volume: 0.0,
            target_volume: 0.0,
            volume_smoothing: 0.1,
            current_reverb: ReverbSettings::default(),
            target_reverb: ReverbSettings::default(),
            reverb_smoothing: 0.1,
            last_distance_check: 0.0,
            active_transition_sounds: Vec::new(),
        })
        .id()
}

/// Helper function to spawn environment audio with debug visualization
pub fn spawn_environment_audio_with_debug(
    commands: &mut Commands,
    environment_type: EnvironmentType,
    transform: Transform,
    configs: &EnvironmentConfigs,
) -> Entity {
    let config = configs.configs.get(&environment_type).unwrap_or(&EnvironmentConfig::default());
    
    let color = match environment_type {
        EnvironmentType::City => Color::rgb(0.8, 0.2, 0.2),
        EnvironmentType::Forest => Color::rgb(0.2, 0.8, 0.2),
        EnvironmentType::Beach => Color::rgb(0.8, 0.8, 0.2),
        EnvironmentType::Mountain => Color::rgb(0.5, 0.5, 0.8),
        EnvironmentType::Desert => Color::rgb(0.8, 0.6, 0.2),
        EnvironmentType::Highway => Color::rgb(0.4, 0.4, 0.4),
        EnvironmentType::Swamp => Color::rgb(0.4, 0.6, 0.2),
    };

    commands.spawn((
        EnvironmentAudioSource {
            environment_type,
            weather_type: WeatherType::Clear,
            time_of_day: 12.0,
            crossfade_duration: 2.0,
            crossfade_progress: 0.0,
            previous_environment: None,
            previous_weather: None,
            weather_instance: None,
            ambient_instances: Vec::new(),
            oneshot_timer: Timer::from_seconds(5.0, TimerMode::Repeating),
            current_volume: 0.0,
            target_volume: 0.0,
            volume_smoothing: 0.1,
            current_reverb: ReverbSettings::default(),
            target_reverb: ReverbSettings::default(),
            reverb_smoothing: 0.1,
            last_distance_check: 0.0,
            active_transition_sounds: Vec::new(),
        },
        AudioZoneDebug {
            color,
            radius: config.transition_radius,
            show_transition: true,
        },
        transform,
        SpatialAudioSource::new(transform.translation),
    )).id()
}

/// Helper function for weather transitions
fn apply_transition_curve(progress: f32, curve: TransitionCurve) -> f32 {
    match curve {
        TransitionCurve::Linear => progress,
        TransitionCurve::EaseIn => progress * progress,
        TransitionCurve::EaseOut => 1.0 - (1.0 - progress) * (1.0 - progress),
        TransitionCurve::EaseInOut => {
            if progress < 0.5 {
                2.0 * progress * progress
            } else {
                1.0 - (-2.0 * progress + 2.0).powi(2) / 2.0
            }
        }
    }
}

// Helper function to determine if a sound should play based on conditions
fn should_play_sound(
    config: &EnvironmentSoundConfig,
    source: &EnvironmentAudioSource,
    position: Vec3,
    game_time: &GameTime,
) -> bool {
    // Check weather conditions
    if !config.weather_conditions.contains(&source.weather_type) {
        return false;
    }
    
    // Check time range if specified
    if let Some((start_time, end_time)) = config.time_range {
        let current_time = game_time.hour() as f32 + game_time.minute() as f32 / 60.0;
        if end_time > start_time {
            if current_time < start_time || current_time > end_time {
                return false;
            }
        } else {
            // Handle wraparound (e.g., 22:00 - 04:00)
            if current_time < start_time && current_time > end_time {
                return false;
            }
        }
    }
    
    // Check height range if specified
    if let Some((min_height, max_height)) = config.height_range {
        if position.y < min_height || position.y > max_height {
            return false;
        }
    }
    
    // Add random chance based on distance
    let distance_factor = (config.distance_range.1 - position.length()) /
        (config.distance_range.1 - config.distance_range.0);
    let play_chance = distance_factor.clamp(0.0, 1.0);
    
    rand::random::<f32>() < play_chance
}

#[derive(Component)]
pub struct AudioZone {
    pub environment_type: String,
    pub position: Vec3,
    pub radius: f32,
    pub priority: i32,
    pub blend_factor: f32,
}

#[derive(Component)]
pub struct AudioZoneOverlap {
    pub zones: Vec<(Entity, f32)>, // Entity and blend factor
    pub current_blend: HashMap<Entity, f32>,
    pub transition_timer: f32,
}

#[derive(Resource)]
pub struct EnvironmentAudioState {
    pub config: EnvironmentAudioConfig,
    pub active_zones: HashSet<Entity>,
    pub overlap_regions: Vec<(Vec3, f32)>, // Position and radius of overlap regions
    pub last_update: f32,
}

impl Default for EnvironmentAudioState {
    fn default() -> Self {
        Self {
            config: create_example_config(),
            active_zones: HashSet::new(),
            overlap_regions: Vec::new(),
            last_update: 0.0,
        }
    }
}

pub fn update_audio_zones(
    mut commands: Commands,
    time: Res<Time>,
    mut state: ResMut<EnvironmentAudioState>,
    zones: Query<(Entity, &AudioZone, &GlobalTransform)>,
    mut overlaps: Query<(Entity, &mut AudioZoneOverlap)>,
) {
    state.overlap_regions.clear();
    let mut new_overlaps = Vec::new();
    let mut existing_overlaps = HashSet::new();

    // Find overlapping zones
    for (entity_a, zone_a, transform_a) in zones.iter() {
        let pos_a = transform_a.translation();
        
        for (entity_b, zone_b, transform_b) in zones.iter() {
            if entity_a == entity_b {
                continue;
            }

            let pos_b = transform_b.translation();
            let distance = pos_a.distance(pos_b);
            let combined_radius = zone_a.radius + zone_b.radius;

            if distance < combined_radius {
                let overlap_center = pos_a.lerp(pos_b, 0.5);
                let overlap_radius = (combined_radius - distance) * 0.5;
                state.overlap_regions.push((overlap_center, overlap_radius));

                // Calculate blend factors based on priority and distance
                let blend_a = if zone_a.priority > zone_b.priority {
                    1.0 - (distance / combined_radius)
                } else {
                    distance / combined_radius
                };

                new_overlaps.push((
                    entity_a,
                    entity_b,
                    blend_a,
                    overlap_center,
                    overlap_radius
                ));
            }
        }
    }

    // Update or create overlap components
    for (entity_a, entity_b, blend_a, center, radius) in new_overlaps {
        let mut found = false;
        
        // Check existing overlaps
        for (overlap_entity, mut overlap) in overlaps.iter_mut() {
            if overlap.zones.iter().any(|(e, _)| *e == entity_a) &&
               overlap.zones.iter().any(|(e, _)| *e == entity_b) {
                // Update existing overlap
                overlap.zones = vec![(entity_a, blend_a), (entity_b, 1.0 - blend_a)];
                existing_overlaps.insert(overlap_entity);
                found = true;
                break;
            }
        }

        if !found {
            // Create new overlap
            commands.spawn((
                AudioZoneOverlap {
                    zones: vec![(entity_a, blend_a), (entity_b, 1.0 - blend_a)],
                    current_blend: HashMap::new(),
                    transition_timer: 0.0,
                },
                Transform::from_translation(center),
            ));
        }
    }

    // Remove outdated overlaps
    for (entity, _) in overlaps.iter() {
        if !existing_overlaps.contains(&entity) {
            commands.entity(entity).despawn();
        }
    }

    state.last_update = time.elapsed_seconds();
}

pub fn apply_audio_zone_effects(
    time: Res<Time>,
    state: Res<EnvironmentAudioState>,
    mut zones: Query<(&AudioZone, &mut Transform)>,
    mut overlaps: Query<&mut AudioZoneOverlap>,
) {
    let dt = time.delta_seconds();

    // Update audio effects for each overlap region
    for mut overlap in overlaps.iter_mut() {
        let transition_duration = state.config.global_settings.default_transition_duration;
        overlap.transition_timer = (overlap.transition_timer + dt).min(transition_duration);
        let t = overlap.transition_timer / transition_duration;

        // Calculate and apply blended audio parameters
        for (zone_entity, target_blend) in overlap.zones.iter() {
            let current_blend = overlap.current_blend.entry(*zone_entity)
                .or_insert(0.0);
            *current_blend = lerp(*current_blend, *target_blend, t);

            if let Ok((zone, mut transform)) = zones.get_mut(*zone_entity) {
                if let Some(env_config) = state.config.get_environment_config(&zone.environment_type) {
                    // Apply blended parameters (volume, reverb, etc.)
                    zone.blend_factor = *current_blend;
                    // Additional audio parameter blending can be added here
                }
            }
        }
    }
}

pub fn visualize_audio_zones(
    mut gizmos: Gizmos,
    state: Res<EnvironmentAudioState>,
    zones: Query<(&AudioZone, &GlobalTransform)>,
    overlaps: Query<(&AudioZoneOverlap, &Transform)>,
) {
    if !state.config.global_settings.debug_visualization.enabled {
        return;
    }

    // Draw zone boundaries
    for (zone, transform) in zones.iter() {
        let color = get_environment_color(&zone.environment_type);
        gizmos.circle(
            transform.translation(),
            transform.up(),
            zone.radius,
            color.with_alpha(0.3),
        );
    }

    // Draw overlap regions
    if state.config.global_settings.debug_visualization.show_overlap_regions {
        for (overlap, transform) in overlaps.iter() {
            gizmos.circle(
                transform.translation,
                Vec3::Y,
                10.0, // Overlap indicator size
                Color::YELLOW.with_alpha(0.5),
            );

            // Draw lines to connected zones
            for (zone_entity, blend) in overlap.zones.iter() {
                if let Ok((zone, zone_transform)) in zones.get(*zone_entity) {
                    let alpha = *blend;
                    gizmos.line(
                        transform.translation,
                        zone_transform.translation(),
                        Color::WHITE.with_alpha(alpha),
                    );
                }
            }
        }
    }
}

fn get_environment_color(env_type: &str) -> Color {
    match env_type.to_lowercase().as_str() {
        "desert" => Color::rgb(0.8, 0.7, 0.2),
        "mountain" => Color::rgb(0.6, 0.6, 0.6),
        "forest" => Color::rgb(0.2, 0.8, 0.2),
        "city" => Color::rgb(0.7, 0.7, 0.7),
        _ => Color::WHITE,
    }
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

pub struct EnvironmentAudioPlugin;

impl Plugin for EnvironmentAudioPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<EnvironmentAudioState>()
            .add_systems(Update, (
                update_audio_zones,
                apply_audio_zone_effects,
                visualize_audio_zones,
            ).chain());
    }
} 