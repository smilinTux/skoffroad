// Mud puddles: 10 visible dark muddy puddle quads placed at low-elevation
// terrain points around the map. Provides clear visual targets for the
// existing mud.rs drag system. Purely visual — no physics colliders.
//
// Puddles are found at startup by sampling ~80 LCG-seeded XZ candidates and
// selecting the 10 with the lowest terrain height, skipping any that are
// within 8m of an already-chosen puddle or within 30m of the origin.
//
// Each puddle gets a slightly randomised size (2–4m), a wet/glossy dark-mud
// material, and a barely-perceptible Y-shimmer each frame.
//
// Public API:
//   MudPuddlesPlugin
//   MudPuddle { center: Vec3, radius: f32 }

use bevy::prelude::*;

use crate::terrain::terrain_height_at;

// ---------------------------------------------------------------------------
// Public component — readable by splash_particles.rs and other systems
// ---------------------------------------------------------------------------

/// Marker on each spawned mud puddle. `radius` is the max half-extent of the
/// plane (used by splash_particles.rs to decide whether a wheel is "inside").
#[derive(Component)]
pub struct MudPuddle {
    pub center: Vec3,
    pub radius: f32,
}

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct MudPuddlesPlugin;

impl Plugin for MudPuddlesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_puddles)
           .add_systems(Update, shimmer_puddles);
    }
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// LCG seed for this subsystem — distinct from mud.rs (TERRAIN_SEED + 7).
const PUDDLE_SEED: u32 = 37;

/// Half-width of the XZ search area in metres.
const SEARCH_HALF: f32 = 100.0;

/// Candidates generated before selecting the best 10.
const CANDIDATES: usize = 80;

/// Number of puddles to place.
const PUDDLE_COUNT: usize = 10;

/// Minimum XZ distance from origin — keeps spawn area clear.
const ORIGIN_CLEAR: f32 = 30.0;

/// Minimum distance between any two chosen puddle centres.
const MIN_PUDDLE_SEP: f32 = 8.0;

/// Puddles sit this many metres above the terrain to prevent z-fighting.
const LIFT: f32 = 0.05;

/// Shimmer speed (radians / second).
const SHIMMER_SPEED: f32 = 0.5;

/// Maximum Y offset of the shimmer (metres). Barely perceptible.
const SHIMMER_AMP: f32 = 0.01;

// ---------------------------------------------------------------------------
// Startup: find low spots and spawn puddle quads
// ---------------------------------------------------------------------------

fn spawn_puddles(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // --- Step 1: Generate ~80 candidate positions ----------------------------

    let mut lcg_x = lcg_init(PUDDLE_SEED);
    let mut lcg_z = lcg_init(PUDDLE_SEED ^ 0xCAFE_BABE);
    // Third stream drives the per-puddle size variation.
    let mut lcg_s = lcg_init(PUDDLE_SEED.wrapping_add(99));

    struct Candidate {
        x: f32,
        z: f32,
        height: f32,
        size_offset: f32, // [0, 1) — added to base radius of 1.25 to give 1.25–2.25 half-size
    }

    let mut candidates: Vec<Candidate> = Vec::with_capacity(CANDIDATES);

    for _ in 0..CANDIDATES {
        let (fx, next_x) = lcg_next(lcg_x);
        let (fz, next_z) = lcg_next(lcg_z);
        let (fs, next_s) = lcg_next(lcg_s);
        lcg_x = next_x;
        lcg_z = next_z;
        lcg_s = next_s;

        // Map [0, 1) → [-SEARCH_HALF, +SEARCH_HALF)
        let wx = (fx - 0.5) * 2.0 * SEARCH_HALF;
        let wz = (fz - 0.5) * 2.0 * SEARCH_HALF;

        // Skip positions too close to the origin (spawn area).
        if wx * wx + wz * wz < ORIGIN_CLEAR * ORIGIN_CLEAR {
            continue;
        }

        let h = terrain_height_at(wx, wz);
        candidates.push(Candidate { x: wx, z: wz, height: h, size_offset: fs });
    }

    // --- Step 2: Sort by elevation (ascending) and pick 10 spread out --------

    // Sort lowest height first.
    candidates.sort_by(|a, b| a.height.partial_cmp(&b.height).unwrap_or(std::cmp::Ordering::Equal));

    let mut chosen: Vec<(f32, f32, f32, f32)> = Vec::with_capacity(PUDDLE_COUNT); // (x, z, h, size_off)

    'outer: for c in &candidates {
        if chosen.len() >= PUDDLE_COUNT {
            break;
        }
        // Reject if too close to an already-chosen puddle.
        for &(px, pz, _, _) in &chosen {
            let dx = c.x - px;
            let dz = c.z - pz;
            if dx * dx + dz * dz < MIN_PUDDLE_SEP * MIN_PUDDLE_SEP {
                continue 'outer;
            }
        }
        chosen.push((c.x, c.z, c.height, c.size_offset));
    }

    // --- Step 3: Spawn a quad for each chosen position -----------------------

    // Wet, dark mud material: low base colour, very low roughness (glossy/wet),
    // slight metallic for a thin reflectivity hint, semi-transparent blend.
    let mud_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(0.05, 0.04, 0.02, 0.85),
        alpha_mode: AlphaMode::Blend,
        perceptual_roughness: 0.1,
        metallic: 0.3,
        double_sided: true,
        cull_mode: None,
        ..default()
    });

    for (idx, &(wx, wz, h, size_off)) in chosen.iter().enumerate() {
        // Half-extents: base 1.25 + up to 1.0 of variation → 1.25–2.25 m.
        // Full quad footprint therefore ranges from 2.5 to 4.5 m, meeting the
        // "irregularly sized 2–4 m" target at the edges.
        let half = 1.25 + size_off;
        let radius = half; // max half-extent for MudPuddle.radius

        let mesh = meshes.add(Plane3d::new(Vec3::Y, Vec2::splat(half)));

        let y = h + LIFT;
        let center = Vec3::new(wx, y, wz);

        commands.spawn((
            MudPuddle { center, radius },
            PuddleIdx(idx),
            Mesh3d(mesh),
            MeshMaterial3d(mud_mat.clone()),
            Transform::from_translation(center),
        ));
    }
}

// ---------------------------------------------------------------------------
// Update: barely-perceptible Y shimmer ("ripple") on each puddle
// ---------------------------------------------------------------------------

/// Stores the spawn index so each puddle shimmers at a different phase.
#[derive(Component)]
struct PuddleIdx(usize);

fn shimmer_puddles(
    time: Res<Time>,
    mut query: Query<(&PuddleIdx, &MudPuddle, &mut Transform)>,
) {
    let t = time.elapsed_secs();
    for (idx, puddle, mut tf) in query.iter_mut() {
        let phase = idx.0 as f32;
        let offset = (t * SHIMMER_SPEED + phase).sin() * SHIMMER_AMP;
        tf.translation.y = puddle.center.y + offset;
    }
}

// ---------------------------------------------------------------------------
// LCG helpers — deterministic float in [0, 1)
// (Copied from mud.rs; kept local to avoid a cross-module dependency on
// private functions.)
// ---------------------------------------------------------------------------

/// Initialise LCG state from a u32 seed (Wang-hash to avoid low-entropy seeds).
#[inline]
fn lcg_init(seed: u32) -> u64 {
    let mut s = seed as u64;
    s ^= s << 17;
    s ^= s >> 31;
    s ^= s << 8;
    (s | 1) as u64
}

/// Advance the LCG and return a float in [0, 1) plus the new state.
#[inline]
fn lcg_next(state: u64) -> (f32, u64) {
    let next = state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    let f = (next >> 33) as f32 / (u32::MAX as f32);
    (f, next)
}
