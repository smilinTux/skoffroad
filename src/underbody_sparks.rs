// Underbody contact sparks — Effect 4 of Sprint 67.
//
// A raycast from chassis bottom-center downward (length = CHASSIS_HALF.y + 0.05)
// detects when the underbody is near terrain. When contact has lasted > 0.05 s
// and the chassis speed > 2 m/s, spawn 5-10 small emissive "spark" spheres.
// Each spark shrinks to zero over 0.4 s lifetime and is despawned afterward.
//
// Uses avian3d SpatialQuery (same API as suspension in vehicle.rs).
//
// Public API:
//   UnderbodySparksPlugin

use bevy::prelude::*;
use avian3d::prelude::{LinearVelocity, SpatialQuery, SpatialQueryFilter};

use crate::vehicle::{Chassis, VehicleRoot};

// ---- Plugin -----------------------------------------------------------------

pub struct UnderbodySparksPlugin;

impl Plugin for UnderbodySparksPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SparksState>()
           .add_systems(Update, (
               detect_underbody_contact,
               update_sparks,
           ));
    }
}

// ---- Resources / Components -------------------------------------------------

/// Per-spark state.
struct Spark {
    entity:    Entity,
    velocity:  Vec3,
    lifetime:  f32,  // [0, MAX_LIFETIME]
    max_life:  f32,
}

#[derive(Resource, Default)]
struct SparksState {
    sparks:          Vec<Spark>,
    contact_timer:   f32,
    spark_mat:       Option<Handle<StandardMaterial>>,
    spark_mesh:      Option<Handle<Mesh>>,
}

/// Marker on each spark entity.
#[derive(Component)]
struct SparkParticle;

// ---- Constants --------------------------------------------------------------

const CHASSIS_HALF_Y:        f32 = 0.4;   // must match vehicle.rs CHASSIS_HALF.y
const RAY_EXTRA:             f32 = 0.05;  // metres below chassis floor
const CONTACT_GRACE:         f32 = 0.05;  // seconds before sparks start
const MIN_SPEED:             f32 = 2.0;   // m/s
const SPARK_LIFETIME:        f32 = 0.4;   // seconds
const GRAVITY:               f32 = 9.81;
const MAX_SPARKS:            usize = 60;

// ---- System: detect underbody contact ---------------------------------------

fn detect_underbody_contact(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut state: ResMut<SparksState>,
    vehicle: Option<Res<VehicleRoot>>,
    chassis_q: Query<(&Transform, &LinearVelocity), With<Chassis>>,
    spatial: SpatialQuery,
    time: Res<Time>,
) {
    let Some(vehicle) = vehicle else { return };
    let Ok((c_tf, lin_vel)) = chassis_q.get(vehicle.chassis) else { return };

    let dt = time.delta_secs();
    let speed = Vec3::new(lin_vel.x, lin_vel.y, lin_vel.z).length();

    // Ray from chassis bottom-center downward.
    let ray_origin = c_tf.translation - Vec3::Y * CHASSIS_HALF_Y;
    let hit = spatial.cast_ray(
        ray_origin,
        Dir3::NEG_Y,
        RAY_EXTRA + 0.01,
        true,
        &SpatialQueryFilter::default(),
    );

    if hit.is_some() && speed > MIN_SPEED {
        state.contact_timer += dt;
    } else {
        state.contact_timer = (state.contact_timer - dt * 2.0).max(0.0);
    }

    if state.contact_timer < CONTACT_GRACE {
        return;
    }
    if state.sparks.len() >= MAX_SPARKS {
        return;
    }

    // Ensure shared mesh/material are created.
    if state.spark_mat.is_none() {
        state.spark_mat = Some(materials.add(StandardMaterial {
            base_color: Color::srgb(1.0, 0.85, 0.20),
            emissive: LinearRgba::rgb(8.0, 5.0, 1.0),
            unlit: false,
            ..default()
        }));
    }
    if state.spark_mesh.is_none() {
        state.spark_mesh = Some(meshes.add(Sphere::new(0.02)));
    }

    let mat_handle  = state.spark_mat.clone().unwrap();
    let mesh_handle = state.spark_mesh.clone().unwrap();

    // Contact point: bottom of chassis.
    let contact_pt = ray_origin - Vec3::Y * RAY_EXTRA;

    // Deterministic seed from elapsed time.
    let seed = (time.elapsed_secs() * 1000.0) as u32;
    let rng = |n: u32| -> f32 {
        let v = seed.wrapping_add(n).wrapping_mul(2_654_435_761);
        ((v >> 16) as f32 / 65535.0) * 2.0 - 1.0
    };

    let count = 5 + (rng(77).abs() * 5.0) as usize; // 5-10

    for ci in 0..count.min(MAX_SPARKS - state.sparks.len()) {
        let i = ci as u32;
        let vx = rng(i)     * 2.0;
        let vy = 1.5 + rng(i + 1).abs() * 0.5;  // upward 1.5-2 m/s
        let vz = rng(i + 2) * 2.0;
        let velocity = Vec3::new(vx, vy, vz);

        let entity = commands.spawn((
            SparkParticle,
            Mesh3d(mesh_handle.clone()),
            MeshMaterial3d(mat_handle.clone()),
            Transform::from_translation(contact_pt),
        )).id();

        state.sparks.push(Spark {
            entity,
            velocity,
            lifetime: SPARK_LIFETIME,
            max_life: SPARK_LIFETIME,
        });
    }
}

// ---- System: update spark positions + despawn --------------------------------

fn update_sparks(
    mut commands: Commands,
    mut state: ResMut<SparksState>,
    mut transforms: Query<&mut Transform, With<SparkParticle>>,
    time: Res<Time>,
) {
    let dt = time.delta_secs();

    state.sparks.retain_mut(|spark| {
        spark.lifetime -= dt;
        if spark.lifetime <= 0.0 {
            commands.entity(spark.entity).despawn();
            return false;
        }

        spark.velocity.y -= GRAVITY * dt;

        // Shrink scale toward zero as lifetime expires.
        let t = spark.lifetime / spark.max_life;

        if let Ok(mut tf) = transforms.get_mut(spark.entity) {
            tf.translation += spark.velocity * dt;
            tf.scale = Vec3::splat(t);
        }

        true
    });
}
