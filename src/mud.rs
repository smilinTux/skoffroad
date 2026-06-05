// mud.rs — Sprint B4 Enhanced
//
// Additions over the original Sprint 65 implementation:
//
// 1. MUD SHEEN — the mud patch material gains a wet specular sheen that
//    brightens when it rains (WeatherState.intensity > 0) and dims in clear
//    weather.  This is achieved by live-editing the mud StandardMaterial's
//    perceptual_roughness and emissive each frame (Medium+ only).
//
// 2. MUD SPRAY — when the chassis enters a mud zone at speed >= MIN_SPRAY_MPS,
//    small billboard quads are spawned in a fan behind the chassis simulating
//    mud splash.  They are pooled in a VecDeque capped at MUD_SPRAY_CAP.
//
// Tier gating:
//   Low    — no sheen updates, no mud spray.
//   Medium — sheen updates; mud spray capped at MUD_SPRAY_CAP_MED.
//   High   — sheen + mud spray capped at MUD_SPRAY_CAP_HIGH.

use bevy::prelude::*;
use avian3d::prelude::*;
use noise::{NoiseFn, Perlin};
use std::collections::VecDeque;

use crate::terrain::{terrain_height_at, TERRAIN_SEED};
use crate::vehicle::{Chassis, VehicleRoot};
use crate::graphics_quality::GraphicsQuality;
use crate::weather_director::WeatherState;

pub struct MudPlugin;

impl Plugin for MudPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MudActive>()
           .init_resource::<MudSheenState>()
           .init_resource::<MudSprayPool>()
           .add_systems(Startup, spawn_mud_patches)
           .add_systems(Update, (
               update_mud_sheen,
               emit_mud_spray,
               fade_mud_spray,
           ))
           .add_systems(PhysicsSchedule,
               apply_mud_drag
                   .after(PhysicsStepSystems::NarrowPhase)
                   .before(PhysicsStepSystems::Solver),
           );
    }
}

// ---------------------------------------------------------------------------
// Resources / Components
// ---------------------------------------------------------------------------

#[derive(Resource, Default)]
pub struct MudActive {
    /// True if the chassis is currently in any mud patch.
    pub in_mud: bool,
    /// 0..=1, max submersion across all overlapping patches this frame.
    pub max_submersion: f32,
}

/// Marker placed on each mud patch entity.
#[derive(Component)]
pub struct MudZone {
    pub radius: f32,
}

/// Tracks last-baked sheen value so we don't update materials every frame.
#[derive(Resource, Default)]
struct MudSheenState {
    last_roughness: f32,
}

/// Marks each mud-spray splash entity.
#[derive(Component)]
struct MudSplash {
    life:     f32,
    lifetime: f32,
}

/// Ring-buffer pool for mud splash entities.
#[derive(Resource, Default)]
struct MudSprayPool {
    entities:   VecDeque<Entity>,
    dist_accum: f32,
    last_pos:   Vec3,
    has_pos:    bool,
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

// --- Spray ---
const MUD_SPRAY_CAP_MED:  usize = 40;
const MUD_SPRAY_CAP_HIGH: usize = 80;
const MUD_SPRAY_INTERVAL_M: f32 = 0.4;
const MUD_SPRAY_LIFETIME:   f32 = 1.2;
const MUD_SPRAY_HALF:       f32 = 0.28;
const MUD_SPRAY_ALPHA_MAX:  f32 = 0.65;
const MUD_SPRAY_DRIFT_Y:    f32 = 1.2;
const MIN_SPRAY_MPS:        f32 = 1.8;

const WORLD_HALF: f32 = 90.0;
// Minimum XZ distance from origin so we don't drown the spawn point.
const SPAWN_CLEAR_RADIUS: f32 = 15.0;
// Target number of mud patches. The LCG loop tries this many candidates and
// skips any that land inside SPAWN_CLEAR_RADIUS.
const PATCH_COUNT: usize = 12;
// Mud patches seeded distinctly from trees (+1) and rocks (+2).
const MUD_SEED: u32 = TERRAIN_SEED + 7;
// Drag coefficient (N per unit submersion). Reduced 400 → 120 after
// playtest: 400 stacked across overlapping patches pinned the chassis.
// 120 still feels viscous but chassis can power through.
const MUD_DRAG_COEFF: f32 = 120.0;
// Chassis mass mirrored from vehicle.rs — used for the slight sinking force.
const CHASSIS_MASS: f32 = 1500.0;

// ---------------------------------------------------------------------------
// Startup: spawn mud patches
// ---------------------------------------------------------------------------

fn spawn_mud_patches(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Muddy-brown material: highly rough, faintly emissive so it reads against
    // the varied terrain colours without being garish.
    let mud_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.30, 0.20, 0.12),
        perceptual_roughness: 0.95,
        emissive: LinearRgba::new(0.04, 0.025, 0.008, 1.0),
        ..default()
    });

    // LCG state seeded from MUD_SEED. Two independent streams for x/z so
    // the patch positions aren't correlated along the diagonal.
    let mut lcg_x = lcg_init(MUD_SEED);
    let mut lcg_z = lcg_init(MUD_SEED ^ 0xDEAD_BEEF);
    let mut lcg_r = lcg_init(MUD_SEED.wrapping_add(13));

    // Perlin noise used to bias placement toward low-lying wet areas
    // (low noise value → depression → more likely to hold water/mud).
    let perlin = Perlin::new(MUD_SEED);

    let mut spawned = 0usize;
    let mut attempts = 0usize;

    while spawned < PATCH_COUNT && attempts < PATCH_COUNT * 20 {
        attempts += 1;

        let (x, lcg_x2) = lcg_next(lcg_x);
        let (z, lcg_z2) = lcg_next(lcg_z);
        let (r_raw, lcg_r2) = lcg_next(lcg_r);
        lcg_x = lcg_x2;
        lcg_z = lcg_z2;
        lcg_r = lcg_r2;

        let wx = (x - 0.5) * 2.0 * WORLD_HALF;
        let wz = (z - 0.5) * 2.0 * WORLD_HALF;

        // Skip if too close to the spawn origin.
        if wx * wx + wz * wz < SPAWN_CLEAR_RADIUS * SPAWN_CLEAR_RADIUS {
            continue;
        }

        // Use Perlin noise to prefer low-lying depressions.  Patches with noise
        // < 0.35 (normalised 0..1) are discarded — those tend to be on ridges.
        let nx = (wx / 200.0 + 0.5) as f64;
        let nz = (wz / 200.0 + 0.5) as f64;
        let n_val = perlin.get([nx * 4.0, nz * 4.0]) as f32 * 0.5 + 0.5;
        if n_val < 0.35 {
            continue;
        }

        // Radius in [2.0, 6.0] m.
        let radius = 2.0 + r_raw * 4.0;

        let y = terrain_height_at(wx, wz) + 0.05;

        // Thin cylinder approximates a flat disk (height = 0.05 m).
        let mesh = meshes.add(Cylinder::new(radius, 0.05));

        commands.spawn((
            MudZone { radius },
            Mesh3d(mesh),
            MeshMaterial3d(mud_mat.clone()),
            Transform::from_translation(Vec3::new(wx, y, wz)),
        ));

        spawned += 1;
    }
}

// ---------------------------------------------------------------------------
// Physics: drag + sinking force
// ---------------------------------------------------------------------------

fn apply_mud_drag(
    vehicle: Option<Res<VehicleRoot>>,
    mut chassis_q: Query<(Forces, &Transform), With<Chassis>>,
    mud_zones: Query<(&Transform, &MudZone)>,
    mut mud_active: ResMut<MudActive>,
) {
    // Reset frame state.
    mud_active.in_mud = false;
    mud_active.max_submersion = 0.0;

    let Some(vehicle) = vehicle else { return };
    let Ok((mut forces, chassis_tf)) = chassis_q.get_mut(vehicle.chassis) else { return };

    let chassis_xz = Vec2::new(chassis_tf.translation.x, chassis_tf.translation.z);

    for (zone_tf, zone) in mud_zones.iter() {
        let zone_xz = Vec2::new(zone_tf.translation.x, zone_tf.translation.z);
        let dist = (chassis_xz - zone_xz).length();

        if dist >= zone.radius {
            continue;
        }

        // 1.0 at the centre, 0.0 at the edge.
        let submersion = 1.0 - dist / zone.radius;

        mud_active.in_mud = true;
        if submersion > mud_active.max_submersion {
            mud_active.max_submersion = submersion;
        }

        // Horizontal drag opposes the chassis's current XZ velocity.
        let vel = forces.linear_velocity();
        let drag_coeff = MUD_DRAG_COEFF * submersion;
        forces.apply_force(Vec3::new(
            -vel.x * drag_coeff,
            0.0,
            -vel.z * drag_coeff,
        ));

        // Slight downward press — chassis feels like it's sinking into the mire.
        // 1.0 multiplier was too aggressive; 0.3 gives a gentle suction without
        // burying the wheels.
        forces.apply_force(Vec3::new(0.0, -CHASSIS_MASS * 0.3 * submersion, 0.0));
    }
}

// ---------------------------------------------------------------------------
// Sprint B4: mud sheen + mud spray
// ---------------------------------------------------------------------------

/// Update the mud patch material roughness to simulate wet shine after rain.
/// Reads WeatherState (optional — gracefully absent in headless harness).
/// Medium+: updates roughness from 0.95 (dry) → 0.55 (soaked).
fn update_mud_sheen(
    quality:    Res<GraphicsQuality>,
    weather:    Option<Res<WeatherState>>,
    mud_zones:  Query<&MeshMaterial3d<StandardMaterial>, With<MudZone>>,
    mut mats:   ResMut<Assets<StandardMaterial>>,
    mut state:  ResMut<MudSheenState>,
) {
    if *quality == GraphicsQuality::Low { return; }

    let weather_intensity = weather.as_deref().map_or(0.0, |ws| ws.intensity);

    // Roughness: 0.95 dry → 0.55 fully wet. Emissive: adds subtle wet glint.
    let target_roughness = 0.95 - weather_intensity * 0.40;
    let emissive_scale   = weather_intensity * 0.06;

    // Only update if the change is meaningful.
    if (target_roughness - state.last_roughness).abs() < 0.01 { return; }
    state.last_roughness = target_roughness;

    for mat_handle in &mud_zones {
        if let Some(mat) = mats.get_mut(&mat_handle.0) {
            mat.perceptual_roughness = target_roughness;
            // Wet mud glints slightly under diffuse light.
            mat.emissive = LinearRgba::new(
                0.04 + emissive_scale,
                0.025 + emissive_scale * 0.6,
                0.008 + emissive_scale * 0.3,
                1.0,
            );
        }
    }
}

/// Emit mud splash billboards when in a mud zone at speed.
fn emit_mud_spray(
    mut commands:  Commands,
    mut meshes:    ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    quality:       Res<GraphicsQuality>,
    vehicle:       Option<Res<VehicleRoot>>,
    chassis_q:     Query<&Transform, With<Chassis>>,
    mud_active:    Res<MudActive>,
    mut pool:      ResMut<MudSprayPool>,
    time:          Res<Time>,
) {
    let cap = match *quality {
        GraphicsQuality::Low    => return,
        GraphicsQuality::Medium => MUD_SPRAY_CAP_MED,
        GraphicsQuality::High   => MUD_SPRAY_CAP_HIGH,
    };

    if !mud_active.in_mud { return; }

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

    if speed_mps < MIN_SPRAY_MPS { return; }

    pool.dist_accum += speed_mps * dt;
    if pool.dist_accum < MUD_SPRAY_INTERVAL_M { return; }
    pool.dist_accum -= MUD_SPRAY_INTERVAL_M;

    // Spray spawns slightly behind the chassis at ground level.
    let rear_local = Vec3::new(0.0, 0.05, 1.4);
    let spawn_pos  = chassis_pos + chassis_tf.rotation * rear_local;

    let alpha = (MUD_SPRAY_ALPHA_MAX * mud_active.max_submersion).clamp(0.15, MUD_SPRAY_ALPHA_MAX);
    let mud_color = Color::srgba(0.25, 0.17, 0.09, alpha);

    let mesh = meshes.add(build_spray_quad());
    let mat  = materials.add(StandardMaterial {
        base_color:   mud_color,
        alpha_mode:   AlphaMode::Blend,
        unlit:        true,
        double_sided: true,
        cull_mode:    None,
        ..default()
    });

    if pool.entities.len() >= cap {
        if let Some(old) = pool.entities.pop_front() {
            commands.entity(old).despawn();
        }
    }

    let entity = commands.spawn((
        MudSplash { life: MUD_SPRAY_LIFETIME, lifetime: MUD_SPRAY_LIFETIME },
        Mesh3d(mesh),
        MeshMaterial3d(mat),
        Transform::from_translation(spawn_pos),
    )).id();
    pool.entities.push_back(entity);
}

/// Fade and drift mud splash upward over their lifetime.
fn fade_mud_spray(
    mut commands:  Commands,
    mut splashes:  Query<(Entity, &mut MudSplash, &mut Transform, &MeshMaterial3d<StandardMaterial>)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    time:          Res<Time>,
) {
    let dt = time.delta_secs();
    for (entity, mut splash, mut tf, mat_handle) in &mut splashes {
        splash.life -= dt;
        if splash.life <= 0.0 {
            commands.entity(entity).despawn();
            continue;
        }
        tf.translation.y += MUD_SPRAY_DRIFT_Y * dt;
        let alpha = (splash.life / splash.lifetime).clamp(0.0, 1.0) * MUD_SPRAY_ALPHA_MAX;
        if let Some(mat) = materials.get_mut(&mat_handle.0) {
            let base = mat.base_color.to_srgba();
            mat.base_color = Color::srgba(base.red, base.green, base.blue, alpha);
        }
    }
}

fn build_spray_quad() -> Mesh {
    use bevy::mesh::{Indices, PrimitiveTopology};
    use bevy::asset::RenderAssetUsages;

    let h = MUD_SPRAY_HALF;
    let positions: Vec<[f32; 3]> = vec![
        [-h, 0.0, -h],
        [ h, 0.0, -h],
        [ h, 0.0,  h],
        [-h, 0.0,  h],
    ];
    let normals: Vec<[f32; 3]> = vec![[0.0, 1.0, 0.0]; 4];
    let uvs: Vec<[f32; 2]> = vec![
        [0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0],
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
// LCG helpers — deterministic float in [0, 1)
// ---------------------------------------------------------------------------

/// Initialise LCG state from a u32 seed (Wang hash to avoid low-entropy seeds).
#[inline]
fn lcg_init(seed: u32) -> u64 {
    let mut s = seed as u64;
    s ^= s << 17;
    s ^= s >> 31;
    s ^= s << 8;
    (s | 1) as u64 // must be odd for the multiplier to form a full-period LCG
}

/// Advance the LCG and return a float in [0, 1) plus the new state.
#[inline]
fn lcg_next(state: u64) -> (f32, u64) {
    // Knuth's MMIX coefficients.
    let next = state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    let f = (next >> 33) as f32 / (u32::MAX as f32);
    (f, next)
}
