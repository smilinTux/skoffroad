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

use bevy::{
    asset::RenderAssetUsages,
    mesh::{Indices, PrimitiveTopology},
    prelude::*,
};

use crate::graphics_quality::GraphicsQuality;
use crate::terrain::terrain_height_at;
use crate::wind::WindState;

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct GrassTuftsPlugin;

impl Plugin for GrassTuftsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_grass_tufts)
            .add_systems(Update, sway_grass_tufts);
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
    quality: Res<GraphicsQuality>,
) {
    let mut lcg = Lcg::new(300);

    // ---- Shared blade mesh (one allocation, reused for all 1 600 blades) ----
    // On Medium+ we use a cross-triangle billboard (2 tris, 6 verts per blade)
    // which reads as actual grass instead of a green stick. The bottom is wider
    // and darker, the tip is narrower and brighter; a darker "stem" keeps it
    // grounded. Cuboids stay on Low for the cheaper geometry path.
    let blade_mesh: Handle<Mesh> = if quality.grass_billboards() {
        meshes.add(build_cross_blade_mesh())
    } else {
        meshes.add(Cuboid::new(BLADE_W, BLADE_H, BLADE_D))
    };

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

// ---------------------------------------------------------------------------
// Wind sway (Sprint 42 commit 2) — CPU-side
// ---------------------------------------------------------------------------
//
// Each tuft tilts about an axis perpendicular to the current wind direction
// (in the horizontal plane). Lean magnitude is sin(t*freq + phase) where phase
// is derived from the tuft's world position, so the field of grass sways
// non-uniformly. Cuboid tufts on Low don't sway — they were never meant to
// look like grass.

fn sway_grass_tufts(
    time:        Res<Time>,
    wind:        Option<Res<WindState>>,
    quality:     Res<GraphicsQuality>,
    mut tufts:   Query<&mut Transform, With<GrassTuft>>,
) {
    if !quality.grass_billboards() {
        return;
    }

    let t = time.elapsed_secs();
    // Tie amplitude to wind speed: 0 m/s -> still, 10 m/s -> ~8° max lean.
    let wind_speed = wind.as_ref().map(|w| w.speed_mps).unwrap_or(2.0);
    let amp = (wind_speed * 0.014).clamp(0.0, 0.15);

    let wind_dir = wind
        .as_ref()
        .map(|w| Vec3::new(w.direction.x, 0.0, w.direction.z))
        .unwrap_or(Vec3::X);
    let wind_xz = wind_dir.normalize_or_zero();
    if wind_xz.length_squared() < 1e-4 {
        return;
    }
    // Lean axis is the horizontal vector perpendicular to wind.
    let lean_axis = Vec3::new(-wind_xz.z, 0.0, wind_xz.x).normalize();

    const FREQ: f32 = 1.6;
    for mut transform in &mut tufts {
        let phase = transform.translation.x * 0.13 + transform.translation.z * 0.17;
        let lean  = (t * FREQ + phase).sin() * amp;
        transform.rotation = Quat::from_axis_angle(lean_axis, lean);
    }
}

// ---------------------------------------------------------------------------
// Cross-triangle blade mesh (Sprint 42)
// ---------------------------------------------------------------------------
//
// Two perpendicular triangles, both rooted at y=0 with tip at y=BLADE_H, share
// the tip vertex. Per-vertex alpha is 1 at the base and 1 at the tip too —
// alpha-as-fade isn't used here; commit 2 layers an alpha-tested texture.
// Vertex colors fade from a darker green at the base to a brighter green at
// the tip so blades read against the terrain even without textures.

fn build_cross_blade_mesh() -> Mesh {
    // Triangle base half-width: the base edge runs from (-w, 0, 0) to (w, 0, 0).
    // We pick a slightly wider blade than the cuboid path so the cross-quads
    // catch the eye at distance.
    let w = BLADE_W * 1.5;
    let h = BLADE_H;

    // Bottom-darker, tip-brighter green (linear, not sRGB-wrapped). We rely on
    // vertex_color flowing through StandardMaterial because terrain.rs already
    // does the same trick for slope shading.
    let base_color = [GRASS_R * 0.55, GRASS_G * 0.55, GRASS_B * 0.55, 1.0];
    let tip_color  = [GRASS_R * 1.20, GRASS_G * 1.20, GRASS_B * 1.20, 1.0];

    // 6 vertices: (left, right, tip) × 2 perpendicular planes.
    // Plane A spans X-axis, Plane B spans Z-axis. Both meet at (0, h, 0).
    let positions: Vec<[f32; 3]> = vec![
        // Plane A (XY-extended)
        [-w, 0.0,  0.0],   // 0: bottom-left
        [ w, 0.0,  0.0],   // 1: bottom-right
        [0.0, h,   0.0],   // 2: tip-A
        // Plane B (YZ-extended)
        [0.0, 0.0, -w],    // 3: bottom-left
        [0.0, 0.0,  w],    // 4: bottom-right
        [0.0, h,   0.0],   // 5: tip-B (geometric duplicate of 2 — separate
                           //          for normals)
    ];

    let normals: Vec<[f32; 3]> = vec![
        [0.0, 0.0,  1.0],
        [0.0, 0.0,  1.0],
        [0.0, 0.0,  1.0],
        [1.0, 0.0,  0.0],
        [1.0, 0.0,  0.0],
        [1.0, 0.0,  0.0],
    ];

    let uvs: Vec<[f32; 2]> = vec![
        [0.0, 1.0], [1.0, 1.0], [0.5, 0.0],
        [0.0, 1.0], [1.0, 1.0], [0.5, 0.0],
    ];

    let colors: Vec<[f32; 4]> = vec![
        base_color, base_color, tip_color,
        base_color, base_color, tip_color,
    ];

    let indices: Vec<u32> = vec![0, 1, 2, 3, 4, 5];

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL,   normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0,     uvs);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR,    colors);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}
