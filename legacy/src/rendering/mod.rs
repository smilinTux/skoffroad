use bevy::prelude::*;
use bevy::render::render_resource::*;

pub struct RenderingPlugin;

impl Plugin for RenderingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_rendering);
        app.add_systems(Update, handle_particle_effects);
    }
}

#[derive(Component)]
pub struct MainCamera {
    pub target: Option<Entity>,
    pub offset: Vec3,
    pub smoothness: f32,
}

#[derive(Component)]
pub struct ParticleEffect {
    pub lifetime: f32,
    pub elapsed: f32,
}

fn setup_rendering(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Add lighting
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(4.0, 8.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });

    // Add skybox as a large cube
    commands.spawn(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Box::new(100.0, 100.0, 100.0))),
        material: materials.add(StandardMaterial {
            base_color: Color::rgb(0.5, 0.7, 1.0),
            unlit: true,
            cull_mode: None,
            ..default()
        }),
        transform: Transform::from_xyz(0.0, 0.0, 0.0),
        ..default()
    });
}

fn update_camera(
    mut camera_query: Query<(&mut Transform, &MainCamera)>,
    target_query: Query<&Transform, Without<MainCamera>>,
    time: Res<Time>,
) {
    for (mut transform, camera) in camera_query.iter_mut() {
        if let Some(target_entity) = camera.target {
            if let Ok(target_transform) = target_query.get(target_entity) {
                let target_position = target_transform.translation + camera.offset;
                transform.translation = transform.translation.lerp(
                    target_position,
                    camera.smoothness * time.delta_seconds(),
                );
                transform.look_at(target_transform.translation, Vec3::Y);
            }
        }
    }
}

fn handle_particle_effects(
    mut commands: Commands,
    time: Res<Time>,
    mut particles: Query<(Entity, &mut ParticleEffect)>,
) {
    for (entity, mut particle) in particles.iter_mut() {
        particle.elapsed += time.delta_seconds();
        if particle.elapsed >= particle.lifetime {
            commands.entity(entity).despawn();
        }
    }
} 