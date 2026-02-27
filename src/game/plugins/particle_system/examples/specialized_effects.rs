use bevy::{
    prelude::*,
    diagnostic::FrameTimeDiagnosticsPlugin,
};

use crate::game::plugins::particle_system::{
    ParticleSystem, ParticleSystemPlugin,
    // Trail, TrailPlugin,
    BasicParticleEffect, BasicParticleConfig,
    spawn_basic_particle_effect,
};

pub struct SpecializedEffectsExamplePlugin;

impl Plugin for SpecializedEffectsExamplePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
                ParticleSystemPlugin,
                // TrailPlugin,
                FrameTimeDiagnosticsPlugin::default(),
            ))
            .insert_resource(SpecialEffectConfig::default())
            .add_systems(Startup, setup_specialized_example)
            .add_systems(Update, update_lightning_effect)
            .add_systems(Update, update_vortex_effect)
            .add_systems(Update, update_energy_field_effect)
            .add_systems(Update, handle_specialized_input)
            .add_systems(Update, update_specialized_debug);
    }
}

#[derive(Resource)]
struct SpecialEffectConfig {
    lightning_frequency: f32,
    vortex_pull_strength: f32,
    energy_field_intensity: f32,
    debug_mode: DebugMode,
}

#[derive(Default)]
enum DebugMode {
    #[default]
    None,
    Forces,
    Paths,
    EmissionPoints,
    All,
}

impl Default for SpecialEffectConfig {
    fn default() -> Self {
        Self {
            lightning_frequency: 1.0,
            vortex_pull_strength: 2.0,
            energy_field_intensity: 1.0,
            debug_mode: DebugMode::None,
        }
    }
}

#[derive(Component)]
enum SpecializedEffect {
    Lightning {
        start_pos: Vec3,
        end_pos: Vec3,
        branches: u32,
        time: f32,
    },
    Vortex {
        radius: f32,
        height: f32,
        rotation: f32,
    },
    EnergyField {
        radius: f32,
        frequency: f32,
        phase: f32,
    },
}

fn setup_specialized_example(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    // Camera setup
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(-12.0, 10.0, 12.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });

    // Lighting
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            illuminance: 15000.0,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_rotation(Quat::from_euler(
            EulerRot::XYZ,
            -std::f32::consts::FRAC_PI_4,
            std::f32::consts::FRAC_PI_4,
            0.0,
        )),
        ..default()
    });

    // Spawn initial effects
    spawn_lightning(&mut commands, Vec3::new(-5.0, 5.0, 0.0), Vec3::new(-5.0, 0.0, 0.0));
    spawn_vortex(&mut commands, Vec3::new(0.0, 0.0, 0.0));
    spawn_energy_field(&mut commands, Vec3::new(5.0, 0.0, 0.0));

    // Debug UI
    commands.spawn(TextBundle::from_sections([
        TextSection::new(
            "Specialized Effects\n",
            TextStyle {
                font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                font_size: 24.0,
                color: Color::WHITE,
            },
        ),
        TextSection::new(
            "\nControls:\n1 - Lightning\n2 - Vortex\n3 - Energy Field\n\nDebug:\nD - Cycle Debug Modes\nF1-F3 - Adjust Parameters\n",
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
    }));
}

fn spawn_lightning(commands: &mut Commands, start_pos: Vec3, end_pos: Vec3) {
    let config = BasicParticleConfig {
        effect_type: BasicParticleEffect::Magic,
        transform: Transform::from_translation(start_pos),
        ..Default::default()
    };

    let entity = spawn_basic_particle_effect(commands, config);
    
    commands.entity(entity).insert(SpecializedEffect::Lightning {
        start_pos,
        end_pos,
        branches: 3,
        time: 0.0,
    });

    // Add trail for lightning path
    commands.spawn((
        // Trail {
        //     width: 0.1,
        //     fade_time: 0.2,
        //     point_distance: 0.05,
        //     max_points: 20,
        // },
        Transform::from_translation(start_pos),
    ));
}

fn spawn_vortex(commands: &mut Commands, position: Vec3) {
    let config = BasicParticleConfig {
        effect_type: BasicParticleEffect::Magic,
        transform: Transform::from_translation(position),
        ..Default::default()
    };

    let entity = spawn_basic_particle_effect(commands, config);
    
    commands.entity(entity).insert(SpecializedEffect::Vortex {
        radius: 3.0,
        height: 5.0,
        rotation: 0.0,
    });
}

fn spawn_energy_field(commands: &mut Commands, position: Vec3) {
    let config = BasicParticleConfig {
        effect_type: BasicParticleEffect::Magic,
        transform: Transform::from_translation(position),
        ..Default::default()
    };

    let entity = spawn_basic_particle_effect(commands, config);
    
    commands.entity(entity).insert(SpecializedEffect::EnergyField {
        radius: 2.0,
        frequency: 1.0,
        phase: 0.0,
    });
}

fn update_lightning_effect(
    time: Res<Time>,
    config: Res<SpecialEffectConfig>,
    mut gizmos: Gizmos,
    mut query: Query<(&mut Transform, &mut SpecializedEffect, &mut ParticleSystem)>,
) {
    for (mut transform, mut effect, mut system) in query.iter_mut() {
        if let SpecializedEffect::Lightning { time: ref mut effect_time, start_pos, end_pos, branches } = *effect {
            *effect_time += time.delta_seconds() * config.lightning_frequency;
            
            // Generate lightning branches
            for i in 0..branches {
                let t = (*effect_time + i as f32 * 0.3) % 1.0;
                let offset = Vec3::new(
                    (t * 10.0).sin() * 0.5,
                    0.0,
                    (t * 8.0).cos() * 0.5
                );
                
                let pos = start_pos.lerp(end_pos, t) + offset;
                // TODO: Implement correct particle emission here. No spawn_particle method on ParticleSystem.
                // system.spawn_particle(pos, (end_pos - start_pos).normalize() * 2.0, Color::rgba(0.5, 0.8, 1.0, 0.8));
            }

            // Debug visualization
            if matches!(config.debug_mode, DebugMode::Paths | DebugMode::All) {
                gizmos.line(start_pos, end_pos, Color::YELLOW);
            }
        }
    }
}

fn update_vortex_effect(
    time: Res<Time>,
    config: Res<SpecialEffectConfig>,
    mut gizmos: Gizmos,
    mut query: Query<(&mut Transform, &mut SpecializedEffect, &mut ParticleSystem)>,
) {
    for (mut transform, mut effect, mut system) in query.iter_mut() {
        if let SpecializedEffect::Vortex { ref mut rotation, radius, height } = *effect {
            *rotation += time.delta_seconds() * config.vortex_pull_strength;
            
            // Spiral particle emission
            let points = 12;
            for i in 0..points {
                let angle = i as f32 * std::f32::consts::TAU / points as f32 + *rotation;
                let height_offset = ((*rotation * 2.0).sin() + 1.0) * height * 0.5;
                let pos = transform.translation + Vec3::new(
                    angle.cos() * radius,
                    height_offset,
                    angle.sin() * radius,
                );
                
                let pull_center = transform.translation + Vec3::new(0.0, height_offset, 0.0);
                let pull_dir = (pull_center - pos).normalize();
                
                // TODO: Implement correct particle emission here. No spawn_particle method on ParticleSystem.
                // system.spawn_particle(pos, pull_dir * config.vortex_pull_strength, Color::rgba(0.7, 0.2, 1.0, 0.6));
            }

            // Debug visualization
            if matches!(config.debug_mode, DebugMode::Forces | DebugMode::All) {
                gizmos.circle(transform.translation, Vec3::Y, radius, Color::GREEN);
                gizmos.ray(transform.translation, Vec3::Y * height, Color::BLUE);
            }
        }
    }
}

fn update_energy_field_effect(
    time: Res<Time>,
    config: Res<SpecialEffectConfig>,
    mut gizmos: Gizmos,
    mut query: Query<(&mut Transform, &mut SpecializedEffect, &mut ParticleSystem)>,
) {
    for (mut transform, mut effect, mut system) in query.iter_mut() {
        if let SpecializedEffect::EnergyField { ref mut phase, radius, frequency } = *effect {
            *phase += time.delta_seconds() * frequency * config.energy_field_intensity;
            
            // Emit particles in a spherical pattern
            let points = 20;
            for i in 0..points {
                let theta = i as f32 * std::f32::consts::TAU / points as f32;
                let phi = (*phase * 2.0).sin() * std::f32::consts::PI;
                
                let pos = transform.translation + Vec3::new(
                    radius * theta.sin() * phi.cos(),
                    radius * phi.sin(),
                    radius * theta.cos() * phi.cos(),
                );
                
                let dir = (pos - transform.translation).normalize();
                // TODO: Implement correct particle emission here. No spawn_particle method on ParticleSystem.
                // system.spawn_particle(pos, dir * config.energy_field_intensity, Color::rgba(0.2, 0.9, 0.4, 0.7));
            }

            // Debug visualization
            if matches!(config.debug_mode, DebugMode::EmissionPoints | DebugMode::All) {
                gizmos.sphere(transform.translation, transform.rotation, radius, Color::CYAN);
            }
        }
    }
}

fn handle_specialized_input(
    mut commands: Commands,
    keyboard: Res<Input<KeyCode>>,
    mut config: ResMut<SpecialEffectConfig>,
) {
    // Spawn effects
    if keyboard.just_pressed(KeyCode::Key1) {
        spawn_lightning(&mut commands, Vec3::new(-5.0, 5.0, 0.0), Vec3::new(-5.0, 0.0, 0.0));
    }
    if keyboard.just_pressed(KeyCode::Key2) {
        spawn_vortex(&mut commands, Vec3::new(0.0, 0.0, 0.0));
    }
    if keyboard.just_pressed(KeyCode::Key3) {
        spawn_energy_field(&mut commands, Vec3::new(5.0, 0.0, 0.0));
    }

    // Debug mode cycling
    if keyboard.just_pressed(KeyCode::D) {
        config.debug_mode = match config.debug_mode {
            DebugMode::None => DebugMode::Forces,
            DebugMode::Forces => DebugMode::Paths,
            DebugMode::Paths => DebugMode::EmissionPoints,
            DebugMode::EmissionPoints => DebugMode::All,
            DebugMode::All => DebugMode::None,
        };
    }

    // Parameter adjustments
    if keyboard.just_pressed(KeyCode::F1) {
        config.lightning_frequency = (config.lightning_frequency + 0.5).min(3.0);
    }
    if keyboard.just_pressed(KeyCode::F2) {
        config.vortex_pull_strength = (config.vortex_pull_strength + 0.5).min(5.0);
    }
    if keyboard.just_pressed(KeyCode::F3) {
        config.energy_field_intensity = (config.energy_field_intensity + 0.5).min(3.0);
    }
}

fn update_specialized_debug(
    config: Res<SpecialEffectConfig>,
    mut gizmos: Gizmos,
    query: Query<(&Transform, &SpecializedEffect)>,
) {
    if matches!(config.debug_mode, DebugMode::All) {
        for (transform, effect) in query.iter() {
            match effect {
                SpecializedEffect::Lightning { .. } => {
                    gizmos.circle(transform.translation, Vec3::Y, 0.2, Color::RED);
                }
                SpecializedEffect::Vortex { radius, .. } => {
                    gizmos.circle(transform.translation, Vec3::Y, *radius, Color::GREEN);
                }
                SpecializedEffect::EnergyField { radius, .. } => {
                    gizmos.sphere(transform.translation, transform.rotation, *radius, Color::BLUE);
                }
            }
        }
    }
} 