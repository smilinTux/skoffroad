// Persistent tire-track decals on the terrain.
//
// Sprint B4 — Ground & Atmosphere Realism
//
// As the truck drives, each grounded wheel lays down a flat quad decal on the
// terrain surface below it. Decals persist and fade out over TRACK_LIFETIME_S
// seconds. A ring-buffer caps the total entity count at TRACK_CAP so memory
// never grows unbounded.
//
// Placement:
//   - Sampled every SAMPLE_INTERVAL_M of travel per wheel.
//   - Y position comes from terrain_height_at (read-only call — terrain.rs
//     is NOT edited). Lifted TRACK_LIFT metres above terrain to avoid
//     Z-fighting.
//   - Rotated to match chassis heading so tread marks are directional.
//
// Tier gating:
//   Low    — no tire tracks (returns immediately).
//   Medium — TRACK_CAP_MED  decals, shorter lifetime.
//   High   — TRACK_CAP_HIGH decals, full lifetime.
//
// Ring-buffer strategy:
//   Track entities are stored in a fixed-size VecDeque. When the queue is
//   full, the oldest entity is despawned and removed before the new one is
//   pushed. The cap is logged once at startup.
//
// Public API:
//   TireTracksPlugin

use std::collections::VecDeque;
use bevy::prelude::*;
use crate::terrain::terrain_height_at;
use crate::vehicle::{Chassis, VehicleRoot, Wheel};
use crate::graphics_quality::GraphicsQuality;

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct TireTracksPlugin;

impl Plugin for TireTracksPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TireTrackPool>()
           .add_systems(Startup, log_track_cap)
           .add_systems(Update, (
               lay_tire_tracks,
               fade_tire_tracks,
           ));
    }
}

// ---------------------------------------------------------------------------
// Marker component
// ---------------------------------------------------------------------------

/// Marks each tire-track decal entity. Carries lifetime state.
#[derive(Component)]
pub struct TireTrack {
    /// Remaining life in seconds (starts at TRACK_LIFETIME_S, fades to 0).
    pub life_remaining: f32,
    /// Initial lifetime (for alpha curve computation).
    pub lifetime: f32,
}

// ---------------------------------------------------------------------------
// Per-wheel sampling state
// ---------------------------------------------------------------------------

/// Tracks how far each wheel has travelled since the last decal drop.
/// Indexed [FL, FR, RL, RR] matching vehicle.rs WHEEL_OFFSETS.
#[derive(Resource, Default)]
pub struct TireTrackPool {
    /// Ring buffer of live track entities (oldest → front).
    entities:  VecDeque<Entity>,
    /// Per-wheel accumulated travel distance since last track drop.
    wheel_dist: [f32; 4],
    /// Last recorded chassis world position (for delta calculation).
    last_pos:   Vec3,
    /// Whether the last_pos has been initialised.
    has_pos:    bool,
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Decals are dropped every this many metres of wheel travel.
const SAMPLE_INTERVAL_M: f32 = 0.55;

/// Decal quad half-size (metres). Total width = 2 × half = ~0.45 m
/// (slightly wider than the wheel to look like a real tread mark).
const TRACK_HALF_W: f32 = 0.22;
const TRACK_HALF_L: f32 = 0.40;

/// How far above the terrain to float the decal to avoid Z-fighting.
const TRACK_LIFT: f32 = 0.018;

/// Maximum decal count on Medium quality.
const TRACK_CAP_MED:  usize = 256;
/// Maximum decal count on High quality.
const TRACK_CAP_HIGH: usize = 512;

/// Decal lifetime (seconds) at Medium quality.
const TRACK_LIFETIME_MED:  f32 = 18.0;
/// Decal lifetime (seconds) at High quality.
const TRACK_LIFETIME_HIGH: f32 = 30.0;

/// Peak alpha of a freshly-laid track decal.
const TRACK_ALPHA_MAX: f32 = 0.55;

/// Tread mark dark brown-black colour.
const TRACK_COLOR: Color = Color::srgba(0.12, 0.09, 0.06, TRACK_ALPHA_MAX);

/// Minimum chassis speed (m/s) required to lay a track — avoids tracks when
/// the truck is nearly stationary (idling in place).
const MIN_SPEED_MPS: f32 = 0.25;

// Wheel local offsets (mirrors vehicle.rs WHEEL_OFFSETS — chassis-local space).
// We must NOT import them from vehicle.rs (they are private), so we mirror them
// here.  If vehicle.rs ever changes WHEEL_OFFSETS this const must match.
const WHEEL_LOCAL: [Vec3; 4] = [
    Vec3::new(-1.1, -0.35, -1.4), // FL
    Vec3::new( 1.1, -0.35, -1.4), // FR
    Vec3::new(-1.1, -0.35,  1.4), // RL
    Vec3::new( 1.1, -0.35,  1.4), // RR
];

// ---------------------------------------------------------------------------
// Startup: log the ring-buffer cap
// ---------------------------------------------------------------------------

fn log_track_cap(quality: Res<GraphicsQuality>) {
    let cap = match *quality {
        GraphicsQuality::Low    => 0,
        GraphicsQuality::Medium => TRACK_CAP_MED,
        GraphicsQuality::High   => TRACK_CAP_HIGH,
    };
    info!(
        "[TireTracks] ring-buffer cap = {} decals (quality = {})",
        cap,
        quality.as_str()
    );
}

// ---------------------------------------------------------------------------
// System: lay track decals
// ---------------------------------------------------------------------------

fn lay_tire_tracks(
    mut commands:  Commands,
    mut meshes:    ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    quality:       Res<GraphicsQuality>,
    vehicle:       Option<Res<VehicleRoot>>,
    chassis_q:     Query<&Transform, With<Chassis>>,
    wheel_q:       Query<(&Wheel, &Transform), Without<Chassis>>,
    mut pool:      ResMut<TireTrackPool>,
    time:          Res<Time>,
) {
    // Low quality: no tracks.
    if *quality == GraphicsQuality::Low { return; }

    let (cap, lifetime) = match *quality {
        GraphicsQuality::Low    => return,
        GraphicsQuality::Medium => (TRACK_CAP_MED,  TRACK_LIFETIME_MED),
        GraphicsQuality::High   => (TRACK_CAP_HIGH, TRACK_LIFETIME_HIGH),
    };

    let Some(vr) = vehicle else { return };
    // We only need the chassis transform to rotate the decal and estimate speed.
    let Ok(chassis_tf) = chassis_q.get(vr.chassis) else { return };

    let chassis_pos = chassis_tf.translation;

    // Compute chassis speed from position delta.
    let dt = time.delta_secs();
    if dt <= 0.0 { return; }

    let speed_mps = if pool.has_pos {
        (chassis_pos - pool.last_pos).length() / dt
    } else {
        0.0
    };
    pool.last_pos = chassis_pos;
    pool.has_pos  = true;

    // Don't drop tracks when nearly stationary.
    if speed_mps < MIN_SPEED_MPS { return; }

    // Chassis heading on the XZ plane (ignore Y).
    let chassis_fwd = {
        let f = *chassis_tf.forward();
        Vec3::new(f.x, 0.0, f.z).normalize_or_zero()
    };
    let heading_yaw = chassis_fwd.x.atan2(chassis_fwd.z);

    // Collect which wheels are grounded this frame.
    let mut grounded = [false; 4];
    for (wheel, _) in wheel_q.iter() {
        if wheel.index < 4 {
            grounded[wheel.index] = wheel.is_grounded;
        }
    }

    // Advance per-wheel travel distance.
    let travel = speed_mps * dt;
    for i in 0..4 {
        pool.wheel_dist[i] += travel;
    }

    // Shared mesh + material for new decals (built lazily each spawning frame).
    let mut mesh_handle: Option<Handle<Mesh>>             = None;
    let mut mat_handle:  Option<Handle<StandardMaterial>> = None;

    for i in 0..4 {
        if !grounded[i]                       { continue; }
        if pool.wheel_dist[i] < SAMPLE_INTERVAL_M { continue; }

        pool.wheel_dist[i] -= SAMPLE_INTERVAL_M;

        // Compute wheel world position.
        let local_offset = WHEEL_LOCAL[i];
        let world_wheel  = chassis_pos
            + chassis_tf.rotation * local_offset;

        // Sample terrain height at this XZ position (READ ONLY — never edited).
        let terrain_y = terrain_height_at(world_wheel.x, world_wheel.z);
        let decal_y   = terrain_y + TRACK_LIFT;

        // Build mesh once per frame.
        if mesh_handle.is_none() {
            mesh_handle = Some(meshes.add(build_track_quad()));
        }
        if mat_handle.is_none() {
            mat_handle = Some(materials.add(StandardMaterial {
                base_color:   TRACK_COLOR,
                alpha_mode:   AlphaMode::Blend,
                double_sided: true,
                cull_mode:    None,
                unlit:        true,
                ..default()
            }));
        }

        // Retire oldest decal if the pool is full.
        if pool.entities.len() >= cap {
            if let Some(old) = pool.entities.pop_front() {
                commands.entity(old).despawn();
            }
        }

        let entity = commands.spawn((
            TireTrack { life_remaining: lifetime, lifetime },
            Mesh3d(mesh_handle.clone().unwrap()),
            MeshMaterial3d(mat_handle.clone().unwrap()),
            Transform {
                translation: Vec3::new(world_wheel.x, decal_y, world_wheel.z),
                rotation:    Quat::from_rotation_y(heading_yaw),
                scale:       Vec3::ONE,
            },
        )).id();

        pool.entities.push_back(entity);
    }
}

// ---------------------------------------------------------------------------
// System: fade and despawn expired tracks
// ---------------------------------------------------------------------------

fn fade_tire_tracks(
    mut commands:  Commands,
    mut tracks:    Query<(Entity, &mut TireTrack, &MeshMaterial3d<StandardMaterial>)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    time:          Res<Time>,
) {
    let dt = time.delta_secs();

    for (entity, mut track, mat_handle) in &mut tracks {
        track.life_remaining -= dt;

        if track.life_remaining <= 0.0 {
            commands.entity(entity).despawn();
            continue;
        }

        // Fade alpha linearly from TRACK_ALPHA_MAX → 0 as lifetime expires.
        let alpha = (track.life_remaining / track.lifetime).clamp(0.0, 1.0)
            * TRACK_ALPHA_MAX;

        if let Some(mat) = materials.get_mut(&mat_handle.0) {
            let base = mat.base_color.to_srgba();
            mat.base_color = Color::srgba(base.red, base.green, base.blue, alpha);
        }
    }
}

// ---------------------------------------------------------------------------
// Mesh helpers
// ---------------------------------------------------------------------------

/// Build a flat XZ-aligned quad for a tire track. TRACK_HALF_W × TRACK_HALF_L.
fn build_track_quad() -> Mesh {
    use bevy::mesh::{Indices, PrimitiveTopology};
    use bevy::asset::RenderAssetUsages;

    let w = TRACK_HALF_W;
    let l = TRACK_HALF_L;

    let positions: Vec<[f32; 3]> = vec![
        [-w, 0.0, -l],
        [ w, 0.0, -l],
        [ w, 0.0,  l],
        [-w, 0.0,  l],
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
