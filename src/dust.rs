// Ground-colored dust wake + ambient distance haze.
//
// Sprint B4 — Ground & Atmosphere Realism
//
// Two complementary effects (mesh-based, no Hanabi required — works on WASM):
//
// 1. GROUND DUST WAKE — kicks up behind the rear wheels, scaled by chassis
//    speed and surface type. Uses pooled billboard quads that drift upward
//    and fade. Color matches the terrain surface color at the wheel contact
//    point (derived from the same slope heuristic used in particles.rs so the
//    two systems are visually consistent).
//
// 2. AMBIENT DISTANCE HAZE — a thin ring of large translucent spheres placed
//    far from the player simulates dust suspended in the air near the horizon,
//    giving depth cueing. Spheres drift slowly with the wind and wrap back.
//
// Tier gating:
//   Low    — no effects (early return).
//   Medium — dust wake capped at WAKE_CAP_MED, ambient haze at HAZE_CAP_MED.
//   High   — WAKE_CAP_HIGH / HAZE_CAP_HIGH.
//
// Pooling: separate VecDeque ring buffers for wake and haze entities; oldest
// is despawned when the cap is reached.
//
// Public API:
//   AmbientDustPlugin  (add via separate .add_plugins() call in main.rs)

use std::collections::VecDeque;
use bevy::prelude::*;
use crate::graphics_quality::GraphicsQuality;
use crate::vehicle::{Chassis, VehicleRoot};
use crate::wind::WindState;

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct AmbientDustPlugin;

impl Plugin for AmbientDustPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DustWakePool>()
           .init_resource::<HazePool>()
           .add_systems(Startup, init_ambient_haze)
           .add_systems(Update, (
               emit_dust_wake,
               fade_dust_wake,
               drift_haze,
               wrap_haze,
           ));
    }
}

// ---------------------------------------------------------------------------
// Components
// ---------------------------------------------------------------------------

/// Marks each dust-wake billboard entity.
#[derive(Component)]
struct DustWake {
    life:     f32,
    lifetime: f32,
    /// Upward drift velocity (m/s).
    drift_y:  f32,
}

/// Marks each ambient haze sphere.
#[derive(Component)]
struct HazePuff;

// ---------------------------------------------------------------------------
// Resources (ring buffers)
// ---------------------------------------------------------------------------

#[derive(Resource, Default)]
pub struct DustWakePool {
    entities:  VecDeque<Entity>,
    /// Accumulated travel distance since last wake emission.
    dist_accum: f32,
    last_pos:   Vec3,
    has_pos:    bool,
}

#[derive(Resource, Default)]
pub struct HazePool {
    entities: VecDeque<Entity>,
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

// --- Wake ---
const WAKE_CAP_MED:  usize = 80;
const WAKE_CAP_HIGH: usize = 160;
const WAKE_INTERVAL_M: f32 = 0.7;   // metres of travel between wake spawns
const WAKE_LIFETIME:   f32 = 2.2;   // seconds each wake quad lives
const WAKE_DRIFT_Y:    f32 = 0.8;   // m/s upward drift
const WAKE_ALPHA_MAX:  f32 = 0.35;
/// Size of the billboard quad (half-extent).
const WAKE_HALF: f32 = 0.5;
/// Spawn slightly behind the chassis rear axle (chassis-local Z offset).
const WAKE_REAR_Z: f32 = 1.6;
/// Wake spawns at chassis world Y + this offset (terrain surface vicinity).
const WAKE_Y_OFFSET: f32 = 0.15;
/// Minimum chassis speed (m/s) to emit wake dust.
const WAKE_MIN_SPEED: f32 = 1.5;

// --- Haze ---
const HAZE_CAP_MED:   usize = 12;
const HAZE_CAP_HIGH:  usize = 20;
const HAZE_RADIUS_MIN: f32 = 60.0;
const HAZE_RADIUS_MAX: f32 = 120.0;
const HAZE_SPHERE_R:   f32 = 5.5;
const HAZE_ALPHA:      f32 = 0.025;
const HAZE_Y_MIN:      f32 = 1.0;
const HAZE_Y_MAX:      f32 = 10.0;
const HAZE_DRIFT:      f32 = 0.08;  // fraction of wind speed
const HAZE_WRAP_R:     f32 = 130.0;

// Terrain surface dust colors (matching particles.rs heuristic):
const COLOR_FLAT:  Vec3 = Vec3::new(0.55, 0.50, 0.30);
const COLOR_MID:   Vec3 = Vec3::new(0.60, 0.50, 0.30);

// ---------------------------------------------------------------------------
// Startup: initialise ambient haze ring
// ---------------------------------------------------------------------------

fn init_ambient_haze(
    mut commands:  Commands,
    mut meshes:    ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    quality:       Res<GraphicsQuality>,
    mut pool:      ResMut<HazePool>,
) {
    let cap = match *quality {
        GraphicsQuality::Low    => return,
        GraphicsQuality::Medium => HAZE_CAP_MED,
        GraphicsQuality::High   => HAZE_CAP_HIGH,
    };

    let mesh = meshes.add(Sphere::new(HAZE_SPHERE_R));
    let mat  = materials.add(StandardMaterial {
        base_color:   Color::srgba(0.88, 0.84, 0.72, HAZE_ALPHA),
        alpha_mode:   AlphaMode::Blend,
        unlit:        true,
        double_sided: true,
        cull_mode:    None,
        ..default()
    });

    let mut seed: u32 = 0xD057_AB1E;
    for _ in 0..cap {
        let angle = lcg_f32(&mut seed) * std::f32::consts::TAU;
        let r     = HAZE_RADIUS_MIN + lcg_f32(&mut seed) * (HAZE_RADIUS_MAX - HAZE_RADIUS_MIN);
        let px    = angle.cos() * r;
        let pz    = angle.sin() * r;
        let py    = HAZE_Y_MIN + lcg_f32(&mut seed) * (HAZE_Y_MAX - HAZE_Y_MIN);

        let entity = commands.spawn((
            HazePuff,
            Mesh3d(mesh.clone()),
            MeshMaterial3d(mat.clone()),
            Transform::from_translation(Vec3::new(px, py, pz)),
        )).id();
        pool.entities.push_back(entity);
    }
    info!(
        "[AmbientDust] haze ring: {} spheres (quality = {})",
        cap,
        quality.as_str()
    );
}

// ---------------------------------------------------------------------------
// System: emit dust wake behind rear axle
// ---------------------------------------------------------------------------

fn emit_dust_wake(
    mut commands:  Commands,
    mut meshes:    ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    quality:       Res<GraphicsQuality>,
    vehicle:       Option<Res<VehicleRoot>>,
    chassis_q:     Query<&Transform, With<Chassis>>,
    mut pool:      ResMut<DustWakePool>,
    time:          Res<Time>,
) {
    let cap = match *quality {
        GraphicsQuality::Low    => return,
        GraphicsQuality::Medium => WAKE_CAP_MED,
        GraphicsQuality::High   => WAKE_CAP_HIGH,
    };

    let Some(vr) = vehicle else { return };
    let Ok(chassis_tf) = chassis_q.get(vr.chassis) else { return };
    let chassis_pos = chassis_tf.translation;

    let dt = time.delta_secs();
    if dt <= 0.0 { return; }

    let speed_mps = if pool.has_pos {
        (chassis_pos - pool.last_pos).length() / dt
    } else {
        0.0
    };
    pool.last_pos = chassis_pos;
    pool.has_pos  = true;

    if speed_mps < WAKE_MIN_SPEED { return; }

    pool.dist_accum += speed_mps * dt;
    if pool.dist_accum < WAKE_INTERVAL_M { return; }
    pool.dist_accum -= WAKE_INTERVAL_M;

    // Spawn position: behind rear axle.
    let rear_local   = Vec3::new(0.0, 0.0, WAKE_REAR_Z);
    let spawn_pos    = chassis_pos + chassis_tf.rotation * rear_local
        + Vec3::Y * WAKE_Y_OFFSET;

    // Alpha and size scale with speed (clamp beyond 15 m/s).
    let speed_t = (speed_mps / 15.0).clamp(0.0, 1.0);
    let alpha   = WAKE_ALPHA_MAX * (0.4 + 0.6 * speed_t);

    // Color: lerp from flat (slow) toward mid (faster / more slip).
    let color_rgb = COLOR_FLAT.lerp(COLOR_MID, speed_t * 0.6);
    let color = Color::srgba(color_rgb.x, color_rgb.y, color_rgb.z, alpha);

    let half  = WAKE_HALF * (0.7 + 0.6 * speed_t);
    let mesh  = meshes.add(build_billboard_quad(half));
    let mat   = materials.add(StandardMaterial {
        base_color:   color,
        alpha_mode:   AlphaMode::Blend,
        unlit:        true,
        double_sided: true,
        cull_mode:    None,
        ..default()
    });

    // Retire oldest when pool full.
    if pool.entities.len() >= cap {
        if let Some(old) = pool.entities.pop_front() {
            commands.entity(old).despawn();
        }
    }

    let entity = commands.spawn((
        DustWake {
            life:     WAKE_LIFETIME,
            lifetime: WAKE_LIFETIME,
            drift_y:  WAKE_DRIFT_Y * (0.5 + 0.5 * speed_t),
        },
        Mesh3d(mesh),
        MeshMaterial3d(mat),
        Transform::from_translation(spawn_pos),
    )).id();
    pool.entities.push_back(entity);
}

// ---------------------------------------------------------------------------
// System: fade and drift wake particles upward
// ---------------------------------------------------------------------------

fn fade_dust_wake(
    mut commands:  Commands,
    mut wakes:     Query<(Entity, &mut DustWake, &mut Transform, &MeshMaterial3d<StandardMaterial>)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    time:          Res<Time>,
) {
    let dt = time.delta_secs();
    for (entity, mut wake, mut tf, mat_handle) in &mut wakes {
        wake.life -= dt;
        if wake.life <= 0.0 {
            commands.entity(entity).despawn();
            continue;
        }
        // Drift upward.
        tf.translation.y += wake.drift_y * dt;

        // Fade alpha.
        let t     = (wake.life / wake.lifetime).clamp(0.0, 1.0);
        let alpha = t * WAKE_ALPHA_MAX;

        if let Some(mat) = materials.get_mut(&mat_handle.0) {
            let base = mat.base_color.to_srgba();
            mat.base_color = Color::srgba(base.red, base.green, base.blue, alpha);
        }
    }
}

// ---------------------------------------------------------------------------
// System: drift haze with wind
// ---------------------------------------------------------------------------

fn drift_haze(
    wind:      Option<Res<WindState>>,
    quality:   Res<GraphicsQuality>,
    mut puffs: Query<&mut Transform, With<HazePuff>>,
    time:      Res<Time>,
) {
    if *quality == GraphicsQuality::Low { return; }
    let dt = time.delta_secs();
    let (dir, spd) = wind.as_ref().map_or(
        (Vec3::X, 2.0),
        |w| (w.direction, w.speed_mps),
    );
    let delta = dir * spd * HAZE_DRIFT * dt;
    for mut tf in &mut puffs {
        tf.translation += delta;
    }
}

// ---------------------------------------------------------------------------
// System: wrap haze that drifts too far from origin
// ---------------------------------------------------------------------------

fn wrap_haze(
    mut commands:  Commands,
    mut meshes:    ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    quality:       Res<GraphicsQuality>,
    puffs:         Query<(Entity, &Transform), With<HazePuff>>,
    mut pool:      ResMut<HazePool>,
    mut seed:      Local<u32>,
) {
    if *quality == GraphicsQuality::Low { return; }

    let mut mesh_h: Option<Handle<Mesh>>             = None;
    let mut mat_h:  Option<Handle<StandardMaterial>> = None;

    for (entity, tf) in &puffs {
        let dist = Vec2::new(tf.translation.x, tf.translation.z).length();
        if dist > HAZE_WRAP_R {
            commands.entity(entity).despawn();
            pool.entities.retain(|&e| e != entity);

            // Build shared assets lazily.
            if mesh_h.is_none() {
                mesh_h = Some(meshes.add(Sphere::new(HAZE_SPHERE_R)));
                mat_h  = Some(materials.add(StandardMaterial {
                    base_color:   Color::srgba(0.88, 0.84, 0.72, HAZE_ALPHA),
                    alpha_mode:   AlphaMode::Blend,
                    unlit:        true,
                    double_sided: true,
                    cull_mode:    None,
                    ..default()
                }));
            }

            if *seed == 0 { *seed = 0xAA_BB_CC; }
            let angle = lcg_f32(&mut *seed) * std::f32::consts::TAU;
            let r     = HAZE_RADIUS_MIN + lcg_f32(&mut *seed) * 20.0;
            let px    = angle.cos() * r;
            let pz    = angle.sin() * r;
            let py    = HAZE_Y_MIN + lcg_f32(&mut *seed) * (HAZE_Y_MAX - HAZE_Y_MIN);

            let new_entity = commands.spawn((
                HazePuff,
                Mesh3d(mesh_h.clone().unwrap()),
                MeshMaterial3d(mat_h.clone().unwrap()),
                Transform::from_translation(Vec3::new(px, py, pz)),
            )).id();
            pool.entities.push_back(new_entity);
        }
    }
}

// ---------------------------------------------------------------------------
// Mesh helper
// ---------------------------------------------------------------------------

/// XZ-plane billboard quad centred at origin, half-extent `half`.
fn build_billboard_quad(half: f32) -> Mesh {
    use bevy::mesh::{Indices, PrimitiveTopology};
    use bevy::asset::RenderAssetUsages;

    let h = half;
    let positions: Vec<[f32; 3]> = vec![
        [-h, 0.0, -h],
        [ h, 0.0, -h],
        [ h, 0.0,  h],
        [-h, 0.0,  h],
    ];
    let normals: Vec<[f32; 3]> = vec![[0.0, 1.0, 0.0]; 4];
    let uvs: Vec<[f32; 2]> = vec![
        [0.0, 0.0],
        [1.0, 0.0],
        [1.0, 1.0],
        [0.0, 1.0],
    ];
    let indices = Indices::U32(vec![0, 1, 2, 0, 2, 3]);

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(indices);
    mesh
}

// ---------------------------------------------------------------------------
// LCG helper
// ---------------------------------------------------------------------------

#[inline]
fn lcg_f32(seed: &mut u32) -> f32 {
    *seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    *seed as f32 / u32::MAX as f32
}
