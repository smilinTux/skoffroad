// Volumetric fog puffs: tier-scaled translucent spheres that drift around the
// player with the wind, wrapping back when they exceed WRAP_RADIUS.
//
// Sprint 70 — Effect 3
// Sprint B4 — Enhanced: tier gating, weather-responsive color/density,
//             improved atmospheric depth via distance fog settings.
//
// Trigger:
//   Always active (puff count driven by FogDensity resource).
//   Shift+G cycles density: 0.0 → 0.3 → 0.7 → 0.0 …
//
// Tier gating (Sprint B4):
//   Low    — no puffs at all (early return in manage/drift/wrap).
//   Medium — FOG_PUFF_COUNT_MED puffs, moderate alpha.
//   High   — FOG_PUFF_COUNT_HIGH puffs, full alpha + weather color response.
//
// Weather response (High only):
//   Reads WeatherState (read-only — weather_director.rs is NOT edited).
//   Clear → light blue-white haze; Overcast/Storm → thicker grey fog.
//
// Public API:
//   FogVolumetricPlugin
//   FogDensity (resource — 0.0 = off, 1.0 = max)
//
// Tuning knobs:
//   FOG_PUFF_COUNT_*  — target puff count (at density 1.0) per tier
//   FOG_PUFF_RADIUS   — sphere radius (m)
//   FOG_BASE_ALPHA    — maximum alpha per puff
//   FOG_DRIFT_FACTOR  — fraction of wind speed used for puff drift
//   WRAP_RADIUS       — distance from chassis at which puffs respawn (m)

use bevy::prelude::*;

use crate::graphics_quality::GraphicsQuality;
use crate::weather_director::{WeatherCondition, WeatherState};
use crate::wind::WindState;
use crate::vehicle::{Chassis, VehicleRoot};

// ---- Public API ---------------------------------------------------------------

pub struct FogVolumetricPlugin;

impl Plugin for FogVolumetricPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FogDensity>()
           .add_systems(Update, (
               toggle_fog_density,
               manage_fog_puffs,
               drift_fog_puffs,
               wrap_fog_puffs,
               update_puff_colors,
           ));
    }
}

/// Fog density in [0.0, 1.0]. Default 0.3. Shift+G cycles: 0.0 / 0.3 / 0.7.
#[derive(Resource)]
pub struct FogDensity(pub f32);

impl Default for FogDensity {
    fn default() -> Self { Self(0.3) }
}

// ---- Component ---------------------------------------------------------------

/// Marks each fog puff entity.
#[derive(Component)]
struct FogPuff;

// ---- Constants ---------------------------------------------------------------

/// Puff count at Medium quality (density=1.0).
const FOG_PUFF_COUNT_MED:  usize = 14;
/// Puff count at High quality (density=1.0).
const FOG_PUFF_COUNT_HIGH: usize = 26;
const FOG_PUFF_RADIUS: f32   = 3.5;
const FOG_BASE_ALPHA:  f32   = 0.04;
/// Increased alpha for stormy/overcast weather.
const FOG_STORM_ALPHA: f32   = 0.07;
const FOG_DRIFT_FACTOR: f32  = 0.15;  // fraction of wind speed
const WRAP_RADIUS:      f32  = 90.0;

/// Spawn puffs between 5 and WRAP_RADIUS m from the chassis.
const SPAWN_MIN_R: f32 = 5.0;

/// Puffs float at 0–10 m above the terrain surface.
const SPAWN_Y_MIN: f32 = 0.5;
const SPAWN_Y_MAX: f32 = 10.0;

// Cycle values for Shift+G toggle.
const DENSITY_STEPS: [f32; 3] = [0.0, 0.3, 0.7];

// Fog puff colors per weather condition.
// Clear → near-white blue tint; Storm → warm grey for thick murk.
const COLOR_CLEAR:    (f32, f32, f32) = (0.92, 0.93, 0.96);
const COLOR_OVERCAST: (f32, f32, f32) = (0.80, 0.80, 0.82);
const COLOR_STORM:    (f32, f32, f32) = (0.65, 0.65, 0.66);

// ---- LCG helpers --------------------------------------------------------------

#[inline]
fn lcg_next(seed: &mut u32) -> f32 {
    *seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    *seed as f32 / u32::MAX as f32
}

// ---- System: Shift+G density cycle -------------------------------------------

fn toggle_fog_density(
    keys:        Res<ButtonInput<KeyCode>>,
    mut density: ResMut<FogDensity>,
    mut step:    Local<usize>,
) {
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if shift && keys.just_pressed(KeyCode::KeyG) {
        *step = (*step + 1) % DENSITY_STEPS.len();
        density.0 = DENSITY_STEPS[*step];
        info!("[FogVolumetric] density → {:.1}", density.0);
    }
}

// ---- System: maintain puff pool ----------------------------------------------

fn manage_fog_puffs(
    mut commands:  Commands,
    mut meshes:    ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    density:       Res<FogDensity>,
    quality:       Res<GraphicsQuality>,
    vehicle:       Option<Res<VehicleRoot>>,
    chassis_q:     Query<&Transform, With<Chassis>>,
    puffs:         Query<Entity, With<FogPuff>>,
    mut seed:      Local<u32>,
) {
    // Low tier: no volumetric puffs.
    if *quality == GraphicsQuality::Low { return; }

    let tier_max = match *quality {
        GraphicsQuality::Low    => 0,
        GraphicsQuality::Medium => FOG_PUFF_COUNT_MED,
        GraphicsQuality::High   => FOG_PUFF_COUNT_HIGH,
    };

    let target = if density.0 <= 0.0 {
        0
    } else {
        ((tier_max as f32) * density.0.sqrt()) as usize
    };

    let existing = puffs.iter().count();

    // Despawn excess puffs when density drops.
    if existing > target {
        for entity in puffs.iter().take(existing - target) {
            commands.entity(entity).despawn();
        }
        return;
    }

    if existing >= target { return; }

    let chassis_pos = if let Some(ref vr) = vehicle {
        if let Ok(tf) = chassis_q.get(vr.chassis) { tf.translation } else { Vec3::ZERO }
    } else {
        Vec3::ZERO
    };

    if *seed == 0 { *seed = 0xF06_F060; }

    let mesh = meshes.add(Sphere::new(FOG_PUFF_RADIUS));
    let mat  = materials.add(StandardMaterial {
        base_color:   Color::srgba(0.92, 0.93, 0.95, FOG_BASE_ALPHA),
        alpha_mode:   AlphaMode::Blend,
        unlit:        true,
        double_sided: true,
        cull_mode:    None,
        ..default()
    });

    for _ in 0..(target - existing) {
        // Scatter in a ring around the chassis.
        let angle = lcg_next(&mut *seed) * std::f32::consts::TAU;
        let r     = SPAWN_MIN_R + lcg_next(&mut *seed) * (WRAP_RADIUS - SPAWN_MIN_R);
        let px    = chassis_pos.x + angle.cos() * r;
        let pz    = chassis_pos.z + angle.sin() * r;
        let py    = chassis_pos.y + SPAWN_Y_MIN + lcg_next(&mut *seed) * (SPAWN_Y_MAX - SPAWN_Y_MIN);

        commands.spawn((
            FogPuff,
            Mesh3d(mesh.clone()),
            MeshMaterial3d(mat.clone()),
            Transform::from_translation(Vec3::new(px, py, pz)),
        ));
    }
}

// ---- System: drift puffs with wind -------------------------------------------

fn drift_fog_puffs(
    time:      Res<Time>,
    wind:      Option<Res<WindState>>,
    density:   Res<FogDensity>,
    quality:   Res<GraphicsQuality>,
    mut puffs: Query<&mut Transform, With<FogPuff>>,
) {
    if *quality == GraphicsQuality::Low { return; }
    if density.0 <= 0.0 { return; }

    let dt = time.delta_secs();
    let (dir, spd) = if let Some(ref w) = wind {
        (w.direction, w.speed_mps)
    } else {
        (Vec3::X, 2.0)
    };

    let delta = dir * spd * FOG_DRIFT_FACTOR * dt;
    for mut tf in &mut puffs {
        tf.translation += delta;
    }
}

// ---- System: wrap puffs that drift too far -----------------------------------

fn wrap_fog_puffs(
    mut commands: Commands,
    mut meshes:   ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    density:      Res<FogDensity>,
    quality:      Res<GraphicsQuality>,
    vehicle:      Option<Res<VehicleRoot>>,
    chassis_q:    Query<&Transform, With<Chassis>>,
    puffs:        Query<(Entity, &Transform), With<FogPuff>>,
    mut seed:     Local<u32>,
) {
    if *quality == GraphicsQuality::Low { return; }
    if density.0 <= 0.0 { return; }

    let chassis_pos = if let Some(ref vr) = vehicle {
        if let Ok(tf) = chassis_q.get(vr.chassis) { tf.translation } else { Vec3::ZERO }
    } else {
        Vec3::ZERO
    };

    if *seed == 0 { *seed = 0xABCD_EF01; }

    let mesh = meshes.add(Sphere::new(FOG_PUFF_RADIUS));
    let mat  = materials.add(StandardMaterial {
        base_color:   Color::srgba(0.92, 0.93, 0.95, FOG_BASE_ALPHA),
        alpha_mode:   AlphaMode::Blend,
        unlit:        true,
        double_sided: true,
        cull_mode:    None,
        ..default()
    });

    for (entity, tf) in &puffs {
        let dx = tf.translation.x - chassis_pos.x;
        let dz = tf.translation.z - chassis_pos.z;
        let dist = (dx * dx + dz * dz).sqrt();

        if dist > WRAP_RADIUS {
            // Despawn and respawn near the chassis on the opposite side.
            commands.entity(entity).despawn();

            let angle = lcg_next(&mut *seed) * std::f32::consts::TAU;
            let r     = SPAWN_MIN_R + lcg_next(&mut *seed) * 20.0; // close re-entry
            let px    = chassis_pos.x + angle.cos() * r;
            let pz    = chassis_pos.z + angle.sin() * r;
            let py    = chassis_pos.y + SPAWN_Y_MIN
                + lcg_next(&mut *seed) * (SPAWN_Y_MAX - SPAWN_Y_MIN);

            commands.spawn((
                FogPuff,
                Mesh3d(mesh.clone()),
                MeshMaterial3d(mat.clone()),
                Transform::from_translation(Vec3::new(px, py, pz)),
            ));
        }
    }
}

// ---- System: weather-responsive puff color (High only) ----------------------

/// On High quality, lerps puff base_color toward the target for the current
/// WeatherCondition, giving the fog a warm grey hue during storms and a clean
/// blue-white tint on clear days. Reads WeatherState (read-only).
fn update_puff_colors(
    quality:   Res<GraphicsQuality>,
    density:   Res<FogDensity>,
    weather:   Option<Res<WeatherState>>,
    puffs:     Query<&MeshMaterial3d<StandardMaterial>, With<FogPuff>>,
    mut mats:  ResMut<Assets<StandardMaterial>>,
    time:      Res<Time>,
) {
    // Only on High; Low/Medium get the static clear-day color.
    if *quality != GraphicsQuality::High { return; }
    if density.0 <= 0.0 { return; }

    // Read weather condition (optional — weather_director may not be registered
    // in all contexts, e.g. headless harness).
    let (target_r, target_g, target_b, target_a) = match weather.as_deref() {
        None => {
            let (r, g, b) = COLOR_CLEAR;
            (r, g, b, FOG_BASE_ALPHA)
        }
        Some(ws) => match ws.condition {
            WeatherCondition::Clear | WeatherCondition::Clearing => {
                let (r, g, b) = COLOR_CLEAR;
                (r, g, b, FOG_BASE_ALPHA)
            }
            WeatherCondition::Cloudy | WeatherCondition::Overcast => {
                let t = ws.intensity;
                let (cr, cg, cb) = COLOR_CLEAR;
                let (or_, og, ob) = COLOR_OVERCAST;
                (
                    cr + (or_ - cr) * t,
                    cg + (og - cg) * t,
                    cb + (ob - cb) * t,
                    FOG_BASE_ALPHA + (FOG_STORM_ALPHA - FOG_BASE_ALPHA) * t * 0.5,
                )
            }
            WeatherCondition::Rain | WeatherCondition::Storm => {
                let t = ws.intensity;
                let (or_, og, ob) = COLOR_OVERCAST;
                let (sr, sg, sb) = COLOR_STORM;
                (
                    or_ + (sr - or_) * t,
                    og  + (sg - og)  * t,
                    ob  + (sb - ob)  * t,
                    FOG_BASE_ALPHA + (FOG_STORM_ALPHA - FOG_BASE_ALPHA) * t,
                )
            }
        },
    };

    let dt      = time.delta_secs();
    let lerp_t  = (dt * 0.5).clamp(0.0, 1.0); // gentle 2-second lerp

    for mat_handle in &puffs {
        if let Some(mat) = mats.get_mut(&mat_handle.0) {
            let cur = mat.base_color.to_srgba();
            mat.base_color = Color::srgba(
                cur.red   + (target_r - cur.red)   * lerp_t,
                cur.green + (target_g - cur.green)  * lerp_t,
                cur.blue  + (target_b - cur.blue)   * lerp_t,
                cur.alpha + (target_a - cur.alpha)  * lerp_t,
            );
        }
    }
}
