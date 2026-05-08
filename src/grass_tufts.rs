// Grass tufts: 400 small scattered grass clumps across the terrain.
// Each tuft = 4 thin tall green cuboids in a small cluster for organic detail.
//
// Placement: LCG-deterministic seed=300, 200×200 m area, 12 m exclusion
// zone around origin (player spawn). No physics — pure decoration.
//
// Shared mesh handle across all 1 600 blade entities keeps draw-call
// overhead low; per-tuft material allows colour variation.
//
// Public API:
//   GrassTuftsPlugin
//   GrassTuft  (component on each tuft parent)

use bevy::prelude::*;

use crate::terrain::terrain_height_at;

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct GrassTuftsPlugin;

impl Plugin for GrassTuftsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_grass_tufts);
    }
}

// ---------------------------------------------------------------------------
// Marker component
// ---------------------------------------------------------------------------

/// Marker placed on each grass-tuft parent entity.
#[derive(Component)]
pub struct GrassTuft;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Number of tufts to scatter across the terrain.
const TUFT_COUNT: usize = 400;

/// Half-extent of the scatter area (tufts live in ±AREA_HALF on X and Z).
const AREA_HALF: f32 = 100.0; // 200 × 200 m area

/// Exclusion radius around the world origin (player spawn point).
const SPAWN_EXCLUSION_R: f32 = 12.0;

/// Number of grass blades per tuft.
const BLADES_PER_TUFT: usize = 4;

/// Blade dimensions: width × height × depth.
const BLADE_W: f32 = 0.04;
const BLADE_H: f32 = 0.30;
const BLADE_D: f32 = 0.02;

/// Maximum XZ offset for a blade within its tuft cluster.
const BLADE_CLUSTER_R: f32 = 0.12;

/// Base grass colour (unlit sRGB).
const GRASS_R: f32 = 0.18;
const GRASS_G: f32 = 0.50;
const GRASS_B: f32 = 0.20;

/// Per-tuft colour tint range (±TINT_RANGE on each channel).
const TINT_RANGE: f32 = 0.05;

/// Maximum Y-axis rotation for a blade (full circle spread, radians).
const BLADE_ROT_RANGE: f32 = std::f32::consts::TAU;

/// Tilt angle range for each blade leaning away from vertical (radians ≈ 10°–20°).
const TILT_MIN: f32 = 0.175; // ~10°
const TILT_MAX: f32 = 0.349; // ~20°

// ---------------------------------------------------------------------------
// LCG  — identical implementation style to rock_garden.rs / obstacles.rs
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
// Startup system
// ---------------------------------------------------------------------------

fn spawn_grass_tufts(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mut lcg = Lcg::new(300);

    // ---- Shared blade mesh (one allocation, reused for all 1 600 blades) ----
    let blade_mesh: Handle<Mesh> = meshes.add(Cuboid::new(BLADE_W, BLADE_H, BLADE_D));

    // ---- Generate tuft positions -------------------------------------------

    // We generate more candidates than needed so we can skip the exclusion
    // zone without making the sequence length dependent on geometry.
    let mut positions: Vec<(f32, f32)> = Vec::with_capacity(TUFT_COUNT);
    let exclusion_sq = SPAWN_EXCLUSION_R * SPAWN_EXCLUSION_R;

    // Generate at most 4× TUFT_COUNT candidates; stop early when we have enough.
    let max_candidates = TUFT_COUNT * 4;
    for _ in 0..max_candidates {
        if positions.len() >= TUFT_COUNT {
            break;
        }
        let tx = lcg.signed(AREA_HALF);
        let tz = lcg.signed(AREA_HALF);
        // Always consume the exclusion check draw from the LCG so the
        // sequence length is predictable.
        if tx * tx + tz * tz < exclusion_sq {
            continue;
        }
        positions.push((tx, tz));
    }

    // ---- Per-tuft colour draws (consume before blade draws for determinism) -
    // Pre-generate tint values.
    let mut tints: Vec<[f32; 3]> = Vec::with_capacity(positions.len());
    for _ in 0..positions.len() {
        let tr = lcg.range(-TINT_RANGE, TINT_RANGE);
        let tg = lcg.range(-TINT_RANGE, TINT_RANGE);
        let tb = lcg.range(-TINT_RANGE, TINT_RANGE);
        tints.push([tr, tg, tb]);
    }

    // ---- Spawn tufts --------------------------------------------------------

    for (i, &(tx, tz)) in positions.iter().enumerate() {
        let ty = terrain_height_at(tx, tz);
        let tint = tints[i];

        // Per-tuft unlit material with slight colour variation.
        let mat = materials.add(StandardMaterial {
            base_color: Color::srgb(
                (GRASS_R + tint[0]).clamp(0.0, 1.0),
                (GRASS_G + tint[1]).clamp(0.0, 1.0),
                (GRASS_B + tint[2]).clamp(0.0, 1.0),
            ),
            unlit: true,
            ..default()
        });

        // Parent entity — sits exactly on the terrain surface.
        commands
            .spawn((
                GrassTuft,
                Transform::from_xyz(tx, ty, tz),
                Visibility::default(),
            ))
            .with_children(|parent| {
                for _ in 0..BLADES_PER_TUFT {
                    // Small XZ cluster offset so blades spread around the root.
                    let bx = lcg.signed(BLADE_CLUSTER_R);
                    let bz = lcg.signed(BLADE_CLUSTER_R);
                    // Blade bottom sits at terrain level; cylinder/cuboid origin
                    // is at its centre, so lift by half the height.
                    let by = BLADE_H * 0.5;

                    // Random Y-axis rotation so blades face different directions.
                    let rot_y = lcg.range(0.0, BLADE_ROT_RANGE);

                    // Random tilt axis (direction the blade leans in XZ).
                    let tilt_dir = lcg.range(0.0, BLADE_ROT_RANGE);
                    let tilt_mag = lcg.range(TILT_MIN, TILT_MAX);

                    // Compose: first rotate around Y (facing), then tilt.
                    let facing = Quat::from_rotation_y(rot_y);
                    // Tilt axis is horizontal, perpendicular to the lean direction.
                    let tilt_axis =
                        Vec3::new(tilt_dir.cos(), 0.0, tilt_dir.sin()).normalize();
                    let tilt = Quat::from_axis_angle(tilt_axis, tilt_mag);
                    let rotation = tilt * facing;

                    parent.spawn((
                        Mesh3d(blade_mesh.clone()),
                        MeshMaterial3d(mat.clone()),
                        Transform {
                            translation: Vec3::new(bx, by, bz),
                            rotation,
                            scale: Vec3::ONE,
                        },
                    ));
                }
            });
    }

    bevy::log::info!(
        "grass_tufts: spawned {} tufts ({} blade entities) across 200×200 m terrain",
        positions.len(),
        positions.len() * BLADES_PER_TUFT,
    );
}
