// Wildlife: 8 deer-like creatures wander the 200m map at slow pace. When the
// player chassis approaches within 20m they flee in the opposite direction for
// 4 seconds at 6 m/s. Pure decoration — no physics, no Avian colliders.
//
// Public API:
//   WildlifePlugin

use std::f32::consts::PI;

use bevy::prelude::*;

use crate::terrain::terrain_height_at;
use crate::vehicle::{Chassis, VehicleRoot};

// ---- Plugin -----------------------------------------------------------------

pub struct WildlifePlugin;

impl Plugin for WildlifePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_deer)
           .add_systems(Update, tick_deer);
    }
}

// ---- Components -------------------------------------------------------------

#[derive(Default, PartialEq)]
enum DeerState {
    #[default]
    Wander,
    Flee,
}

#[derive(Component)]
struct Deer {
    state: DeerState,
    target: Vec2,       // XZ wander/flee target
    state_timer: f32,   // time spent in current state
}

// ---- Constants --------------------------------------------------------------

const DEER_COUNT: usize = 8;
const WORLD_HALF: f32 = 100.0;   // ±100 m → 200 m square
const EXCLUSION_R: f32 = 30.0;   // skip radius around origin
const DETECT_DIST: f32 = 20.0;
const ARRIVE_DIST: f32 = 3.0;
const WANDER_SPEED: f32 = 1.5;
const FLEE_SPEED: f32 = 6.0;
const FLEE_DURATION: f32 = 4.0;
const BODY_Y_OFFSET: f32 = 0.3;   // snap offset above terrain

// ---- LCG helpers ------------------------------------------------------------

/// Single LCG step — Numerical Recipes constants.
#[inline]
fn lcg_next(state: &mut u64) -> u64 {
    *state = state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    *state
}

/// Map a u64 to [−range, +range).
#[inline]
fn lcg_range(state: &mut u64, range: f32) -> f32 {
    let v = lcg_next(state);
    let t = (v >> 11) as f32 / (1u64 << 53) as f32; // [0, 1)
    t * range * 2.0 - range
}

/// Generate a wander target that avoids the exclusion radius.
fn random_target(state: &mut u64) -> Vec2 {
    loop {
        let x = lcg_range(state, WORLD_HALF);
        let z = lcg_range(state, WORLD_HALF);
        if x * x + z * z > EXCLUSION_R * EXCLUSION_R {
            return Vec2::new(x, z);
        }
    }
}

// ---- Spawn ------------------------------------------------------------------

fn spawn_deer(
    mut commands: Commands,
    mut meshes:   ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Shared materials.
    let body_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.55, 0.40, 0.25),
        ..default()
    });
    let antler_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.35, 0.25, 0.15),
        ..default()
    });

    // Shared meshes.
    let body_mesh   = meshes.add(Cuboid::new(1.0, 0.6, 1.6));
    let head_mesh   = meshes.add(Cuboid::new(0.4, 0.5, 0.5));
    let leg_mesh    = meshes.add(Cylinder::new(0.1, 0.6));
    let antler_mesh = meshes.add(Cone { radius: 0.06, height: 0.35 });

    let mut lcg: u64 = 77; // seed=77

    for _ in 0..DEER_COUNT {
        let target = random_target(&mut lcg);
        let x = target.x;
        let z = target.y;
        let y = terrain_height_at(x, z) + BODY_Y_OFFSET;

        // Parent entity — carries the Deer component and transform.
        let parent = commands.spawn((
            Deer {
                state: DeerState::Wander,
                target,
                state_timer: 0.0,
            },
            Transform::from_xyz(x, y, z),
            Visibility::default(),
        )).id();

        // --- Body (centred at parent origin) ---
        let body_id = commands.spawn((
            Mesh3d(body_mesh.clone()),
            MeshMaterial3d(body_mat.clone()),
            Transform::IDENTITY,
        )).id();
        commands.entity(parent).add_child(body_id);

        // --- Head: forward (+Z local) and upward ---
        // Body half-height = 0.3 → top at +0.3; head half-height = 0.25 → centre at +0.55
        // Body half-depth  = 0.8 → front face at +0.8; head half-depth = 0.25 → centre at +1.05
        let head_id = commands.spawn((
            Mesh3d(head_mesh.clone()),
            MeshMaterial3d(body_mat.clone()),
            Transform::from_xyz(0.0, 0.55, 1.05),
        )).id();
        commands.entity(parent).add_child(head_id);

        // --- Antlers on head (relative to parent, above head) ---
        // Head centre at (0, 0.55, 1.05); antler tips at ±0.15 X, top of head +0.5
        for side in [-1.0_f32, 1.0_f32] {
            let antler_id = commands.spawn((
                Mesh3d(antler_mesh.clone()),
                MeshMaterial3d(antler_mat.clone()),
                Transform::from_xyz(side * 0.15, 0.55 + 0.25 + 0.175, 1.05),
            )).id();
            commands.entity(parent).add_child(antler_id);
        }

        // --- 4 legs at body corners ---
        // Body half-extents: X=0.5, Z=0.8; leg half-height=0.3 → bottom = body bottom (−0.3) − 0.3 = −0.6
        let leg_y = -0.3 - 0.3; // body bottom then leg centre below
        let leg_offsets = [
            Vec3::new( 0.4, leg_y,  0.7),
            Vec3::new(-0.4, leg_y,  0.7),
            Vec3::new( 0.4, leg_y, -0.7),
            Vec3::new(-0.4, leg_y, -0.7),
        ];
        for offset in leg_offsets {
            let leg_id = commands.spawn((
                Mesh3d(leg_mesh.clone()),
                MeshMaterial3d(body_mat.clone()),
                Transform::from_translation(offset),
            )).id();
            commands.entity(parent).add_child(leg_id);
        }
    }
}

// ---- Update -----------------------------------------------------------------

fn tick_deer(
    time: Res<Time>,
    vehicle_root: Option<Res<VehicleRoot>>,
    chassis_q: Query<&Transform, With<Chassis>>,
    mut deer_q: Query<(&mut Deer, &mut Transform), Without<Chassis>>,
) {
    let dt = time.delta_secs();

    // Obtain chassis XZ — if vehicle isn't ready yet, skip the whole system.
    let chassis_xz: Vec2 = if let Some(root) = vehicle_root.as_ref() {
        match chassis_q.get(root.chassis) {
            Ok(tf) => Vec2::new(tf.translation.x, tf.translation.z),
            Err(_) => return,
        }
    } else {
        return;
    };

    // Time-based LCG seed for wander target regeneration.
    let time_seed = (time.elapsed_secs() * 1000.0) as u64;

    for (mut deer, mut transform) in deer_q.iter_mut() {
        let deer_xz = Vec2::new(transform.translation.x, transform.translation.z);
        let to_chassis = chassis_xz - deer_xz;
        let chassis_dist = to_chassis.length();

        // ---- State transitions ----
        match deer.state {
            DeerState::Wander => {
                if chassis_dist < DETECT_DIST {
                    // Switch to flee — run away from chassis.
                    let away_dir = if chassis_dist > 0.001 {
                        -to_chassis.normalize()
                    } else {
                        Vec2::new(1.0, 0.0)
                    };
                    deer.target = chassis_xz + away_dir * 50.0;
                    deer.state = DeerState::Flee;
                    deer.state_timer = 0.0;
                } else if (deer.target - deer_xz).length() < ARRIVE_DIST {
                    // Pick a new wander target via time-seeded LCG.
                    let mut lcg = time_seed
                        .wrapping_add(deer_xz.x.to_bits() as u64)
                        .wrapping_add(deer_xz.y.to_bits() as u64);
                    deer.target = random_target(&mut lcg);
                }
            }
            DeerState::Flee => {
                if deer.state_timer > FLEE_DURATION {
                    // Back to wander with a fresh target.
                    let mut lcg = time_seed
                        .wrapping_add(deer_xz.x.to_bits() as u64)
                        .wrapping_add(deer_xz.y.to_bits() as u64);
                    deer.target = random_target(&mut lcg);
                    deer.state = DeerState::Wander;
                    deer.state_timer = 0.0;
                }
            }
        }

        // ---- Movement ----
        let speed = match deer.state {
            DeerState::Wander => WANDER_SPEED,
            DeerState::Flee => FLEE_SPEED,
        };

        let to_target = deer.target - deer_xz;
        let dir = if to_target.length() > 0.001 {
            to_target.normalize()
        } else {
            Vec2::ZERO
        };

        let new_x = transform.translation.x + dir.x * speed * dt;
        let new_z = transform.translation.z + dir.y * speed * dt;
        // Clamp inside terrain bounds.
        let new_x = new_x.clamp(-99.0, 99.0);
        let new_z = new_z.clamp(-99.0, 99.0);
        let new_y = terrain_height_at(new_x, new_z) + BODY_Y_OFFSET;

        transform.translation = Vec3::new(new_x, new_y, new_z);

        // Face direction of motion.
        if dir.length_squared() > 0.001 {
            let yaw = -dir.x.atan2(dir.y) + PI;
            transform.rotation = Quat::from_rotation_y(yaw);
        }

        deer.state_timer += dt;
    }
}
