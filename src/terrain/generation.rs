use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use noise::{NoiseFn, Perlin, Fbm, Turbulence, Worley};
use bevy::prelude::Vec2;
use bevy::prelude::Vec3;
use bevy::prelude::TransformBundle;
use bevy::prelude::App;
use bevy::prelude::Plugin;

use crate::terrain::TerrainChunk;
use crate::terrain::TerrainType;
use crate::terrain::BiomeType;

pub struct TerrainGenerationPlugin;

impl Plugin for TerrainGenerationPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(TerrainGenerationSettings::default())
           .insert_resource(BiomeSettings::default())
           // .add_systems(Update, generate_terrain_chunks) // TODO: Revisit system registration after signature review
           ;
    }
}

#[derive(Resource)]
pub struct TerrainGenerationSettings {
    pub water_level: f32,
    pub mountain_level: f32,
    pub snow_temperature: f32,
    pub continent_scale: f64,
    pub hills_scale: f64,
    pub mountain_scale: f64,
    pub continent_weight: f64,
    pub hills_weight: f64,
    pub mountain_weight: f64,
    pub erosion_scale: f64,
    pub erosion_strength: f64,
    pub temperature_range: f32,
    pub min_temperature: f32,
    pub temperature_height_factor: f32,
    pub temperature_variation: f32,
    pub world_size: i32,
    pub moisture_scale: f64,
    pub wind_direction: f32,
    pub wind_variability: f32,
    pub wind_strength: f32,
    pub chunk_size: i32,
    pub view_distance: i32,
    pub seed: u32,
    pub season_time: f32,
    pub day_night_cycle: f32,
    pub erosion_iterations: usize,
    pub detail_noise_scale: f64,
    pub noise_scale: f64,
    pub height_scale: f32,
}

impl Default for TerrainGenerationSettings {
    fn default() -> Self {
        Self {
            water_level: 0.3,
            mountain_level: 0.7,
            snow_temperature: 0.2,
            continent_scale: 0.01,
            hills_scale: 0.01,
            mountain_scale: 0.01,
            continent_weight: 0.5,
            hills_weight: 0.5,
            mountain_weight: 2.0,
            erosion_scale: 0.01,
            erosion_strength: 0.5,
            temperature_range: 30.0,
            min_temperature: -10.0,
            temperature_height_factor: 0.0065,
            temperature_variation: 5.0,
            world_size: 128,
            moisture_scale: 0.002,
            wind_direction: 0.0,
            wind_variability: 0.5,
            wind_strength: 5.0,
            chunk_size: 32,
            view_distance: 3,
            seed: 42,
            season_time: 0.0,
            day_night_cycle: 0.0,
            erosion_iterations: 3,
            detail_noise_scale: 0.05,
            noise_scale: 0.01,
            height_scale: 100.0,
        }
    }
}

#[derive(Resource)]
pub struct BiomeSettings {
    pub temperature_scale: f64,
    pub moisture_scale: f64,
    pub biome_blend_scale: f64,
}

impl Default for BiomeSettings {
    fn default() -> Self {
        Self {
            temperature_scale: 0.003,
            moisture_scale: 0.002,
            biome_blend_scale: 0.01,
        }
    }
}

#[derive(Component)]
pub struct TerrainGenerator {
    noise: Perlin,
    biome_noise: Perlin,
    feature_noise: Perlin,
    temperature_noise: Perlin,
    moisture_noise: Perlin,
}

impl TerrainGenerator {
    pub fn new(seed: u32) -> Self {
        Self {
            noise: Perlin::new(seed),
            biome_noise: Perlin::new(seed.wrapping_add(1)),
            feature_noise: Perlin::new(seed.wrapping_add(2)),
            temperature_noise: Perlin::new(seed.wrapping_add(3)),
            moisture_noise: Perlin::new(seed.wrapping_add(4)),
        }
    }
}

fn generate_terrain_chunks(
    mut commands: Commands,
    settings: Res<TerrainGenerationSettings>,
    biome_settings: Res<BiomeSettings>,
    // config: Res<TerrainGenerationConfig>, // Commented out, not defined
    query: Query<(Entity, &Transform), With<TerrainGenerator>>,
) {
    for (entity, transform) in query.iter() {
        let center_pos = transform.translation;
        let chunk_size = settings.chunk_size as f32;
        for x in -settings.view_distance..=settings.view_distance {
            for z in -settings.view_distance..=settings.view_distance {
                let chunk_pos = Vec3::new(
                    (center_pos.x / chunk_size).floor() * chunk_size + x as f32 * chunk_size,
                    0.0,
                    (center_pos.z / chunk_size).floor() * chunk_size + z as f32 * chunk_size,
                );
                let chunk_data = generate_terrain_chunk(
                    (chunk_pos.x / chunk_size) as i32,
                    (chunk_pos.z / chunk_size) as i32,
                    &*settings,
                );
                commands.spawn((
                    TransformBundle::from_transform(Transform::from_translation(chunk_pos)),
                    create_chunk_collider(&*settings, &chunk_data),
                    chunk_data,
                ));
            }
        }
    }
}

pub fn generate_terrain_chunk(
    x: i32,
    z: i32,
    settings: &TerrainGenerationSettings,
) -> TerrainChunk {
    let size = settings.chunk_size as usize;
    let mut chunk = TerrainChunk::new_default(x, z, size);
    // Generate base terrain heightmap
    for z_ in 0..size {
        for x_ in 0..size {
            let world_x = (x_ as i32 + chunk.position.x as i32) as f64;
            let world_z = (z_ as i32 + chunk.position.y as i32) as f64;
            // Generate continent-scale features with improved noise
            let perlin = Perlin::new(settings.seed);
            let fbm = Fbm::<Perlin>::new(settings.seed);
            let _worley = Worley::new(settings.seed);
            let turbulence = Turbulence::<Perlin, Perlin>::new(Perlin::new(settings.seed));
            let coords = [world_x * settings.continent_scale, world_z * settings.continent_scale];
            let continent = perlin.get(coords);
            let hills = fbm.get([world_x * settings.hills_scale, world_z * settings.hills_scale]);
            let mountains = turbulence.get([world_x * settings.mountain_scale, world_z * settings.mountain_scale]);
            let height = continent * settings.continent_weight
                      + hills * settings.hills_weight
                      + mountains * settings.mountain_weight;
            chunk.set_height(x_, z_, (height as f32) * settings.height_scale);
            // Generate temperature based on height, latitude, and season
            let latitude_factor = (world_z / 1000.0).abs();
            let season_temp_offset = (settings.season_time * std::f32::consts::PI * 2.0).sin()
                                   * settings.temperature_variation;
            let temp_noise = perlin.get([world_x * settings.temperature_variation as f64, world_z * settings.temperature_variation as f64]);
            let temperature = settings.min_temperature
                + season_temp_offset
                + (temp_noise as f32 * settings.temperature_variation)
                - ((height as f32) * settings.temperature_height_factor)
                - (latitude_factor as f32 * settings.temperature_height_factor);
            chunk.set_temperature(x_, z_, temperature);
            // Generate moisture with seasonal variation
            let base_moisture = perlin.get([world_x * settings.moisture_scale, world_z * settings.moisture_scale]);
            let season_moisture_offset = (settings.season_time * std::f32::consts::PI * 2.0 + std::f32::consts::PI * 0.5).sin() * 0.2;
            chunk.set_moisture(x_, z_, (base_moisture as f32 + season_moisture_offset).clamp(0.0, 1.0));
            // Calculate wind with improved directional coherence
            let wind_noise = perlin.get([world_x * settings.wind_strength as f64, world_z * settings.wind_strength as f64]);
            let wind_speed = (wind_noise as f32 * settings.wind_variability).abs();
            chunk.wind_speed[z_ * size + x_] = wind_speed;
            let wind_dir_noise = perlin.get([world_x * (settings.wind_strength as f64) * 1.5, world_z * (settings.wind_strength as f64) * 1.5]);
            let wind_angle = settings.wind_direction + wind_dir_noise as f32 * settings.wind_variability;
            chunk.wind_direction[z_ * size + x_] = Vec2::new(wind_angle.cos(), wind_angle.sin());
            // Calculate erosion based on slope, wind, and seasonal factors
            let slope = if x_ > 0 && z_ > 0 && x_ < size - 1 && z_ < size - 1 {
                let h_x = chunk.get_height(x_ + 1, z_) - chunk.get_height(x_ - 1, z_);
                let h_z = chunk.get_height(x_, z_ + 1) - chunk.get_height(x_, z_ - 1);
                (h_x * h_x + h_z * h_z).sqrt() * 0.5
            } else {
                0.0
            };
            // Add seasonal erosion variation (more in spring due to melting)
            let season_erosion = (settings.season_time * std::f32::consts::PI * 2.0 + std::f32::consts::PI * 0.25).sin() * 0.2;
            chunk.erosion[z_ * size + x_] = (slope * settings.erosion_strength as f32
                               + wind_speed * settings.wind_strength as f32
                               + season_erosion).clamp(0.0, 1.0);
            // Calculate vegetation based on temperature, moisture, and season
            let season_veg = (settings.season_time * std::f32::consts::PI * 2.0).sin() * 0.3;
            let optimal_temp = 20.0; // Optimal temperature for vegetation
            let temp_factor = 1.0 - (temperature - optimal_temp).abs() / settings.temperature_range;
            let vegetation_potential = (1.0 - chunk.erosion[z_ * size + x_])
                * temp_factor
                * chunk.get_moisture(x_, z_)
                * (1.0 + season_veg);
            chunk.vegetation[z_ * size + x_] = vegetation_potential.clamp(0.0, 1.0);
            // Determine snow coverage based on temperature, height, and season
            let season_snow = (settings.season_time * std::f32::consts::PI * 2.0 + std::f32::consts::PI).sin() * 0.3;
            let snow_coverage = if temperature < settings.snow_temperature {
                ((settings.snow_temperature - temperature) / settings.temperature_range
                    + season_snow).clamp(0.0, 1.0)
            } else {
                0.0
            };
            chunk.snow_coverage[z_ * size + x_] = snow_coverage;
            // Determine terrain type based on all factors
            chunk.terrain_types[z_ * size + x_] = determine_terrain_type(
                height as f32,
                temperature,
                chunk.get_moisture(x_, z_),
            );
        }
    }
    chunk
}

fn calculate_temperature(
    base_temp: f32,
    settings: &TerrainGenerationSettings,
    height: f32,
    noise_temp: f32,
) -> f32 {
    // Season temperature variation (-20 to +30 Celsius)
    let season_temp = match settings.season_time as u32 % 4 {
        0 => -10.0, // Winter
        1 => 10.0,  // Spring
        2 => 25.0,  // Summer
        3 => 5.0,   // Fall
        _ => 0.0,
    };

    // Day/night cycle variation (-5 to +5 Celsius)
    let time_of_day = (settings.day_night_cycle * std::f32::consts::PI * 2.0).sin();
    let day_night_temp = time_of_day * 5.0;

    // Height-based temperature decrease (roughly -6.5°C per 1000m)
    let height_temp = -(height * 0.0065);

    // Combine all temperature factors
    base_temp + season_temp + day_night_temp + height_temp + (noise_temp * settings.temperature_variation)
}

fn calculate_snow_drift(
    settings: &TerrainGenerationSettings,
    position: Vec2,
    height: f32,
    slope_normal: Vec3,
) -> f32 {
    // Calculate wind exposure based on slope normal and wind direction
    let wind_dir_2d = Vec2::new(settings.wind_direction.cos(), settings.wind_direction.sin());
    let wind_dir_3d = Vec3::new(wind_dir_2d.x, 0.0, wind_dir_2d.y).normalize();
    let wind_exposure = slope_normal.dot(wind_dir_3d);

    // Add wind variability using noise
    let wind_noise = Perlin::new(settings.seed + 1).get([
        position.x as f64 * 0.01,
        position.y as f64 * 0.01,
    ]) as f32;
    
    let wind_strength = settings.wind_strength as f32 * (1.0 + wind_noise * settings.wind_variability as f32);

    // Calculate drift accumulation
    // More snow accumulates on leeward slopes (negative wind exposure)
    let drift_factor = if wind_exposure < 0.0 {
        (-wind_exposure * wind_strength * 0.3).max(-0.8)
    } else {
        // Less snow on windward slopes
        (-wind_exposure * wind_strength * 0.3).max(-0.8)
    };

    drift_factor as f32
}

pub fn determine_terrain_type(height: f32, temperature: f32, moisture: f32) -> TerrainType {
    let relative_height = height / 256.0; // Normalize height to 0-1 range

    // Ocean and Beach detection
    if relative_height < 0.3 {
        return TerrainType::Ocean;
    } else if relative_height < 0.32 {
        return TerrainType::Beach;
    }

    // Mountain detection
    if relative_height > 0.7 {
        if temperature < 0.2 {
            return TerrainType::Tundra;
        }
        return TerrainType::Mountain;
    }

    // Other terrain types based on temperature and moisture
    match (temperature, moisture) {
        // Hot climates
        (t, m) if t > 0.7 => {
            if m < 0.2 {
                TerrainType::Desert
            } else if m > 0.6 {
                TerrainType::Rainforest
            } else {
                TerrainType::Plains
            }
        },
        // Temperate climates
        (t, m) if t > 0.3 => {
            if m > 0.6 {
                TerrainType::Forest
            } else {
                TerrainType::Plains
            }
        },
        // Cold climates
        _ => TerrainType::Tundra,
    }
}

fn apply_thermal_erosion(heights: &mut Vec<f32>, settings: &TerrainGenerationSettings) {
    let chunk_size = settings.chunk_size as usize;
    let talus_angle = 0.7; // Maximum stable slope angle
    
    for _ in 0..settings.erosion_iterations {
        for x in 1..chunk_size-1 {
            for z in 1..chunk_size-1 {
                let idx = z * chunk_size + x;
                let current_height = heights[idx];
                
                // Check neighbors
                let neighbors = [
                    (x-1, z), (x+1, z),
                    (x, z-1), (x, z+1),
                ];
                
                for (nx, nz) in neighbors {
                    let n_idx = nz * chunk_size + nx;
                    let height_diff = current_height - heights[n_idx];
                    
                    if height_diff > talus_angle {
                        let transfer = height_diff * 0.5;
                        heights[idx] -= transfer;
                        heights[n_idx] += transfer;
                    }
                }
            }
        }
    }
}

fn create_chunk_collider(
    settings: &TerrainGenerationSettings,
    chunk: &TerrainChunk,
) -> Collider {
    let size = settings.chunk_size as usize;
    let mut vertices = Vec::with_capacity(size * size);
    for z in 0..size {
        for x in 0..size {
            let height = chunk.get_height(x, z);
            vertices.push(Vec3::new(x as f32, height, z as f32));
        }
    }
    let mut indices = Vec::new();
    for x in 0..size - 1 {
        for z in 0..size - 1 {
            let i = z * size + x;
            indices.push([
                i as u32,
                (i + 1) as u32,
                (i + size) as u32,
            ]);
            indices.push([
                (i + 1) as u32,
                (i + size + 1) as u32,
                (i + size) as u32,
            ]);
        }
    }
    Collider::trimesh(vertices, indices)
} 