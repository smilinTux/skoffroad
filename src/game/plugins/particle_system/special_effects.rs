use bevy::prelude::*;
use crate::game::plugins::particle_system::presets::PresetConfig;
use crate::game::plugins::particle_system::presets::ParticlePresets;

/// Collection of specialized particle effect combinations
pub struct SpecialEffects;

impl SpecialEffects {
    /// Creates a portal effect with swirling particles and energy pulses
    pub fn magic_portal(commands: &mut Commands, transform: Transform) -> Vec<Entity> {
        let mut entities = Vec::new();
        
        // Outer swirl
        entities.push(ParticlePresets::dark_void(
            commands,
            transform,
            Some(PresetConfig {
                scale: 1.5,
                intensity: 0.8,
                speed: 0.5,
                emission_strength: 1.2,
                gravity: Vec3::ZERO,
                ..Default::default()
            }),
        ));
        
        // Inner energy pulse
        entities.push(ParticlePresets::energy_pulse(
            commands,
            transform,
            Some(PresetConfig {
                scale: 0.8,
                intensity: 1.2,
                speed: 0.3,
                emission_strength: 2.0,
                gravity: Vec3::ZERO,
                ..Default::default()
            }),
        ));
        
        entities
    }

    /// Creates a thunderstorm effect with lightning strikes and rain
    pub fn thunderstorm(commands: &mut Commands, transform: Transform) -> Vec<Entity> {
        let mut entities = Vec::new();
        
        // Rain effect
        entities.push(ParticlePresets::water(
            commands,
            transform,
            Some(PresetConfig {
                scale: 0.05,
                intensity: 2.0,
                speed: 2.0,
                lifetime: 1.5,
                gravity: Vec3::new(0.0, -15.0, 0.0),
                emission_strength: 0.5,
            }),
        ));
        
        // Lightning strikes at random intervals
        entities.push(ParticlePresets::lightning_strike(
            commands,
            transform,
            Some(PresetConfig {
                scale: 2.0,
                intensity: 0.3,
                speed: 1.5,
                lifetime: 0.2,
                gravity: Vec3::ZERO,
                emission_strength: 3.0,
            }),
        ));
        
        entities
    }

    /// Creates a toxic waste effect with bubbling acid and smoke
    pub fn toxic_waste(commands: &mut Commands, transform: Transform) -> Vec<Entity> {
        let mut entities = Vec::new();
        
        // Bubbling acid base
        entities.push(ParticlePresets::acid_splash(
            commands,
            transform,
            Some(PresetConfig {
                scale: 1.0,
                intensity: 0.5,
                speed: 0.3,
                lifetime: 2.0,
                gravity: Vec3::new(0.0, 0.5, 0.0),
                emission_strength: 1.5,
            }),
        ));
        
        // Toxic smoke
        entities.push(ParticlePresets::smoke(
            commands,
            transform.with_translation(transform.translation + Vec3::new(0.0, 0.5, 0.0)),
            Some(PresetConfig {
                scale: 1.2,
                intensity: 0.3,
                speed: 0.2,
                lifetime: 3.0,
                gravity: Vec3::new(0.0, 0.2, 0.0),
                emission_strength: 0.3,
            }),
        ));
        
        entities
    }

    /// Creates a rainbow fountain effect
    pub fn rainbow_fountain(commands: &mut Commands, transform: Transform) -> Vec<Entity> {
        let mut entities = Vec::new();
        
        // Main fountain spray
        entities.push(ParticlePresets::rainbow_trail(
            commands,
            transform,
            Some(PresetConfig {
                scale: 0.8,
                intensity: 2.0,
                speed: 2.0,
                lifetime: 1.5,
                gravity: Vec3::new(0.0, -9.81, 0.0),
                emission_strength: 1.2,
            }),
        ));
        
        // Mist at the base
        entities.push(ParticlePresets::water(
            commands,
            transform,
            Some(PresetConfig {
                scale: 1.5,
                intensity: 0.5,
                speed: 0.2,
                lifetime: 2.0,
                gravity: Vec3::new(0.0, 0.1, 0.0),
                emission_strength: 0.5,
            }),
        ));
        
        entities
    }
}

/// System to showcase the special effects
pub fn spawn_special_effects_demo(mut commands: Commands) {
    // Magic portal
    SpecialEffects::magic_portal(
        &mut commands,
        Transform::from_xyz(-6.0, 1.0, 0.0),
    );
    
    // Thunderstorm
    SpecialEffects::thunderstorm(
        &mut commands,
        Transform::from_xyz(-2.0, 5.0, 0.0),
    );
    
    // Toxic waste
    SpecialEffects::toxic_waste(
        &mut commands,
        Transform::from_xyz(2.0, 0.0, 0.0),
    );
    
    // Rainbow fountain
    SpecialEffects::rainbow_fountain(
        &mut commands,
        Transform::from_xyz(6.0, 0.0, 0.0),
    );
} 