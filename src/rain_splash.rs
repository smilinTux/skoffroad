// Rain splashes: flat ring/ripple decals that appear on the ground (and the
// chassis roof) whenever the storm is active.  Triggered by StormState.active
// (which corresponds to any rain intensity > 0 in the existing system).
//
// Sprint 70 — Effect 1
//
// Public API:
//   RainSplashPlugin
//   RainIntensity (resource — 0.0 = dry, 1.0 = downpour)
//
// Tuning knobs:
//   SPLASH_RADIUS          — ground scatter radius around chassis (m)
//   SPLASH_LIFETIME_SECS   — how long each splash lives (s)
//   SPLASH_CAP             — maximum simultaneous ground splashes
//   ROOF_SPLASHES_PER_FRAME — roof-batch size regardless of intensity

use bevy::prelude::*;

use crate::storm::StormState;
use crate::vehicle::{Chassis, VehicleRoot};
use crate::terrain::terrain_height_at;

// ---- Public API ---------------------------------------------------------------

pub struct RainSplashPlugin;

impl Plugin for RainSplashPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RainIntensity>()
           .add_systems(Update, (
               sync_rain_intensity,
               spawn_ground_splashes,
               spawn_roof_splashes,
               tick_splashes,
           ));
    }
}

/// Rain intensity in [0.0, 1.0].  Automatically driven by StormState but can
/// also be set externally (e.g. by a future weather-intensity system).
#[derive(Resource, Default, Clone, Copy)]
pub struct RainIntensity(pub f32);

// ---- Internal components ------------------------------------------------------

/// Marker + lifetime tracker on each splash entity.
#[derive(Component)]
struct RainSplash {
    age:      f32,
    lifetime: f32,
    /// Handle to the material so we can update alpha without a separate query.
    mat:      Handle<StandardMaterial>,
}

// ---- Constants ---------------------------------------------------------------

/// Horizontal scatter radius for ground splashes around the chassis (m).
const SPLASH_RADIUS: f32 = 40.0;

/// How long a single splash lives before being despawned (s).
const SPLASH_LIFETIME_SECS: f32 = 0.5;

/// Cap on simultaneous ground splashes.
const SPLASH_CAP: usize = 200;

/// Roof splash batch size per frame (independent of intensity).
const ROOF_SPLASHES_PER_FRAME: usize = 2;

/// Cap on roof splashes (they age out quickly so this stays low).
const ROOF_SPLASH_CAP: usize = 20;

/// Splash disc geometry: Cylinder height (very thin), radius.
const DISC_HEIGHT: f32 = 0.005;
const DISC_RADIUS: f32 = 0.05;

/// Base alpha when a splash is freshly spawned.
const SPLASH_START_ALPHA: f32 = 0.55;

/// Chassis-local Y offset so the roof splash sits atop the cab.
const ROOF_Y: f32 = 0.90;

// ---- System: sync intensity from StormState ----------------------------------

fn sync_rain_intensity(
    storm:     Option<Res<StormState>>,
    mut intensity: ResMut<RainIntensity>,
) {
    let active = storm.map(|s| s.active).unwrap_or(false);
    // Simple mapping: storm active = full intensity; storm off = 0.
    // A future weather system can write finer-grained values directly.
    intensity.0 = if active { 1.0 } else { 0.0 };
}

// ---- LCG helpers (no external deps) -----------------------------------------

#[inline]
fn lcg_next(seed: &mut u32) -> f32 {
    *seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    *seed as f32 / u32::MAX as f32
}

#[inline]
fn lcg_signed(seed: &mut u32) -> f32 {
    lcg_next(seed) * 2.0 - 1.0
}

// ---- System: spawn ground splashes -------------------------------------------

fn spawn_ground_splashes(
    mut commands:   Commands,
    mut meshes:     ResMut<Assets<Mesh>>,
    mut materials:  ResMut<Assets<StandardMaterial>>,
    intensity:      Res<RainIntensity>,
    vehicle:        Option<Res<VehicleRoot>>,
    chassis_q:      Query<&Transform, With<Chassis>>,
    splashes:       Query<(), With<RainSplash>>,
    mut seed:       Local<u32>,
) {
    if intensity.0 <= 0.0 { return; }

    let existing = splashes.iter().count();
    if existing >= SPLASH_CAP { return; }

    // Spawn rate: 1–5 per frame scaled by intensity.
    let max_per_frame = (1.0 + intensity.0 * 4.0) as usize;
    let to_spawn = max_per_frame.min(SPLASH_CAP - existing);

    // Get chassis position for centering the scatter.
    let chassis_pos = if let Some(ref vr) = vehicle {
        if let Ok(tf) = chassis_q.get(vr.chassis) {
            tf.translation
        } else {
            Vec3::ZERO
        }
    } else {
        Vec3::ZERO
    };

    let mesh   = meshes.add(Cylinder::new(DISC_RADIUS, DISC_HEIGHT));
    let mat_h  = materials.add(StandardMaterial {
        base_color:   Color::srgba(0.55, 0.75, 1.0, SPLASH_START_ALPHA),
        alpha_mode:   AlphaMode::Blend,
        unlit:        true,
        double_sided: true,
        cull_mode:    None,
        ..default()
    });

    if *seed == 0 {
        *seed = 0xBEEF_CAFE;
    }

    for _ in 0..to_spawn {
        let rx = lcg_signed(&mut *seed) * SPLASH_RADIUS;
        let rz = lcg_signed(&mut *seed) * SPLASH_RADIUS;
        let gx = chassis_pos.x + rx;
        let gz = chassis_pos.z + rz;
        let gy = terrain_height_at(gx, gz) + 0.01; // just above terrain

        commands.spawn((
            RainSplash {
                age:      0.0,
                lifetime: SPLASH_LIFETIME_SECS,
                mat:      mat_h.clone(),
            },
            Mesh3d(mesh.clone()),
            MeshMaterial3d(mat_h.clone()),
            Transform::from_translation(Vec3::new(gx, gy, gz)),
        ));
    }
}

// ---- System: spawn roof splashes ---------------------------------------------

/// Marker that differentiates roof splashes from ground splashes.
#[derive(Component)]
struct RoofSplash;

fn spawn_roof_splashes(
    mut commands:   Commands,
    mut meshes:     ResMut<Assets<Mesh>>,
    mut materials:  ResMut<Assets<StandardMaterial>>,
    intensity:      Res<RainIntensity>,
    vehicle:        Option<Res<VehicleRoot>>,
    chassis_q:      Query<&Transform, With<Chassis>>,
    roof_splashes:  Query<(), With<RoofSplash>>,
    mut seed:       Local<u32>,
) {
    if intensity.0 <= 0.0 { return; }

    let existing = roof_splashes.iter().count();
    if existing >= ROOF_SPLASH_CAP { return; }

    let Some(ref vr) = vehicle else { return };
    let Ok(chassis_tf) = chassis_q.get(vr.chassis) else { return };
    let chassis_pos = chassis_tf.translation;

    let mesh  = meshes.add(Cylinder::new(DISC_RADIUS * 0.6, DISC_HEIGHT));
    let mat_h = materials.add(StandardMaterial {
        base_color:   Color::srgba(0.6, 0.8, 1.0, SPLASH_START_ALPHA * 0.7),
        alpha_mode:   AlphaMode::Blend,
        unlit:        true,
        double_sided: true,
        cull_mode:    None,
        ..default()
    });

    if *seed == 0 {
        *seed = 0xCAFE_1234;
    }

    let to_spawn = ROOF_SPLASHES_PER_FRAME.min(ROOF_SPLASH_CAP - existing);

    for _ in 0..to_spawn {
        // Scatter along the roof (X: ±0.5 m, Z: ±0.8 m).
        let rx = lcg_signed(&mut *seed) * 0.5;
        let rz = lcg_signed(&mut *seed) * 0.8;

        let pos = Vec3::new(
            chassis_pos.x + rx,
            chassis_pos.y + ROOF_Y,
            chassis_pos.z + rz,
        );

        commands.spawn((
            RoofSplash,
            RainSplash {
                age:      0.0,
                lifetime: SPLASH_LIFETIME_SECS * 0.6,
                mat:      mat_h.clone(),
            },
            Mesh3d(mesh.clone()),
            MeshMaterial3d(mat_h.clone()),
            Transform::from_translation(pos),
        ));
    }
}

// ---- System: age + fade splashes ---------------------------------------------

fn tick_splashes(
    mut commands:   Commands,
    time:           Res<Time>,
    mut materials:  ResMut<Assets<StandardMaterial>>,
    mut splashes:   Query<(Entity, &mut RainSplash)>,
) {
    let dt = time.delta_secs();
    for (entity, mut splash) in &mut splashes {
        splash.age += dt;
        let t = (splash.age / splash.lifetime).clamp(0.0, 1.0);

        // Fade alpha from SPLASH_START_ALPHA → 0 over the lifetime.
        if let Some(mat) = materials.get_mut(&splash.mat) {
            let a = SPLASH_START_ALPHA * (1.0 - t);
            mat.base_color.set_alpha(a);
        }

        if splash.age >= splash.lifetime {
            commands.entity(entity).despawn();
        }
    }
}
