// Wet-ground reflections: when rain stops (StormState transitions active→false),
// spawn ~10 large transparent puddle decals around the chassis that persist
// for 30 s then fade out and despawn.
//
// Sprint 70 — Effect 4
//
// Trigger:
//   Rising edge of StormState.active going false (rain→clear transition).
//   Puddles also despawn immediately if a new storm starts (rain washes them
//   out, then re-spawns a fresh batch when it stops again).
//
// Public API:
//   WetGroundPlugin
//
// Tuning knobs:
//   PUDDLE_COUNT        — number of puddles per rain-stop event
//   PUDDLE_LIFETIME_S   — seconds before puddles are fully gone
//   PUDDLE_FADE_START   — fraction of lifetime at which fading begins
//   PUDDLE_HALF_W/D     — puddle plane half-extents (m)
//   SCATTER_RADIUS      — scatter radius around chassis (m)

use bevy::prelude::*;

use crate::storm::StormState;
use crate::vehicle::{Chassis, VehicleRoot};
use crate::terrain::terrain_height_at;

// ---- Public API ---------------------------------------------------------------

pub struct WetGroundPlugin;

impl Plugin for WetGroundPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (
            detect_rain_stop,
            tick_puddles,
        ));
    }
}

// ---- Component ---------------------------------------------------------------

/// Marks each puddle decal entity.  Tracks lifetime for fade-out.
#[derive(Component)]
struct Puddle {
    age:      f32,
    lifetime: f32,
    mat:      Handle<StandardMaterial>,
}

// ---- Constants ---------------------------------------------------------------

const PUDDLE_COUNT:        usize = 10;
const PUDDLE_LIFETIME_S:   f32   = 30.0;
const PUDDLE_FADE_START:   f32   = 0.7;   // begin fading at 70% of lifetime

const PUDDLE_HALF_W: f32 = 2.0;
const PUDDLE_HALF_D: f32 = 2.0;

const SCATTER_RADIUS: f32 = 35.0;

/// Starting alpha when puddles first appear.
const PUDDLE_ALPHA: f32 = 0.45;

// ---- LCG helpers --------------------------------------------------------------

#[inline]
fn lcg_next(seed: &mut u32) -> f32 {
    *seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    *seed as f32 / u32::MAX as f32
}

#[inline]
fn lcg_signed(seed: &mut u32) -> f32 { lcg_next(seed) * 2.0 - 1.0 }

// ---- System: detect rain stop transition ------------------------------------

fn detect_rain_stop(
    storm:       Option<Res<StormState>>,
    mut commands: Commands,
    mut meshes:   ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    vehicle:     Option<Res<VehicleRoot>>,
    chassis_q:   Query<&Transform, With<Chassis>>,
    puddles:     Query<Entity, With<Puddle>>,
    mut was_raining: Local<bool>,
    mut seed:    Local<u32>,
) {
    let currently_raining = storm.map(|s| s.active).unwrap_or(false);

    // Rising-edge detection: was raining last frame, not raining now.
    let just_stopped = *was_raining && !currently_raining;
    // Rain restarted: despawn existing puddles so they don't overlap new batch.
    let just_started = !*was_raining && currently_raining;
    *was_raining = currently_raining;

    if just_started {
        for entity in &puddles {
            commands.entity(entity).despawn();
        }
        return;
    }

    if !just_stopped { return; }

    let chassis_pos = if let Some(ref vr) = vehicle {
        if let Ok(tf) = chassis_q.get(vr.chassis) { tf.translation } else { Vec3::ZERO }
    } else {
        Vec3::ZERO
    };

    if *seed == 0 { *seed = 0x57E7_6D00; } // deterministic seed for wet-ground puddle scatter

    let mesh = meshes.add(Plane3d::new(Vec3::Y, Vec2::new(PUDDLE_HALF_W, PUDDLE_HALF_D)));
    let mat  = materials.add(StandardMaterial {
        base_color:           Color::srgba(0.05, 0.06, 0.08, PUDDLE_ALPHA),
        alpha_mode:           AlphaMode::Blend,
        metallic:             0.85,
        perceptual_roughness: 0.10,
        unlit:                false,
        double_sided:         true,
        cull_mode:            None,
        reflectance:          0.9,
        ..default()
    });

    for _ in 0..PUDDLE_COUNT {
        let rx = lcg_signed(&mut *seed) * SCATTER_RADIUS;
        let rz = lcg_signed(&mut *seed) * SCATTER_RADIUS;
        let px = chassis_pos.x + rx;
        let pz = chassis_pos.z + rz;
        let py = terrain_height_at(px, pz) + 0.005; // just above terrain

        commands.spawn((
            Puddle {
                age:      0.0,
                lifetime: PUDDLE_LIFETIME_S,
                mat:      mat.clone(),
            },
            Mesh3d(mesh.clone()),
            MeshMaterial3d(mat.clone()),
            Transform::from_translation(Vec3::new(px, py, pz)),
        ));
    }
}

// ---- System: age + fade puddles ----------------------------------------------

fn tick_puddles(
    mut commands:  Commands,
    time:          Res<Time>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut puddles:   Query<(Entity, &mut Puddle)>,
) {
    let dt = time.delta_secs();
    for (entity, mut puddle) in &mut puddles {
        puddle.age += dt;

        // Fade out in the final (1 - PUDDLE_FADE_START) fraction of lifetime.
        let fade_t = ((puddle.age / puddle.lifetime - PUDDLE_FADE_START)
            / (1.0 - PUDDLE_FADE_START))
            .clamp(0.0, 1.0);
        let alpha = PUDDLE_ALPHA * (1.0 - fade_t);

        if let Some(mat) = materials.get_mut(&puddle.mat) {
            mat.base_color.set_alpha(alpha);
        }

        if puddle.age >= puddle.lifetime {
            commands.entity(entity).despawn();
        }
    }
}
