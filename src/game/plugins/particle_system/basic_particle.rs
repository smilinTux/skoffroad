use bevy::prelude::*;
use crate::game::plugins::particle_system::presets::{PresetConfig, ParticlePresets, Emitter, EmitterConfig, EmitterShape, SimulationParams};
use super::particle::ParticleSystem;

/// Basic particle effect types
#[derive(Component, Debug, Clone, Copy)]
pub enum BasicParticleEffect {
    Fire,
    Smoke,
    Dust,
    Magic,
    Water,
    Heal,
}

/// Configuration for basic particle effects
#[derive(Debug, Clone)]
pub struct BasicParticleConfig {
    /// Effect type
    pub effect_type: BasicParticleEffect,
    /// Initial transform
    pub transform: Transform,
    /// Scale multiplier
    pub scale: f32,
    /// Intensity multiplier
    pub intensity: f32,
    /// Lifetime multiplier
    pub lifetime: f32,
    /// Speed multiplier
    pub speed: f32,
    /// Emission strength multiplier
    pub emission_strength: f32,
    /// Whether to enable soft particles
    pub soft_particles: bool,
    /// Whether to use LOD
    pub use_lod: bool,
}

impl Default for BasicParticleConfig {
    fn default() -> Self {
        Self {
            effect_type: BasicParticleEffect::Fire,
            transform: Transform::default(),
            scale: 1.0,
            intensity: 1.0,
            lifetime: 1.0,
            speed: 1.0,
            emission_strength: 1.0,
            soft_particles: true,
            use_lod: true,
        }
    }
}

/// System to spawn basic particle effects
pub fn spawn_basic_particle_effect(
    commands: &mut Commands,
    config: BasicParticleConfig,
) -> Entity {
    let preset_config = PresetConfig {
        scale: config.scale,
        intensity: config.intensity,
        lifetime: config.lifetime,
        speed: config.speed,
        emission_strength: config.emission_strength,
        ..Default::default()
    };

    let entity = match config.effect_type {
        BasicParticleEffect::Fire => ParticlePresets::fire(commands, config.transform, Some(preset_config)),
        BasicParticleEffect::Smoke => ParticlePresets::smoke(commands, config.transform, Some(preset_config)),
        BasicParticleEffect::Dust => ParticlePresets::dust_trail(commands, config.transform, Some(preset_config)),
        BasicParticleEffect::Magic => {
            let mut params = SimulationParams::default();
            params.colors.emission_strength = 3.0 * config.emission_strength;
            params.lifetime = 1.5 * config.lifetime;
            params.spawn_rate = 35.0 * config.intensity;
            params.size_begin = 0.2 * config.scale;
            params.size_end = 0.0 * config.scale;

            commands.spawn((
                ParticleSystem::new(params),
                Emitter::new(EmitterConfig {
                    shape: EmitterShape::Sphere { radius: 0.3 * config.scale },
                    ..Default::default()
                }),
                config.transform,
            )).id()
        },
        BasicParticleEffect::Water => {
            let mut params = SimulationParams::default();
            params.colors.emission_strength = 0.5 * config.emission_strength;
            params.lifetime = 1.0 * config.lifetime;
            params.spawn_rate = 30.0 * config.intensity;
            params.size_begin = 0.15 * config.scale;
            params.size_end = 0.3 * config.scale;

            commands.spawn((
                ParticleSystem::new(params),
                Emitter::new(EmitterConfig {
                    shape: EmitterShape::Box { 
                        size: Vec3::new(0.4, 0.1, 0.4) * config.scale 
                    },
                    ..Default::default()
                }),
                config.transform,
            )).id()
        },
        BasicParticleEffect::Heal => ParticlePresets::heal(commands, config.transform, Some(preset_config)),
    };

    // Add the BasicParticleEffect component to presets that don't add it themselves
    if matches!(config.effect_type, BasicParticleEffect::Fire | BasicParticleEffect::Smoke | BasicParticleEffect::Dust | BasicParticleEffect::Heal) {
        commands.entity(entity).insert(config.effect_type.clone());
    }

    entity
}

/// System to update basic particle effects
pub fn update_basic_particle_effects(
    mut commands: Commands,
    mut particles: Query<(Entity, &mut ParticleSystem, &BasicParticleEffect)>,
    time: Res<Time>,
) {
    for (entity, mut particle_system, effect) in particles.iter_mut() {
        // Update particle system based on effect type
        match effect.config.effect_type {
            BasicParticleEffect::Fire => {
                particle_system.params.colors.emission_strength = 
                    (1.5 + (time.elapsed_seconds() * 2.0).sin() * 0.5) * effect.config.emission_strength;
            },
            BasicParticleEffect::Magic => {
                particle_system.params.colors.emission_strength = 
                    (2.0 + (time.elapsed_seconds() * 3.0).sin() * 1.0) * effect.config.emission_strength;
            },
            _ => {} // Other effects don't need continuous updates
        }
    }
}

/// Plugin for basic particle effects
pub struct BasicParticlePlugin;

impl Plugin for BasicParticlePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, update_basic_particle_effects);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_particle_config_default() {
        let config = BasicParticleConfig::default();
        assert!(matches!(config.effect_type, BasicParticleEffect::Fire));
        assert_eq!(config.scale, 1.0);
        assert_eq!(config.intensity, 1.0);
        assert_eq!(config.lifetime, 1.0);
        assert_eq!(config.speed, 1.0);
        assert_eq!(config.emission_strength, 1.0);
        assert!(config.soft_particles);
        assert!(config.use_lod);
    }

    #[test]
    fn test_spawn_basic_particle_effect() {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            BasicParticlePlugin,
        ));

        let config = BasicParticleConfig {
            effect_type: BasicParticleEffect::Fire,
            transform: Transform::from_xyz(1.0, 2.0, 3.0),
            scale: 2.0,
            intensity: 1.5,
            ..Default::default()
        };

        app.world.resource_scope(|world, mut commands: Commands| {
            let entity = spawn_basic_particle_effect(&mut commands, config.clone());
            
            // Verify entity was spawned with correct components
            let entity_ref = world.entity(entity);
            assert!(entity_ref.contains::<ParticleSystem>());
            assert!(entity_ref.contains::<Emitter>());
            assert!(entity_ref.contains::<BasicParticleEffect>());
        });
    }

    #[test]
    fn test_update_basic_particle_effects() {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            BasicParticlePlugin,
        ));

        // Spawn a fire effect
        let config = BasicParticleConfig {
            effect_type: BasicParticleEffect::Fire,
            emission_strength: 2.0,
            ..Default::default()
        };

        app.world.resource_scope(|world, mut commands: Commands| {
            spawn_basic_particle_effect(&mut commands, config);
        });

        // Run the update system
        app.update();

        // Verify the emission strength was updated
        let query = app.world.query::<(&ParticleSystem, &BasicParticleEffect)>();
        for (particle_system, _) in query.iter(&app.world) {
            assert!(particle_system.params.colors.emission_strength > 0.0);
        }
    }
} 