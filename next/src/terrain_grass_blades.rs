// terrain_grass_blades.rs — Sprint 40
//
// Dense per-blade grass detail: ~2 000 individual triangle-billboard grass blades
// scattered across grassy terrain areas.  Distinct from grass_tufts.rs which places
// ~50 ground-level cuboid tuft markers.
//
// Design:
//   • One shared Mesh3d (single thin triangle, 3 vertices).
//   • 6 pre-baked StandardMaterial colour variants; each blade picks one by hash.
//   • Hash-deterministic LCG placement in the ±100 m area.
//   • Slope rejection: blades on slopes > 0.3 (|dy/dx| or |dy/dz|) are skipped.
//   • No physics — pure decoration.
//
// Public API:
//   TerrainGrassBladesPlugin
//   GrassBlade  (marker component)

use bevy::{
    asset::RenderAssetUsages,
    mesh::{Indices, PrimitiveTopology},
    prelude::*,
};

use crate::terrain::terrain_height_at;

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct TerrainGrassBladesPlugin;

impl Plugin for TerrainGrassBladesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_grass_blades);
    }
}

// ---------------------------------------------------------------------------
// Marker component
// ---------------------------------------------------------------------------

/// Marker placed on every individual grass-blade entity.
#[derive(Component)]
pub struct GrassBlade;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Total number of grass blade entities to spawn.
const BLADE_COUNT: usize = 2_000;

/// Half-extent of the scatter area (blades live in ±AREA_HALF on X and Z).
const AREA_HALF: f32 = 100.0;

/// Blade triangle geometry (local space, origin at base centre).
const BLADE_BASE_HALF: f32 = 0.03; // half of 0.06 m base width
const BLADE_HEIGHT: f32 = 0.25;    // 0.25 m tall

/// Maximum slope magnitude (finite-difference dY/dX or dY/dZ) for placement.
const MAX_SLOPE: f32 = 0.3;

/// Finite-difference step used for slope estimation.
const SLOPE_STEP: f32 = 0.5;

/// Number of pre-baked colour variants (one material per variant).
const COLOR_VARIANTS: usize = 6;

/// LCG seed for blade placement.
const LCG_SEED: u32 = 777;

// ---------------------------------------------------------------------------
// LCG  — same style as grass_tufts.rs / rock_garden.rs
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
// Helpers
// ---------------------------------------------------------------------------

/// Estimate the terrain slope at (x, z) using central finite differences.
/// Returns the maximum absolute gradient in either axis.
fn terrain_slope_at(x: f32, z: f32) -> f32 {
    let h_px = terrain_height_at(x + SLOPE_STEP, z);
    let h_nx = terrain_height_at(x - SLOPE_STEP, z);
    let h_pz = terrain_height_at(x, z + SLOPE_STEP);
    let h_nz = terrain_height_at(x, z - SLOPE_STEP);
    let grad_x = ((h_px - h_nx) / (2.0 * SLOPE_STEP)).abs();
    let grad_z = ((h_pz - h_nz) / (2.0 * SLOPE_STEP)).abs();
    grad_x.max(grad_z)
}

/// Build the single-triangle grass blade mesh.
///
/// Geometry (local space, Y-up):
///   v0 = bottom-left  (-base_half, 0,       0)
///   v1 = bottom-right ( base_half, 0,       0)
///   v2 = tip          (0,          height,  0)
///
/// Normal for all three vertices: +Y blended with +Z so the blade is
/// visible from slightly above (pointing generally "up and toward camera").
fn build_blade_mesh() -> Mesh {
    let b = BLADE_BASE_HALF;
    let h = BLADE_HEIGHT;

    // Three vertex positions.
    let positions: Vec<[f32; 3]> = vec![
        [-b,  0.0, 0.0], // v0  bottom-left
        [ b,  0.0, 0.0], // v1  bottom-right
        [ 0.0, h,  0.0], // v2  tip
    ];

    // Upward-pointing normals (blades are thin billboards; +Y works well for
    // grass lit from above, with a slight forward lean so they catch sunlight).
    let up_fwd = Vec3::new(0.0, 0.85, 0.53).normalize();
    let normals: Vec<[f32; 3]> = vec![
        up_fwd.to_array(),
        up_fwd.to_array(),
        up_fwd.to_array(),
    ];

    // Simple UVs: base corners + tip.
    let uvs: Vec<[f32; 2]> = vec![
        [0.0, 0.0],
        [1.0, 0.0],
        [0.5, 1.0],
    ];

    // Single triangle.
    let indices: Vec<u32> = vec![0, 1, 2];

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL,   normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0,     uvs);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

/// Build the COLOR_VARIANTS pre-baked materials.
///
/// Green tones: sRGB(0.15–0.30, 0.45–0.65, 0.20–0.30).
fn build_color_variants(materials: &mut Assets<StandardMaterial>) -> Vec<Handle<StandardMaterial>> {
    // Evenly-spaced sample points across the colour ranges — no RNG needed,
    // so the palette is stable across runs.
    let n = COLOR_VARIANTS as f32;
    (0..COLOR_VARIANTS)
        .map(|i| {
            let t = i as f32 / (n - 1.0).max(1.0);
            let r = 0.15 + t * (0.30 - 0.15);
            let g = 0.45 + t * (0.65 - 0.45);
            let b = 0.20 + t * (0.30 - 0.20);
            materials.add(StandardMaterial {
                base_color: Color::srgb(r, g, b),
                unlit: true,
                double_sided: true,
                cull_mode: None,
                ..default()
            })
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Startup system
// ---------------------------------------------------------------------------

fn spawn_grass_blades(
    mut commands:  Commands,
    mut meshes:    ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // ---- Shared blade mesh (single triangle, allocated once) ----------------
    let blade_mesh: Handle<Mesh> = meshes.add(build_blade_mesh());

    // ---- 6 colour variants --------------------------------------------------
    let color_handles: Vec<Handle<StandardMaterial>> =
        build_color_variants(&mut materials);

    // ---- LCG placement ------------------------------------------------------
    let mut lcg = Lcg::new(LCG_SEED);
    let mut spawned: usize = 0;

    // Try up to 8× BLADE_COUNT candidates to handle slope rejections.
    let max_candidates = BLADE_COUNT * 8;
    for _ in 0..max_candidates {
        if spawned >= BLADE_COUNT {
            break;
        }

        // Candidate XZ position.
        let cx = lcg.signed(AREA_HALF);
        let cz = lcg.signed(AREA_HALF);

        // Slope rejection — always draw rotation/color from LCG first so the
        // sequence is deterministic regardless of how many are rejected.
        let rot_y     = lcg.range(0.0, std::f32::consts::TAU);
        let color_idx = (lcg.next_f32() * COLOR_VARIANTS as f32) as usize;
        let color_idx = color_idx.min(COLOR_VARIANTS - 1);

        if terrain_slope_at(cx, cz) > MAX_SLOPE {
            continue; // too steep — skip this candidate
        }

        let cy = terrain_height_at(cx, cz);

        // Blade mesh origin is at the base, so no vertical offset needed.
        // Rotate around Y for random facing direction.
        let transform = Transform {
            translation: Vec3::new(cx, cy, cz),
            rotation: Quat::from_rotation_y(rot_y),
            scale: Vec3::ONE,
        };

        commands.spawn((
            GrassBlade,
            Mesh3d(blade_mesh.clone()),
            MeshMaterial3d(color_handles[color_idx].clone()),
            transform,
        ));

        spawned += 1;
    }

    bevy::log::info!(
        "terrain_grass_blades: spawned {} individual blade entities across 200×200 m terrain",
        spawned,
    );
}
