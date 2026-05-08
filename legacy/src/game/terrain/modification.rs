use bevy::prelude::*;
use crate::math::*;
use crate::terrain::{Heightmap, TerrainChunk, CHUNK_SIZE};

/// Types of terrain modifications
#[derive(Debug, Clone)]
pub enum ModificationType {
    Raise(f32),
    Lower(f32),
    Flatten(f32),
    Smooth,
}

/// Component for tracking terrain modifications
#[derive(Component)]
pub struct TerrainModification {
    pub position: Vec2,
    pub radius: f32,
    pub falloff: f32,
    pub modification_type: ModificationType,
    pub strength: f32,
}

/// Resource for tracking affected chunks that need mesh updates
#[derive(Default, Resource)]
pub struct ModifiedChunks {
    pub chunks: Vec<UVec2>,
}

/// System to apply terrain modifications
pub fn apply_terrain_modifications(
    mut commands: Commands,
    mut heightmap: ResMut<Heightmap>,
    mut modified_chunks: ResMut<ModifiedChunks>,
    modifications: Query<(Entity, &TerrainModification)>,
) {
    for (entity, modification) in modifications.iter() {
        // Convert world position to heightmap coordinates
        let heightmap_pos = Vec2::new(
            (modification.position.x + heightmap.size.x / 2.0) / heightmap.size.x * heightmap.dimensions.x as f32,
            (modification.position.y + heightmap.size.y / 2.0) / heightmap.size.y * heightmap.dimensions.y as f32,
        );
        
        // Calculate affected region
        let radius_pixels = modification.radius / heightmap.size.x * heightmap.dimensions.x as f32;
        let min_x = ((heightmap_pos.x - radius_pixels).max(0.0) as u32).min(heightmap.dimensions.x);
        let min_y = ((heightmap_pos.y - radius_pixels).max(0.0) as u32).min(heightmap.dimensions.y);
        let max_x = ((heightmap_pos.x + radius_pixels).min(heightmap.dimensions.x as f32) as u32).min(heightmap.dimensions.x);
        let max_y = ((heightmap_pos.y + radius_pixels).min(heightmap.dimensions.y as f32) as u32).min(heightmap.dimensions.y);
        
        // Track affected chunks
        let min_chunk = UVec2::new(min_x / CHUNK_SIZE, min_y / CHUNK_SIZE);
        let max_chunk = UVec2::new(max_x / CHUNK_SIZE, max_y / CHUNK_SIZE);
        
        for chunk_y in min_chunk.y..=max_chunk.y {
            for chunk_x in min_chunk.x..=max_chunk.x {
                let chunk_pos = UVec2::new(chunk_x, chunk_y);
                if !modified_chunks.chunks.contains(&chunk_pos) {
                    modified_chunks.chunks.push(chunk_pos);
                }
            }
        }
        
        // Apply modification
        for y in min_y..max_y {
            for x in min_x..max_x {
                let pos = Vec2::new(x as f32, y as f32);
                let distance = pos.distance(heightmap_pos);
                
                if distance <= radius_pixels {
                    let falloff = 1.0 - (distance / radius_pixels).powf(modification.falloff);
                    let strength = modification.strength * falloff;
                    
                    if let Some(height) = heightmap.get_height_mut(x, y) {
                        match &modification.modification_type {
                            ModificationType::Raise(max_height) => {
                                *height = (*height + strength).min(*max_height);
                            }
                            ModificationType::Lower(min_height) => {
                                *height = (*height - strength).max(*min_height);
                            }
                            ModificationType::Flatten(target_height) => {
                                *height = lerp(*height, *target_height, strength);
                            }
                            ModificationType::Smooth => {
                                let avg = calculate_average_height(&heightmap, x, y, 1);
                                *height = lerp(*height, avg, strength);
                            }
                        }
                    }
                }
            }
        }
        
        // Remove the modification entity after applying
        commands.entity(entity).despawn();
    }
}

/// Helper function to calculate average height in an area
fn calculate_average_height(heightmap: &Heightmap, center_x: u32, center_y: u32, radius: u32) -> f32 {
    let mut sum = 0.0;
    let mut count = 0;
    
    let min_x = center_x.saturating_sub(radius);
    let min_y = center_y.saturating_sub(radius);
    let max_x = (center_x + radius).min(heightmap.dimensions.x - 1);
    let max_y = (center_y + radius).min(heightmap.dimensions.y - 1);
    
    for y in min_y..=max_y {
        for x in min_x..=max_x {
            if let Some(height) = heightmap.get_height(x, y) {
                sum += height;
                count += 1;
            }
        }
    }
    
    if count > 0 {
        sum / count as f32
    } else {
        heightmap.get_height(center_x, center_y).unwrap_or(0.0)
    }
}

/// Helper function for linear interpolation
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

#[cfg(test)]
mod tests {
    use super::*;
    
    fn setup_test_heightmap() -> Heightmap {
        Heightmap {
            dimensions: UVec2::new(100, 100),
            size: Vec2::new(1000.0, 1000.0),
            heights: vec![10.0; 100 * 100],
        }
    }
    
    #[test]
    fn test_raise_terrain() {
        let mut heightmap = setup_test_heightmap();
        let mut modified_chunks = ModifiedChunks::default();
        
        let modification = TerrainModification {
            position: Vec2::ZERO,
            radius: 100.0,
            falloff: 2.0,
            modification_type: ModificationType::Raise(20.0),
            strength: 5.0,
        };
        
        let mut commands = Commands::default();
        let entity = commands.spawn(modification).id();
        
        apply_terrain_modifications(
            commands,
            &mut heightmap,
            &mut modified_chunks,
            Query::new().single(entity),
        );
        
        // Check that height was increased
        let center_height = heightmap.get_height(50, 50).unwrap();
        assert!(center_height > 10.0);
        assert!(center_height <= 20.0);
        
        // Check that chunks were marked as modified
        assert!(!modified_chunks.chunks.is_empty());
    }
    
    #[test]
    fn test_smooth_terrain() {
        let mut heightmap = setup_test_heightmap();
        let mut heights = vec![10.0; 100 * 100];
        // Create a spike
        heights[5050] = 20.0; // Center point
        heightmap.heights = heights;
        
        let mut modified_chunks = ModifiedChunks::default();
        
        let modification = TerrainModification {
            position: Vec2::ZERO,
            radius: 50.0,
            falloff: 1.0,
            modification_type: ModificationType::Smooth,
            strength: 1.0,
        };
        
        let mut commands = Commands::default();
        let entity = commands.spawn(modification).id();
        
        apply_terrain_modifications(
            commands,
            &mut heightmap,
            &mut modified_chunks,
            Query::new().single(entity),
        );
        
        // Check that spike was smoothed
        let center_height = heightmap.get_height(50, 50).unwrap();
        assert!(center_height < 20.0);
        assert!(center_height > 10.0);
    }
} 