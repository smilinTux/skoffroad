use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use crate::physics::wheel::Wheel;
use crate::terrain::deformation::{SnowDeformation, DeformationType, TerrainDeformationEvent};
use crate::weather::WeatherState;

#[derive(Clone, Copy, PartialEq)]
pub enum Season {
    Winter,
    Spring,
    Summer,
    Fall,
}

#[derive(Clone)]
pub struct SnowLayer {
    pub snow_type: SnowType,
    pub depth: f32,
    pub temperature: f32,
    pub age: f32,
    pub density: f32,
}

#[derive(Resource)]
pub struct SnowHandlingSettings {
    pub snow_grip_factor: f32,        // Base grip multiplier for snowy conditions
    pub ice_grip_factor: f32,         // Base grip multiplier for icy conditions
    pub powder_resistance: f32,        // Rolling resistance in powder snow
    pub packed_resistance: f32,        // Rolling resistance in packed snow
    pub ice_resistance: f32,          // Rolling resistance on ice
    pub snow_sinkage_factor: f32,     // How much vehicles sink into snow
    pub track_width_factor: f32,      // How track width affects snow handling
    pub temperature_influence: f32,    // How temperature affects snow properties
    pub wind_influence: f32,          // How wind affects snow behavior
    pub snow_density_factor: f32,     // How snow density affects physics
    pub surface_hardness: f32,        // Surface hardness multiplier
    pub powder_displacement: f32,      // How easily powder snow is displaced
    pub wind_drift_factor: f32,     // How strongly wind affects snow drift
    pub drift_angle_influence: f32,  // How wind direction affects drift shape
    pub melt_rate: f32,             // Base rate of snow melting
    pub refreeze_rate: f32,         // Rate at which melted snow refreezes
    pub wet_snow_density: f32,      // Density multiplier for wet snow
    pub dry_snow_density: f32,      // Density multiplier for dry snow
    pub packed_snow_density: f32,   // Density multiplier for packed snow
    pub weather_blend_time: f32,    // Time to transition between weather states

    // Seasonal settings
    pub seasonal_density_mult: f32,    // How seasons affect snow density
    pub seasonal_hardness_mult: f32,   // How seasons affect snow hardness
    pub spring_melt_mult: f32,         // Increased melting in spring
    pub fall_freeze_mult: f32,         // Increased freezing in fall

    // Temperature gradient settings
    pub surface_cooling_rate: f32,     // How quickly surface temp changes
    pub depth_insulation_factor: f32,  // How depth affects temperature change
    pub temp_gradient_strength: f32,   // Strength of temperature gradient
    pub max_temp_difference: f32,      // Max temp difference between layers

    // Layer interaction settings
    pub layer_blend_factor: f32,       // How much layers blend together
    pub max_layers: u32,               // Maximum number of snow layers
    pub min_layer_depth: f32,          // Minimum depth for a layer
    pub layer_compression_rate: f32,   // Rate at which layers compress
}

impl Default for SnowHandlingSettings {
    fn default() -> Self {
        Self {
            snow_grip_factor: 0.6,     // 60% grip on snow
            ice_grip_factor: 0.3,      // 30% grip on ice
            powder_resistance: 2.0,     // High resistance in powder
            packed_resistance: 1.2,     // Moderate resistance in packed snow
            ice_resistance: 0.8,        // Low resistance on ice
            snow_sinkage_factor: 0.8,   // Significant sinkage
            track_width_factor: 1.5,    // Wide tracks/tires help in snow
            temperature_influence: 0.5,  // Moderate temperature effect
            wind_influence: 0.3,        // Wind affects snow conditions
            snow_density_factor: 1.0,   // Base density multiplier
            surface_hardness: 1.0,      // Base hardness multiplier
            powder_displacement: 1.2,    // High displacement in powder
            wind_drift_factor: 0.8,
            drift_angle_influence: 0.6,
            melt_rate: 0.02,
            refreeze_rate: 0.015,
            wet_snow_density: 1.2,
            dry_snow_density: 0.7,
            packed_snow_density: 1.5,
            weather_blend_time: 30.0, // 30 seconds for weather transitions

            // Seasonal settings
            seasonal_density_mult: 1.2,
            seasonal_hardness_mult: 1.1,
            spring_melt_mult: 1.5,
            fall_freeze_mult: 1.3,

            // Temperature gradient settings
            surface_cooling_rate: 0.05,
            depth_insulation_factor: 0.8,
            temp_gradient_strength: 0.2,
            max_temp_difference: 5.0,

            // Layer interaction settings
            layer_blend_factor: 0.3,
            max_layers: 4,
            min_layer_depth: 0.05,
            layer_compression_rate: 0.02,
        }
    }
}

#[derive(Component)]
pub struct SnowHandling {
    pub current_snow_depth: f32,
    pub is_on_ice: bool,
    pub snow_compression: f32,
    pub track_width: f32,
    pub surface_temperature: f32,
    pub snow_density: f32,
    pub surface_hardness: f32,
    pub displacement_factor: f32,
    pub wind_drift_accumulation: f32,
    pub drift_direction: Vec3,
    pub melt_water_content: f32,
    pub snow_type: SnowType,
    pub weather_transition_timer: f32,
    pub previous_weather: Option<WeatherType>,
    pub snow_layers: Vec<SnowLayer>,
    pub surface_temp_gradient: f32,
    pub depth_temp_gradient: f32,
    pub current_season: Season,
    pub season_transition: f32,
    pub layer_compression: f32,
    pub visual_feedback: SnowVisualFeedback,
}

#[derive(Clone, Copy, PartialEq)]
pub enum SnowType {
    Dry { powder_factor: f32 },
    Wet { water_content: f32 },
    Packed { compression: f32 },
    Mixed { wet_ratio: f32, packed_ratio: f32 }
}

#[derive(Clone, Default)]
pub struct SnowVisualFeedback {
    pub surface_sparkle: f32,      // For fresh or icy snow
    pub displacement_amount: f32,   // For powder effects
    pub wetness_sheen: f32,        // For wet snow shine
    pub track_darkness: f32,       // For track visibility
    pub drift_particles: f32,      // For blowing snow
    pub melt_droplets: f32,        // For melting effects
}

pub struct SnowHandlingPlugin;

impl Plugin for SnowHandlingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SnowHandlingSettings>()
            .add_systems(Update, (
                update_snow_handling,
                apply_snow_forces,
                generate_snow_tracks,
            ));
    }
}

fn update_snow_handling(
    mut query: Query<(&mut SnowHandling, &Transform, &Wheel)>,
    snow_query: Query<&SnowDeformation>,
    settings: Res<SnowHandlingSettings>,
    weather: Res<WeatherState>,
    time: Res<Time>,
) {
    for (mut handling, transform, wheel) in query.iter_mut() {
        // Update seasonal effects
        let season_melt_mult = match handling.current_season {
            Season::Spring => settings.spring_melt_mult,
            Season::Summer => settings.spring_melt_mult * 1.2,
            Season::Fall => 1.0,
            Season::Winter => 0.7,
        };

        let season_density_mult = match handling.current_season {
            Season::Winter => settings.seasonal_density_mult * 1.2,
            Season::Spring => settings.seasonal_density_mult * 0.8,
            Season::Fall => settings.seasonal_density_mult * 0.9,
            Season::Summer => settings.seasonal_density_mult * 0.6,
        };

        // Update temperature gradients
        let ambient_temp = weather.temperature;
        let surface_temp_delta = (ambient_temp - handling.surface_temp_gradient) * 
            settings.surface_cooling_rate * time.delta_seconds();
        handling.surface_temp_gradient += surface_temp_delta;

        // Calculate temperature gradient through snow layers
        if !handling.snow_layers.is_empty() {
            let mut prev_temp = handling.surface_temp_gradient;
            for layer in handling.snow_layers.iter_mut() {
                let depth_factor = (-layer.depth * settings.depth_insulation_factor).exp();
                let target_temp = prev_temp + 
                    (ambient_temp - prev_temp) * depth_factor * settings.temp_gradient_strength;
                
                let temp_delta = (target_temp - layer.temperature)
                    .clamp(-settings.max_temp_difference, settings.max_temp_difference);
                layer.temperature += temp_delta * time.delta_seconds();
                
                prev_temp = layer.temperature;
            }
        }

        // Update snow layers
        if !handling.snow_layers.is_empty() {
            // Age and compress existing layers
            for layer in handling.snow_layers.iter_mut() {
                layer.age += time.delta_seconds();
                
                // Compress layer based on age and weight above
                let compression = settings.layer_compression_rate * 
                    time.delta_seconds() * 
                    (1.0 + layer.age / 3600.0); // Increased compression with age
                
                layer.depth *= 1.0 - compression;
                layer.density *= 1.0 + compression * 0.5;
                
                // Update snow type based on conditions
                layer.snow_type = determine_snow_type(
                    layer.temperature,
                    layer.density,
                    layer.age,
                    &settings
                );
            }

            // Remove thin layers
            handling.snow_layers.retain(|layer| layer.depth >= settings.min_layer_depth);

            // Merge similar adjacent layers if exceeding max layers
            while handling.snow_layers.len() > settings.max_layers as usize {
                let mut most_similar_idx = 0;
                let mut smallest_diff = f32::MAX;

                for i in 0..handling.snow_layers.len() - 1 {
                    let temp_diff = (handling.snow_layers[i].temperature - 
                        handling.snow_layers[i + 1].temperature).abs();
                    let density_diff = (handling.snow_layers[i].density - 
                        handling.snow_layers[i + 1].density).abs();
                    
                    let total_diff = temp_diff + density_diff;
                    if total_diff < smallest_diff {
                        smallest_diff = total_diff;
                        most_similar_idx = i;
                    }
                }

                // Merge layers
                let layer1 = &handling.snow_layers[most_similar_idx];
                let layer2 = &handling.snow_layers[most_similar_idx + 1];
                let total_depth = layer1.depth + layer2.depth;
                let weight1 = layer1.depth / total_depth;
                let weight2 = layer2.depth / total_depth;

                let merged_layer = SnowLayer {
                    depth: total_depth,
                    temperature: layer1.temperature * weight1 + layer2.temperature * weight2,
                    density: layer1.density * weight1 + layer2.density * weight2,
                    age: (layer1.age * weight1 + layer2.age * weight2),
                    snow_type: if weight1 > weight2 { layer1.snow_type } else { layer2.snow_type },
                };

                handling.snow_layers.remove(most_similar_idx);
                handling.snow_layers[most_similar_idx] = merged_layer;
            }
        }

        // Add new snow layer during snowfall
        if matches!(weather.current_weather, WeatherType::HeavySnow | WeatherType::LightSnow) {
            let snowfall_rate = if matches!(weather.current_weather, WeatherType::HeavySnow) {
                0.02 // 2cm per second in heavy snow
            } else {
                0.005 // 0.5cm per second in light snow
            };

            let new_snow_depth = snowfall_rate * time.delta_seconds() * season_melt_mult;
            
            if new_snow_depth >= settings.min_layer_depth {
                // Create new powder snow layer
                let new_layer = SnowLayer {
                    snow_type: SnowType::Dry { powder_factor: 1.0 },
                    depth: new_snow_depth,
                    temperature: handling.surface_temp_gradient,
                    age: 0.0,
                    density: settings.dry_snow_density * season_density_mult,
                };

                // Add to top of snow layers
                if handling.snow_layers.len() < settings.max_layers as usize {
                    handling.snow_layers.insert(0, new_layer);
                } else {
                    // Merge with top layer
                    let top_layer = &mut handling.snow_layers[0];
                    let total_depth = top_layer.depth + new_layer.depth;
                    let weight1 = top_layer.depth / total_depth;
                    let weight2 = new_layer.depth / total_depth;

                    top_layer.depth = total_depth;
                    top_layer.temperature = top_layer.temperature * weight1 + new_layer.temperature * weight2;
                    top_layer.density = top_layer.density * weight1 + new_layer.density * weight2;
                    top_layer.age = top_layer.age * weight1;
                }
            }
        }

        // Update visual feedback
        handling.visual_feedback = SnowVisualFeedback {
            surface_sparkle: if handling.snow_layers.is_empty() { 0.0 } else {
                let top_layer = &handling.snow_layers[0];
                match top_layer.snow_type {
                    SnowType::Dry { powder_factor } => powder_factor * 0.8,
                    SnowType::Packed { compression } => compression * 0.4,
                    _ => 0.2,
                }
            },
            displacement_amount: if handling.snow_layers.is_empty() { 0.0 } else {
                let powder_depth = handling.snow_layers.iter()
                    .filter(|layer| matches!(layer.snow_type, SnowType::Dry { .. }))
                    .map(|layer| layer.depth)
                    .sum::<f32>();
                powder_depth.min(1.0)
            },
            wetness_sheen: if handling.snow_layers.is_empty() { 0.0 } else {
                handling.snow_layers.iter()
                    .filter_map(|layer| match layer.snow_type {
                        SnowType::Wet { water_content } => Some(water_content),
                        _ => None,
                    })
                    .next()
                    .unwrap_or(0.0)
            },
            track_darkness: handling.snow_compression * 0.8,
            drift_particles: handling.wind_drift_accumulation,
            melt_droplets: handling.melt_water_content * 0.5,
        };

        // Update overall snow properties based on layers
        if !handling.snow_layers.is_empty() {
            handling.current_snow_depth = handling.snow_layers.iter()
                .map(|layer| layer.depth)
                .sum();

            // Calculate weighted averages for overall properties
            let mut total_density = 0.0;
            let mut total_hardness = 0.0;
            let mut total_weight = 0.0;

            for layer in handling.snow_layers.iter() {
                let weight = layer.depth / handling.current_snow_depth;
                total_density += layer.density * weight;
                total_hardness += get_layer_hardness(layer) * weight;
                total_weight += weight;
            }

            handling.snow_density = total_density / total_weight;
            handling.surface_hardness = total_hardness / total_weight;
        }

        // Update weather transition
        if let Some(prev_weather) = handling.previous_weather {
            if prev_weather != weather.current_weather {
                handling.weather_transition_timer = settings.weather_blend_time;
            }
        }
        handling.previous_weather = Some(weather.current_weather);
        
        // Calculate weather transition progress
        let transition_factor = if handling.weather_transition_timer > 0.0 {
            handling.weather_transition_timer -= time.delta_seconds();
            1.0 - (handling.weather_transition_timer / settings.weather_blend_time).clamp(0.0, 1.0)
        } else {
            1.0
        };

        // Update snow conditions at wheel position
        if let Some(snow) = get_snow_at_position(transform.translation, &snow_query) {
            handling.current_snow_depth = calculate_snow_depth(snow, transform.translation);
            handling.is_on_ice = check_for_ice(snow, transform.translation);
            handling.snow_compression = calculate_snow_compression(snow, transform.translation);
            
            // Calculate wind drift
            let wind_direction = Vec3::new(
                weather.wind_direction.cos(),
                0.0,
                weather.wind_direction.sin()
            );
            
            // Update drift accumulation based on wind and terrain
            let terrain_angle = transform.up().angle_between(Vec3::Y);
            let wind_terrain_factor = terrain_angle.cos() * settings.drift_angle_influence;
            let drift_strength = weather.wind_speed * settings.wind_drift_factor * wind_terrain_factor;
            
            handling.wind_drift_accumulation = (handling.wind_drift_accumulation + 
                drift_strength * time.delta_seconds()).clamp(0.0, 2.0);
            handling.drift_direction = wind_direction;

            // Temperature-based melting and refreezing
            let melt_threshold = 0.0; // 0°C
            if weather.temperature > melt_threshold {
                // Melting
                let melt_amount = settings.melt_rate * 
                    (weather.temperature - melt_threshold) * 
                    time.delta_seconds();
                handling.melt_water_content = (handling.melt_water_content + melt_amount)
                    .clamp(0.0, handling.current_snow_depth);
                handling.current_snow_depth -= melt_amount;
            } else {
                // Refreezing
                let refreeze_amount = settings.refreeze_rate * 
                    (melt_threshold - weather.temperature) * 
                    time.delta_seconds();
                let refrozen = refreeze_amount.min(handling.melt_water_content);
                handling.melt_water_content -= refrozen;
                // Refrozen snow increases density and hardness
                handling.snow_density *= 1.0 + (refrozen * 0.2);
                handling.surface_hardness *= 1.0 + (refrozen * 0.3);
            }

            // Determine snow type based on conditions
            handling.snow_type = determine_snow_type(
                handling.melt_water_content / handling.current_snow_depth.max(0.01),
                handling.snow_compression,
                weather.temperature,
                &settings
            );

            // Update snow properties based on type
            match handling.snow_type {
                SnowType::Wet { water_content } => {
                    handling.snow_density *= settings.wet_snow_density;
                    handling.surface_hardness *= 0.7 + water_content * 0.6;
                    handling.displacement_factor *= 0.6;
                },
                SnowType::Dry { powder_factor } => {
                    handling.snow_density *= settings.dry_snow_density;
                    handling.surface_hardness *= 0.5;
                    handling.displacement_factor *= 1.0 + powder_factor * 0.5;
                },
                SnowType::Packed { compression } => {
                    handling.snow_density *= settings.packed_snow_density;
                    handling.surface_hardness *= 0.8 + compression * 0.4;
                    handling.displacement_factor *= 0.4;
                },
                SnowType::Mixed { wet_ratio, packed_ratio } => {
                    handling.snow_density *= settings.wet_snow_density * wet_ratio +
                        settings.packed_snow_density * packed_ratio +
                        settings.dry_snow_density * (1.0 - wet_ratio - packed_ratio);
                    handling.surface_hardness *= 0.6 + wet_ratio * 0.2 + packed_ratio * 0.4;
                    handling.displacement_factor *= 0.8 - packed_ratio * 0.4;
                }
            }

            // Apply weather transition effects
            match weather.current_weather {
                WeatherType::HeavySnow | WeatherType::Blizzard => {
                    let effect_strength = transition_factor;
                    handling.current_snow_depth *= 1.0 + 0.2 * effect_strength;
                    handling.snow_density *= 1.0 - 0.2 * effect_strength;
                    handling.surface_hardness *= 1.0 - 0.3 * effect_strength;
                    handling.displacement_factor *= 1.0 + 0.3 * effect_strength;
                },
                WeatherType::FreezingRain => {
                    let effect_strength = transition_factor;
                    handling.is_on_ice = effect_strength > 0.5;
                    handling.surface_hardness *= 1.0 + 0.5 * effect_strength;
                    handling.displacement_factor *= 1.0 - 0.5 * effect_strength;
                    handling.melt_water_content += 0.02 * effect_strength * time.delta_seconds();
                },
                WeatherType::LightSnow => {
                    let effect_strength = transition_factor;
                    handling.current_snow_depth *= 1.0 + 0.1 * effect_strength;
                    handling.snow_density *= 1.0 - 0.1 * effect_strength;
                    handling.surface_hardness *= 1.0 - 0.2 * effect_strength;
                    handling.displacement_factor *= 1.0 + 0.1 * effect_strength;
                },
                _ => {}
            }
        }

        // Update track width effect
        handling.track_width = wheel.width * settings.track_width_factor;
    }
}

fn determine_snow_type(
    temperature: f32,
    density: f32,
    age: f32,
    settings: &SnowHandlingSettings,
) -> SnowType {
    // Temperature thresholds for wet snow formation
    const WET_SNOW_TEMP: f32 = -1.0; // Celsius
    const MELT_TEMP: f32 = 0.0; // Celsius
    
    // Density thresholds for packed snow
    const PACKED_DENSITY_THRESHOLD: f32 = 400.0; // kg/m³
    const HIGH_DENSITY_THRESHOLD: f32 = 600.0; // kg/m³
    
    // Age thresholds (in seconds)
    const FRESH_SNOW_AGE: f32 = 3600.0; // 1 hour
    const OLD_SNOW_AGE: f32 = 86400.0; // 24 hours

    // Calculate base factors
    let age_factor = (age / OLD_SNOW_AGE).min(1.0);
    let density_factor = ((density - settings.dry_snow_density) / 
        (HIGH_DENSITY_THRESHOLD - settings.dry_snow_density)).clamp(0.0, 1.0);
    
    // Temperature-based wetness
    if temperature > MELT_TEMP {
        // Snow is melting - always wet
        let water_content = ((temperature - MELT_TEMP) / 2.0).min(1.0);
        SnowType::Wet { water_content }
    } else if temperature > WET_SNOW_TEMP {
        // Near melting - mix of wet and other types
        let wet_ratio = (temperature - WET_SNOW_TEMP) / (MELT_TEMP - WET_SNOW_TEMP);
        
        if density > PACKED_DENSITY_THRESHOLD {
            // Dense enough to be packed
            let packed_ratio = density_factor;
            SnowType::Mixed { 
                wet_ratio,
                packed_ratio,
            }
        } else {
            // Still mostly powder
            let powder_factor = 1.0 - (wet_ratio * 0.7 + age_factor * 0.3);
            SnowType::Dry { powder_factor }
        }
    } else {
        // Cold snow - either powder or packed
        if density > PACKED_DENSITY_THRESHOLD {
            // Dense enough to be packed
            let compression = density_factor;
            SnowType::Packed { compression }
        } else {
            // Powder snow
            let powder_factor = 1.0 - (density_factor * 0.6 + age_factor * 0.4);
            SnowType::Dry { powder_factor }
        }
    }
}

fn apply_snow_forces(
    mut query: Query<(&SnowHandling, &mut ExternalForce, &Wheel)>,
    settings: Res<SnowHandlingSettings>,
    time: Res<Time>,
) {
    for (handling, mut ext_force, wheel) in query.iter_mut() {
        // Calculate base grip reduction based on snow type
        let grip_factor = if handling.is_on_ice {
            settings.ice_grip_factor
        } else {
            let (base_grip, density_mult, hardness_mult) = match handling.snow_type {
                SnowType::Wet { water_content } => (
                    settings.snow_grip_factor * 0.8,
                    0.4,
                    0.3 + water_content * 0.4
                ),
                SnowType::Dry { powder_factor } => (
                    settings.snow_grip_factor * 0.6,
                    0.2,
                    0.2 + powder_factor * 0.3
                ),
                SnowType::Packed { compression } => (
                    settings.snow_grip_factor * 1.1,
                    0.5,
                    0.6 + compression * 0.3
                ),
                SnowType::Mixed { wet_ratio, packed_ratio } => (
                    settings.snow_grip_factor * (0.8 + packed_ratio * 0.3),
                    0.3 + wet_ratio * 0.2,
                    0.4 + packed_ratio * 0.3
                ),
            };

            let density_influence = handling.snow_density * density_mult;
            let hardness_influence = handling.surface_hardness * hardness_mult;
            let temp_influence = (1.0 - (handling.surface_temperature + 5.0) * 0.05).clamp(0.3, 1.0);
            
            base_grip * 
                (density_influence + hardness_influence) * 
                temp_influence *
                (1.0 - handling.snow_compression).max(0.2)
        };

        // Calculate resistance based on snow type
        let resistance = if handling.is_on_ice {
            settings.ice_resistance
        } else {
            let base_resistance = match handling.snow_type {
                SnowType::Wet { water_content } => 
                    settings.packed_resistance * (1.0 + water_content * 0.5),
                SnowType::Dry { powder_factor } => 
                    settings.powder_resistance * (1.0 + powder_factor * 0.7),
                SnowType::Packed { compression } => 
                    settings.packed_resistance * (1.0 + compression * 0.3),
                SnowType::Mixed { wet_ratio, packed_ratio } =>
                    settings.packed_resistance * (1.0 + wet_ratio * 0.4 + packed_ratio * 0.3),
            };
            
            base_resistance * (1.0 + handling.displacement_factor * 0.5)
        };

        // Apply wind drift effects to forces
        let drift_force = if handling.wind_drift_accumulation > 0.1 {
            let drift_resistance = handling.wind_drift_accumulation * 
                settings.wind_drift_factor * 
                wheel.velocity.length() * 0.5;
            
            handling.drift_direction * drift_resistance
        } else {
            Vec3::ZERO
        };

        // Apply sinkage with snow type considerations
        let sinkage_factor = match handling.snow_type {
            SnowType::Wet { water_content } => 1.2 + water_content * 0.4,
            SnowType::Dry { powder_factor } => 1.5 + powder_factor * 0.5,
            SnowType::Packed { compression } => 0.7 + compression * 0.2,
            SnowType::Mixed { wet_ratio, packed_ratio } => 
                1.0 + wet_ratio * 0.3 + (1.0 - packed_ratio) * 0.4,
        };

        let sinkage = handling.current_snow_depth * settings.snow_sinkage_factor * 
            sinkage_factor *
            (wheel.mass / handling.track_width).min(1.0) *
            (1.0 - handling.snow_density * 0.7) *
            (1.0 - handling.surface_hardness * 0.5);

        // Calculate displacement resistance
        let displacement_resistance = if !handling.is_on_ice {
            let base_displacement = wheel.velocity.length() * handling.displacement_factor;
            match handling.snow_type {
                SnowType::Wet { water_content } => 
                    base_displacement * (0.7 + water_content * 0.4),
                SnowType::Dry { powder_factor } => 
                    base_displacement * (1.2 + powder_factor * 0.6),
                SnowType::Packed { .. } => 
                    base_displacement * 0.5,
                SnowType::Mixed { wet_ratio, packed_ratio } =>
                    base_displacement * (0.8 + wet_ratio * 0.3 - packed_ratio * 0.2),
            }
        } else {
            0.0
        };

        // Modify forces
        ext_force.force *= grip_factor;
        ext_force.force -= wheel.velocity.normalize_or_zero() * (resistance + displacement_resistance);
        ext_force.force -= drift_force;
        ext_force.force.y -= sinkage * 9.81 * wheel.mass;
        
        // Add lateral resistance in deep snow with type consideration
        if handling.current_snow_depth > 0.3 {
            let lateral_velocity = wheel.velocity - wheel.velocity.project_onto(wheel.forward_direction);
            let lateral_factor = match handling.snow_type {
                SnowType::Wet { water_content } => 1.2 + water_content * 0.4,
                SnowType::Dry { powder_factor } => 0.8 + powder_factor * 0.6,
                SnowType::Packed { compression } => 1.5 + compression * 0.3,
                SnowType::Mixed { wet_ratio, packed_ratio } =>
                    1.0 + wet_ratio * 0.3 + packed_ratio * 0.5,
            };
            
            ext_force.force -= lateral_velocity.normalize_or_zero() * 
                handling.current_snow_depth * 
                handling.displacement_factor *
                lateral_factor *
                wheel.mass * 2.0;
        }
    }
}

fn generate_snow_tracks(
    query: Query<(&SnowHandling, &Transform, &Wheel)>,
    mut deformation_events: EventWriter<TerrainDeformationEvent>,
    settings: Res<SnowHandlingSettings>,
) {
    for (handling, transform, wheel) in query.iter() {
        if handling.current_snow_depth > 0.05 {
            // Calculate deformation strength based on snow type
            let (strength_mult, displacement_mult) = match handling.snow_type {
                SnowType::Wet { water_content } => (
                    1.2 + water_content * 0.4,  // Wet snow deforms more easily
                    0.7 + water_content * 0.3   // But displaces less
                ),
                SnowType::Dry { powder_factor } => (
                    0.8 + powder_factor * 0.3,  // Dry snow deforms less
                    1.3 + powder_factor * 0.5   // But displaces more
                ),
                SnowType::Packed { compression } => (
                    0.6 + compression * 0.2,    // Packed snow resists deformation
                    0.5 + compression * 0.2     // And displaces little
                ),
                SnowType::Mixed { wet_ratio, packed_ratio } => (
                    1.0 + wet_ratio * 0.3 - packed_ratio * 0.2,
                    1.0 + wet_ratio * 0.2 + (1.0 - packed_ratio) * 0.3
                ),
            };

            let base_strength = handling.snow_compression * 
                (1.0 - handling.snow_density * 0.5) *
                (1.0 - handling.surface_hardness * 0.3);
            
            let strength = base_strength * strength_mult;

            // Generate track deformation
            deformation_events.send(TerrainDeformationEvent {
                chunk_pos: transform.translation.floor().as_ivec3(),
                world_pos: transform.translation,
                radius: handling.track_width * 0.5,
                strength,
                deformation_type: DeformationType::SnowTrack {
                    vehicle_weight: wheel.mass * 9.81,
                    vehicle_speed: wheel.velocity.length(),
                    track_width: handling.track_width,
                },
            });

            // Generate additional snow displacement based on type and conditions
            let should_displace = match handling.snow_type {
                SnowType::Dry { powder_factor } => 
                    powder_factor > 0.6 && wheel.velocity.length() > 4.0,
                SnowType::Wet { water_content } => 
                    water_content < 0.4 && wheel.velocity.length() > 6.0,
                SnowType::Mixed { packed_ratio, .. } => 
                    packed_ratio < 0.4 && wheel.velocity.length() > 5.0,
                _ => false,
            };

            if should_displace {
                let displacement_dir = wheel.velocity.normalize() + 
                    handling.drift_direction * handling.wind_drift_accumulation;
                
                deformation_events.send(TerrainDeformationEvent {
                    chunk_pos: transform.translation.floor().as_ivec3(),
                    world_pos: transform.translation + displacement_dir.normalize() * handling.track_width,
                    radius: handling.track_width * 1.5,
                    strength: handling.displacement_factor * 0.3 * displacement_mult,
                    deformation_type: DeformationType::SnowPile {
                        density: handling.snow_density,
                    },
                });
            }

            // Generate additional drift piles in strong wind
            if handling.wind_drift_accumulation > 1.5 && 
               matches!(handling.snow_type, SnowType::Dry { .. }) {
                deformation_events.send(TerrainDeformationEvent {
                    chunk_pos: transform.translation.floor().as_ivec3(),
                    world_pos: transform.translation + handling.drift_direction * handling.track_width * 2.0,
                    radius: handling.track_width * 2.0,
                    strength: handling.wind_drift_accumulation * 0.2,
                    deformation_type: DeformationType::SnowPile {
                        density: handling.snow_density * 0.8, // Drifted snow is less dense
                    },
                });
            }
        }
    }
}

// Helper functions for snow condition calculations
fn get_snow_at_position(position: Vec3, snow_query: &Query<&SnowDeformation>) -> Option<&SnowDeformation> {
    // Find the closest snow deformation entity to the given position
    snow_query.iter()
        .min_by_key(|deform| {
            // Find the closest track to the position
            deform.tracks.iter()
                .map(|track| {
                    let dist_to_start = (track.start_pos - position).length_squared();
                    let dist_to_end = (track.end_pos - position).length_squared();
                    dist_to_start.min(dist_to_end)
                })
                .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                .unwrap_or(f32::MAX)
                .sqrt() as i32
        })
}

fn calculate_snow_depth(snow: &SnowDeformation, position: Vec3) -> f32 {
    let mut total_depth = 0.0;
    let mut weight_sum = 0.0;

    // Consider all nearby tracks
    for track in &snow.tracks {
        let dist_to_track = point_to_line_segment_distance(
            position,
            track.start_pos,
            track.end_pos
        );

        if dist_to_track < track.width * 2.0 {
            // Weight based on distance and track age
            let distance_weight = 1.0 - (dist_to_track / (track.width * 2.0));
            let age_weight = 1.0 - (track.age / 300.0).min(1.0); // Fade over 5 minutes
            let weight = distance_weight * age_weight;

            total_depth += (track.depth * weight);
            weight_sum += weight;
        }
    }

    if weight_sum > 0.0 {
        total_depth / weight_sum
    } else {
        0.2 // Default snow depth when no tracks are nearby
    }
}

fn check_for_ice(snow: &SnowDeformation, position: Vec3) -> bool {
    // Check compression map for highly compressed snow
    let mut is_icy = false;
    let mut total_weight = 0.0;

    for (i, compression) in snow.compression_map.iter().enumerate() {
        let pos = Vec3::new(
            (i as f32 % 16.0) * 2.0,
            0.0,
            (i as f32 / 16.0).floor() * 2.0
        );
        
        let dist = (position - pos).length();
        if dist < 4.0 {
            let weight = 1.0 - (dist / 4.0);
            // High compression + low temperature = ice
            if *compression > 0.9 {
                is_icy = true;
                break;
            }
            total_weight += weight;
        }
    }

    is_icy
}

fn calculate_snow_compression(snow: &SnowDeformation, position: Vec3) -> f32 {
    let mut total_compression = 0.0;
    let mut weight_sum = 0.0;

    // Consider all tracks for compression calculation
    for track in &snow.tracks {
        let dist_to_track = point_to_line_segment_distance(
            position,
            track.start_pos,
            track.end_pos
        );

        if dist_to_track < track.width * 1.5 {
            let distance_weight = 1.0 - (dist_to_track / (track.width * 1.5));
            let age_weight = 1.0 - (track.age / 300.0).min(1.0);
            let weight = distance_weight * age_weight;

            // Compressed tracks contribute more
            let compression = if track.compressed {
                0.8 + (0.2 * (1.0 - age_weight))
            } else {
                0.4 + (0.3 * (1.0 - age_weight))
            };

            total_compression += compression * weight;
            weight_sum += weight;
        }
    }

    if weight_sum > 0.0 {
        total_compression / weight_sum
    } else {
        0.5 // Default compression when no tracks are nearby
    }
}

fn point_to_line_segment_distance(point: Vec3, start: Vec3, end: Vec3) -> f32 {
    let line_vec = end - start;
    let point_vec = point - start;
    let line_len = line_vec.length();
    
    if line_len == 0.0 {
        return point_vec.length();
    }
    
    let t = (point_vec.dot(line_vec) / line_len).clamp(0.0, line_len);
    let projection = start + line_vec * (t / line_len);
    
    (point - projection).length()
}

fn get_layer_hardness(layer: &SnowLayer) -> f32 {
    match layer.snow_type {
        SnowType::Dry { powder_factor } => 0.3 + (1.0 - powder_factor) * 0.3,
        SnowType::Wet { water_content } => 0.5 + water_content * 0.4,
        SnowType::Packed { compression } => 0.7 + compression * 0.3,
        SnowType::Mixed { wet_ratio, packed_ratio } => 
            0.5 + wet_ratio * 0.3 + packed_ratio * 0.4,
    }
} 