// Canyon biome overlay: tall red-rock pillars, dusty haze, reddish sky tint.
// Activates when ActiveMap == Canyon.
//
// Public API:
//   BiomeCanyonPlugin

use bevy::{
    pbr::{DistanceFog, FogFalloff},
    prelude::*,
};
use avian3d::prelude::*;

use crate::maps::{ActiveMap, MapKind};
use crate::terrain::terrain_height_at;

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct BiomeCanyonPlugin;

impl Plugin for BiomeCanyonPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(CanyonState { spawned: false })
           .add_systems(Update, (
               canyon_enter_exit,
               canyon_atmosphere,
           ).chain());
    }
}

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Marker component placed on every prop the canyon biome spawns.
/// Used to identify and despawn them when the biome is deactivated.
#[derive(Component)]
pub struct CanyonProp;

/// Tracks whether canyon props have been spawned this session.
#[derive(Resource)]
pub struct CanyonState {
    pub spawned: bool,
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Target fog tint while inside the canyon.
const CANYON_FOG_COLOR: Color = Color::srgba(0.85, 0.55, 0.40, 1.0);
/// Narrower fog end distance for that close-and-enclosed feel.
const CANYON_FOG_START: f32 = 60.0;
const CANYON_FOG_END:   f32 = 180.0;

/// Default fog restored when leaving the canyon.
const DEFAULT_FOG_COLOR: Color = Color::srgba(0.62, 0.76, 0.90, 1.0);
const DEFAULT_FOG_START: f32 = 80.0;
const DEFAULT_FOG_END:   f32 = 250.0;

/// Warm reddish ambient while in the canyon.
const CANYON_AMBIENT_COLOR: Color = Color::srgb(1.0, 0.85, 0.75);
const CANYON_AMBIENT_BRIGHTNESS: f32 = 1200.0;
/// Neutral ambient restored on leave (matches sky.rs noon values).
const DEFAULT_AMBIENT_COLOR: Color = Color::srgb(0.50, 0.55, 0.65);
const DEFAULT_AMBIENT_BRIGHTNESS: f32 = 1500.0;

/// Base color for the red-rock pillars.
const ROCK_COLOR: Color = Color::srgb(0.65, 0.30, 0.18);

/// The 4 cluster centres placed at the corners of a 120 m square around origin.
const CLUSTER_CENTERS: [(f32, f32); 4] = [
    ( 60.0,  60.0),
    (-60.0,  60.0),
    ( 60.0, -60.0),
    (-60.0, -60.0),
];
const PILLARS_PER_CLUSTER: u32 = 6;
/// Maximum radial scatter from each cluster centre (metres).
const CLUSTER_RADIUS: f32 = 25.0;
/// Do not place a pillar closer than this to the player spawn at origin.
const SPAWN_CLEAR: f32 = 30.0;

// ---------------------------------------------------------------------------
// LCG deterministic noise — no external crate needed
// ---------------------------------------------------------------------------

/// Returns a pseudo-random float in [0, 1) from a seed.
/// Classic 32-bit LCG: multiplier and increment from Numerical Recipes.
#[inline]
fn lcg_f32(seed: u32) -> f32 {
    let v = seed.wrapping_mul(1664525).wrapping_add(1013904223);
    (v as f32) / (u32::MAX as f32)
}

/// Produce an independent float from two u32 inputs by mixing them first.
#[inline]
fn lcg2(a: u32, b: u32) -> f32 {
    let mixed = a
        .wrapping_mul(374761393)
        .wrapping_add(b.wrapping_mul(668265263));
    lcg_f32(mixed)
}

// ---------------------------------------------------------------------------
// Enter / exit state machine
// ---------------------------------------------------------------------------

fn canyon_enter_exit(
    active_map: Res<ActiveMap>,
    mut state:  ResMut<CanyonState>,
    mut commands: Commands,
    mut meshes:   ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    props: Query<Entity, With<CanyonProp>>,
) {
    let in_canyon = active_map.0 == MapKind::Canyon;

    if in_canyon && !state.spawned {
        // --- spawn pillars ---
        let mat = materials.add(StandardMaterial {
            base_color: ROCK_COLOR,
            perceptual_roughness: 0.95,
            ..default()
        });

        let mut pillar_index: u32 = 0;
        for (ci, &(cx, cz)) in CLUSTER_CENTERS.iter().enumerate() {
            for pi in 0..PILLARS_PER_CLUSTER {
                // Deterministic radial scatter: angle and distance each from
                // independent LCG seeds composed of cluster index and pillar index.
                let angle  = lcg2(ci as u32 * 1000 + pi, 1) * std::f32::consts::TAU;
                let dist   = lcg2(ci as u32 * 1000 + pi, 2) * CLUSTER_RADIUS;

                let wx = cx + angle.cos() * dist;
                let wz = cz + angle.sin() * dist;

                // Skip pillar if too close to origin.
                if wx * wx + wz * wz < SPAWN_CLEAR * SPAWN_CLEAR {
                    pillar_index += 1;
                    continue;
                }

                // Random radius in [2, 4] and height in [12, 22].
                let radius = 2.0 + lcg2(pillar_index, 10) * 2.0;
                let height = 12.0 + lcg2(pillar_index, 20) * 10.0;

                let terrain_y = terrain_height_at(wx, wz);
                // Cylinder origin is at its centre; base at terrain_y.
                let pillar_y = terrain_y + height * 0.5;

                let mesh = meshes.add(Cylinder::new(radius, height));

                commands.spawn((
                    Mesh3d(mesh),
                    MeshMaterial3d(mat.clone()),
                    Transform::from_xyz(wx, pillar_y, wz),
                    RigidBody::Static,
                    Collider::cylinder(radius, height),
                    CanyonProp,
                ));

                pillar_index += 1;
            }
        }

        state.spawned = true;
    } else if !in_canyon && state.spawned {
        // --- despawn all props ---
        for entity in &props {
            commands.entity(entity).despawn();
        }
        state.spawned = false;
    }
}

// ---------------------------------------------------------------------------
// Atmosphere: fog + ambient lerp each frame
// ---------------------------------------------------------------------------

fn canyon_atmosphere(
    active_map: Res<ActiveMap>,
    time:       Res<Time>,
    mut fogs:   Query<&mut DistanceFog>,
    mut ambient: ResMut<GlobalAmbientLight>,
) {
    let in_canyon = active_map.0 == MapKind::Canyon;
    // ~2-second blend time.
    let dt = time.delta_secs();
    let alpha = (dt * 0.5).clamp(0.0, 1.0);

    // --- target values ---
    let (target_fog_color, target_fog_start, target_fog_end,
         target_ambient_color, target_ambient_brightness) = if in_canyon {
        (CANYON_FOG_COLOR, CANYON_FOG_START, CANYON_FOG_END,
         CANYON_AMBIENT_COLOR, CANYON_AMBIENT_BRIGHTNESS)
    } else {
        (DEFAULT_FOG_COLOR, DEFAULT_FOG_START, DEFAULT_FOG_END,
         DEFAULT_AMBIENT_COLOR, DEFAULT_AMBIENT_BRIGHTNESS)
    };

    // --- lerp fog ---
    for mut fog in &mut fogs {
        let cur = fog.color.to_srgba();
        let tgt = target_fog_color.to_srgba();
        fog.color = Color::srgba(
            cur.red   + (tgt.red   - cur.red)   * alpha,
            cur.green + (tgt.green - cur.green)  * alpha,
            cur.blue  + (tgt.blue  - cur.blue)   * alpha,
            1.0,
        );
        // Lerp the linear fog falloff distances.
        if let FogFalloff::Linear { ref mut start, ref mut end } = fog.falloff {
            *start += (target_fog_start - *start) * alpha;
            *end   += (target_fog_end   - *end)   * alpha;
        }
    }

    // --- lerp ambient ---
    let cur_a = ambient.color.to_srgba();
    let tgt_a = target_ambient_color.to_srgba();
    ambient.color = Color::srgb(
        cur_a.red   + (tgt_a.red   - cur_a.red)   * alpha,
        cur_a.green + (tgt_a.green - cur_a.green)  * alpha,
        cur_a.blue  + (tgt_a.blue  - cur_a.blue)   * alpha,
    );
    ambient.brightness +=
        (target_ambient_brightness - ambient.brightness) * alpha;
}
