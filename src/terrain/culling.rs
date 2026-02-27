use bevy::prelude::*;
use bevy::render::primitives::Frustum;
use bevy::render::primitives::HalfSpace;

/// Represents a terrain chunk's bounding box for culling
#[derive(Debug, Clone)]
pub struct ChunkBounds {
    /// Center point of the chunk
    pub center: Vec3,
    /// Half-extents of the chunk
    pub extents: Vec3,
}

impl ChunkBounds {
    /// Create new chunk bounds from min/max points
    pub fn from_min_max(min: Vec3, max: Vec3) -> Self {
        let center = (min + max) * 0.5;
        let extents = (max - min) * 0.5;
        Self { center, extents }
    }

    /// Check if the chunk is visible in the given frustum
    pub fn is_visible(&self, frustum: &Frustum) -> bool {
        // Test against each frustum plane
        for plane in &frustum.half_spaces {
            let r = self.extents.x.abs() * plane.normal().x.abs() +
                    self.extents.y.abs() * plane.normal().y.abs() +
                    self.extents.z.abs() * plane.normal().z.abs();
            
            let d = plane.normal().dot(self.center.into()) + plane.d();
            
            if d + r < 0.0 {
                return false;
            }
        }
        true
    }

    /// Update bounds based on new height range
    pub fn update_height_range(&mut self, min_height: f32, max_height: f32) {
        self.center.y = (min_height + max_height) * 0.5;
        self.extents.y = (max_height - min_height) * 0.5;
    }
}

/// System for culling terrain chunks
#[derive(Resource)]
pub struct TerrainCullingSystem {
    /// Cached chunk bounds
    chunk_bounds: Vec<(Entity, ChunkBounds)>,
    /// Statistics for monitoring
    stats: CullingStats,
}

/// Statistics for culling system monitoring
#[derive(Debug, Default, Clone)]
pub struct CullingStats {
    pub total_chunks: usize,
    pub visible_chunks: usize,
    pub culled_chunks: usize,
}

impl Default for TerrainCullingSystem {
    fn default() -> Self {
        Self {
            chunk_bounds: Vec::new(),
            stats: CullingStats::default(),
        }
    }
}

impl TerrainCullingSystem {
    /// Register a chunk for culling
    pub fn register_chunk(&mut self, entity: Entity, bounds: ChunkBounds) {
        self.chunk_bounds.push((entity, bounds));
        self.stats.total_chunks += 1;
    }

    /// Remove a chunk from culling
    pub fn remove_chunk(&mut self, entity: Entity) {
        if let Some(index) = self.chunk_bounds.iter().position(|(e, _)| *e == entity) {
            self.chunk_bounds.swap_remove(index);
            self.stats.total_chunks -= 1;
        }
    }

    /// Update chunk bounds
    pub fn update_chunk_bounds(&mut self, entity: Entity, bounds: ChunkBounds) {
        if let Some(index) = self.chunk_bounds.iter().position(|(e, _)| *e == entity) {
            self.chunk_bounds[index].1 = bounds;
        }
    }

    /// Get current culling statistics
    pub fn get_stats(&self) -> CullingStats {
        self.stats.clone()
    }

    /// Perform frustum culling and return visible chunks
    pub fn cull_chunks(&mut self, frustum: &Frustum) -> Vec<Entity> {
        let mut visible = Vec::new();
        self.stats.visible_chunks = 0;
        self.stats.culled_chunks = 0;

        for (entity, bounds) in &self.chunk_bounds {
            if bounds.is_visible(frustum) {
                visible.push(*entity);
                self.stats.visible_chunks += 1;
            } else {
                self.stats.culled_chunks += 1;
            }
        }

        visible
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::render::primitives::HalfSpace;

    #[test]
    fn test_chunk_bounds_visibility() {
        let bounds = ChunkBounds {
            center: Vec3::new(0.0, 0.0, 0.0),
            extents: Vec3::new(1.0, 1.0, 1.0),
        };

        // Create a simple frustum with one half-space (plane)
        let frustum = Frustum {
            half_spaces: [
                HalfSpace::new(Vec4::new(0.0, 0.0, 1.0, -5.0)),
                HalfSpace::new(Vec4::new(0.0, 0.0, -1.0, -5.0)),
                HalfSpace::new(Vec4::new(1.0, 0.0, 0.0, -5.0)),
                HalfSpace::new(Vec4::new(-1.0, 0.0, 0.0, -5.0)),
                HalfSpace::new(Vec4::new(0.0, 1.0, 0.0, -5.0)),
                HalfSpace::new(Vec4::new(0.0, -1.0, 0.0, -5.0)),
            ],
        };

        assert!(bounds.is_visible(&frustum));

        // Test chunk outside frustum
        let outside_bounds = ChunkBounds {
            center: Vec3::new(0.0, 0.0, 10.0),
            extents: Vec3::new(1.0, 1.0, 1.0),
        };
        assert!(!outside_bounds.is_visible(&frustum));
    }

    #[test]
    fn test_culling_system() {
        let mut system = TerrainCullingSystem::default();
        let entity = Entity::from_raw(1);
        let bounds = ChunkBounds {
            center: Vec3::new(0.0, 0.0, 0.0),
            extents: Vec3::new(1.0, 1.0, 1.0),
        };

        // Register chunk
        system.register_chunk(entity, bounds.clone());
        assert_eq!(system.get_stats().total_chunks, 1);

        // Create test frustum
        let frustum = Frustum {
            half_spaces: [
                HalfSpace::new(Vec4::new(0.0, 0.0, 1.0, -5.0)),
                HalfSpace::new(Vec4::new(0.0, 0.0, -1.0, -5.0)),
                HalfSpace::new(Vec4::new(1.0, 0.0, 0.0, -5.0)),
                HalfSpace::new(Vec4::new(-1.0, 0.0, 0.0, -5.0)),
                HalfSpace::new(Vec4::new(0.0, 1.0, 0.0, -5.0)),
                HalfSpace::new(Vec4::new(0.0, -1.0, 0.0, -5.0)),
            ],
        };

        // Test culling
        let visible = system.cull_chunks(&frustum);
        assert_eq!(visible.len(), 1);
        assert_eq!(system.get_stats().visible_chunks, 1);
        assert_eq!(system.get_stats().culled_chunks, 0);

        // Remove chunk
        system.remove_chunk(entity);
        assert_eq!(system.get_stats().total_chunks, 0);
    }
} 