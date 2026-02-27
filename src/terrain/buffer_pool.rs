use std::collections::HashMap;
use bevy::prelude::*;
use bevy::render::mesh::Mesh;

/// Key for identifying reusable mesh buffers
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]
pub struct MeshKey {
    /// LOD level of the mesh
    pub lod_level: u32,
    /// Whether the mesh includes skirts
    pub has_skirts: bool,
    /// Vertex count for validation
    pub vertex_count: u32,
}

/// Pool for reusing mesh buffers
#[derive(Resource, Default)]
pub struct TerrainBufferPool {
    /// Available mesh buffers by configuration
    available: HashMap<MeshKey, Vec<Mesh>>,
    /// Currently in-use mesh buffers
    in_use: HashMap<Entity, MeshKey>,
    /// Statistics for monitoring
    stats: BufferPoolStats,
}

/// Statistics for buffer pool monitoring
#[derive(Debug, Default, Clone)]
pub struct BufferPoolStats {
    pub total_buffers: usize,
    pub available_buffers: usize,
    pub reuse_count: usize,
    pub allocation_count: usize,
}

impl TerrainBufferPool {
    /// Request a mesh buffer for the given configuration
    pub fn request_buffer(&mut self, key: MeshKey) -> Option<Mesh> {
        if let Some(buffers) = self.available.get_mut(&key) {
            if let Some(mesh) = buffers.pop() {
                self.stats.reuse_count += 1;
                self.stats.available_buffers -= 1;
                return Some(mesh);
            }
        }
        None
    }

    /// Return a mesh buffer to the pool
    pub fn return_buffer(&mut self, entity: Entity, mesh: Mesh) {
        if let Some(key) = self.in_use.remove(&entity) {
            self.available.entry(key).or_default().push(mesh);
            self.stats.available_buffers += 1;
        }
    }

    /// Register a newly allocated mesh buffer
    pub fn register_buffer(&mut self, entity: Entity, key: MeshKey, mesh: Mesh) {
        self.in_use.insert(entity, key);
        self.stats.total_buffers += 1;
        self.stats.allocation_count += 1;
    }

    /// Get current pool statistics
    pub fn get_stats(&self) -> BufferPoolStats {
        self.stats.clone()
    }

    /// Clean up unused buffers that exceed a threshold
    pub fn cleanup(&mut self, max_unused_per_key: usize) {
        for buffers in self.available.values_mut() {
            if buffers.len() > max_unused_per_key {
                buffers.truncate(max_unused_per_key);
                self.stats.available_buffers -= buffers.len() - max_unused_per_key;
                self.stats.total_buffers -= buffers.len() - max_unused_per_key;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buffer_pool_reuse() {
        let mut pool = TerrainBufferPool::default();
        let key = MeshKey {
            lod_level: 1,
            has_skirts: true,
            vertex_count: 100,
        };
        let entity = Entity::from_raw(1);
        let mesh = Mesh::new(bevy::render::mesh::PrimitiveTopology::TriangleList);

        // Register new buffer
        pool.register_buffer(entity, key, mesh.clone());
        assert_eq!(pool.get_stats().total_buffers, 1);

        // Return buffer to pool
        pool.return_buffer(entity, mesh.clone());
        assert_eq!(pool.get_stats().available_buffers, 1);

        // Request buffer
        let reused = pool.request_buffer(key);
        assert!(reused.is_some());
        assert_eq!(pool.get_stats().reuse_count, 1);
    }

    #[test]
    fn test_buffer_pool_cleanup() {
        let mut pool = TerrainBufferPool::default();
        let key = MeshKey {
            lod_level: 1,
            has_skirts: true,
            vertex_count: 100,
        };

        // Add multiple buffers
        for i in 0..5 {
            let entity = Entity::from_raw(i);
            let mesh = Mesh::new(bevy::render::mesh::PrimitiveTopology::TriangleList);
            pool.register_buffer(entity, key, mesh.clone());
            pool.return_buffer(entity, mesh);
        }

        assert_eq!(pool.get_stats().available_buffers, 5);

        // Cleanup excess buffers
        pool.cleanup(3);
        assert_eq!(pool.get_stats().available_buffers, 3);
    }
} 