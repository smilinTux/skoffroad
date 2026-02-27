use bevy::prelude::*;
use bevy_kira_audio::{Audio, AudioControl};
use crate::game::plugins::particle_system::{
    // ParticleEffect,
    // ParticleEffectBundle,
    // ParticleEmitter,
    ParticleMaterial,
};
use super::{Weather, WeatherState, WeatherSoundSettings, TimeOfDay};
use super::profiler::WeatherProfiler;
use glam::FloatExt;

// Minimal placeholder for ParticleEffectBundle
pub struct ParticleEffectBundle {
    // Add fields as needed for weather effects
}

/// Types of weather-related particle effects
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WeatherEffectType {
    Rain,
    HeavyRain,
    DrizzleRain,
    Snow,
    Blizzard,
    LightSnow,
    Fog,
    ThickFog,
    Dust,
    Sandstorm,
    Lightning,
    ThunderStorm,
    Hail,
    Mist,
    FreezingRain,
    RainbowMist,
    IceParticles,
    Puddles,
    SnowAccumulation,
    DustDeposit,
    LeafDebris,
    Pollen,
    AshParticles,
}

/// Resource that manages weather-related particle effects
#[derive(Resource)]
pub struct WeatherEffects {
    /// Currently active effect entities
    active_effects: Vec<(WeatherEffectType, Entity)>,
    /// Ground effect entities
    ground_effects: Vec<(WeatherEffectType, Entity)>,
    /// Sound effect handles
    sound_effects: Vec<Handle<AudioSource>>,
    /// Maximum number of effects per type
    max_effects: usize,
    /// Current transition state for effects
    transitions: Vec<(WeatherEffectType, f32, f32)>, // (type, current_intensity, target_intensity)
    /// Random offset for particle variation
    random_seed: f32,
    /// Wind influence factor
    wind_factor: f32,
    /// Persistence duration for ground effects
    persistence_duration: f32,
    /// Time since last ground effect update
    ground_effect_timer: f32,
    profiler: WeatherProfiler,
}

impl Default for WeatherEffects {
    fn default() -> Self {
        Self {
            active_effects: Vec::new(),
            ground_effects: Vec::new(),
            sound_effects: Vec::new(),
            max_effects: 3,
            transitions: Vec::new(),
            random_seed: 0.0,
            wind_factor: 1.0,
            persistence_duration: 30.0, // Ground effects last for 30 seconds after weather changes
            ground_effect_timer: 0.0,
            profiler: WeatherProfiler::new(),
        }
    }
}

impl WeatherEffects {
    /// Update weather effects based on current weather and time of day
    pub fn update(&mut self, weather: &WeatherState, time: TimeOfDay, audio: &Audio, sound_settings: &WeatherSoundSettings, asset_server: &AssetServer, delta: f32, commands: &mut Commands) {
        self.profiler.start_update();
        
        self.update_transitions(delta);
        self.update_random(delta);
        self.update_ground_effects(weather, time, delta, commands);
        self.update_sound_effects(weather, audio, sound_settings, asset_server, commands);

        // Special dawn/dusk effect
        if (time == TimeOfDay::Dawn || time == TimeOfDay::Dusk) && weather.fog_density > 0.2 {
            self.ensure_effect(weather, WeatherEffectType::RainbowMist, weather.fog_density * 0.5, commands);
        }

        // Temperature-based effects
        if weather.temperature < 0.0 && weather.precipitation > 0.3 {
            self.ensure_effect(weather, WeatherEffectType::FreezingRain, weather.precipitation, commands);
            if weather.temperature < -10.0 {
                self.ensure_effect(weather, WeatherEffectType::IceParticles, -weather.temperature / 20.0, commands);
            }
        }

        // Update or spawn effects based on weather conditions and time of day
        match weather.weather {
            Weather::Rain | Weather::Storm => {
                let intensity = weather.precipitation;
                if intensity > 0.7 {
                    self.ensure_effect(weather, WeatherEffectType::HeavyRain, intensity, commands);
                } else if intensity > 0.3 {
                    self.ensure_effect(weather, WeatherEffectType::Rain, intensity, commands);
                } else {
                    self.ensure_effect(weather, WeatherEffectType::DrizzleRain, intensity, commands);
                }
                
                if weather.weather == Weather::Storm {
                    if weather.wind_speed > 12.0 {
                        self.ensure_effect(weather, WeatherEffectType::ThunderStorm, 1.0, commands);
                    } else {
                        self.ensure_effect(weather, WeatherEffectType::Lightning, 1.0, commands);
                    }
                    
                    if weather.precipitation > 0.8 {
                        self.ensure_effect(weather, WeatherEffectType::Hail, weather.precipitation - 0.5, commands);
                    }
                }
            }
            Weather::Snow => {
                let intensity = weather.precipitation;
                if intensity > 0.7 && weather.wind_speed > 10.0 {
                    self.ensure_effect(weather, WeatherEffectType::Blizzard, intensity, commands);
                } else if intensity > 0.4 {
                    self.ensure_effect(weather, WeatherEffectType::Snow, intensity, commands);
                } else {
                    self.ensure_effect(weather, WeatherEffectType::LightSnow, intensity, commands);
                }
            }
            Weather::Fog => {
                if weather.fog_density > 0.6 {
                    self.ensure_effect(weather, WeatherEffectType::ThickFog, weather.fog_density, commands);
                } else if weather.fog_density > 0.3 {
                    self.ensure_effect(weather, WeatherEffectType::Fog, weather.fog_density, commands);
                } else {
                    self.ensure_effect(weather, WeatherEffectType::Mist, weather.fog_density, commands);
                }
            }
            _ => {
                if weather.wind_speed > 12.0 {
                    self.ensure_effect(weather, WeatherEffectType::Sandstorm, weather.wind_speed / 15.0, commands);
                } else if weather.wind_speed > 8.0 {
                    self.ensure_effect(weather, WeatherEffectType::Dust, weather.wind_speed / 15.0, commands);
                }
            }
        }

        // Add environmental effects based on season and time
        match time {
            TimeOfDay::Morning | TimeOfDay::Afternoon if weather.temperature > 15.0 => {
                self.ensure_effect(weather, WeatherEffectType::Pollen, 0.3, commands);
            }
            TimeOfDay::Dawn | TimeOfDay::Dusk if weather.wind_speed > 5.0 => {
                self.ensure_effect(weather, WeatherEffectType::LeafDebris, weather.wind_speed / 15.0, commands);
            }
            _ => {}
        }

        let update_time = self.profiler.end_update();
    }

    /// Update effect transitions
    fn update_transitions(&mut self, delta: f32) {
        let transition_speed = 2.0 * delta; // Complete transition in 0.5 seconds
        self.transitions.retain_mut(|(effect_type, current, target)| {
            if (*target - *current).abs() < 0.01 {
                false // Remove completed transitions
            } else {
                *current = (*current).lerp(*target, transition_speed);
                true
            }
        });
    }

    /// Start transitioning an effect to a new intensity
    pub fn transition_effect(&mut self, weather: &WeatherState, effect_type: WeatherEffectType, target_intensity: f32, commands: &mut Commands) {
        if let Some(current_intensity) = self.active_effects.iter()
            .find(|(t, _)| *t == effect_type)
            .map(|_| 1.0)
        {
            self.transitions.push((effect_type, current_intensity, target_intensity));
        } else if target_intensity > 0.1 {
            self.transitions.push((effect_type, 0.0, target_intensity));
        }
    }

    /// Get time-of-day adjusted color for effects
    fn get_time_adjusted_color(&self, base_color: Color, time: TimeOfDay) -> Color {
        match time {
            TimeOfDay::Dawn => base_color * [1.1, 0.9, 0.8],
            TimeOfDay::Morning => base_color * [1.0, 1.0, 0.9],
            TimeOfDay::Noon => base_color,
            TimeOfDay::Afternoon => base_color * [1.0, 0.95, 0.85],
            TimeOfDay::Dusk => base_color * [0.9, 0.8, 0.7],
            TimeOfDay::Night => base_color * [0.6, 0.6, 0.8],
        }
    }

    /// Ensure an effect is active with the given intensity
    fn ensure_effect(&mut self, weather: &WeatherState, effect_type: WeatherEffectType, intensity: f32, commands: &mut Commands) {
        let count = self.active_effects.iter()
            .filter(|(t, _)| *t == effect_type)
            .count();

        if count < self.max_effects && intensity > 0.1 {
            self.spawn_effect(weather, effect_type, intensity, TimeOfDay::Noon, commands);
        }
    }

    /// Spawn a new weather effect
    fn spawn_effect(&mut self, weather: &WeatherState, effect_type: WeatherEffectType, intensity: f32, time: TimeOfDay, commands: &mut Commands) {
        self.profiler.start_spawn();
        let mut materials = Assets::<ParticleMaterial>::default();
        let (visibility, particle_size, lifetime_mult) = self.get_time_adjusted_params(time);

        let effect = match effect_type {
            WeatherEffectType::FreezingRain => {
                // ParticleEffect {
                //     emitter: ParticleEmitter {
                //         shape: EmitterShape::Box {
                //             size: Vec3::new(50.0, 1.0, 50.0),
                //         },
                //         spawn_rate: 800.0 * intensity,
                //         lifetime: 1.5 * lifetime_mult,
                //         initial_velocity: self.get_wind_velocity(
                //             Vec3::new(0.0, -25.0, 0.0),
                //             weather.wind_speed,
                //             Vec2::new(weather.wind_direction, 0.0)
                //         ),
                //         size: 0.12 * particle_size,
                //         ..default()
                //     },
                //     material: materials.add(ParticleMaterial::default().with_color_tint(Vec4::new(0.8, 0.9, 1.0, 0.7 * visibility))),
                // }
            },
            WeatherEffectType::RainbowMist => {
                // ParticleEffect {
                //     emitter: ParticleEmitter {
                //         shape: EmitterShape::Box {
                //             size: Vec3::new(80.0, 15.0, 80.0),
                //         },
                //         spawn_rate: 50.0 * intensity,
                //         lifetime: 8.0 * lifetime_mult,
                //         initial_velocity: self.get_wind_velocity(
                //             Vec3::new(0.0, 0.5, 0.0),
                //             weather.wind_speed * 0.2,
                //             Vec2::new(weather.wind_direction, 0.0)
                //         ),
                //         size: 8.0 * particle_size,
                //         ..default()
                //     },
                //     material: materials.add(ParticleMaterial::default().with_color_tint(Vec4::new(1.0, 0.8, 0.6, 0.2 * visibility))),
                // }
            },
            WeatherEffectType::IceParticles => {
                // ParticleEffect {
                //     emitter: ParticleEmitter {
                //         shape: EmitterShape::Box {
                //             size: Vec3::new(60.0, 20.0, 60.0),
                //         },
                //         spawn_rate: 200.0 * intensity,
                //         lifetime: 4.0 * lifetime_mult,
                //         initial_velocity: self.get_wind_velocity(
                //             Vec3::new(0.0, -2.0, 0.0),
                //             weather.wind_speed * 0.5,
                //             Vec2::new(weather.wind_direction, 0.0)
                //         ),
                //         size: 0.15 * particle_size,
                //         ..default()
                //     },
                //     material: materials.add(ParticleMaterial::default().with_color_tint(Vec4::new(0.9, 0.95, 1.0, 0.6 * visibility))),
                // }
            },
            WeatherEffectType::Rain => {
                // ParticleEffect {
                //     emitter: ParticleEmitter {
                //         shape: EmitterShape::Box {
                //             size: Vec3::new(50.0, 1.0, 50.0),
                //         },
                //         spawn_rate: 1000.0 * intensity,
                //         lifetime: 2.0 * lifetime_mult,
                //         initial_velocity: self.get_wind_velocity(
                //             Vec3::new(0.0, -20.0, 0.0),
                //             weather.wind_speed,
                //             Vec2::new(weather.wind_direction, 0.0)
                //         ),
                //         size: 0.1 * particle_size,
                //         ..default()
                //     },
                //     material: materials.add(ParticleMaterial::default().with_color_tint(Vec4::new(0.7, 0.7, 0.8, 0.5 * visibility))),
                // }
            },
            WeatherEffectType::HeavyRain => {
                // ParticleEffect {
                //     emitter: ParticleEmitter {
                //         shape: EmitterShape::Box {
                //             size: Vec3::new(60.0, 1.0, 60.0),
                //         },
                //         spawn_rate: 2000.0 * intensity,
                //         lifetime: 1.5 * lifetime_mult,
                //         initial_velocity: self.get_wind_velocity(
                //             Vec3::new(0.0, -25.0, 0.0),
                //             weather.wind_speed,
                //             Vec2::new(weather.wind_direction, 0.0)
                //         ),
                //         size: 0.15 * particle_size,
                //         ..default()
                //     },
                //     material: materials.add(ParticleMaterial::default().with_color_tint(Vec4::new(0.6, 0.6, 0.7, 0.6 * visibility))),
                // }
            },
            WeatherEffectType::DrizzleRain => {
                // ParticleEffect {
                //     emitter: ParticleEmitter {
                //         shape: EmitterShape::Box {
                //             size: Vec3::new(40.0, 1.0, 40.0),
                //         },
                //         spawn_rate: 500.0 * intensity,
                //         lifetime: 3.0 * lifetime_mult,
                //         initial_velocity: self.get_wind_velocity(
                //             Vec3::new(0.0, -15.0, 0.0),
                //             weather.wind_speed,
                //             Vec2::new(weather.wind_direction, 0.0)
                //         ),
                //         size: 0.05 * particle_size,
                //         ..default()
                //     },
                //     material: materials.add(ParticleMaterial::default().with_color_tint(Vec4::new(0.8, 0.8, 0.9, 0.4 * visibility))),
                // }
            },
            WeatherEffectType::Snow => {
                // ParticleEffect {
                //     emitter: ParticleEmitter {
                //         shape: EmitterShape::Box {
                //             size: Vec3::new(50.0, 1.0, 50.0),
                //         },
                //         spawn_rate: 500.0 * intensity,
                //         lifetime: 5.0 * lifetime_mult,
                //         initial_velocity: self.get_wind_velocity(
                //             Vec3::new(0.0, -5.0, 0.0),
                //             weather.wind_speed,
                //             Vec2::new(weather.wind_direction, 0.0)
                //         ),
                //         size: 0.2 * particle_size,
                //         ..default()
                //     },
                //     material: materials.add(ParticleMaterial::default().with_color_tint(Vec4::new(1.0, 1.0, 1.0, 0.8 * visibility))),
                // }
            },
            WeatherEffectType::Blizzard => {
                // ParticleEffect {
                //     emitter: ParticleEmitter {
                //         shape: EmitterShape::Box {
                //             size: Vec3::new(50.0, 1.0, 50.0),
                //         },
                //         spawn_rate: 500.0 * intensity,
                //         lifetime: 5.0 * lifetime_mult,
                //         initial_velocity: self.get_wind_velocity(
                //             Vec3::new(0.0, -5.0, 0.0),
                //             weather.wind_speed,
                //             Vec2::new(weather.wind_direction, 0.0)
                //         ),
                //         size: 0.2 * particle_size,
                //         ..default()
                //     },
                //     material: materials.add(ParticleMaterial::default().with_color_tint(Vec4::new(1.0, 1.0, 1.0, 0.8 * visibility))),
                // }
            },
            WeatherEffectType::LightSnow => {
                // ParticleEffect {
                //     emitter: ParticleEmitter {
                //         shape: EmitterShape::Box {
                //             size: Vec3::new(50.0, 1.0, 50.0),
                //         },
                //         spawn_rate: 500.0 * intensity,
                //         lifetime: 5.0 * lifetime_mult,
                //         initial_velocity: self.get_wind_velocity(
                //             Vec3::new(0.0, -5.0, 0.0),
                //             weather.wind_speed,
                //             Vec2::new(weather.wind_direction, 0.0)
                //         ),
                //         size: 0.2 * particle_size,
                //         ..default()
                //     },
                //     material: materials.add(ParticleMaterial::default().with_color_tint(Vec4::new(1.0, 1.0, 1.0, 0.8 * visibility))),
                // }
            },
            WeatherEffectType::Fog => {
                // ParticleEffect {
                //     emitter: ParticleEmitter {
                //         shape: EmitterShape::Box {
                //             size: Vec3::new(100.0, 10.0, 100.0),
                //         },
                //         spawn_rate: 100.0 * intensity,
                //         lifetime: 10.0 * lifetime_mult,
                //         initial_velocity: Vec3::ZERO,
                //         size: 5.0 * particle_size,
                //         ..default()
                //     },
                //     material: materials.add(ParticleMaterial::default().with_color_tint(Vec4::new(0.8, 0.8, 0.85, 0.3 * visibility))),
                // }
            },
            WeatherEffectType::ThickFog => {
                // ParticleEffect {
                //     emitter: ParticleEmitter {
                //         shape: EmitterShape::Box {
                //             size: Vec3::new(100.0, 10.0, 100.0),
                //         },
                //         spawn_rate: 100.0 * intensity,
                //         lifetime: 10.0 * lifetime_mult,
                //         initial_velocity: Vec3::ZERO,
                //         size: 5.0 * particle_size,
                //         ..default()
                //     },
                //     material: materials.add(ParticleMaterial::default().with_color_tint(Vec4::new(0.8, 0.8, 0.85, 0.3 * visibility))),
                // }
            },
            WeatherEffectType::Dust => {
                // ParticleEffect {
                //     emitter: ParticleEmitter {
                //         shape: EmitterShape::Box {
                //             size: Vec3::new(50.0, 2.0, 50.0),
                //         },
                //         spawn_rate: 200.0 * intensity,
                //         lifetime: 3.0 * lifetime_mult,
                //         initial_velocity: self.get_wind_velocity(
                //             Vec3::new(5.0, 2.0, 0.0),
                //             weather.wind_speed,
                //             Vec2::new(weather.wind_direction, 0.0)
                //         ),
                //         size: 0.3 * particle_size,
                //         ..default()
                //     },
                //     material: materials.add(ParticleMaterial::default().with_color_tint(Vec4::new(0.8, 0.7, 0.6, 0.4 * visibility))),
                // }
            },
            WeatherEffectType::Sandstorm => {
                // ParticleEffect {
                //     emitter: ParticleEmitter {
                //         shape: EmitterShape::Box {
                //             size: Vec3::new(50.0, 2.0, 50.0),
                //         },
                //         spawn_rate: 200.0 * intensity,
                //         lifetime: 3.0 * lifetime_mult,
                //         initial_velocity: self.get_wind_velocity(
                //             Vec3::new(5.0, 2.0, 0.0),
                //             weather.wind_speed,
                //             Vec2::new(weather.wind_direction, 0.0)
                //         ),
                //         size: 0.3 * particle_size,
                //         ..default()
                //     },
                //     material: materials.add(ParticleMaterial::default().with_color_tint(Vec4::new(0.8, 0.7, 0.6, 0.4 * visibility))),
                // }
            },
            WeatherEffectType::Lightning => {
                // ParticleEffect {
                //     emitter: ParticleEmitter {
                //         shape: EmitterShape::Point,
                //         spawn_rate: 1000.0,
                //         lifetime: 0.2 * lifetime_mult,
                //         initial_velocity: Vec3::ZERO,
                //         size: 0.5 * particle_size,
                //         ..default()
                //     },
                //     material: materials.add(ParticleMaterial::default().with_color_tint(Vec4::new(1.0, 1.0, 1.0, 0.8 * visibility))),
                // }
            },
            WeatherEffectType::ThunderStorm => {
                // ParticleEffect {
                //     emitter: ParticleEmitter {
                //         shape: EmitterShape::Point,
                //         spawn_rate: 1000.0,
                //         lifetime: 0.2 * lifetime_mult,
                //         initial_velocity: Vec3::ZERO,
                //         size: 0.5 * particle_size,
                //         ..default()
                //     },
                //     material: materials.add(ParticleMaterial::default().with_color_tint(Vec4::new(1.0, 1.0, 1.0, 0.8 * visibility))),
                // }
            },
            WeatherEffectType::Hail => {
                // ParticleEffect {
                //     emitter: ParticleEmitter {
                //         shape: EmitterShape::Point,
                //         spawn_rate: 1000.0,
                //         lifetime: 0.2 * lifetime_mult,
                //         initial_velocity: Vec3::ZERO,
                //         size: 0.5 * particle_size,
                //         ..default()
                //     },
                //     material: materials.add(ParticleMaterial::default().with_color_tint(Vec4::new(1.0, 1.0, 1.0, 0.8 * visibility))),
                // }
            },
            WeatherEffectType::Mist => {
                // ParticleEffect {
                //     emitter: ParticleEmitter {
                //         shape: EmitterShape::Box {
                //             size: Vec3::new(100.0, 10.0, 100.0),
                //         },
                //         spawn_rate: 100.0 * intensity,
                //         lifetime: 10.0 * lifetime_mult,
                //         initial_velocity: Vec3::ZERO,
                //         size: 5.0 * particle_size,
                //         ..default()
                //     },
                //     material: materials.add(ParticleMaterial::default().with_color_tint(Vec4::new(0.8, 0.8, 0.85, 0.3 * visibility))),
                // }
            },
        };

        let entity = commands.spawn(ParticleEffectBundle {
            effect,
            transform: Transform::from_xyz(0.0, 20.0, 0.0),
            ..default()
        }).id();

        self.active_effects.push((effect_type, entity));
        
        let spawn_time = self.profiler.end_spawn();
    }

    /// Remove all active effects
    pub fn clear_effects(&mut self, commands: &mut Commands) {
        for (_, entity) in self.active_effects.drain(..) {
            commands.entity(entity).despawn_recursive();
        }
    }

    /// Calculate wind-influenced velocity
    fn get_wind_velocity(&self, base_velocity: Vec3, wind_speed: f32, wind_direction: Vec2) -> Vec3 {
        let wind_vec = Vec3::new(
            wind_direction.x * wind_speed * self.wind_factor,
            base_velocity.y,
            wind_direction.y * wind_speed * self.wind_factor
        );
        
        base_velocity + wind_vec + Vec3::new(
            (self.random_seed.sin() * 0.5) * wind_speed,
            0.0,
            (self.random_seed.cos() * 0.5) * wind_speed
        )
    }

    /// Update random variation
    fn update_random(&mut self, delta: f32) {
        self.random_seed = (self.random_seed + delta * 2.0) % std::f32::consts::TAU;
    }

    /// Get time-of-day adjusted parameters
    fn get_time_adjusted_params(&self, time: TimeOfDay) -> (f32, f32, f32) { // (visibility, particle_size, lifetime)
        match time {
            TimeOfDay::Dawn => (1.2, 1.1, 1.2),   // Better visibility, slightly larger particles
            TimeOfDay::Morning => (1.1, 1.0, 1.0), // Slightly better visibility
            TimeOfDay::Noon => (1.0, 1.0, 1.0),   // Base values
            TimeOfDay::Afternoon => (1.1, 1.0, 1.0),
            TimeOfDay::Dusk => (0.8, 1.2, 1.3),   // Reduced visibility, larger particles, longer lifetime
            TimeOfDay::Night => (0.6, 1.3, 1.5),  // Poor visibility, larger particles, longer lifetime
        }
    }

    /// Update ground effects
    fn update_ground_effects(&mut self, weather: &WeatherState, time: TimeOfDay, delta: f32, commands: &mut Commands) {
        self.ground_effect_timer += delta;
        
        if self.ground_effect_timer >= 1.0 {
            self.ground_effect_timer = 0.0;
            
            match weather.weather {
                Weather::Rain | Weather::Storm if weather.precipitation > 0.3 => {
                    self.ensure_ground_effect(weather, WeatherEffectType::Puddles, weather.precipitation, commands);
                }
                Weather::Snow if weather.precipitation > 0.4 => {
                    self.ensure_ground_effect(weather, WeatherEffectType::SnowAccumulation, weather.precipitation, commands);
                }
                _ if weather.wind_speed > 8.0 => {
                    self.ensure_ground_effect(weather, WeatherEffectType::DustDeposit, weather.wind_speed / 15.0, commands);
                }
                _ => {}
            }
        }

        // Fade out ground effects over time
        self.ground_effects.retain_mut(|(effect_type, entity)| {
            true // Keep all for now
        });
    }

    /// Update sound effects based on weather
    fn update_sound_effects(&mut self, weather: &WeatherState, audio: &Audio, sound_settings: &WeatherSoundSettings, asset_server: &AssetServer, commands: &mut Commands) {
        let master_volume = sound_settings.master_volume;
        
        match weather.weather {
            Weather::Rain => {
                if weather.precipitation > 0.7 {
                    let volume = sound_settings.effect_volumes.get("heavy_rain").unwrap_or(&0.8) * master_volume;
                    let handle = asset_server.load("sounds/weather/heavy_rain.ogg");
                    audio.play(handle)
                        .with_volume(volume as f64)
                        .looped();
                } else {
                    let volume = sound_settings.effect_volumes.get("light_rain").unwrap_or(&0.7) * master_volume;
                    let handle = asset_server.load("sounds/weather/light_rain.ogg");
                    audio.play(handle)
                        .with_volume(volume as f64)
                        .looped();
                }
            }
            Weather::Storm => {
                let storm_volume = sound_settings.effect_volumes.get("storm").unwrap_or(&1.0) * master_volume;
                let handle = asset_server.load("sounds/weather/storm.ogg");
                audio.play(handle)
                    .with_volume(storm_volume as f64)
                    .looped();
                
                if weather.wind_speed > 12.0 {
                    let wind_volume = sound_settings.effect_volumes.get("strong_wind").unwrap_or(&0.9) * master_volume;
                    let handle = asset_server.load("sounds/weather/strong_wind.ogg");
                    audio.play(handle)
                        .with_volume((wind_volume * (weather.wind_speed - 12.0) / 8.0) as f64)
                        .looped();
                }
            }
            Weather::Snow if weather.wind_speed > 8.0 => {
                let volume = sound_settings.effect_volumes.get("blizzard").unwrap_or(&0.85) * master_volume;
                let handle = asset_server.load("sounds/weather/blizzard.ogg");
                audio.play(handle)
                    .with_volume((volume * (weather.wind_speed - 8.0) / 12.0) as f64)
                    .looped();
            }
            _ if weather.wind_speed > 10.0 => {
                let volume = sound_settings.effect_volumes.get("wind").unwrap_or(&0.6) * master_volume;
                let handle = asset_server.load("sounds/weather/wind.ogg");
                audio.play(handle)
                    .with_volume((volume * (weather.wind_speed - 10.0) / 10.0) as f64)
                    .looped();
            }
            _ => {
                audio.stop();
            }
        }
    }

    /// Ensure a ground effect is active
    fn ensure_ground_effect(&mut self, weather: &WeatherState, effect_type: WeatherEffectType, intensity: f32, commands: &mut Commands) {
        let count = self.ground_effects.iter()
            .filter(|(t, _)| *t == effect_type)
            .count();

        if count < self.max_effects && intensity > 0.1 {
            self.spawn_ground_effect(weather, effect_type, intensity, commands);
        }
    }

    /// Spawn a ground effect
    fn spawn_ground_effect(&mut self, weather: &WeatherState, effect_type: WeatherEffectType, intensity: f32, commands: &mut Commands) {
        self.profiler.start_spawn();
        let mut materials = Assets::<ParticleMaterial>::default();

        let effect = match effect_type {
            WeatherEffectType::Puddles => {
                // ParticleEffect {
                //     emitter: ParticleEmitter {
                //         shape: EmitterShape::Plane {
                //             size: Vec2::new(100.0, 100.0),
                //             subdivisions: 1,
                //         },
                //         spawn_rate: 0.0, // Static effect
                //         lifetime: self.persistence_duration,
                //         initial_velocity: Vec3::ZERO,
                //         size: 2.0,
                //         ..default()
                //     },
                //     material: materials.add(ParticleMaterial::default()),
                // }
            },
            WeatherEffectType::SnowAccumulation => {
                // ParticleEffect {
                //     emitter: ParticleEmitter {
                //         shape: EmitterShape::Plane {
                //             size: Vec2::new(100.0, 100.0),
                //             subdivisions: 1,
                //         },
                //         spawn_rate: 0.0,
                //         lifetime: self.persistence_duration,
                //         initial_velocity: Vec3::ZERO,
                //         size: 3.0,
                //         ..default()
                //     },
                //     material: materials.add(ParticleMaterial::default()),
                // }
            },
            WeatherEffectType::DustDeposit => {
                // ParticleEffect {
                //     emitter: ParticleEmitter {
                //         shape: EmitterShape::Plane {
                //             size: Vec2::new(80.0, 80.0),
                //             subdivisions: 1,
                //         },
                //         spawn_rate: 0.0,
                //         lifetime: self.persistence_duration,
                //         initial_velocity: Vec3::ZERO,
                //         size: 1.0,
                //         ..default()
                //     },
                //     material: materials.add(ParticleMaterial::default()),
                // }
            },
            _ => return,
        };

        let entity = commands.spawn(ParticleEffectBundle {
            effect,
            transform: Transform::from_xyz(0.0, 0.1, 0.0), // Slightly above ground
            ..default()
        }).id();

        self.ground_effects.push((effect_type, entity));
        
        let spawn_time = self.profiler.end_spawn();
    }

    pub fn render(&mut self) {
        self.profiler.start_render();
        
        // ... existing render code ...
        
        let render_time = self.profiler.end_render();
    }
} 