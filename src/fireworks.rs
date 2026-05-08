// Fireworks: spawn colorful particle bursts at race finish or course
// completion. Each "shell" launches upward, explodes into 24 sparks at
// peak, sparks fade over 2s. Triggered by detecting RaceState.phase
// transition to Finished, or CourseState.completed rising edge.
//
// Public API:
//   FireworksPlugin

use std::collections::VecDeque;

use bevy::prelude::*;

use crate::course::CourseState;
use crate::race::{RacePhase, RaceState};
use crate::vehicle::{Chassis, VehicleRoot};

// ---------------------------------------------------------------------------
// Public plugin
// ---------------------------------------------------------------------------

pub struct FireworksPlugin;

impl Plugin for FireworksPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FireworksAssets>()
           .init_resource::<SparkQueue>()
           .add_systems(
               Update,
               (detect_finish_events, tick_shells, tick_sparks).chain(),
           );
    }
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Number of sparks spawned when a shell explodes.
const SPARKS_PER_SHELL: usize = 24;

/// Shell explodes after this many seconds regardless.
const SHELL_MAX_AGE: f32 = 1.5;

/// Spark lifetime in seconds.
const SPARK_LIFETIME: f32 = 2.0;

/// Spark sphere radius.
const SPARK_RADIUS: f32 = 0.2;

/// Maximum simultaneous active particles (shells + sparks combined).
const MAX_PARTICLES: usize = 200;

/// Colour palette: (r, g, b) in linear [0..1].
const PALETTE: [(f32, f32, f32); 6] = [
    (1.0,  0.10, 0.10), // red
    (0.15, 0.40, 1.0),  // blue
    (1.0,  0.80, 0.05), // gold
    (0.10, 0.90, 0.20), // green
    (1.0,  1.0,  1.0),  // white
    (0.90, 0.10, 0.90), // magenta
];

/// Emissive intensity multiplier.
const EMIT_INTENSITY: f32 = 6.0;

// ---------------------------------------------------------------------------
// Components
// ---------------------------------------------------------------------------

/// An airborne firework shell climbing toward its apex.
#[derive(Component)]
struct Shell {
    vel:   Vec3,
    color: (f32, f32, f32),
    age:   f32,
}

/// A single spark particle after a shell explodes.
#[derive(Component)]
struct Spark {
    vel:        Vec3,
    age:        f32,
    color:      (f32, f32, f32),
    mat_handle: Handle<StandardMaterial>,
}

// ---------------------------------------------------------------------------
// Resources
// ---------------------------------------------------------------------------

/// Shared sphere mesh for all sparks (shells don't need a visible mesh).
#[derive(Resource, Default)]
struct FireworksAssets {
    spark_mesh: Option<Handle<Mesh>>,
}

/// FIFO queue of active spark entities so we can cap MAX_PARTICLES by
/// despawning the oldest first.
#[derive(Resource, Default)]
struct SparkQueue(VecDeque<Entity>);

// ---------------------------------------------------------------------------
// LCG helpers (no rand crate dependency)
// ---------------------------------------------------------------------------

#[inline]
fn lcg_next(seed: &mut u32) -> f32 {
    *seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    *seed as f32 / u32::MAX as f32
}

#[inline]
fn lcg_signed(seed: &mut u32) -> f32 {
    lcg_next(seed) * 2.0 - 1.0
}

/// Generate a random unit-sphere direction.
fn random_unit_sphere(seed: &mut u32) -> Vec3 {
    // Rejection sample until inside sphere, then normalise.
    for _ in 0..32 {
        let v = Vec3::new(lcg_signed(seed), lcg_signed(seed), lcg_signed(seed));
        if v.length_squared() <= 1.0 && v.length_squared() > 1e-8 {
            return v.normalize();
        }
    }
    Vec3::Y
}

// ---------------------------------------------------------------------------
// System: detect_finish_events
//
// Uses Local<bool> rising-edge detectors and Local<f32> timers to schedule
// shell launches spaced over time.
// ---------------------------------------------------------------------------

/// Pending shell launches: (position, colour_idx, delay_remaining_s)
#[derive(Default)]
struct PendingShells(Vec<(Vec3, usize, f32)>);

fn detect_finish_events(
    mut commands:         Commands,
    mut assets:           ResMut<FireworksAssets>,
    mut meshes:           ResMut<Assets<Mesh>>,
    race_state:           Option<Res<RaceState>>,
    course_state:         Option<Res<CourseState>>,
    vehicle:              Option<Res<VehicleRoot>>,
    chassis_q:            Query<&Transform, With<Chassis>>,
    time:                 Res<Time>,
    mut last_race_phase:  Local<RacePhase>,
    mut last_course_done: Local<bool>,
    mut pending:          Local<PendingShells>,
    mut seed:             Local<u32>,
) {
    let dt = time.delta_secs();

    // Ensure mesh is initialised.
    if assets.spark_mesh.is_none() {
        assets.spark_mesh = Some(meshes.add(Sphere::new(SPARK_RADIUS).mesh().ico(1).unwrap()));
    }

    // Get chassis position (fallback to origin).
    let chassis_pos = vehicle
        .as_ref()
        .and_then(|v| chassis_q.get(v.chassis).ok())
        .map(|tf| tf.translation)
        .unwrap_or(Vec3::ZERO);

    // ---- Rising-edge: race → Finished ----------------------------------------
    if let Some(rs) = &race_state {
        let current_phase = rs.phase;
        let was_finished = *last_race_phase == RacePhase::Finished;
        let now_finished = current_phase == RacePhase::Finished;
        if now_finished && !was_finished {
            // Schedule 6 shells over 4 seconds.
            if *seed == 0 {
                *seed = 0xDEAD_BEEF;
            }
            for i in 0..6usize {
                let delay = i as f32 * (4.0 / 6.0);
                let col_idx = i % PALETTE.len();
                let ox = lcg_signed(&mut *seed) * 30.0;
                let oz = lcg_signed(&mut *seed) * 30.0;
                let pos = chassis_pos + Vec3::new(ox, 0.0, oz);
                pending.0.push((pos, col_idx, delay));
            }
        }
        *last_race_phase = current_phase;
    }

    // ---- Rising-edge: course completed ---------------------------------------
    if let Some(cs) = &course_state {
        let now_done = cs.completed;
        let was_done = *last_course_done;
        if now_done && !was_done {
            if *seed == 0 {
                *seed = 0xCAFE_BABE;
            }
            // Schedule 3 shells over 2 seconds.
            for i in 0..3usize {
                let delay = i as f32 * (2.0 / 3.0);
                let col_idx = (i + 2) % PALETTE.len();
                let ox = lcg_signed(&mut *seed) * 30.0;
                let oz = lcg_signed(&mut *seed) * 30.0;
                let pos = chassis_pos + Vec3::new(ox, 0.0, oz);
                pending.0.push((pos, col_idx, delay));
            }
        }
        *last_course_done = now_done;
    }

    // ---- Tick pending delays and spawn shells ---------------------------------
    let mut i = 0;
    while i < pending.0.len() {
        pending.0[i].2 -= dt;
        if pending.0[i].2 <= 0.0 {
            let (pos, col_idx, _) = pending.0.remove(i);
            let color = PALETTE[col_idx % PALETTE.len()];
            let speed = 25.0 + lcg_next(&mut *seed) * 5.0;
            let vel = Vec3::new(0.0, speed, 0.0);
            commands.spawn((
                Shell { vel, color, age: 0.0 },
                Transform::from_translation(pos),
                Visibility::Visible,
            ));
        } else {
            i += 1;
        }
    }
}

// ---------------------------------------------------------------------------
// System: tick_shells
//
// Integrates each shell upward; detects apex (vel.y < 0 or age > 1.5 s) and
// then explodes into 24 sparks.
// ---------------------------------------------------------------------------

fn tick_shells(
    mut commands:  Commands,
    assets:        Res<FireworksAssets>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut spark_q:   ResMut<SparkQueue>,
    time:          Res<Time>,
    mut shell_q:   Query<(Entity, &mut Transform, &mut Shell)>,
) {
    let dt = time.delta_secs();
    let Some(mesh) = assets.spark_mesh.clone() else { return };

    let mut seed_base: u32 = (time.elapsed_secs() * 1_234_567.0) as u32;

    for (entity, mut transform, mut shell) in &mut shell_q {
        // Integrate position.
        transform.translation += shell.vel * dt;
        shell.vel.y -= 9.81 * dt;
        shell.age += dt;

        let should_explode = shell.age >= SHELL_MAX_AGE || shell.vel.y < 0.0;
        if !should_explode {
            continue;
        }

        let explode_pos = transform.translation;
        let color = shell.color;

        // Despawn the shell.
        commands.entity(entity).despawn();

        // Spawn 24 sparks.
        seed_base = seed_base.wrapping_add(0x9E3779B9);
        let mut seed = seed_base;
        for _ in 0..SPARKS_PER_SHELL {
            let dir = random_unit_sphere(&mut seed);
            let speed = 8.0 + lcg_next(&mut seed) * 4.0;
            let vel = dir * speed;

            let mat_handle = materials.add(StandardMaterial {
                base_color: Color::srgba(color.0, color.1, color.2, 1.0),
                emissive: LinearRgba::new(
                    color.0 * EMIT_INTENSITY,
                    color.1 * EMIT_INTENSITY,
                    color.2 * EMIT_INTENSITY,
                    1.0,
                ),
                unlit: true,
                alpha_mode: AlphaMode::Blend,
                ..default()
            });

            let spark_entity = commands
                .spawn((
                    Spark {
                        vel,
                        age: 0.0,
                        color,
                        mat_handle: mat_handle.clone(),
                    },
                    Mesh3d(mesh.clone()),
                    MeshMaterial3d(mat_handle),
                    Transform::from_translation(explode_pos),
                    Visibility::Visible,
                ))
                .id();

            spark_q.0.push_back(spark_entity);
        }

        // Cap total active sparks at MAX_PARTICLES; despawn oldest first.
        while spark_q.0.len() > MAX_PARTICLES {
            if let Some(old) = spark_q.0.pop_front() {
                commands.entity(old).despawn();
            }
        }
    }
}

// ---------------------------------------------------------------------------
// System: tick_sparks
//
// Applies gravity, moves each spark, fades its material alpha, and despawns
// at age >= SPARK_LIFETIME.
// ---------------------------------------------------------------------------

fn tick_sparks(
    mut commands:  Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut spark_q:   ResMut<SparkQueue>,
    time:          Res<Time>,
    mut query:     Query<(Entity, &mut Transform, &mut Spark)>,
) {
    let dt = time.delta_secs();

    for (entity, mut transform, mut spark) in &mut query {
        spark.vel.y -= 9.81 * dt;
        transform.translation += spark.vel * dt;
        spark.age += dt;

        if spark.age >= SPARK_LIFETIME {
            commands.entity(entity).despawn();
            spark_q.0.retain(|&e| e != entity);
            continue;
        }

        // Fade alpha from 1 → 0 over the lifetime.
        let alpha = 1.0 - (spark.age / SPARK_LIFETIME).clamp(0.0, 1.0);
        let emissive_scale = alpha * EMIT_INTENSITY;
        let (r, g, b) = spark.color;

        if let Some(mat) = materials.get_mut(&spark.mat_handle) {
            mat.base_color = Color::srgba(r, g, b, alpha);
            mat.emissive = LinearRgba::new(r * emissive_scale, g * emissive_scale, b * emissive_scale, 1.0);
        }
    }
}
