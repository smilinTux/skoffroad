use bevy::prelude::*;
use avian3d::prelude::*;

use crate::terrain::{terrain_height_at, TERRAIN_SEED};
use crate::vehicle::{Chassis, VehicleRoot};

pub struct TrampolinesPlugin;

impl Plugin for TrampolinesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_trampolines)
           .add_systems(
               PhysicsSchedule,
               apply_trampoline_bounce
                   .after(PhysicsStepSystems::NarrowPhase)
                   .before(PhysicsStepSystems::Solver),
           );
    }
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

#[derive(Component)]
pub struct Trampoline {
    pub radius: f32,
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

// Distinct LCG seed so trampoline positions don't overlap mud/trees.
const TRAMP_SEED: u32 = TERRAIN_SEED + 13;

// XZ placement bounds (±70 m).
const WORLD_HALF: f32 = 70.0;

// Keep pads at least this far from the origin so the spawn point is clear.
const SPAWN_CLEAR_RADIUS: f32 = 25.0;

// Pad geometry: radius 3 m, disk height 0.15 m.
const PAD_RADIUS: f32 = 3.0;
const PAD_HEIGHT: f32 = 0.15;

// Pole geometry sits on top of the pad.
const POLE_RADIUS: f32 = 0.15;
const POLE_HEIGHT: f32 = 1.0;

// Maximum terrain slope (0 = flat, 1 = vertical) allowed for placement.
const MAX_SLOPE: f32 = 0.20;

// Number of pads to try spawning.
const PAD_COUNT_MIN: usize = 4;
const PAD_COUNT_MAX: usize = 6;

// Upward impulse applied each physics tick while the chassis is on a pad.
// 30 000 N on a 1 500 kg chassis ≈ 20 m/s² upward — launches it noticeably
// over the ~0.1 s contact window.
const BOUNCE_FORCE: f32 = 30_000.0;

// Only apply bounce when the chassis vertical velocity is below this value.
// Prevents re-bouncing the chassis while it is already flying upward.
const MAX_VERT_VEL_TO_BOUNCE: f32 = 1.0;

// The chassis must be within this vertical band above the pad surface to count
// as "on" the pad (not 5 m up in the air after a previous bounce).
const ON_PAD_VERT_THRESHOLD: f32 = 1.5;

// ---------------------------------------------------------------------------
// Startup: procedural placement
// ---------------------------------------------------------------------------

fn slope_at(x: f32, z: f32) -> f32 {
    let step = 1.0_f32;
    let h  = terrain_height_at(x, z);
    let hx = terrain_height_at(x + step, z);
    let hz = terrain_height_at(x, z + step);
    let nx_v = Vec3::new(step, hx - h, 0.0).normalize();
    let nz_v = Vec3::new(0.0, hz - h, step).normalize();
    let n = nx_v.cross(nz_v).normalize();
    1.0 - n.dot(Vec3::Y).clamp(0.0, 1.0)
}

fn spawn_trampolines(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Bright magenta with a glow — unmistakable on any terrain colour.
    let pad_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.2, 0.7),
        emissive: LinearRgba::rgb(0.8, 0.05, 0.4),
        perceptual_roughness: 0.5,
        ..default()
    });
    let pole_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.2, 0.7),
        emissive: LinearRgba::rgb(0.8, 0.05, 0.4),
        perceptual_roughness: 0.5,
        ..default()
    });

    let pad_mesh  = meshes.add(Cylinder::new(PAD_RADIUS, PAD_HEIGHT));
    let pole_mesh = meshes.add(Cylinder::new(POLE_RADIUS, POLE_HEIGHT));

    // Two independent LCG streams so X and Z aren't correlated.
    let mut lcg_x = lcg_init(TRAMP_SEED);
    let mut lcg_z = lcg_init(TRAMP_SEED ^ 0xCAFE_BABE);

    // Decide actual count using a third stream (4–6 pads).
    let (count_raw, lcg_c) = lcg_next(lcg_init(TRAMP_SEED.wrapping_add(99)));
    let _ = lcg_c;
    let count = PAD_COUNT_MIN
        + (count_raw * (PAD_COUNT_MAX - PAD_COUNT_MIN + 1) as f32) as usize;
    let count = count.min(PAD_COUNT_MAX);

    let mut spawned = 0usize;
    let mut attempts = 0usize;

    while spawned < count && attempts < count * 30 {
        attempts += 1;

        let (xf, lx2) = lcg_next(lcg_x);
        let (zf, lz2) = lcg_next(lcg_z);
        lcg_x = lx2;
        lcg_z = lz2;

        // Map [0,1) to [-WORLD_HALF, WORLD_HALF].
        let wx = (xf - 0.5) * 2.0 * WORLD_HALF;
        let wz = (zf - 0.5) * 2.0 * WORLD_HALF;

        // Reject if too close to origin.
        if wx * wx + wz * wz < SPAWN_CLEAR_RADIUS * SPAWN_CLEAR_RADIUS {
            continue;
        }

        // Reject on steep slopes.
        if slope_at(wx, wz) > MAX_SLOPE {
            continue;
        }

        let ground_y = terrain_height_at(wx, wz);
        // Disk centre sits half its height above the ground surface.
        let pad_y = ground_y + PAD_HEIGHT * 0.5;
        // Pole centre: top of the disk + half the pole height.
        let pole_y = ground_y + PAD_HEIGHT + POLE_HEIGHT * 0.5;

        let pad = commands.spawn((
            Trampoline { radius: PAD_RADIUS },
            Mesh3d(pad_mesh.clone()),
            MeshMaterial3d(pad_mat.clone()),
            Transform::from_translation(Vec3::new(wx, pad_y, wz)),
        )).id();

        let pole = commands.spawn((
            Mesh3d(pole_mesh.clone()),
            MeshMaterial3d(pole_mat.clone()),
            Transform::from_translation(Vec3::new(wx, pole_y, wz)),
        )).id();

        // Keep the pole visually attached to the pad entity in the hierarchy.
        commands.entity(pad).add_child(pole);

        spawned += 1;
    }
}

// ---------------------------------------------------------------------------
// Physics: upward bounce impulse
// ---------------------------------------------------------------------------

fn apply_trampoline_bounce(
    vehicle: Option<Res<VehicleRoot>>,
    mut chassis_q: Query<(Forces, &Transform), With<Chassis>>,
    trampoline_q: Query<(&Transform, &Trampoline)>,
) {
    let Some(vehicle) = vehicle else { return };
    let Ok((mut forces, chassis_tf)) = chassis_q.get_mut(vehicle.chassis) else { return };

    let chassis_pos = chassis_tf.translation;
    let chassis_xz  = Vec2::new(chassis_pos.x, chassis_pos.z);

    // Only bounce when the chassis is not already flying fast upward.
    let vel = forces.linear_velocity();
    if vel.y > MAX_VERT_VEL_TO_BOUNCE {
        return;
    }

    for (pad_tf, trampoline) in trampoline_q.iter() {
        let pad_pos = pad_tf.translation;
        let pad_xz  = Vec2::new(pad_pos.x, pad_pos.z);

        let xz_dist = (chassis_xz - pad_xz).length();
        if xz_dist >= trampoline.radius {
            continue;
        }

        // Chassis must be close above the pad surface (not 5 m up in the air).
        let vert_offset = chassis_pos.y - pad_pos.y;
        if vert_offset < 0.0 || vert_offset > ON_PAD_VERT_THRESHOLD {
            continue;
        }

        forces.apply_force(Vec3::Y * BOUNCE_FORCE);
        // A single pad match is enough — exit after first hit to avoid
        // double-applying if pads overlap.
        return;
    }
}

// ---------------------------------------------------------------------------
// LCG helpers — deterministic float in [0, 1)
// ---------------------------------------------------------------------------

/// Initialise LCG state from a u32 seed (Wang-style hash to avoid low-entropy seeds).
#[inline]
fn lcg_init(seed: u32) -> u64 {
    let mut s = seed as u64;
    s ^= s << 17;
    s ^= s >> 31;
    s ^= s << 8;
    (s | 1) as u64 // must be odd for the multiplier to give a full-period LCG
}

/// Advance the LCG and return a float in [0, 1) plus the new state.
#[inline]
fn lcg_next(state: u64) -> (f32, u64) {
    // Knuth MMIX coefficients.
    let next = state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    let f = (next >> 33) as f32 / (u32::MAX as f32);
    (f, next)
}
