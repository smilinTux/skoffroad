use bevy::prelude::*;
use bevy::render::mesh::Mesh;
use bevy::diagnostic::FrameTimeDiagnosticsPlugin;
use bevy::diagnostic::Diagnostics;

use crate::game::plugins::particle_system::{
    ParticleMaterial,
    ParticleSystem,
    ParticleSystemPlugin,
    spawn_basic_particle_effect,
    BasicParticleEffect,
    BasicParticleConfig,
};
use crate::game::plugins::particle_system::examples::advanced_features::Trail;

/// Plugin that sets up the basic particle example scene and systems
pub struct BasicParticleExamplePlugin;

impl Plugin for BasicParticleExamplePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
                ParticleSystemPlugin,
                FrameTimeDiagnosticsPlugin::default(),
            ))
            .insert_resource(ParticleDebugConfig {
                show_bounds: false,
                show_performance: false,
                show_emission_points: false,
                show_particle_paths: false,
            })
            .insert_resource(EffectConfig::default())
            .add_systems(Startup, setup_example)
            .add_systems(Update, (
                update_emitter_position,
                handle_input,
                update_debug_ui.run_if(resource_exists::<ParticleDebugConfig>()),
                update_debug_gizmos,
                handle_explosions,
                update_weather_effects,
            ));
    }
}

#[derive(Resource)]
struct ParticleDebugConfig {
    show_bounds: bool,
    show_performance: bool,
    show_emission_points: bool,
    show_particle_paths: bool,
}

#[derive(Resource)]
struct EffectConfig {
    emission_rate: f32,
    lifetime: f32,
    size: f32,
    velocity_randomness: f32,
    emission_radius: f32,
}

impl Default for EffectConfig {
    fn default() -> Self {
        Self {
            emission_rate: 50.0,
            lifetime: 2.0,
            size: 0.2,
            velocity_randomness: 0.5,
            emission_radius: 0.5,
        }
    }
}

/// Component to track the type of particle effect for movement updates
#[derive(Component)]
enum ParticleEffectType {
    Fire,
    Smoke,
    Magic,
}

#[derive(Component)]
struct Emitter {
    time: f32,
    active: bool,
    effect_type: BasicParticleEffect,
    explosion_time: Option<f32>,
}

#[derive(Component)]
struct DebugText;

/// Sets up the example scene with camera, lighting and ground plane
fn setup_example(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ParticleMaterial>>,
    asset_server: Res<AssetServer>,
) {
    // Setup camera
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(0.0, 5.0, 15.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });

    // Add light
    commands.spawn(PointLightBundle {
        point_light: PointLight {
            intensity: 1500.0,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(4.0, 8.0, 4.0),
        ..default()
    });

    // Setup debug UI
    commands.spawn((
        TextBundle::from_sections([
            TextSection::new(
                "Particle System Debug\n",
                TextStyle {
                    font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                    font_size: 20.0,
                    color: Color::WHITE,
                },
            ),
            TextSection::new(
                "",
                TextStyle {
                    font: asset_server.load("fonts/FiraMono-Medium.ttf"),
                    font_size: 16.0,
                    color: Color::CYAN,
                },
            ),
        ])
        .with_style(Style {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        }),
        DebugText,
    ));

    // Create particle emitter
    let particle_material = materials.add(ParticleMaterial::new_fire());
    
    commands.spawn((
        ParticleSystem {
            emission_rate: 50.0,
            lifetime: 2.0,
            initial_velocity: Vec3::new(0.0, 2.0, 0.0),
            velocity_randomness: 0.5,
            size: Vec2::splat(0.2),
            size_curve: Vec2::new(0.0, 1.0),
            color_over_lifetime: true,
            ..default()
        },
        Transform::from_xyz(0.0, 0.5, 0.0),
        particle_material,
        Emitter {
            time: 0.0,
            active: true,
            effect_type: BasicParticleEffect::Fire,
            explosion_time: None,
        },
    ));

    // Spawn initial effects
    spawn_effect(&mut commands, BasicParticleEffect::Fire, Vec3::new(-4.0, 0.5, 0.0));
    spawn_effect(&mut commands, BasicParticleEffect::Smoke, Vec3::new(-2.0, 0.5, 0.0));
    spawn_effect(&mut commands, BasicParticleEffect::Magic, Vec3::new(0.0, 0.5, 0.0));
    spawn_effect(&mut commands, BasicParticleEffect::Water, Vec3::new(2.0, 0.5, 0.0));
    spawn_effect(&mut commands, BasicParticleEffect::Heal, Vec3::new(4.0, 0.5, 0.0));
}

fn spawn_effect(commands: &mut Commands, effect_type: BasicParticleEffect, position: Vec3) -> Entity {
    let config = BasicParticleConfig {
        effect_type,
        transform: Transform::from_translation(position),
        ..Default::default()
    };

    let entity = spawn_basic_particle_effect(commands, config.clone());
    
    commands.entity(entity).insert(Emitter {
        time: 0.0,
        active: true,
        effect_type,
        explosion_time: None,
    });

    // Add trail for certain effects
    if matches!(effect_type, BasicParticleEffect::Magic | BasicParticleEffect::Fire) {
        commands.spawn((
            Trail {
                width: 0.2,
                fade_time: 1.0,
                point_distance: 0.1,
                max_points: 50,
            },
            Transform::from_translation(position),
        ));
    }

    entity
}

fn spawn_explosion(commands: &mut Commands, position: Vec3) -> Entity {
    let mut config = BasicParticleConfig {
        effect_type: BasicParticleEffect::Fire,
        transform: Transform::from_translation(position),
        ..Default::default()
    };
    
    config.emission_rate = 200.0;
    config.lifetime = 0.5;
    config.velocity_randomness = 1.0;
    config.initial_velocity = Vec3::new(0.0, 2.0, 0.0);
    config.size = 0.4;

    let entity = spawn_basic_particle_effect(commands, config);
    
    commands.entity(entity).insert(Emitter {
        time: 0.0,
        active: true,
        effect_type: BasicParticleEffect::Fire,
        explosion_time: Some(0.0),
    });

    entity
}

fn spawn_weather_system(commands: &mut Commands, effect_type: BasicParticleEffect, bounds: Vec3) {
    for x in (-bounds.x as i32..=bounds.x as i32).step_by(4) {
        for z in (-bounds.z as i32..=bounds.z as i32).step_by(4) {
            let position = Vec3::new(x as f32, bounds.y, z as f32);
            let config = BasicParticleConfig {
                effect_type,
                transform: Transform::from_translation(position),
                ..Default::default()
            };
            
            let entity = spawn_basic_particle_effect(commands, config);
            commands.entity(entity).insert(Emitter {
                time: 0.0,
                active: true,
                effect_type,
                explosion_time: None,
            });
        }
    }
}

fn handle_input(
    mut commands: Commands,
    keyboard: Res<Input<KeyCode>>,
    mut debug_config: ResMut<ParticleDebugConfig>,
    mut effect_config: ResMut<EffectConfig>,
) {
    // Toggle effects
    if keyboard.just_pressed(KeyCode::Space) {
        for mut emitter in commands.query::<&mut Emitter>().iter_mut() {
            emitter.active = !emitter.active;
        }
    }

    // Spawn new effects
    if keyboard.just_pressed(KeyCode::Key1) {
        spawn_effect(&mut commands, BasicParticleEffect::Fire, Vec3::new(0.0, 0.5, 0.0));
    }
    if keyboard.just_pressed(KeyCode::Key2) {
        spawn_effect(&mut commands, BasicParticleEffect::Smoke, Vec3::new(0.0, 0.5, 0.0));
    }
    if keyboard.just_pressed(KeyCode::Key3) {
        spawn_effect(&mut commands, BasicParticleEffect::Magic, Vec3::new(0.0, 0.5, 0.0));
    }
    if keyboard.just_pressed(KeyCode::Key4) {
        spawn_effect(&mut commands, BasicParticleEffect::Water, Vec3::new(0.0, 0.5, 0.0));
    }
    if keyboard.just_pressed(KeyCode::Key5) {
        spawn_effect(&mut commands, BasicParticleEffect::Heal, Vec3::new(0.0, 0.5, 0.0));
    }

    // Advanced effects
    if keyboard.just_pressed(KeyCode::E) {
        spawn_explosion(&mut commands, Vec3::new(0.0, 0.5, 0.0));
    }
    if keyboard.just_pressed(KeyCode::R) {
        spawn_weather_system(&mut commands, BasicParticleEffect::Water, Vec3::new(10.0, 10.0, 10.0));
    }
    if keyboard.just_pressed(KeyCode::T) {
        spawn_weather_system(&mut commands, BasicParticleEffect::Smoke, Vec3::new(10.0, 10.0, 10.0));
    }

    // Debug controls
    if keyboard.just_pressed(KeyCode::B) {
        debug_config.show_bounds = !debug_config.show_bounds;
    }
    if keyboard.just_pressed(KeyCode::P) {
        debug_config.show_performance = !debug_config.show_performance;
    }
    if keyboard.just_pressed(KeyCode::M) {
        debug_config.show_emission_points = !debug_config.show_emission_points;
    }
    if keyboard.just_pressed(KeyCode::L) {
        debug_config.show_particle_paths = !debug_config.show_particle_paths;
    }

    // Real-time configuration controls
    if keyboard.pressed(KeyCode::Up) {
        effect_config.emission_rate *= 1.1;
    }
    if keyboard.pressed(KeyCode::Down) {
        effect_config.emission_rate *= 0.9;
    }
    if keyboard.pressed(KeyCode::Left) {
        effect_config.size *= 0.9;
    }
    if keyboard.pressed(KeyCode::Right) {
        effect_config.size *= 1.1;
    }
}

fn update_emitter_position(
    time: Res<Time>,
    mut query: Query<(&mut Transform, &mut Emitter)>,
) {
    for (mut transform, mut emitter) in query.iter_mut() {
        if !emitter.active {
            continue;
        }

        emitter.time += time.delta_seconds();
        
        // Different movement patterns for different effects
        match emitter.effect_type {
            BasicParticleEffect::Fire => {
                let radius = 3.0;
                transform.translation = Vec3::new(
                    radius * (emitter.time * 0.8).cos(),
                    0.5 + (emitter.time * 0.5).sin() * 0.3,
                    radius * (emitter.time * 0.8).sin(),
                );
            }
            BasicParticleEffect::Magic => {
                let radius = 2.5;
                transform.translation = Vec3::new(
                    radius * (emitter.time * 1.2).cos(),
                    1.0 + (emitter.time * 0.8).sin() * 0.5,
                    radius * (emitter.time * 1.2).sin(),
                );
            }
            BasicParticleEffect::Water => {
                transform.translation.y = 0.5 + (emitter.time * 0.6).sin() * 0.3;
                transform.translation.x = 2.0 * (emitter.time * 0.4).sin();
            }
            _ => {
                let radius = 2.0;
                transform.translation = Vec3::new(
                    radius * (emitter.time * 0.6).cos(),
                    0.5,
                    radius * (emitter.time * 0.6).sin(),
                );
            }
        }
    }
}

fn handle_explosions(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut Emitter)>,
) {
    for (entity, mut emitter) in query.iter_mut() {
        if let Some(ref mut explosion_time) = emitter.explosion_time {
            *explosion_time += time.delta_seconds();
            
            if *explosion_time > 0.5 {
                commands.entity(entity).despawn_recursive();
            }
        }
    }
}

fn update_weather_effects(
    time: Res<Time>,
    mut query: Query<(&mut Transform, &Emitter), With<ParticleSystem>>,
) {
    for (mut transform, emitter) in query.iter_mut() {
        if !emitter.active {
            continue;
        }

        match emitter.effect_type {
            BasicParticleEffect::Water => {
                // Rain effect: particles fall straight down
                transform.translation.y -= time.delta_seconds() * 5.0;
                if transform.translation.y < 0.0 {
                    transform.translation.y = 10.0;
                }
            }
            BasicParticleEffect::Smoke => {
                // Fog/cloud effect: slow horizontal drift
                transform.translation.x += time.delta_seconds() * 0.5;
                if transform.translation.x > 10.0 {
                    transform.translation.x = -10.0;
                }
            }
            _ => {}
        }
    }
}

fn update_debug_gizmos(
    mut gizmos: Gizmos,
    debug_config: Res<ParticleDebugConfig>,
    query: Query<(&Transform, &ParticleSystem)>,
) {
    if debug_config.show_emission_points {
        for (transform, _) in query.iter() {
            gizmos.circle(transform.translation, transform.up(), 0.1, Color::YELLOW);
        }
    }

    if debug_config.show_particle_paths {
        for (transform, system) in query.iter() {
            let direction = transform.forward() * system.initial_velocity.length();
            gizmos.ray(transform.translation, direction, Color::BLUE);
        }
    }
}

fn update_debug_ui(
    diagnostics: Res<bevy::diagnostic::DiagnosticsStore>,
    debug_config: Res<ParticleDebugConfig>,
    effect_config: Res<EffectConfig>,
    mut query: Query<&mut Text, With<DebugText>>,
    particle_query: Query<(&Transform, &ParticleSystem)>,
) {
    if let Ok(mut text) = query.get_single_mut() {
        let mut info = String::new();

        // Performance stats
        if debug_config.show_performance {
            if let Some(fps) = diagnostics.get(FrameTimeDiagnosticsPlugin::FPS) {
                if let Some(value) = fps.smoothed() {
                    info.push_str(&format!("FPS: {:.1}\n", value));
                }
            }

            let particle_count = particle_query.iter().count();
            info.push_str(&format!("Active Effects: {}\n", particle_count));
            
            let total_particles: usize = particle_query
                .iter()
                .map(|(_, system)| system.params.max_particles)
                .sum();
            info.push_str(&format!("Total Particles: {}\n", total_particles));
        }

        // Effect configuration
        info.push_str(&format!("\nEffect Settings:\n"));
        info.push_str(&format!("Emission Rate: {:.1}\n", effect_config.emission_rate));
        info.push_str(&format!("Size: {:.2}\n", effect_config.size));

        // Controls help
        info.push_str("\nBasic Controls:\n");
        info.push_str("Space - Toggle Effects\n");
        info.push_str("1-5 - Spawn Basic Effects\n");
        
        info.push_str("\nAdvanced Effects:\n");
        info.push_str("E - Spawn Explosion\n");
        info.push_str("R - Toggle Rain\n");
        info.push_str("T - Toggle Fog\n");

        info.push_str("\nDebug Controls:\n");
        info.push_str("B - Toggle Bounds\n");
        info.push_str("P - Toggle Performance\n");
        info.push_str("M - Toggle Emission Points\n");
        info.push_str("L - Toggle Particle Paths\n");

        info.push_str("\nConfig Controls:\n");
        info.push_str("↑/↓ - Adjust Emission Rate\n");
        info.push_str("←/→ - Adjust Particle Size\n");

        text.sections[1].value = info;
    }
} 