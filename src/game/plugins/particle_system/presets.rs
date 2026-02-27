use bevy::prelude::*;
use crate::game::plugins::particle_system::{
    material::ParticleMaterial,
};

/// Configuration options for particle presets
#[derive(Clone)]
pub struct PresetConfig {
    pub scale: f32,               // Scale multiplier for size and dimensions
    pub intensity: f32,           // Multiplier for emission rates and particle counts
    pub speed: f32,              // Multiplier for velocities
    pub lifetime: f32,           // Multiplier for particle lifetime
    pub gravity: Vec3,           // Override gravity direction and strength
    pub emission_strength: f32,   // Override emission strength
}

impl Default for PresetConfig {
    fn default() -> Self {
        Self {
            scale: 1.0,
            intensity: 1.0,
            speed: 1.0,
            lifetime: 1.0,
            gravity: Vec3::new(0.0, -9.81, 0.0),
            emission_strength: 1.0,
        }
    }
}

pub struct ParticlePresets;

impl ParticlePresets {
    // ... existing presets ...

    /// Create an explosion effect
    pub fn explosion(commands: &mut Commands, transform: Transform, config: Option<PresetConfig>) -> Entity {
        let config = config.unwrap_or_default();
        let mut params = SimulationParams::default();
        params.colors = ParticleColors {
            albedo: ParticleColors::fire(),
            emission: ParticleColors::fire(),
            emission_strength: 4.0 * config.emission_strength,
            ease_function: EaseFunction::QuadOut,
        };
        params.lifetime = 1.0 * config.lifetime;
        params.spawn_rate = 200.0 * config.intensity;
        params.initial_velocity = Vec3::ZERO * config.speed;
        params.velocity_randomness = 5.0;
        params.size_begin = 0.3 * config.scale;
        params.size_end = 0.0 * config.scale;
        params.gravity = config.gravity;

        commands.spawn((
            ParticleSystem::new(params),
            Emitter::new(EmitterConfig {
                shape: EmitterShape::Sphere { radius: 0.1 * config.scale },
                ..default()
            }),
            transform,
        )).id()
    }

    /// Create a snow effect
    pub fn snow(commands: &mut Commands, transform: Transform, config: Option<PresetConfig>) -> Entity {
        let config = config.unwrap_or_default();
        let mut params = SimulationParams::default();
        params.colors = ParticleColors {
            albedo: ParticleColors::smoke(),
            emission: ParticleColors::smoke(),
            emission_strength: 0.2 * config.emission_strength,
            ease_function: EaseFunction::SmoothStep,
        };
        params.lifetime = 8.0 * config.lifetime;
        params.spawn_rate = 15.0 * config.intensity;
        params.initial_velocity = Vec3::new(0.0, -0.5, 0.0) * config.speed;
        params.velocity_randomness = 0.2;
        params.size_begin = 0.1 * config.scale;
        params.size_end = 0.1 * config.scale;
        params.gravity = config.gravity;

        commands.spawn((
            ParticleSystem::new(params),
            Emitter::new(EmitterConfig {
                shape: EmitterShape::Box { 
                    size: Vec3::new(10.0, 0.1, 10.0) * config.scale 
                },
                ..default()
            }),
            transform,
        )).id()
    }

    /// Create a healing effect
    pub fn heal(commands: &mut Commands, transform: Transform, config: Option<PresetConfig>) -> Entity {
        let config = config.unwrap_or_default();
        let mut params = SimulationParams::default();
        params.colors = ParticleColors {
            albedo: ParticleColors::nature(),
            emission: ParticleColors::nature(),
            emission_strength: 2.0 * config.emission_strength,
            ease_function: EaseFunction::Sine,
        };
        params.lifetime = 1.5 * config.lifetime;
        params.spawn_rate = 30.0 * config.intensity;
        params.initial_velocity = Vec3::new(0.0, 1.0, 0.0) * config.speed;
        params.velocity_randomness = 0.3;
        params.size_begin = 0.2 * config.scale;
        params.size_end = 0.0 * config.scale;
        params.gravity = config.gravity;

        commands.spawn((
            ParticleSystem::new(params),
            Emitter::new(EmitterConfig {
                shape: EmitterShape::Sphere { 
                    radius: 0.5 * config.scale 
                },
                ..default()
            }),
            transform,
        )).id()
    }

    /// Create a dust trail effect
    pub fn dust_trail(commands: &mut Commands, transform: Transform, config: Option<PresetConfig>) -> Entity {
        let config = config.unwrap_or_default();
        let mut params = SimulationParams::default();
        params.colors = ParticleColors {
            albedo: ParticleColors::fire(),
            emission: ParticleColors::smoke(),
            emission_strength: 0.0,
            ease_function: EaseFunction::QuadOut,
        };
        params.lifetime = 2.0 * config.lifetime;
        params.spawn_rate = 25.0 * config.intensity;
        params.initial_velocity = Vec3::new(0.0, 0.2, 0.0) * config.speed;
        params.velocity_randomness = 0.1;
        params.size_begin = 0.1 * config.scale;
        params.size_end = 0.3 * config.scale;
        params.gravity = config.gravity;

        commands.spawn((
            ParticleSystem::new(params),
            Emitter::new(EmitterConfig {
                shape: EmitterShape::Box { 
                    size: Vec3::new(0.3, 0.1, 0.3) * config.scale 
                },
                ..default()
            }),
            transform,
        )).id()
    }

    /// Create a lightning strike effect
    pub fn lightning_strike(commands: &mut Commands, transform: Transform, config: Option<PresetConfig>) -> Entity {
        let config = config.unwrap_or_default();
        let mut params = SimulationParams::default();
        params.colors = ParticleColors {
            albedo: GradientPreset::Lightning.create_gradient(),
            emission: GradientPreset::Lightning.create_gradient(),
            emission_strength: 4.0 * config.emission_strength,
            ease_function: EaseFunction::QuadIn,
        };
        params.lifetime = 0.3 * config.lifetime;
        params.spawn_rate = 300.0 * config.intensity;
        params.initial_velocity = Vec3::new(0.0, -20.0, 0.0) * config.speed;
        params.velocity_randomness = 2.0;
        params.size_begin = 0.1 * config.scale;
        params.size_end = 0.05 * config.scale;
        params.gravity = config.gravity;

        commands.spawn((
            ParticleSystem::new(params),
            Emitter::new(EmitterConfig {
                shape: EmitterShape::Line {
                    start: Vec3::ZERO,
                    end: Vec3::new(0.0, 5.0, 0.0) * config.scale,
                },
                ..default()
            }),
            transform,
        )).id()
    }

    /// Create a rainbow trail effect
    pub fn rainbow_trail(commands: &mut Commands, transform: Transform, config: Option<PresetConfig>) -> Entity {
        let config = config.unwrap_or_default();
        let mut params = SimulationParams::default();
        params.colors = ParticleColors {
            albedo: GradientPreset::Rainbow.create_gradient(),
            emission: GradientPreset::Rainbow.create_gradient(),
            emission_strength: 1.5 * config.emission_strength,
            ease_function: EaseFunction::from_points(vec![
                ControlPoint::new(0.0, 0.0),
                ControlPoint::new(0.2, 0.8),
                ControlPoint::new(0.8, 0.2),
                ControlPoint::new(1.0, 0.0),
            ]),
        };
        params.lifetime = 2.0 * config.lifetime;
        params.spawn_rate = 50.0 * config.intensity;
        params.initial_velocity = Vec3::new(0.0, 0.5, 0.0) * config.speed;
        params.velocity_randomness = 0.2;
        params.size_begin = 0.2 * config.scale;
        params.size_end = 0.1 * config.scale;
        params.gravity = config.gravity;

        commands.spawn((
            ParticleSystem::new(params),
            Emitter::new(EmitterConfig {
                shape: EmitterShape::Circle { radius: 0.5 * config.scale },
                ..default()
            }),
            transform,
        )).id()
    }

    /// Create an acid splash effect
    pub fn acid_splash(commands: &mut Commands, transform: Transform, config: Option<PresetConfig>) -> Entity {
        let config = config.unwrap_or_default();
        let mut params = SimulationParams::default();
        params.colors = ParticleColors {
            albedo: GradientPreset::Acid.create_gradient(),
            emission: GradientPreset::Acid.create_gradient(),
            emission_strength: 2.0 * config.emission_strength,
            ease_function: EaseFunction::QuadOut,
        };
        params.lifetime = 1.5 * config.lifetime;
        params.spawn_rate = 100.0 * config.intensity;
        params.initial_velocity = Vec3::new(0.0, 3.0, 0.0) * config.speed;
        params.velocity_randomness = 1.0;
        params.size_begin = 0.15 * config.scale;
        params.size_end = 0.05 * config.scale;
        params.gravity = config.gravity;

        commands.spawn((
            ParticleSystem::new(params),
            Emitter::new(EmitterConfig {
                shape: EmitterShape::Cone {
                    angle: std::f32::consts::PI / 4.0,
                    radius: 0.1 * config.scale,
                },
                ..default()
            }),
            transform,
        )).id()
    }

    /// Create an energy pulse effect
    pub fn energy_pulse(commands: &mut Commands, transform: Transform, config: Option<PresetConfig>) -> Entity {
        let config = config.unwrap_or_default();
        let mut params = SimulationParams::default();
        params.colors = ParticleColors {
            albedo: GradientPreset::Energy.create_gradient(),
            emission: GradientPreset::Energy.create_gradient(),
            emission_strength: 3.0 * config.emission_strength,
            ease_function: EaseFunction::Elastic,
        };
        params.lifetime = 1.0 * config.lifetime;
        params.spawn_rate = 80.0 * config.intensity;
        params.initial_velocity = Vec3::new(0.0, 0.0, 0.0) * config.speed;
        params.velocity_randomness = 2.0;
        params.size_begin = 0.3 * config.scale;
        params.size_end = 0.0 * config.scale;
        params.gravity = config.gravity;

        commands.spawn((
            ParticleSystem::new(params),
            Emitter::new(EmitterConfig {
                shape: EmitterShape::Sphere { radius: 1.0 * config.scale },
                ..default()
            }),
            transform,
        )).id()
    }

    /// Create a dark void effect
    pub fn dark_void(commands: &mut Commands, transform: Transform, config: Option<PresetConfig>) -> Entity {
        let config = config.unwrap_or_default();
        let mut params = SimulationParams::default();
        params.colors = ParticleColors {
            albedo: GradientPreset::Dark.create_gradient(),
            emission: GradientPreset::Dark.create_gradient(),
            emission_strength: 1.0 * config.emission_strength,
            ease_function: EaseFunction::QuadIn,
        };
        params.lifetime = 3.0 * config.lifetime;
        params.spawn_rate = 40.0 * config.intensity;
        params.initial_velocity = Vec3::new(0.0, -0.5, 0.0) * config.speed;
        params.velocity_randomness = 0.5;
        params.size_begin = 0.4 * config.scale;
        params.size_end = 0.2 * config.scale;
        params.gravity = config.gravity;

        commands.spawn((
            ParticleSystem::new(params),
            Emitter::new(EmitterConfig {
                shape: EmitterShape::Torus {
                    radius: 2.0 * config.scale,
                    ring_radius: 0.2 * config.scale,
                },
                ..default()
            }),
            transform,
        )).id()
    }

    /// Create a fire effect
    pub fn fire(commands: &mut Commands, transform: Transform, config: Option<PresetConfig>) -> Entity {
        let config = config.unwrap_or_default();
        let mut params = SimulationParams::default();
        params.colors = ParticleColors {
            albedo: ParticleColors::fire(),
            emission: ParticleColors::fire(),
            emission_strength: 4.0 * config.emission_strength,
            ease_function: EaseFunction::QuadOut,
        };
        params.lifetime = 1.0 * config.lifetime;
        params.spawn_rate = 40.0 * config.intensity;
        params.initial_velocity = Vec3::new(0.0, 2.0, 0.0) * config.speed;
        params.velocity_randomness = 0.3;
        params.size_begin = 0.3 * config.scale;
        params.size_end = 0.0 * config.scale;
        params.gravity = config.gravity;

        commands.spawn((
            ParticleSystem::new(params),
            Emitter::new(EmitterConfig {
                shape: EmitterShape::Sphere { 
                    radius: 0.2 * config.scale 
                },
                ..default()
            }),
            transform,
        )).id()
    }

    /// Create a smoke effect
    pub fn smoke(commands: &mut Commands, transform: Transform, config: Option<PresetConfig>) -> Entity {
        let config = config.unwrap_or_default();
        let mut params = SimulationParams::default();
        params.colors = ParticleColors {
            albedo: ParticleColors::smoke(),
            emission: ParticleColors::smoke(),
            emission_strength: 0.0,
            ease_function: EaseFunction::QuadOut,
        };
        params.lifetime = 3.0 * config.lifetime;
        params.spawn_rate = 20.0 * config.intensity;
        params.initial_velocity = Vec3::new(0.0, 1.0, 0.0) * config.speed;
        params.velocity_randomness = 0.2;
        params.size_begin = 0.2 * config.scale;
        params.size_end = 0.8 * config.scale;
        params.gravity = config.gravity;

        commands.spawn((
            ParticleSystem::new(params),
            Emitter::new(EmitterConfig {
                shape: EmitterShape::Sphere { 
                    radius: 0.3 * config.scale 
                },
                ..default()
            }),
            transform,
        )).id()
    }

    /// Create a sparkle effect
    pub fn sparkle(commands: &mut Commands, transform: Transform, config: Option<PresetConfig>) -> Entity {
        let config = config.unwrap_or_default();
        let mut params = SimulationParams::default();
        params.colors = ParticleColors {
            albedo: ParticleColors::sparkle(),
            emission: ParticleColors::sparkle(),
            emission_strength: 5.0 * config.emission_strength,
            ease_function: EaseFunction::Elastic,
        };
        params.lifetime = 0.8 * config.lifetime;
        params.spawn_rate = 50.0 * config.intensity;
        params.initial_velocity = Vec3::new(0.0, 0.5, 0.0) * config.speed;
        params.velocity_randomness = 0.8;
        params.size_begin = 0.05 * config.scale;
        params.size_end = 0.0 * config.scale;
        params.gravity = config.gravity;

        commands.spawn((
            ParticleSystem::new(params),
            Emitter::new(EmitterConfig {
                shape: EmitterShape::Sphere { 
                    radius: 0.5 * config.scale 
                },
                ..default()
            }),
            transform,
        )).id()
    }

    pub fn water(commands: &mut Commands, transform: Transform, config: Option<PresetConfig>) {
        let config = config.unwrap_or_default();
        let mut material = ParticleMaterial::default();
        material.base_color = Color::rgba(0.6, 0.8, 1.0, 0.6);
        material.emission = Color::rgba(0.3, 0.5, 1.0, 0.3);
        material.emission_strength = config.emission_strength;
        material.alpha_mode = AlphaMode::Blend;

        let mut emitter = ParticleEmitter::new(
            transform,
            EmitterShape::Box {
                size: Vec3::new(10.0, 0.1, 10.0),
            },
            material,
        );
        emitter.spawn_rate = 100.0 * config.intensity;
        emitter.lifetime = config.lifetime;
        emitter.initial_velocity = Vec3::new(0.0, -config.speed * 10.0, 0.0);
        emitter.velocity_randomness = 0.2;
        emitter.size = config.scale;
        emitter.size_randomness = 0.2;
        emitter.gravity = config.gravity;

        commands.spawn(emitter);
    }

    pub fn fog(commands: &mut Commands, transform: Transform, config: Option<PresetConfig>) {
        let config = config.unwrap_or_default();
        let mut material = ParticleMaterial::default();
        material.base_color = Color::rgba(0.8, 0.8, 0.8, 0.2);
        material.emission = Color::rgba(0.9, 0.9, 0.9, 0.1);
        material.emission_strength = config.emission_strength;
        material.alpha_mode = AlphaMode::Blend;

        let mut emitter = ParticleEmitter::new(
            transform,
            EmitterShape::Sphere {
                radius: 5.0,
            },
            material,
        );
        emitter.spawn_rate = 20.0 * config.intensity;
        emitter.lifetime = config.lifetime;
        emitter.initial_velocity = Vec3::ZERO;
        emitter.velocity_randomness = 0.1;
        emitter.size = config.scale;
        emitter.size_randomness = 0.4;
        emitter.gravity = config.gravity;
        emitter.angular_velocity = Vec3::new(0.0, 0.2, 0.0);
        emitter.angular_velocity_randomness = 0.5;

        commands.spawn(emitter);
    }
}

// Update the example system to showcase new effects
pub fn spawn_example_effects(
    mut commands: Commands,
    mut materials: ResMut<Assets<ParticleMaterial>>,
    asset_server: Res<AssetServer>,
) {
    let texture = asset_server.load("textures/particle.png");

    // First row - original effects with custom configs
    ParticlePresets::explosion(
        &mut commands,
        Transform::from_xyz(-6.0, 0.0, -3.0),
        Some(PresetConfig {
            scale: 1.2,
            intensity: 1.5,
            speed: 1.3,
            emission_strength: 2.0,
            ..Default::default()
        }),
    );

    // ... other original effects ...

    // Second row - new effects
    ParticlePresets::lightning_strike(
        &mut commands,
        Transform::from_xyz(-6.0, 0.0, 3.0),
        None,
    );

    ParticlePresets::rainbow_trail(
        &mut commands,
        Transform::from_xyz(-3.0, 0.0, 3.0),
        Some(PresetConfig {
            scale: 1.5,
            intensity: 1.2,
            ..Default::default()
        }),
    );

    ParticlePresets::acid_splash(
        &mut commands,
        Transform::from_xyz(0.0, 0.0, 3.0),
        None,
    );

    ParticlePresets::energy_pulse(
        &mut commands,
        Transform::from_xyz(3.0, 0.0, 3.0),
        Some(PresetConfig {
            scale: 2.0,
            emission_strength: 2.0,
            ..Default::default()
        }),
    );

    ParticlePresets::dark_void(
        &mut commands,
        Transform::from_xyz(6.0, 0.0, 3.0),
        None,
    );
}

#[derive(Component, Default, Clone)]
pub struct SimulationParams {
    pub colors: ParticleColors,
    pub lifetime: f32,
    pub spawn_rate: f32,
    pub initial_velocity: Vec3,
    pub velocity_randomness: f32,
    pub size_begin: f32,
    pub size_end: f32,
    pub gravity: Vec3,
}

impl SimulationParams {
    pub fn default() -> Self {
        Self {
            colors: ParticleColors::fire(),
            lifetime: 1.0,
            spawn_rate: 1.0,
            initial_velocity: Vec3::ZERO,
            velocity_randomness: 0.0,
            size_begin: 1.0,
            size_end: 1.0,
            gravity: Vec3::new(0.0, -9.81, 0.0),
        }
    }
}

#[derive(Clone)]
pub struct ParticleColors {
    pub albedo: Color,
    pub emission: Color,
    pub emission_strength: f32,
    pub ease_function: EaseFunction,
}

impl ParticleColors {
    pub fn fire() -> Color { Color::ORANGE }
    pub fn smoke() -> Color { Color::GRAY }
    pub fn nature() -> Color { Color::GREEN }
    pub fn sparkle() -> Color { Color::WHITE }
}

#[derive(Clone)]
pub enum EaseFunction {
    QuadOut,
    QuadIn,
    Elastic,
    Sine,
    SmoothStep,
}

impl EaseFunction {
    pub fn from_points(_points: Vec<ControlPoint>) -> Self { EaseFunction::SmoothStep }
}

pub struct ParticleSystem;
impl ParticleSystem {
    pub fn new(_params: SimulationParams) -> Self { ParticleSystem }
}

pub struct Emitter;
impl Emitter {
    pub fn new(_config: EmitterConfig) -> Self { Emitter }
}

#[derive(Default, Clone)]
pub struct EmitterConfig {
    pub shape: EmitterShape,
}

#[derive(Clone)]
pub enum EmitterShape {
    Sphere { radius: f32 },
    Box { size: Vec3 },
    Line { start: Vec3, end: Vec3 },
    Circle { radius: f32 },
    Cone { angle: f32, radius: f32 },
    Torus { radius: f32, ring_radius: f32 },
}

impl Default for EmitterShape {
    fn default() -> Self {
        EmitterShape::Sphere { radius: 1.0 }
    }
}

pub struct GradientPreset;
impl GradientPreset {
    pub fn Lightning() -> Self { GradientPreset }
    pub fn Rainbow() -> Self { GradientPreset }
    pub fn Acid() -> Self { GradientPreset }
    pub fn Energy() -> Self { GradientPreset }
    pub fn Dark() -> Self { GradientPreset }
    pub fn create_gradient(&self) -> Color { Color::WHITE }
}

pub struct ControlPoint;
impl ControlPoint {
    pub fn new(_x: f32, _y: f32) -> Self { ControlPoint }
}

pub struct ParticleEmitter;
impl ParticleEmitter {
    pub fn new(_transform: Transform, _shape: EmitterShape, _material: ParticleMaterial) -> Self { ParticleEmitter }
}