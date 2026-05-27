// Ambient critters: small coloured cuboid sprites (butterflies, birds) that
// orbit the chassis when the vehicle is idle, adding environmental life.
//
// Sprint 70 — Effect 5
//
// Trigger:
//   Critters spawn when vehicle speed < SPAWN_SPEED_THRESHOLD (5 m/s).
//   They despawn if the vehicle exceeds DESPAWN_SPEED_THRESHOLD (5 m/s).
//
// Critters orbit an anchor point 1–4 m above the terrain using a simple
// circular + Y-bob motion.  Each critter has a random orbit radius, speed,
// phase, and colour (yellow butterfly or dark bird).
//
// Public API:
//   CrittersPlugin
//
// Tuning knobs:
//   CRITTER_COUNT            — target critter count at idle
//   SPAWN_SPEED_THRESHOLD    — m/s below which critters appear
//   DESPAWN_SPEED_THRESHOLD  — m/s above which critters flee
//   ORBIT_RADIUS_MIN/MAX     — orbit ring radius (m)
//   ORBIT_ALT_MIN/MAX        — altitude above terrain (m)
//   ORBIT_SPEED_MIN/MAX      — angular speed (rad/s)
//   BOB_AMP                  — Y bob amplitude (m)
//   BOB_FREQ                 — Y bob frequency (rad/s)

use bevy::prelude::*;
use avian3d::prelude::LinearVelocity;

use crate::vehicle::{Chassis, VehicleRoot};
use crate::terrain::terrain_height_at;

// ---- Public API ---------------------------------------------------------------

pub struct CrittersPlugin;

impl Plugin for CrittersPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (
            manage_critters,
            animate_critters,
        ));
    }
}

// ---- Components ---------------------------------------------------------------

/// Critter kind determines its colour.
#[derive(Clone, Copy)]
enum CritterKind {
    Butterfly,
    Bird,
}

/// State needed to animate a critter.
#[derive(Component)]
struct Critter {
    #[allow(dead_code)]
    kind:          CritterKind,
    orbit_radius:  f32,   // m
    orbit_speed:   f32,   // rad/s
    orbit_phase:   f32,   // initial angle (rad)
    anchor:        Vec3,  // world-space orbit centre
    bob_phase:     f32,   // initial Y-bob phase
    bob_amp:       f32,   // amplitude (m)
    bob_freq:      f32,   // frequency (rad/s)
    total_time:    f32,   // cumulative time for phase tracking
}

// ---- Constants ---------------------------------------------------------------

const CRITTER_COUNT:           usize = 6;
const SPAWN_SPEED_THRESHOLD:   f32   = 5.0;
const DESPAWN_SPEED_THRESHOLD: f32   = 5.0;

const ORBIT_RADIUS_MIN: f32 = 1.5;
const ORBIT_RADIUS_MAX: f32 = 5.0;
const ORBIT_ALT_MIN:    f32 = 1.0;
const ORBIT_ALT_MAX:    f32 = 4.0;
const ORBIT_SPEED_MIN:  f32 = 0.4;
const ORBIT_SPEED_MAX:  f32 = 1.2;

const BOB_AMP:  f32 = 0.15;
const BOB_FREQ: f32 = 2.0;

/// Critter mesh size (m).
const CRITTER_SIZE: f32 = 0.05;

// ---- LCG helpers --------------------------------------------------------------

#[inline]
fn lcg_next(seed: &mut u32) -> f32 {
    *seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    *seed as f32 / u32::MAX as f32
}

#[inline]
fn lcg_range(seed: &mut u32, lo: f32, hi: f32) -> f32 {
    lo + lcg_next(seed) * (hi - lo)
}

// ---- System: manage critter pool ----------------------------------------------

fn manage_critters(
    mut commands:  Commands,
    mut meshes:    ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    vehicle:       Option<Res<VehicleRoot>>,
    chassis_q:     Query<(&Transform, &LinearVelocity), With<Chassis>>,
    critters:      Query<Entity, With<Critter>>,
    mut seed:      Local<u32>,
) {
    let (chassis_pos, speed_mps) = if let Some(ref vr) = vehicle {
        if let Ok((tf, vel)) = chassis_q.get(vr.chassis) {
            let spd = Vec3::new(vel.x, 0.0, vel.z).length();
            (tf.translation, spd)
        } else {
            (Vec3::ZERO, 0.0)
        }
    } else {
        (Vec3::ZERO, 0.0)
    };

    let existing = critters.iter().count();

    // Despawn all if too fast.
    if speed_mps > DESPAWN_SPEED_THRESHOLD {
        for entity in &critters {
            commands.entity(entity).despawn();
        }
        return;
    }

    // Nothing to do if at cap or still too fast to spawn.
    if speed_mps > SPAWN_SPEED_THRESHOLD || existing >= CRITTER_COUNT {
        return;
    }

    if *seed == 0 { *seed = 0xC171_7EAA; }

    let to_spawn = CRITTER_COUNT - existing;

    let butterfly_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.85, 0.05),
        unlit:      true,
        ..default()
    });
    let bird_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.12, 0.12, 0.15),
        unlit:      true,
        ..default()
    });
    let mesh = meshes.add(Cuboid::new(CRITTER_SIZE, CRITTER_SIZE, CRITTER_SIZE));

    for i in 0..to_spawn {
        let kind = if i % 2 == 0 { CritterKind::Butterfly } else { CritterKind::Bird };
        let mat  = match kind {
            CritterKind::Butterfly => butterfly_mat.clone(),
            CritterKind::Bird      => bird_mat.clone(),
        };

        let radius = lcg_range(&mut *seed, ORBIT_RADIUS_MIN, ORBIT_RADIUS_MAX);
        let alt    = lcg_range(&mut *seed, ORBIT_ALT_MIN, ORBIT_ALT_MAX);
        let speed  = lcg_range(&mut *seed, ORBIT_SPEED_MIN, ORBIT_SPEED_MAX);
        let phase  = lcg_next(&mut *seed) * std::f32::consts::TAU;
        let bphase = lcg_next(&mut *seed) * std::f32::consts::TAU;
        let bamp   = BOB_AMP * (0.5 + lcg_next(&mut *seed));
        let bfreq  = BOB_FREQ * (0.8 + lcg_next(&mut *seed) * 0.4);

        // Anchor: random offset from chassis on terrain + altitude.
        let ax  = chassis_pos.x + lcg_range(&mut *seed, -3.0, 3.0);
        let az  = chassis_pos.z + lcg_range(&mut *seed, -3.0, 3.0);
        let ay  = terrain_height_at(ax, az) + alt;

        let anchor = Vec3::new(ax, ay, az);

        // Initial position on the orbit.
        let ix = ax + phase.cos() * radius;
        let iz = az + phase.sin() * radius;

        commands.spawn((
            Critter {
                kind,
                orbit_radius: radius,
                orbit_speed:  speed,
                orbit_phase:  phase,
                anchor,
                bob_phase:    bphase,
                bob_amp:      bamp,
                bob_freq:     bfreq,
                total_time:   0.0,
            },
            Mesh3d(mesh.clone()),
            MeshMaterial3d(mat),
            Transform::from_translation(Vec3::new(ix, ay, iz)),
        ));
    }
}

// ---- System: animate critters ------------------------------------------------

fn animate_critters(
    time:          Res<Time>,
    mut critters:  Query<(&mut Transform, &mut Critter)>,
) {
    let dt = time.delta_secs();
    for (mut tf, mut critter) in &mut critters {
        critter.total_time += dt;
        let t     = critter.total_time;
        let angle = critter.orbit_phase + critter.orbit_speed * t;
        let bob   = (critter.bob_phase + critter.bob_freq * t).sin() * critter.bob_amp;

        tf.translation = Vec3::new(
            critter.anchor.x + angle.cos() * critter.orbit_radius,
            critter.anchor.y + bob,
            critter.anchor.z + angle.sin() * critter.orbit_radius,
        );

        // Face along the orbit tangent so the critter looks like it's flying.
        let tangent = Vec3::new(-angle.sin(), 0.0, angle.cos()).normalize();
        if tangent.length_squared() > 0.001 {
            tf.look_to(tangent, Vec3::Y);
        }
    }
}
