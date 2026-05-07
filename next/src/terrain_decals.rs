// Terrain decals: static ambient ground-variation stains scattered across the
// terrain at startup. Distinct from:
//   - decals.rs        (transient tire-track dots, vehicle-driven)
//   - ground_ruts.rs   (mud-zone wheel ruts, vehicle-driven)
//   - skidmarks.rs     (short slip/brake stripe events, vehicle-driven)
//
// These are non-interactive, fully static decals that add visual noise to
// the ground: dirt patches, leaf scatter, and dried mud splats.
//
// Placement rules
//   - 80 decals in the XZ square [-80, 80].
//   - Each is a Plane3d quad 0.5–1.5 m wide, lifted 0.02 m above terrain.
//   - Type selected by hash: dirt (33%), dry leaves (33%), dried mud (33%).
//   - Random Y-axis rotation over the full circle [0, 2π).
//   - Skipped on slopes > 0.4 (finite-difference of terrain_height_at).
//   - Skipped within 5 m XZ of the world origin (player start exclusion).
//
// Public API:
//   TerrainDecalsPlugin
//   TerrainDecal  (marker component on each spawned entity)

use std::f32::consts::TAU;
use bevy::prelude::*;
use crate::terrain::terrain_height_at;

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct TerrainDecalsPlugin;

impl Plugin for TerrainDecalsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_terrain_decals);
    }
}

// ---------------------------------------------------------------------------
// Marker component
// ---------------------------------------------------------------------------

/// Marker placed on every static terrain-decal entity.
#[derive(Component)]
pub struct TerrainDecal;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Number of decals to attempt to place.
const DECAL_COUNT: usize = 80;

/// Half-extent of the scatter area (decals live in ±AREA_HALF on X and Z).
const AREA_HALF: f32 = 80.0;

/// How far above the terrain surface to lift each quad (avoids z-fighting).
const DECAL_LIFT: f32 = 0.02;

/// Minimum decal half-size (metres). Full width = 0.5 m minimum.
const SIZE_MIN: f32 = 0.25;

/// Maximum decal half-size (metres). Full width = 1.5 m maximum.
const SIZE_MAX: f32 = 0.75;

/// Maximum slope magnitude (finite-difference) before the decal is skipped.
const MAX_SLOPE: f32 = 0.4;

/// XZ exclusion radius around the world origin (player start).
const ORIGIN_EXCLUSION_R: f32 = 5.0;

/// Finite-difference step used to estimate terrain slope (metres).
const SLOPE_STEP: f32 = 0.5;

/// LCG seed for this subsystem — distinct from other placement systems.
const DECAL_SEED: u32 = 0xDECA_1_u32;

// ---------------------------------------------------------------------------
// Decal type
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
enum DecalKind {
    DirtPatch,
    DryLeaves,
    DriedMud,
}

impl DecalKind {
    /// Base colour (sRGB) and alpha for each kind.
    fn color(self) -> Color {
        match self {
            DecalKind::DirtPatch  => Color::srgba(0.32, 0.25, 0.18, 0.6),
            DecalKind::DryLeaves  => Color::srgba(0.55, 0.40, 0.18, 0.5),
            DecalKind::DriedMud   => Color::srgba(0.20, 0.15, 0.10, 0.7),
        }
    }
}

// ---------------------------------------------------------------------------
// LCG — same algorithm used by grass_tufts.rs / rock_garden.rs / etc.
// ---------------------------------------------------------------------------

struct Lcg(u64);

impl Lcg {
    fn new(seed: u32) -> Self {
        Self(seed as u64)
    }

    /// Advance and return next float in [0, 1).
    fn next_f32(&mut self) -> f32 {
        self.0 = self
            .0
            .wrapping_mul(1_664_525)
            .wrapping_add(1_013_904_223)
            & 0xFFFF_FFFF;
        (self.0 as f32) / (u32::MAX as f32)
    }

    /// Float in [lo, hi).
    fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + self.next_f32() * (hi - lo)
    }

    /// Float in [-half, +half).
    fn signed(&mut self, half: f32) -> f32 {
        self.range(-half, half)
    }
}

// ---------------------------------------------------------------------------
// Slope helper
// ---------------------------------------------------------------------------

/// Estimate slope magnitude at (x, z) via central finite difference.
/// Returns the gradient magnitude; values > MAX_SLOPE indicate steep terrain.
#[inline]
fn slope_at(x: f32, z: f32) -> f32 {
    let h_px = terrain_height_at(x + SLOPE_STEP, z);
    let h_nx = terrain_height_at(x - SLOPE_STEP, z);
    let h_pz = terrain_height_at(x, z + SLOPE_STEP);
    let h_nz = terrain_height_at(x, z - SLOPE_STEP);
    let dh_dx = (h_px - h_nx) / (2.0 * SLOPE_STEP);
    let dh_dz = (h_pz - h_nz) / (2.0 * SLOPE_STEP);
    (dh_dx * dh_dx + dh_dz * dh_dz).sqrt()
}

// ---------------------------------------------------------------------------
// Startup system
// ---------------------------------------------------------------------------

fn spawn_terrain_decals(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mut lcg = Lcg::new(DECAL_SEED);

    let origin_excl_sq = ORIGIN_EXCLUSION_R * ORIGIN_EXCLUSION_R;

    // Generate up to 4× candidates so we can afford to reject slopes / exclusion
    // zones without running short.
    let max_candidates = DECAL_COUNT * 4;
    let mut placed = 0usize;

    for _ in 0..max_candidates {
        if placed >= DECAL_COUNT {
            break;
        }

        // Draw position before checking, to keep the LCG sequence stable.
        let wx = lcg.signed(AREA_HALF);
        let wz = lcg.signed(AREA_HALF);

        // Draw remaining values immediately (consumed regardless of placement).
        let size_t   = lcg.next_f32();          // [0, 1) → half-size
        let kind_t   = lcg.next_f32();          // [0, 1) → decal kind
        let rot_y    = lcg.range(0.0, TAU);     // Y rotation

        // --- Exclusion: player start (world origin) ---------------------------
        if wx * wx + wz * wz < origin_excl_sq {
            continue;
        }

        // --- Exclusion: steep slopes -----------------------------------------
        if slope_at(wx, wz) > MAX_SLOPE {
            continue;
        }

        // --- Compute world-space Y -------------------------------------------
        let terrain_y = terrain_height_at(wx, wz);
        let decal_y   = terrain_y + DECAL_LIFT;

        // --- Select kind and colour ------------------------------------------
        let kind = if kind_t < 0.333_33 {
            DecalKind::DirtPatch
        } else if kind_t < 0.666_67 {
            DecalKind::DryLeaves
        } else {
            DecalKind::DriedMud
        };
        let color = kind.color();

        // --- Compute half-size (0.25–0.75 m → full width 0.5–1.5 m) ---------
        let half = SIZE_MIN + size_t * (SIZE_MAX - SIZE_MIN);

        // --- Build mesh and material per-decal --------------------------------
        // Unique mesh per decal so sizes can vary; small cost for 80 statics.
        let mesh     = meshes.add(Plane3d::new(Vec3::Y, Vec2::splat(half)));
        let material = materials.add(StandardMaterial {
            base_color:       color,
            alpha_mode:       AlphaMode::Blend,
            double_sided:     true,
            cull_mode:        None,
            unlit:            true,
            ..default()
        });

        let transform = Transform {
            translation: Vec3::new(wx, decal_y, wz),
            rotation:    Quat::from_rotation_y(rot_y),
            scale:       Vec3::ONE,
        };

        commands.spawn((
            TerrainDecal,
            Mesh3d(mesh),
            MeshMaterial3d(material),
            transform,
        ));

        placed += 1;
    }
}
