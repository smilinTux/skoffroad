use bevy::prelude::*;
use noise::{NoiseFn, Perlin, Fbm, Worley, SuperSimplex};
use std::f64;

use super::TerrainConfig;

/// Represents a single noise layer for terrain generation
#[derive(Clone, Debug)]
pub struct NoiseLayer {
    /// Type of terrain feature this layer generates
    pub feature_type: TerrainFeatureType,
    /// Type of noise for this layer
    pub noise_type: NoiseType,
    /// Base frequency of the noise
    pub frequency: f64,
    /// Amplitude (strength) of this layer
    pub amplitude: f64,
    /// Number of octaves for FBm noise
    pub octaves: u32,
    /// Persistence between octaves
    pub persistence: f64,
    /// Lacunarity between octaves
    pub lacunarity: f64,
    /// Mask frequency for selective application
    pub mask_frequency: f64,
    /// Threshold for feature application
    pub threshold: f64,
    /// Smoothing factor for transitions
    pub smoothing: f64,
    /// Erosion iterations for this layer
    pub erosion_iterations: u32,
    /// Seed for noise generation
    pub seed: u32,
}

impl Default for NoiseLayer {
    fn default() -> Self {
        Self {
            feature_type: TerrainFeatureType::Base,
            noise_type: NoiseType::Perlin,
            frequency: 1.0,
            amplitude: 1.0,
            octaves: 4,
            persistence: 0.5,
            lacunarity: 2.0,
            mask_frequency: 0.5,
            threshold: 0.5,
            smoothing: 0.1,
            erosion_iterations: 0,
            seed: 0,
        }
    }
}

/// Types of terrain features that can be generated
#[derive(Clone, Debug)]
pub enum TerrainFeatureType {
    /// Base terrain heightmap
    Base,
    /// Ridge formations
    Ridge,
    /// Plateau areas
    Plateau,
    /// Valley formations
    Valley,
    /// Warping noise (for domain warping)
    Warp,
    /// Mountain peaks
    Peak,
    /// River channels
    River,
    /// Canyon formations
    Canyon,
    /// Crater impacts
    Crater,
    /// Dune formations
    Dune,
    /// Basin formations
    Basin,
}

/// Types of noise for terrain generation
#[derive(Clone, Debug)]
pub enum NoiseType {
    /// Perlin noise
    Perlin,
    /// FBm noise
    Fbm,
    /// Worley noise
    Worley,
    /// Super Simplex noise
    SuperSimplex,
}

/// Component that stores heightmap data
#[derive(Component)]
pub struct Heightmap {
    /// Raw height data
    pub heights: Vec<f32>,
    /// Dimensions of the heightmap
    pub dimensions: UVec2,
    /// World size of the terrain
    pub size: Vec2,
}

impl Heightmap {
    /// Creates a new heightmap with the given dimensions
    pub fn new(dimensions: UVec2, size: Vec2) -> Self {
        let total_size = (dimensions.x * dimensions.y) as usize;
        Self {
            heights: vec![0.0; total_size],
            dimensions,
            size,
        }
    }

    /// Gets the height at the given grid coordinates
    pub fn get_height(&self, x: u32, y: u32) -> Option<f32> {
        if x >= self.dimensions.x || y >= self.dimensions.y {
            return None;
        }
        let index = (y * self.dimensions.x + x) as usize;
        Some(self.heights[index])
    }

    /// Sets the height at the given grid coordinates
    pub fn set_height(&mut self, x: u32, y: u32, height: f32) -> bool {
        if x >= self.dimensions.x || y >= self.dimensions.y {
            return false;
        }
        let index = (y * self.dimensions.x + x) as usize;
        self.heights[index] = height;
        true
    }

    /// Gets the interpolated height at the given world coordinates
    pub fn get_height_at(&self, position: Vec2) -> Option<f32> {
        // Convert world coordinates to grid coordinates
        let grid_x = ((position.x + self.size.x / 2.0) / self.size.x * self.dimensions.x as f32) as u32;
        let grid_y = ((position.y + self.size.y / 2.0) / self.size.y * self.dimensions.y as f32) as u32;
        
        // Get heights at surrounding grid points
        let h00 = self.get_height(grid_x, grid_y)?;
        let h10 = self.get_height(grid_x + 1, grid_y).unwrap_or(h00);
        let h01 = self.get_height(grid_x, grid_y + 1).unwrap_or(h00);
        let h11 = self.get_height(grid_x + 1, grid_y + 1).unwrap_or(h00);
        
        // Calculate fractional position within grid cell
        let fx = ((position.x + self.size.x / 2.0) / self.size.x * self.dimensions.x as f32).fract();
        let fy = ((position.y + self.size.y / 2.0) / self.size.y * self.dimensions.y as f32).fract();
        
        // Bilinear interpolation
        let top = h00 + fx * (h10 - h00);
        let bottom = h01 + fx * (h11 - h01);
        Some(top + fy * (bottom - top))
    }
}

/// Manages noise generation for terrain
pub struct TerrainNoise {
    base_noise: Fbm<Perlin>,
    warp_noise_x: Option<Fbm<Perlin>>,
    warp_noise_y: Option<Fbm<Perlin>>,
    layers: Vec<(NoiseLayer, Fbm<Perlin>)>,
}

impl TerrainNoise {
    /// Creates a new TerrainNoise instance from config
    pub fn new(config: &TerrainConfig) -> Self {
        let mut base_noise = Fbm::new(config.seed);
        base_noise.octaves = config.octaves;
        base_noise.persistence = config.persistence;
        base_noise.lacunarity = config.lacunarity;
        base_noise.frequency = config.frequency;

        let (warp_noise_x, warp_noise_y) = if config.enable_warping {
            let mut wx = Fbm::new(config.seed.wrapping_add(1));
            let mut wy = Fbm::new(config.seed.wrapping_add(2));
            wx.octaves = 4;
            wy.octaves = 4;
            wx.frequency = config.frequency * 2.0;
            wy.frequency = config.frequency * 2.0;
            (Some(wx), Some(wy))
        } else {
            (None, None)
        };

        let mut layers = Vec::new();
        for layer in &config.additional_layers {
            let mut noise = Fbm::new(config.seed.wrapping_add(layers.len() as u32 + 3));
            noise.octaves = layer.octaves;
            noise.persistence = layer.persistence;
            noise.lacunarity = layer.lacunarity;
            noise.frequency = layer.frequency;
            layers.push((layer.clone(), noise));
        }

        Self {
            base_noise,
            warp_noise_x,
            warp_noise_y,
            layers,
        }
    }

    /// Generates a heightmap using configured noise
    pub fn generate_heightmap(&self, config: &TerrainConfig, size: UVec2) -> Heightmap {
        let mut heightmap = Heightmap::new(size, config.size);
        let scale_x = config.size.x / size.x as f32;
        let scale_y = config.size.y / size.y as f32;

        for y in 0..size.y {
            for x in 0..size.x {
                let mut px = x as f64 * scale_x as f64;
                let mut py = y as f64 * scale_y as f64;

                // Apply domain warping if enabled
                if let (Some(wx), Some(wy)) = (&self.warp_noise_x, &self.warp_noise_y) {
                    let warp_x = wx.get([px, py]) * config.warp_strength;
                    let warp_y = wy.get([px, py]) * config.warp_strength;
                    px += warp_x;
                    py += warp_y;
                }

                // Generate base terrain
                let mut height = self.base_noise.get([px, py]) as f32;

                // Add contribution from each additional layer
                for (layer, noise) in &self.layers {
                    let mut lx = px;
                    let mut ly = py;

                    // Apply per-layer warping if enabled
                    if layer.enable_warping {
                        if let (Some(wx), Some(wy)) = (&self.warp_noise_x, &self.warp_noise_y) {
                            let warp_x = wx.get([lx, ly]) * layer.warp_strength;
                            let warp_y = wy.get([lx, ly]) * layer.warp_strength;
                            lx += warp_x;
                            ly += warp_y;
                        }
                    }

                    let layer_value = noise.get([lx, ly]) as f32;
                    let mask_value = noise.get([lx * layer.mask_frequency, ly * layer.mask_frequency]) as f32;
                    
                    // Apply different operations based on feature type with masking
                    match layer.feature_type {
                        TerrainFeatureType::Base => {
                            height += layer_value * layer.amplitude as f32;
                        }
                        TerrainFeatureType::Ridge => {
                            let ridge = 1.0 - layer_value.abs();
                            let contribution = ridge * layer.amplitude as f32;
                            height += contribution * self.apply_mask(mask_value, layer);
                        }
                        TerrainFeatureType::Plateau => {
                            let plateau = (layer_value * 2.0).min(1.0).max(0.0);
                            let target = plateau * layer.amplitude as f32;
                            height = height.max(target) * self.apply_mask(mask_value, layer);
                        }
                        TerrainFeatureType::Valley => {
                            let valley = (-layer_value).max(0.0);
                            let contribution = valley * layer.amplitude as f32;
                            height = height.min(-contribution) * self.apply_mask(mask_value, layer);
                        }
                        TerrainFeatureType::Peak => {
                            let peak = (layer_value * 2.0).max(0.0).powf(2.0);
                            let contribution = peak * layer.amplitude as f32;
                            height += contribution * self.apply_mask(mask_value, layer);
                        }
                        TerrainFeatureType::River => {
                            let river = (-layer_value.abs()).max(-1.0);
                            let contribution = river * layer.amplitude as f32;
                            if mask_value > layer.threshold {
                                height += contribution;
                            }
                        }
                        TerrainFeatureType::Canyon => {
                            let canyon = (-layer_value.abs()).max(-1.0).powf(0.5);
                            let contribution = canyon * layer.amplitude as f32;
                            height += contribution * self.apply_mask(mask_value, layer);
                        }
                        TerrainFeatureType::Crater => {
                            let crater = -layer_value.abs().powf(0.3);
                            let contribution = crater * layer.amplitude as f32;
                            if mask_value > layer.threshold {
                                height += contribution;
                            }
                        }
                        TerrainFeatureType::Dune => {
                            let dune = (layer_value.sin() + 1.0) * 0.5;
                            let contribution = dune * layer.amplitude as f32;
                            height += contribution * self.apply_mask(mask_value, layer);
                        }
                        TerrainFeatureType::Warp => {} // Warp layers don't contribute directly to height
                    }
                }

                // Apply final height scale
                height *= config.height_scale;
                heightmap.set_height(x, y, height);
            }
        }

        // Apply erosion if configured
        for (layer, _) in &self.layers {
            if layer.erosion_iterations > 0 {
                heightmap = self.apply_erosion(&heightmap, layer.erosion_iterations);
            }
        }

        heightmap
    }

    /// Applies masking with smoothing for feature blending
    fn apply_mask(&self, mask_value: f32, layer: &NoiseLayer) -> f32 {
        let t = ((mask_value - layer.threshold) / layer.smoothing).clamp(0.0, 1.0);
        t * t * (3.0 - 2.0 * t) // Smoothstep interpolation
    }

    /// Applies thermal erosion to the heightmap
    fn apply_erosion(&self, heightmap: &Heightmap, iterations: usize) -> Heightmap {
        let mut eroded = heightmap.clone();
        let talus = 0.5; // Maximum stable slope angle

        for _ in 0..iterations {
            for y in 1..heightmap.dimensions.y - 1 {
                for x in 1..heightmap.dimensions.x - 1 {
                    let current = eroded.get_height(x, y).unwrap();
                    let mut total_delta = 0.0;

                    // Check all neighbors
                    for (dx, dy) in &[(0, 1), (1, 0), (0, -1), (-1, 0)] {
                        let nx = (x as i32 + dx) as u32;
                        let ny = (y as i32 + dy) as u32;
                        
                        if let Some(neighbor) = eroded.get_height(nx, ny) {
                            let height_diff = current - neighbor;
                            if height_diff > talus {
                                let delta = (height_diff - talus) * 0.5;
                                total_delta += delta;
                                eroded.set_height(nx, ny, neighbor + delta);
                            }
                        }
                    }

                    if total_delta > 0.0 {
                        eroded.set_height(x, y, current - total_delta);
                    }
                }
            }
        }

        eroded
    }
}

/// System that generates the initial heightmap
pub fn generate_heightmap(
    mut commands: Commands,
    config: Res<TerrainConfig>,
) {
    let noise = TerrainNoise::new(&config);
    let heightmap = noise.generate_heightmap(&config, config.resolution);
    
    commands.spawn((
        heightmap,
        Name::new("Terrain Heightmap"),
    ));
    
    info!("Generated terrain heightmap with dimensions: {:?}", config.resolution);
}

pub fn generate_heightmap_from_layers(
    size: UVec2,
    layers: &[NoiseLayer],
    base_height: f32,
) -> Vec<f32> {
    let mut heightmap = vec![base_height; (size.x * size.y) as usize];
    
    for layer in layers {
        let noise_fn = match layer.noise_type {
            NoiseType::Perlin => Box::new(Perlin::new(layer.seed)) as Box<dyn NoiseFn<f64, 2>>,
            NoiseType::Fbm => Box::new(Fbm::new(layer.seed)) as Box<dyn NoiseFn<f64, 2>>,
            NoiseType::Worley => Box::new(Worley::new(layer.seed)) as Box<dyn NoiseFn<f64, 2>>,
            NoiseType::SuperSimplex => Box::new(SuperSimplex::new(layer.seed)) as Box<dyn NoiseFn<f64, 2>>,
        };

        let mask_fn = Perlin::new(layer.seed.wrapping_add(1));
        
        for y in 0..size.y {
            for x in 0..size.x {
                let idx = (y * size.x + x) as usize;
                let nx = x as f64 * layer.frequency / size.x as f64;
                let ny = y as f64 * layer.frequency / size.y as f64;
                
                let noise_val = noise_fn.get([nx, ny]);
                let mask_val = mask_fn.get([nx * layer.mask_frequency, ny * layer.mask_frequency]);
                
                let mut height_contribution = match layer.feature_type {
                    TerrainFeatureType::Base => noise_val * layer.amplitude,
                    TerrainFeatureType::Peak => {
                        if noise_val > layer.threshold {
                            (noise_val - layer.threshold).powf(2.0) * layer.amplitude
                        } else {
                            0.0
                        }
                    },
                    TerrainFeatureType::Valley => {
                        if noise_val < layer.threshold {
                            -(layer.threshold - noise_val).powf(2.0) * layer.amplitude
                        } else {
                            0.0
                        }
                    },
                    TerrainFeatureType::River => {
                        let river_val = (noise_val.abs() - layer.threshold).abs();
                        if river_val < layer.smoothing {
                            -layer.amplitude * (1.0 - river_val / layer.smoothing)
                        } else {
                            0.0
                        }
                    },
                    TerrainFeatureType::Canyon => {
                        if mask_val > layer.threshold {
                            -layer.amplitude * noise_val.abs()
                        } else {
                            0.0
                        }
                    },
                    TerrainFeatureType::Crater => {
                        if mask_val > layer.threshold {
                            let dist = ((nx - 0.5).powi(2) + (ny - 0.5).powi(2)).sqrt();
                            -layer.amplitude * (1.0 - dist).max(0.0)
                        } else {
                            0.0
                        }
                    },
                    TerrainFeatureType::Dune => {
                        let angle = (nx * f64::consts::PI * 2.0).sin();
                        layer.amplitude * angle * mask_val
                    },
                    TerrainFeatureType::Ridge => {
                        layer.amplitude * (1.0 - (noise_val.abs() * 2.0 - 1.0).abs())
                    },
                    TerrainFeatureType::Plateau => {
                        if noise_val > layer.threshold {
                            layer.amplitude
                        } else if noise_val > layer.threshold - layer.smoothing {
                            layer.amplitude * (noise_val - (layer.threshold - layer.smoothing)) / layer.smoothing
                        } else {
                            0.0
                        }
                    },
                    TerrainFeatureType::Basin => {
                        if noise_val < layer.threshold {
                            -layer.amplitude
                        } else if noise_val < layer.threshold + layer.smoothing {
                            -layer.amplitude * (1.0 - (noise_val - layer.threshold) / layer.smoothing)
                        } else {
                            0.0
                        }
                    },
                };

                // Apply mask blending
                height_contribution *= mask_val.abs();
                
                heightmap[idx] += height_contribution as f32;
            }
        }

        // Apply thermal erosion if specified
        if layer.erosion_iterations > 0 {
            apply_thermal_erosion(&mut heightmap, size, layer.erosion_iterations);
        }
    }

    heightmap
}

fn apply_thermal_erosion(heightmap: &mut Vec<f32>, size: UVec2, iterations: u32) {
    let talus = 0.5; // Maximum stable slope
    let erosion_rate = 0.1;

    for _ in 0..iterations {
        let mut erosion = vec![0.0; heightmap.len()];

        for y in 1..size.y - 1 {
            for x in 1..size.x - 1 {
                let idx = (y * size.x + x) as usize;
                let height = heightmap[idx];

                // Check neighbors
                let neighbors = [
                    (x - 1, y), (x + 1, y),
                    (x, y - 1), (x, y + 1)
                ];

                for (nx, ny) in neighbors.iter() {
                    let n_idx = (ny * size.x + nx) as usize;
                    let height_diff = height - heightmap[n_idx];

                    if height_diff > talus {
                        let erosion_amount = (height_diff - talus) * erosion_rate;
                        erosion[idx] -= erosion_amount;
                        erosion[n_idx] += erosion_amount;
                    }
                }
            }
        }

        // Apply erosion
        for i in 0..heightmap.len() {
            heightmap[i] += erosion[i];
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_heightmap_creation() {
        let dimensions = UVec2::new(10, 10);
        let size = Vec2::new(100.0, 100.0);
        let heightmap = Heightmap::new(dimensions, size);
        
        assert_eq!(heightmap.heights.len(), 100);
        assert_eq!(heightmap.dimensions, dimensions);
        assert_eq!(heightmap.size, size);
    }

    #[test]
    fn test_height_access() {
        let mut heightmap = Heightmap::new(UVec2::new(10, 10), Vec2::new(100.0, 100.0));
        
        // Test setting and getting height
        assert!(heightmap.set_height(5, 5, 10.0));
        assert_eq!(heightmap.get_height(5, 5), Some(10.0));
        
        // Test out of bounds access
        assert!(!heightmap.set_height(10, 10, 10.0));
        assert_eq!(heightmap.get_height(10, 10), None);
    }

    #[test]
    fn test_noise_generation() {
        let config = TerrainConfig::default();
        let noise = TerrainNoise::new(&config);
        let heightmap = noise.generate_heightmap(&config, config.resolution);
        
        // Verify dimensions
        assert_eq!(heightmap.dimensions, config.resolution);
        assert_eq!(heightmap.size, config.size);
        
        // Verify heights are within expected range
        let max_height = config.height_scale;
        for height in heightmap.heights {
            assert!(height >= -max_height && height <= max_height);
        }
    }

    #[test]
    fn test_height_interpolation() {
        let mut heightmap = Heightmap::new(UVec2::new(2, 2), Vec2::new(10.0, 10.0));
        
        // Set test heights
        heightmap.set_height(0, 0, 0.0);
        heightmap.set_height(1, 0, 1.0);
        heightmap.set_height(0, 1, 1.0);
        heightmap.set_height(1, 1, 2.0);
        
        // Test center point (should be average of all corners)
        let center_height = heightmap.get_height_at(Vec2::ZERO).unwrap();
        assert!((center_height - 1.0).abs() < 0.001);
    }
}

#[derive(Resource)]
pub struct TerrainHeightmap {
    noise: Fbm,
    scale: f32,
    amplitude: f32,
}

impl Default for TerrainHeightmap {
    fn default() -> Self {
        let mut fbm = Fbm::new();
        fbm.octaves = 6;
        fbm.frequency = 0.005;
        fbm.persistence = 0.5;
        fbm.lacunarity = 2.0;

        Self {
            noise: fbm,
            scale: 1.0,
            amplitude: 50.0,
        }
    }
}

impl TerrainHeightmap {
    pub fn sample_height(&self, x: f32, z: f32) -> f32 {
        let nx = x * self.scale;
        let nz = z * self.scale;
        
        let noise_val = self.noise.get([nx as f64, nz as f64]) as f32;
        noise_val * self.amplitude
    }

    pub fn sample_height_at_lod(&self, x: f32, z: f32, lod: u32) -> f32 {
        // Adjust sampling frequency based on LOD level
        let frequency_scale = 1.0 / (1 << lod) as f32;
        let nx = x * self.scale * frequency_scale;
        let nz = z * self.scale * frequency_scale;
        
        let noise_val = self.noise.get([nx as f64, nz as f64]) as f32;
        noise_val * self.amplitude
    }

    pub fn calculate_normal(&self, x: f32, z: f32) -> [f32; 3] {
        let sample_distance = 1.0;
        
        let h_center = self.sample_height(x, z);
        let h_right = self.sample_height(x + sample_distance, z);
        let h_forward = self.sample_height(x, z + sample_distance);
        
        let dx = h_right - h_center;
        let dz = h_forward - h_center;
        
        let normal = Vec3::new(-dx, sample_distance, -dz).normalize();
        [normal.x, normal.y, normal.z]
    }

    pub fn calculate_normal_at_lod(&self, x: f32, z: f32, lod: u32) -> [f32; 3] {
        // Adjust sample distance based on LOD level
        let sample_distance = (1 << lod) as f32;
        
        let h_center = self.sample_height_at_lod(x, z, lod);
        let h_right = self.sample_height_at_lod(x + sample_distance, z, lod);
        let h_forward = self.sample_height_at_lod(x, z + sample_distance, lod);
        
        let dx = h_right - h_center;
        let dz = h_forward - h_center;
        
        let normal = Vec3::new(-dx, sample_distance, -dz).normalize();
        [normal.x, normal.y, normal.z]
    }
} 