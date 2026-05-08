// Autonomous camera-companion drone.
//
// A small NPC drone hovers 15 m above the chassis and traces a lazy circle
// (5 m radius, 8 s period) around it. Press O to toggle visibility and
// whether it keeps following.

use std::f32::consts::TAU;
use bevy::prelude::*;

use crate::vehicle::{Chassis, VehicleRoot};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const DAMPING: f32           = 4.0;
const PROP_SPIN_RAD_PER_SEC: f32 = 30.0;

// ---------------------------------------------------------------------------
// Public resource
// ---------------------------------------------------------------------------

#[derive(Resource)]
pub struct DroneEnabled(pub bool);

impl Default for DroneEnabled {
    fn default() -> Self { Self(true) }
}

// ---------------------------------------------------------------------------
// Components
// ---------------------------------------------------------------------------

#[derive(Component)]
struct Drone;

#[derive(Component)]
struct Propeller;

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct DronePlugin;

impl Plugin for DronePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DroneEnabled>()
           .add_systems(Startup, spawn_drone)
           .add_systems(Update, (
               update_drone.run_if(resource_exists::<crate::vehicle::VehicleRoot>),
               toggle_drone,
               spin_propellers,
           ));
    }
}

// ---------------------------------------------------------------------------
// Spawn
// ---------------------------------------------------------------------------

fn spawn_drone(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Materials
    let body_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.05, 0.05, 0.05),
        perceptual_roughness: 0.7,
        ..default()
    });
    let prop_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.15, 0.15, 0.15),
        perceptual_roughness: 0.8,
        ..default()
    });
    let led_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.05, 0.05),
        emissive: LinearRgba::rgb(8.0, 0.0, 0.0),
        perceptual_roughness: 0.2,
        ..default()
    });

    // Meshes
    let body_mesh = meshes.add(Sphere::new(0.3).mesh().ico(2).unwrap());
    let prop_mesh = meshes.add(Cylinder::new(0.5, 0.05));
    let led_mesh  = meshes.add(Sphere::new(0.08).mesh().ico(1).unwrap());

    // Root drone entity
    let drone = commands.spawn((
        Drone,
        Mesh3d(body_mesh),
        MeshMaterial3d(body_mat),
        Transform::from_translation(Vec3::new(20.0, 30.0, 20.0)),
        Visibility::default(),
    )).id();

    // LED on top
    let led = commands.spawn((
        Mesh3d(led_mesh),
        MeshMaterial3d(led_mat),
        Transform::from_translation(Vec3::new(0.0, 0.32, 0.0)),
    )).id();
    commands.entity(drone).add_child(led);

    // 4 propellers in a + cross around the body
    // Positions: front, back, left, right
    let prop_offsets = [
        Vec3::new( 0.0, 0.0,  0.7),
        Vec3::new( 0.0, 0.0, -0.7),
        Vec3::new(-0.7, 0.0,  0.0),
        Vec3::new( 0.7, 0.0,  0.0),
    ];

    for offset in prop_offsets {
        let prop = commands.spawn((
            Propeller,
            Mesh3d(prop_mesh.clone()),
            MeshMaterial3d(prop_mat.clone()),
            Transform::from_translation(offset),
        )).id();
        commands.entity(drone).add_child(prop);
    }
}

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

fn update_drone(
    enabled: Res<DroneEnabled>,
    time: Res<Time>,
    vehicle: Res<VehicleRoot>,
    chassis_q: Query<&Transform, With<Chassis>>,
    mut drone_q: Query<&mut Transform, (With<Drone>, Without<Chassis>)>,
) {
    if !enabled.0 { return; }

    let Ok(chassis_tf) = chassis_q.get(vehicle.chassis) else { return };
    let Ok(mut drone_tf) = drone_q.single_mut() else { return };

    let chassis_pos = chassis_tf.translation;
    let dt = time.delta_secs();

    // Circular orbit offset around the chassis
    let theta = time.elapsed_secs() * (TAU / 8.0);
    let circle = Vec3::new(theta.cos() * 5.0, 0.0, theta.sin() * 5.0);

    let target = chassis_pos + Vec3::Y * 15.0 + circle;

    // Exponential smooth-follow
    let t = 1.0 - (-DAMPING * dt).exp();
    drone_tf.translation = drone_tf.translation.lerp(target, t);

    // Always face down toward the chassis (filming)
    drone_tf.look_at(chassis_pos, Vec3::Y);
}

fn toggle_drone(
    keys: Res<ButtonInput<KeyCode>>,
    mut enabled: ResMut<DroneEnabled>,
    mut drone_q: Query<&mut Visibility, With<Drone>>,
) {
    if keys.just_pressed(KeyCode::KeyO) {
        enabled.0 = !enabled.0;
        if let Ok(mut vis) = drone_q.single_mut() {
            *vis = if enabled.0 {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
        }
    }
}

fn spin_propellers(
    enabled: Res<DroneEnabled>,
    time: Res<Time>,
    mut prop_q: Query<&mut Transform, With<Propeller>>,
) {
    if !enabled.0 { return; }

    let dt = time.delta_secs();
    let delta_rot = Quat::from_rotation_y(PROP_SPIN_RAD_PER_SEC * dt);

    for mut tf in prop_q.iter_mut() {
        tf.rotation = tf.rotation * delta_rot;
    }
}
