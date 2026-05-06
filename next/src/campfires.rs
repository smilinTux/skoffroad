// Campfires: 3 procedural campfires placed at scenic positions. Each is
// a ring of 6 small dark cylinder "logs" plus a flickering point light
// and rising orange particles for flames.
//
// Public API:
//   CampfiresPlugin

use std::collections::VecDeque;

use bevy::prelude::*;

use crate::terrain::terrain_height_at;

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct CampfiresPlugin;

impl Plugin for CampfiresPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PuffQueue>()
           .add_systems(Startup, spawn_campfires)
           .add_systems(Update, (flicker_lights, spawn_flame_puffs, tick_flame_puffs));
    }
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// World-space (x, z) positions for the 3 campfires.
const CAMPFIRE_XZ: [(f32, f32); 3] = [
    ( 35.0, -45.0),
    (-50.0,  35.0),
    ( 75.0,  85.0),
];

/// Height above terrain where the campfire parent sits.
const GROUND_OFFSET: f32 = 0.1;

// Log geometry
const LOG_RADIUS:    f32 = 0.15;
const LOG_HEIGHT:    f32 = 1.2;
const LOG_RING_R:    f32 = 0.55; // distance of each log centre from fire centre
const LOG_TILT_DEG:  f32 = 30.0; // outward tilt angle

// Ash heap
const ASH_RADIUS:  f32 = 0.5;
const ASH_HEIGHT:  f32 = 0.05;

// Point light
const LIGHT_BASE_INTENSITY: f32 = 850.0;
const LIGHT_RANGE:          f32 = 12.0;
const LIGHT_Y:              f32 = 0.6;

// Flame puff
const PUFF_SPAWN_INTERVAL: f32 = 0.1;
const PUFF_RADIUS:         f32 = 0.18;
const PUFF_LIFETIME:       f32 = 1.2;
const PUFF_RISE_SPEED:     f32 = 1.5;
const PUFF_XZ_JITTER:      f32 = 0.3;
const MAX_PUFFS:           usize = 60;

// ---------------------------------------------------------------------------
// Marker components
// ---------------------------------------------------------------------------

/// Root entity for a single campfire.
#[derive(Component)]
pub struct Campfire;

/// Applied to the PointLight entity inside each campfire.
#[derive(Component)]
struct FlickerLight {
    /// Index (0–2) used to de-correlate flicker between fires.
    idx: f32,
}

/// A single rising flame puff particle.
#[derive(Component)]
struct FlamePuff {
    age_s: f32,
}

// ---------------------------------------------------------------------------
// Resource: FIFO queue of live puff entities for cap enforcement.
// ---------------------------------------------------------------------------

#[derive(Resource, Default)]
struct PuffQueue(VecDeque<Entity>);

// ---------------------------------------------------------------------------
// Startup: spawn all campfires
// ---------------------------------------------------------------------------

fn spawn_campfires(
    mut commands:  Commands,
    mut meshes:    ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Shared meshes
    let log_mesh = meshes.add(Cylinder::new(LOG_RADIUS, LOG_HEIGHT));
    let ash_mesh = meshes.add(Cylinder::new(ASH_RADIUS, ASH_HEIGHT));

    // Shared materials
    let log_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.20, 0.15, 0.10),
        perceptual_roughness: 0.95,
        ..default()
    });
    let ash_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.25, 0.25, 0.24),
        perceptual_roughness: 0.98,
        ..default()
    });

    for (fire_idx, &(fx, fz)) in CAMPFIRE_XZ.iter().enumerate() {
        let fy = terrain_height_at(fx, fz) + GROUND_OFFSET;
        let origin = Vec3::new(fx, fy, fz);

        // ---- parent entity ----
        let parent = commands.spawn((
            Campfire,
            Transform::from_translation(origin),
            Visibility::default(),
        )).id();

        // ---- ash heap ----
        // Sits at ground level; cylinder centred at origin → lift by half height.
        let ash = commands.spawn((
            Mesh3d(ash_mesh.clone()),
            MeshMaterial3d(ash_mat.clone()),
            Transform::from_translation(Vec3::new(0.0, ASH_HEIGHT * 0.5, 0.0)),
        )).id();
        commands.entity(parent).add_child(ash);

        // ---- 6 log cylinders in a tipi ring ----
        for i in 0..6_u32 {
            let angle_rad = std::f32::consts::TAU * (i as f32 / 6.0);

            // Place each log so its centre sits on the ring.
            let log_x = LOG_RING_R * angle_rad.cos();
            let log_z = LOG_RING_R * angle_rad.sin();
            // Log cylinder default axis is Y. We tilt it outward by rotating
            // around the tangent axis (perpendicular to the radial direction in XZ).
            // Radial direction in XZ: (cos θ, 0, sin θ).
            // Tangent (right-hand): (-sin θ, 0, cos θ) → tilting around this
            // leans the log outward.
            let tilt_rad  = LOG_TILT_DEG.to_radians();
            let _radial   = Vec3::new(angle_rad.cos(), 0.0, angle_rad.sin());
            let tangent   = Vec3::new(-angle_rad.sin(), 0.0, angle_rad.cos());
            let rotation  = Quat::from_axis_angle(tangent, tilt_rad);

            // Lift so the log bottom is roughly at ground level.
            let log_y = LOG_HEIGHT * 0.5 * tilt_rad.cos();

            let log = commands.spawn((
                Mesh3d(log_mesh.clone()),
                MeshMaterial3d(log_mat.clone()),
                Transform {
                    translation: Vec3::new(log_x, log_y, log_z),
                    rotation,
                    scale: Vec3::ONE,
                },
            )).id();
            commands.entity(parent).add_child(log);
        }

        // ---- flickering point light ----
        let light = commands.spawn((
            FlickerLight { idx: fire_idx as f32 },
            PointLight {
                intensity: LIGHT_BASE_INTENSITY,
                color:     Color::srgb(1.0, 0.6, 0.2),
                range:     LIGHT_RANGE,
                ..default()
            },
            Transform::from_translation(Vec3::new(0.0, LIGHT_Y, 0.0)),
        )).id();
        commands.entity(parent).add_child(light);
    }
}

// ---------------------------------------------------------------------------
// Update: flicker_lights
// ---------------------------------------------------------------------------

fn flicker_lights(
    time:   Res<Time>,
    mut lights: Query<(&mut PointLight, &FlickerLight)>,
) {
    let t = time.elapsed_secs();

    for (mut light, flicker) in lights.iter_mut() {
        let idx = flicker.idx;
        let intensity = LIGHT_BASE_INTENSITY
            + (t * 5.0  + idx).sin()        * 100.0
            + (t * 13.0 + idx * 0.7).sin()  * 80.0;
        light.intensity = intensity.max(600.0).min(1100.0);
    }
}

// ---------------------------------------------------------------------------
// Update: spawn_flame_puffs
// ---------------------------------------------------------------------------

fn spawn_flame_puffs(
    mut commands:      Commands,
    mut meshes:        ResMut<Assets<Mesh>>,
    mut materials:     ResMut<Assets<StandardMaterial>>,
    campfires:         Query<&GlobalTransform, With<Campfire>>,
    mut queue:         ResMut<PuffQueue>,
    time:              Res<Time>,
    mut spawn_timer:   Local<f32>,
) {
    let dt = time.delta_secs();
    *spawn_timer += dt;

    if *spawn_timer < PUFF_SPAWN_INTERVAL {
        return;
    }
    *spawn_timer -= PUFF_SPAWN_INTERVAL;

    let puff_mesh = meshes.add(Sphere::new(PUFF_RADIUS).mesh().ico(0).unwrap());
    let puff_mat  = materials.add(StandardMaterial {
        base_color: Color::srgba(1.0, 0.5, 0.1, 0.8),
        emissive:   LinearRgba::rgb(1.0, 0.5, 0.1),
        unlit:      true,
        alpha_mode: AlphaMode::Blend,
        ..default()
    });

    let t = time.elapsed_secs();

    for (fire_idx, fire_tf) in campfires.iter().enumerate() {
        let fire_pos = fire_tf.translation();

        // Deterministic XZ jitter from time + fire index, no rand crate needed.
        let seed = t * 1000.0 + fire_idx as f32 * 137.0;
        let jx   = ((seed * 1.618_034).sin() * 43_758.547).fract() * 2.0 - 1.0;
        let jz   = ((seed * 2.718_282).sin() * 43_758.547).fract() * 2.0 - 1.0;

        let spawn_pos = Vec3::new(
            fire_pos.x + jx * PUFF_XZ_JITTER,
            fire_pos.y + LIGHT_Y,
            fire_pos.z + jz * PUFF_XZ_JITTER,
        );

        let entity = commands.spawn((
            FlamePuff { age_s: 0.0 },
            Mesh3d(puff_mesh.clone()),
            MeshMaterial3d(puff_mat.clone()),
            Transform::from_translation(spawn_pos),
        )).id();

        queue.0.push_back(entity);

        // Enforce global cap of MAX_PUFFS.
        while queue.0.len() > MAX_PUFFS {
            if let Some(oldest) = queue.0.pop_front() {
                commands.entity(oldest).try_despawn();
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Update: tick_flame_puffs
// ---------------------------------------------------------------------------

fn tick_flame_puffs(
    mut commands: Commands,
    mut queue:    ResMut<PuffQueue>,
    mut puffs:    Query<(Entity, &mut Transform, &mut FlamePuff)>,
    time:         Res<Time>,
) {
    let dt = time.delta_secs();
    let mut expired: Vec<Entity> = Vec::new();

    for (entity, mut transform, mut puff) in puffs.iter_mut() {
        puff.age_s += dt;

        if puff.age_s >= PUFF_LIFETIME {
            expired.push(entity);
            commands.entity(entity).try_despawn();
            continue;
        }

        // Rise upward.
        transform.translation.y += PUFF_RISE_SPEED * dt;

        // Shrink toward zero at end of lifetime.
        let scale_factor = (1.0 - puff.age_s / PUFF_LIFETIME).max(0.0);
        transform.scale = Vec3::splat(scale_factor);
    }

    if !expired.is_empty() {
        queue.0.retain(|e| !expired.contains(e));
    }
}
