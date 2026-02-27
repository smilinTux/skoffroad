use bevy::prelude::*;
use super::Heightmap;
use rand::{Rng, thread_rng};

/// Parameters for erosion simulation
#[derive(Clone)]
pub struct ErosionConfig {
    pub iterations: u32,
    pub droplet_lifetime: u32,
    pub erosion_rate: f32,
    pub deposition_rate: f32,
    pub evaporation_rate: f32,
    pub min_slope: f32,
}

impl Default for ErosionConfig {
    fn default() -> Self {
        Self {
            iterations: 50000,
            droplet_lifetime: 30,
            erosion_rate: 0.3,
            deposition_rate: 0.3,
            evaporation_rate: 0.02,
            min_slope: 0.01,
        }
    }
}

/// Simulates water droplet erosion on terrain
pub fn simulate_hydraulic_erosion(heightmap: &mut Heightmap, config: &ErosionConfig) {
    let mut rng = thread_rng();
    let width = heightmap.dimensions.x as usize;
    let height = heightmap.dimensions.y as usize;
    
    for _ in 0..config.iterations {
        // Initialize water droplet
        let mut pos_x = rng.gen_range(0.0..width as f32);
        let mut pos_y = rng.gen_range(0.0..height as f32);
        let mut velocity = Vec2::ZERO;
        let mut water = 1.0;
        let mut sediment = 0.0;
        
        for _ in 0..config.droplet_lifetime {
            let cell_x = pos_x as usize;
            let cell_y = pos_y as usize;
            
            if cell_x >= width - 1 || cell_y >= height - 1 {
                break;
            }
            
            // Calculate gradient
            let h = heightmap.get_height(cell_x as u32, cell_y as u32);
            let h_right = heightmap.get_height((cell_x + 1) as u32, cell_y as u32);
            let h_down = heightmap.get_height(cell_x as u32, (cell_y + 1) as u32);
            let gradient = Vec2::new(h_right - h, h_down - h);
            
            // Update velocity
            if gradient.length() <= config.min_slope {
                velocity *= 0.0;
            } else {
                velocity = velocity * 0.3 + gradient.normalize() * 0.7;
            }
            
            // Move droplet
            pos_x += velocity.x;
            pos_y += velocity.y;
            
            // Erode or deposit
            let new_cell_x = pos_x as usize;
            let new_cell_y = pos_y as usize;
            
            if new_cell_x >= width - 1 || new_cell_y >= height - 1 {
                break;
            }
            
            let h_new = heightmap.get_height(new_cell_x as u32, new_cell_y as u32);
            let h_diff = h_new - h;
            
            if h_diff > 0.0 {
                // Moving uphill - deposit sediment
                let deposit_amount = h_diff.min(sediment);
                sediment -= deposit_amount;
                heightmap.set_height(cell_x as u32, cell_y as u32, h + deposit_amount);
            } else {
                // Moving downhill - erode
                let erode_amount = (-h_diff * config.erosion_rate).min(0.1);
                sediment += erode_amount;
                heightmap.set_height(cell_x as u32, cell_y as u32, h - erode_amount);
            }
            
            // Evaporate water
            water *= 1.0 - config.evaporation_rate;
            if water < 0.01 {
                break;
            }
        }
    }
}

/// Simulates thermal erosion (rock weathering)
pub fn simulate_thermal_erosion(heightmap: &mut Heightmap, iterations: u32, talus_angle: f32) {
    let width = heightmap.dimensions.x as usize;
    let height = heightmap.dimensions.y as usize;
    let talus = talus_angle.tan();
    
    for _ in 0..iterations {
        for x in 1..width-1 {
            for y in 1..height-1 {
                let h = heightmap.get_height(x as u32, y as u32);
                let mut total_delta = 0.0;
                
                // Check neighbors
                for (dx, dy) in &[(0, 1), (1, 0), (0, -1), (-1, 0)] {
                    let nx = (x as i32 + dx) as u32;
                    let ny = (y as i32 + dy) as u32;
                    let h_neighbor = heightmap.get_height(nx, ny);
                    let slope = (h - h_neighbor).abs();
                    
                    if slope > talus {
                        let delta = (slope - talus) * 0.5;
                        total_delta += delta;
                        heightmap.set_height(nx, ny, h_neighbor + delta);
                    }
                }
                
                if total_delta > 0.0 {
                    heightmap.set_height(x as u32, y as u32, h - total_delta);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hydraulic_erosion() {
        let mut heightmap = Heightmap::new(UVec2::new(64, 64), Vec2::new(100.0, 100.0));
        let config = ErosionConfig::default();
        
        // Set up test heightmap with a peak
        heightmap.set_height(32, 32, 10.0);
        
        // Run erosion
        simulate_hydraulic_erosion(&mut heightmap, &config);
        
        // Verify peak was eroded
        assert!(heightmap.get_height(32, 32) < 10.0);
    }

    #[test]
    fn test_thermal_erosion() {
        let mut heightmap = Heightmap::new(UVec2::new(64, 64), Vec2::new(100.0, 100.0));
        
        // Set up steep slope
        heightmap.set_height(32, 32, 10.0);
        heightmap.set_height(33, 32, 0.0);
        
        // Run thermal erosion
        simulate_thermal_erosion(&mut heightmap, 10, 0.5);
        
        // Verify slope was reduced
        let final_slope = (heightmap.get_height(32, 32) - heightmap.get_height(33, 32)).abs();
        assert!(final_slope < 10.0);
    }
} 