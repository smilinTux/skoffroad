use bevy::prelude::*;

#[derive(Component, Debug, Clone)]
pub enum PhysicsTerrainType {
    Asphalt,
    Dirt,
    Mud,
    Sand,
    Rock,
    Grass,
    Snow,
    Ice,
    Mountain,
    Urban,
    Tunnel,
    Desert,
    Water,
    Gravel,
}

impl Default for PhysicsTerrainType {
    fn default() -> Self {
        PhysicsTerrainType::Dirt
    }
}

#[derive(Component, Debug, Clone)]
pub struct TerrainProperties {
    pub terrain_type: PhysicsTerrainType,
    pub friction_coefficient: f32,
    pub rolling_resistance: f32,
    pub deformation_resistance: f32,
    pub surface_roughness: f32,
}

impl Default for TerrainProperties {
    fn default() -> Self {
        Self {
            terrain_type: PhysicsTerrainType::default(),
            friction_coefficient: 0.7, // Default medium friction
            rolling_resistance: 0.02,  // Default rolling resistance
            deformation_resistance: 1.0, // No deformation by default
            surface_roughness: 0.1,    // Slightly rough surface
        }
    }
}

impl TerrainProperties {
    pub fn new(terrain_type: PhysicsTerrainType) -> Self {
        match terrain_type {
            PhysicsTerrainType::Asphalt => Self {
                terrain_type,
                friction_coefficient: 0.9,
                rolling_resistance: 0.01,
                deformation_resistance: 2.0,
                surface_roughness: 0.05,
            },
            PhysicsTerrainType::Dirt => Self {
                terrain_type,
                friction_coefficient: 0.6,
                rolling_resistance: 0.03,
                deformation_resistance: 0.8,
                surface_roughness: 0.2,
            },
            PhysicsTerrainType::Mud => Self {
                terrain_type,
                friction_coefficient: 0.3,
                rolling_resistance: 0.08,
                deformation_resistance: 0.2,
                surface_roughness: 0.4,
            },
            PhysicsTerrainType::Sand => Self {
                terrain_type,
                friction_coefficient: 0.4,
                rolling_resistance: 0.06,
                deformation_resistance: 0.3,
                surface_roughness: 0.3,
            },
            PhysicsTerrainType::Rock => Self {
                terrain_type,
                friction_coefficient: 0.8,
                rolling_resistance: 0.04,
                deformation_resistance: 1.8,
                surface_roughness: 0.6,
            },
            PhysicsTerrainType::Grass => Self {
                terrain_type,
                friction_coefficient: 0.5,
                rolling_resistance: 0.04,
                deformation_resistance: 0.6,
                surface_roughness: 0.15,
            },
            PhysicsTerrainType::Snow => Self {
                terrain_type,
                friction_coefficient: 0.2,
                rolling_resistance: 0.05,
                deformation_resistance: 0.4,
                surface_roughness: 0.25,
            },
            PhysicsTerrainType::Ice => Self {
                terrain_type,
                friction_coefficient: 0.1,
                rolling_resistance: 0.01,
                deformation_resistance: 1.5,
                surface_roughness: 0.02,
            },
            PhysicsTerrainType::Mountain => Self {
                terrain_type,
                friction_coefficient: 0.7,
                rolling_resistance: 0.05,
                deformation_resistance: 1.5,
                surface_roughness: 0.5,
            },
            PhysicsTerrainType::Urban => Self {
                terrain_type,
                friction_coefficient: 0.8,
                rolling_resistance: 0.03,
                deformation_resistance: 1.2,
                surface_roughness: 0.1,
            },
            PhysicsTerrainType::Tunnel => Self {
                terrain_type,
                friction_coefficient: 0.6,
                rolling_resistance: 0.04,
                deformation_resistance: 1.0,
                surface_roughness: 0.05,
            },
            PhysicsTerrainType::Desert => Self {
                terrain_type,
                friction_coefficient: 0.3,
                rolling_resistance: 0.07,
                deformation_resistance: 0.3,
                surface_roughness: 0.35,
            },
            PhysicsTerrainType::Water => Self {
                terrain_type,
                friction_coefficient: 0.05,
                rolling_resistance: 0.2,
                deformation_resistance: 0.0,
                surface_roughness: 0.0,
            },
            PhysicsTerrainType::Gravel => Self {
                terrain_type,
                friction_coefficient: 0.5,
                rolling_resistance: 0.05,
                deformation_resistance: 0.7,
                surface_roughness: 0.4,
            },
        }
    }

    pub fn get_effective_friction(&self, velocity: f32, normal_force: f32) -> f32 {
        // Adjust friction based on velocity and normal force
        let velocity_factor = 1.0 / (1.0 + velocity * 0.01);
        let load_factor = (normal_force / 1000.0).min(1.0);
        
        self.friction_coefficient * velocity_factor * load_factor
    }

    pub fn get_rolling_resistance(&self, velocity: f32) -> f32 {
        // Increase rolling resistance with velocity
        self.rolling_resistance * (1.0 + velocity * 0.005)
    }
} 