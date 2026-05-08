use bevy::prelude::*;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct TerrainFeature {
    pub feature_type: TerrainFeatureType,
    pub position: Vec2,
    pub radius: f32,
    pub intensity: f32,
    pub elevation: f32,
    pub slope: f32,
    pub roughness: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TerrainFeatureType {
    Peak,
    Valley,
    Ridge,
    Cliff,
    Plateau,
    Slope,
    Basin,
    Custom(String),
}

#[derive(Debug, Clone)]
pub struct TerrainFeatureDetectorConfig {
    pub min_feature_size: f32,
    pub max_feature_size: f32,
    pub slope_threshold: f32,
    pub height_threshold: f32,
    pub roughness_threshold: f32,
    pub detection_radius: f32,
    pub feature_merge_distance: f32,
}

impl Default for TerrainFeatureDetectorConfig {
    fn default() -> Self {
        Self {
            min_feature_size: 5.0,
            max_feature_size: 50.0,
            slope_threshold: 0.7,
            height_threshold: 10.0,
            roughness_threshold: 0.5,
            detection_radius: 15.0,
            feature_merge_distance: 10.0,
        }
    }
}

#[derive(Resource)]
pub struct TerrainFeatureDetector {
    config: TerrainFeatureDetectorConfig,
    features: HashMap<Entity, Vec<TerrainFeature>>,
}

impl TerrainFeatureDetector {
    pub fn new(config: TerrainFeatureDetectorConfig) -> Self {
        Self {
            config,
            features: HashMap::new(),
        }
    }

    pub fn analyze_chunk(&mut self, chunk_comp: &TerrainChunkComponent) -> Vec<TerrainFeature> {
        let mut features = Vec::new();
        let chunk_size = chunk_comp.chunk.size;
        let step = chunk_size / 8; // Analysis grid size

        for x in (0..chunk_size).step_by(step) {
            for z in (0..chunk_size).step_by(step) {
                let world_pos = chunk_comp.chunk.world_position + Vec3::new(
                    x as f32 * chunk_comp.chunk.scale.x,
                    0.0,
                    z as f32 * chunk_comp.chunk.scale.z
                );
                
                // Calculate local terrain metrics
                let (slope, roughness, curvature) = self.calculate_local_metrics(
                    &chunk_comp.chunk.heightmap,
                    chunk_size,
                    x,
                    z,
                    step
                );
                
                // Detect features based on metrics
                if let Some(feature) = self.detect_feature(slope, roughness, curvature, world_pos.xz(), chunk_comp.chunk.heightmap[z * chunk_size + x]) {
                    features.push(feature);
                }
            }
        }
        
        // Merge nearby similar features
        self.merge_similar_features(&mut features);
        
        // Store the detected features
        if let Some(entity) = chunk_comp.chunk.chunk_entity {
            self.features.insert(entity, features.clone());
        }
        
        features
    }

    fn calculate_local_metrics(&self, heightmap: &[f32], size: usize, x: usize, z: usize, step: usize) -> (f32, f32, f32) {
        let mut heights = Vec::new();
        let mut slopes = Vec::new();
        let center_height = heightmap[z * size + x];
        
        // Sample points in a radius around the center
        for dx in -(step as i32)..=step as i32 {
            for dz in -(step as i32)..=step as i32 {
                let nx = (x as i32 + dx).clamp(0, size as i32 - 1) as usize;
                let nz = (z as i32 + dz).clamp(0, size as i32 - 1) as usize;
                let height = heightmap[nz * size + nx];
                heights.push(height);
                
                // Calculate slope using central differences
                if dx != 0 || dz != 0 {
                    let dist = ((dx * dx + dz * dz) as f32).sqrt();
                    let slope = ((height - center_height) / dist).abs();
                    slopes.push(slope);
                }
            }
        }
        
        // Calculate metrics
        let avg_height = heights.iter().sum::<f32>() / heights.len() as f32;
        let avg_slope = slopes.iter().sum::<f32>() / slopes.len() as f32;
        
        // Calculate roughness (height variance)
        let roughness = heights.iter()
            .map(|h| (h - avg_height).powi(2))
            .sum::<f32>()
            .sqrt() / heights.len() as f32;
            
        // Calculate curvature (rate of change of slope)
        let curvature = heights.iter()
            .zip(slopes.iter())
            .map(|(h, s)| (h - center_height).signum() * s)
            .sum::<f32>() / heights.len() as f32;
        
        (avg_slope, roughness, curvature)
    }

    fn detect_feature(&self, slope: f32, roughness: f32, curvature: f32, position: Vec2, elevation: f32) -> Option<TerrainFeature> {
        let feature_type = if slope > self.config.slope_threshold * 1.5 {
            TerrainFeatureType::Cliff
        } else if slope > self.config.slope_threshold {
            TerrainFeatureType::Slope
        } else if curvature > self.config.height_threshold {
            if roughness > self.config.roughness_threshold {
                TerrainFeatureType::Peak
            } else {
                TerrainFeatureType::Plateau
            }
        } else if curvature < -self.config.height_threshold {
            if roughness > self.config.roughness_threshold {
                TerrainFeatureType::Valley
            } else {
                TerrainFeatureType::Basin
            }
        } else if roughness > self.config.roughness_threshold {
            TerrainFeatureType::Ridge
        } else {
            return None;
        };

        Some(TerrainFeature {
            feature_type,
            position,
            radius: self.calculate_feature_radius(slope, roughness),
            intensity: self.calculate_feature_intensity(slope, roughness, curvature),
            elevation,
            slope,
            roughness,
        })
    }

    fn calculate_feature_radius(&self, slope: f32, roughness: f32) -> f32 {
        let base_size = (self.config.max_feature_size - self.config.min_feature_size) * 0.5;
        let size_factor = 1.0 - (slope + roughness) * 0.5;
        self.config.min_feature_size + base_size * size_factor
    }

    fn calculate_feature_intensity(&self, slope: f32, roughness: f32, curvature: f32) -> f32 {
        let slope_factor = (slope / self.config.slope_threshold).min(1.0);
        let roughness_factor = (roughness / self.config.roughness_threshold).min(1.0);
        let curvature_factor = (curvature.abs() / self.config.height_threshold).min(1.0);
        
        (slope_factor + roughness_factor + curvature_factor) / 3.0
    }

    fn merge_similar_features(&self, features: &mut Vec<TerrainFeature>) {
        let mut i = 0;
        while i < features.len() {
            let mut j = i + 1;
            while j < features.len() {
                let dist = features[i].position.distance(features[j].position);
                if dist < self.config.feature_merge_distance && 
                   features[i].feature_type == features[j].feature_type {
                    // Merge features by averaging their properties
                    features[i].radius = (features[i].radius + features[j].radius) * 0.5;
                    features[i].intensity = features[i].intensity.max(features[j].intensity);
                    features[i].elevation = (features[i].elevation + features[j].elevation) * 0.5;
                    features[i].slope = features[i].slope.max(features[j].slope);
                    features[i].roughness = features[i].roughness.max(features[j].roughness);
                    features.remove(j);
                } else {
                    j += 1;
                }
            }
            i += 1;
        }
    }

    pub fn get_features(&self, chunk_entity: Entity) -> Option<&Vec<TerrainFeature>> {
        self.features.get(&chunk_entity)
    }
}

// System to update terrain features
pub fn update_terrain_features(
    mut feature_detector: ResMut<TerrainFeatureDetector>,
    terrain_chunks: Query<(Entity, &TerrainChunkComponent)>,
) {
    for (entity, chunk_comp) in terrain_chunks.iter() {
        if chunk_comp.chunk.needs_feature_update {
            let features = feature_detector.analyze_chunk(chunk_comp);
            feature_detector.features.insert(entity, features);
        }
    }
}