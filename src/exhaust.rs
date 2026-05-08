// Exhaust trail: mesh-based smoke puffs spawned from the rear of the chassis.
// Each puff is a small semi-transparent sphere that grows and fades over ~1.5 s.
// Independent from particles.rs (no Hanabi).

use std::collections::VecDeque;
use bevy::prelude::*;
use avian3d::prelude::LinearVelocity;

use crate::vehicle::{Chassis, DriveInput, VehicleRoot};

// ---- Constants ----

const SPAWN_RATE: f32 = 12.0; // puffs per second at full throttle
const MAX_PUFFS:  usize = 60;
const LIFETIME:   f32 = 1.5;  // seconds
const PUFF_RADIUS: f32 = 0.15;

// Exhaust pipe position in chassis local space: rear centre, just above the axle.
const EXHAUST_LOCAL_BACK: f32 = 1.9; // +Z = chassis back
const EXHAUST_LOCAL_Y:    f32 = 0.0;

// ---- Components / Resources ----

#[derive(Component)]
struct PuffParticle {
    velocity:   Vec3,
    lifetime_s: f32,
}

#[derive(Resource, Default)]
pub struct ExhaustQueue(VecDeque<Entity>);

#[derive(Resource)]
struct ExhaustAssets {
    mesh:     Handle<Mesh>,
    material: Handle<StandardMaterial>,
}

// ---- Plugin ----

pub struct ExhaustPlugin;

impl Plugin for ExhaustPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ExhaustQueue>()
           .add_systems(Startup, init_exhaust_assets)
           .add_systems(Update, (
               spawn_puffs.run_if(resource_exists::<VehicleRoot>),
               update_puffs,
           ));
    }
}

// ---- Startup: pre-allocate shared mesh + material ----

fn init_exhaust_assets(
    mut commands:  Commands,
    mut meshes:    ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mesh = meshes.add(Sphere::new(PUFF_RADIUS).mesh().ico(1).unwrap());
    let material = materials.add(StandardMaterial {
        base_color: Color::srgba(0.18, 0.18, 0.20, 0.7),
        alpha_mode: AlphaMode::Blend,
        ..default()
    });
    commands.insert_resource(ExhaustAssets { mesh, material });
}

// ---- Spawn puffs (Update) ----

fn spawn_puffs(
    mut commands:   Commands,
    assets:         Option<Res<ExhaustAssets>>,
    vehicle:        Res<VehicleRoot>,
    chassis_q:      Query<(&Transform, &LinearVelocity), With<Chassis>>,
    input:          Res<DriveInput>,
    mut queue:      ResMut<ExhaustQueue>,
    time:           Res<Time>,
    mut accumulator: Local<f32>,
) {
    let Some(assets) = assets else { return };
    let Ok((transform, lin_vel)) = chassis_q.get(vehicle.chassis) else { return };

    // Only emit when throttle is applied and the vehicle is moving.
    let throttle = input.drive.abs();
    if throttle <= 0.1 {
        *accumulator = 0.0;
        return;
    }
    let speed = lin_vel.0.length();
    if speed < 0.5 {
        return;
    }

    let dt = time.delta_secs();
    let effective_rate = SPAWN_RATE * throttle;
    *accumulator += dt;

    let interval = 1.0 / effective_rate;
    while *accumulator >= interval {
        *accumulator -= interval;

        // Exhaust position: chassis world-space rear.
        let back = *transform.back(); // Dir3 → Vec3; chassis back = +Z local
        let spawn_pos = transform.translation
            + back * EXHAUST_LOCAL_BACK
            + Vec3::Y * EXHAUST_LOCAL_Y;

        // Small randomised velocity: backward + upward jitter.
        // Use a deterministic cheap hash based on transform position to avoid
        // pulling in a rand dependency.
        let jitter = pseudo_jitter(spawn_pos, *accumulator);
        let velocity = back * 0.4 + Vec3::Y * 0.5 + jitter * 0.3;

        let entity = commands.spawn((
            PuffParticle { velocity, lifetime_s: LIFETIME },
            Mesh3d(assets.mesh.clone()),
            MeshMaterial3d(assets.material.clone()),
            Transform::from_translation(spawn_pos),
        )).id();

        queue.0.push_back(entity);

        // Enforce cap: despawn oldest puff if over the limit.
        if queue.0.len() > MAX_PUFFS {
            if let Some(oldest) = queue.0.pop_front() {
                commands.entity(oldest).despawn();
            }
        }
    }
}

// ---- Update puffs (Update) ----

fn update_puffs(
    mut commands: Commands,
    mut queue:    ResMut<ExhaustQueue>,
    mut puffs:    Query<(Entity, &mut Transform, &mut PuffParticle)>,
    time:         Res<Time>,
) {
    let dt = time.delta_secs();
    let mut to_remove: Vec<Entity> = Vec::new();

    for (entity, mut transform, mut puff) in puffs.iter_mut() {
        // Integrate.
        transform.translation += puff.velocity * dt;
        // Rising air.
        puff.velocity.y += 0.3 * dt;
        // Decay lifetime.
        puff.lifetime_s -= dt;

        if puff.lifetime_s <= 0.0 {
            to_remove.push(entity);
            commands.entity(entity).despawn();
        } else {
            // Grow as it ages: starts at scale 1.0, ends at ~1.75 at expiry.
            let scale = 1.0 + (LIFETIME - puff.lifetime_s) * 0.5;
            transform.scale = Vec3::splat(scale);
        }
    }

    // Purge despawned entities from the queue.
    if !to_remove.is_empty() {
        queue.0.retain(|e| !to_remove.contains(e));
    }
}

// ---- Helpers ----

/// Cheap deterministic jitter — avoids pulling in the `rand` crate.
/// Returns a roughly unit-sphere-distributed Vec3 using sin/cos of the inputs.
fn pseudo_jitter(pos: Vec3, t: f32) -> Vec3 {
    let seed = pos.x * 1.3 + pos.y * 7.7 + pos.z * 3.1 + t * 97.3;
    let a = seed.sin() * 43758.545;
    let b = (seed * 1.618).sin() * 43758.545;
    let c = (seed * 2.718).sin() * 43758.545;
    // Fractional parts in [0, 1], mapped to [-1, 1].
    let x = (a - a.floor()) * 2.0 - 1.0;
    let y = (b - b.floor()) * 2.0 - 1.0;
    let z = (c - c.floor()) * 2.0 - 1.0;
    Vec3::new(x, y, z)
}
