// Rock garden: a dedicated 60×60 m zone at world (120, 0, 0) with ~40 large
// procedurally-placed irregular boulders, sized 0.5-3 m radius. Each big
// boulder uses a compound sphere collider (3-5 overlapping spheres) so wheels
// actually contour over the curvature instead of flatlining on cuboid faces.
//
// Placement: iterate 60 LCG candidates within ±30 m of (120, ?, 0), keep ~40
// that are at least 1.5 m apart from each other.
//
// Visual sign-post: tall yellow sign at entrance (110, terrain_y+3, 0).
//
// LCG seed: 99 (deterministic).
//
// Public API:
//   RockGardenPlugin
//   RockGardenRock (component)

use bevy::prelude::*;
use avian3d::prelude::*;

use crate::terrain::terrain_height_at;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

pub struct RockGardenPlugin;

impl Plugin for RockGardenPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_rock_garden);
    }
}

/// Marker component placed on every boulder root entity.
#[derive(Component)]
pub struct RockGardenRock;

// ---------------------------------------------------------------------------
// Zone constants
// ---------------------------------------------------------------------------

/// World-space centre X of the rock garden zone.
const ZONE_CX: f32 = 120.0;
/// World-space centre Z of the rock garden zone.
const ZONE_CZ: f32 = 0.0;
/// Half-width of the zone (zone is 60×60 m).
const ZONE_HALF: f32 = 30.0;

/// How many LCG candidate positions to generate.
const CANDIDATES: usize = 60;
/// Maximum rocks to keep after proximity filtering.
const MAX_ROCKS: usize = 40;
/// Minimum XZ separation between two boulder centres (metres).
const MIN_SEP: f32 = 1.5;

/// Rock base-radius range [min, max).
const RADIUS_MIN: f32 = 0.5;
const RADIUS_MAX: f32 = 3.0;

/// The rock colour (neutral grey-brown).
const ROCK_R: f32 = 0.45;
const ROCK_G: f32 = 0.40;
const ROCK_B: f32 = 0.35;
/// Per-rock random tint range ±0.05.
const TINT_RANGE: f32 = 0.05;

// ---------------------------------------------------------------------------
// Sign-post constants
// ---------------------------------------------------------------------------

const SIGN_POST_W: f32 = 0.2;
const SIGN_POST_H: f32 = 4.0;
const SIGN_POST_D: f32 = 0.2;

const SIGN_BOARD_W: f32 = 2.0;
const SIGN_BOARD_H: f32 = 0.5;
const SIGN_BOARD_D: f32 = 0.05;

const SIGN_COLOR: Color = Color::srgb(1.0, 0.95, 0.0); // bright yellow

// ---------------------------------------------------------------------------
// LCG — same style as ramps.rs / obstacles.rs
// ---------------------------------------------------------------------------

struct Lcg(u64);

impl Lcg {
    fn new(seed: u32) -> Self {
        Self(seed as u64)
    }

    /// Next float in [0, 1).
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
// Per-boulder sphere layout — 3 to 5 spheres that approximate an irregular blob
// ---------------------------------------------------------------------------

/// Description of one sphere sub-shape within a boulder.
struct SphereDesc {
    /// Offset from the boulder's local origin.
    offset: Vec3,
    /// Radius of this sphere sub-shape.
    radius: f32,
}

/// Generate 3-5 sphere descriptions for a boulder of `base_radius`.
/// Uses the provided `lcg` to vary count, offsets, and radii.
fn gen_spheres(lcg: &mut Lcg, base_radius: f32) -> Vec<SphereDesc> {
    // Number of sub-spheres: 3 or 4 or 5 depending on base size.
    let count: usize = if base_radius < 1.0 {
        3
    } else if base_radius < 2.0 {
        4
    } else {
        5
    };

    let mut spheres = Vec::with_capacity(count);
    for _ in 0..count {
        let ox = lcg.signed(base_radius * 0.5);
        let oy = lcg.signed(base_radius * 0.3);
        let oz = lcg.signed(base_radius * 0.5);
        // Each sub-sphere radius is 0.5..1.0 of base.
        let r = lcg.range(0.5, 1.0) * base_radius;
        spheres.push(SphereDesc {
            offset: Vec3::new(ox, oy, oz),
            radius: r,
        });
    }
    spheres
}

// ---------------------------------------------------------------------------
// Spawn system
// ---------------------------------------------------------------------------

fn spawn_rock_garden(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mut lcg = Lcg::new(99);

    // ---- 1. Generate candidate positions ----------------------------------

    struct Candidate {
        wx: f32,
        wz: f32,
        base_radius: f32,
        spheres: Vec<SphereDesc>,
        tint: [f32; 3],
    }

    let mut placed: Vec<Candidate> = Vec::with_capacity(MAX_ROCKS);

    for _ in 0..CANDIDATES {
        if placed.len() >= MAX_ROCKS {
            break;
        }

        let ox = lcg.signed(ZONE_HALF);
        let oz = lcg.signed(ZONE_HALF);
        let wx = ZONE_CX + ox;
        let wz = ZONE_CZ + oz;

        // Consume the base_radius draw regardless of whether this candidate
        // is accepted, so LCG sequence length is deterministic.
        let base_radius = lcg.range(RADIUS_MIN, RADIUS_MAX);

        // Tint draws (3 channels).
        let tr = lcg.range(-TINT_RANGE, TINT_RANGE);
        let tg = lcg.range(-TINT_RANGE, TINT_RANGE);
        let tb = lcg.range(-TINT_RANGE, TINT_RANGE);

        // Sub-sphere draws.
        let spheres = gen_spheres(&mut lcg, base_radius);

        // Proximity filter — reject if too close to an already-placed rock.
        let too_close = placed.iter().any(|p| {
            let dx = p.wx - wx;
            let dz = p.wz - wz;
            dx * dx + dz * dz < MIN_SEP * MIN_SEP
        });
        if too_close {
            continue;
        }

        placed.push(Candidate {
            wx,
            wz,
            base_radius,
            spheres,
            tint: [tr, tg, tb],
        });
    }

    // ---- 2. Spawn rocks ---------------------------------------------------

    for rock in &placed {
        let terrain_y = terrain_height_at(rock.wx, rock.wz);
        // Half-bury the rock: lift its centre by 70 % of base radius.
        let ry = terrain_y + rock.base_radius * 0.7;

        // Build material with per-rock random tint.
        let mat = materials.add(StandardMaterial {
            base_color: Color::srgb(
                (ROCK_R + rock.tint[0]).clamp(0.0, 1.0),
                (ROCK_G + rock.tint[1]).clamp(0.0, 1.0),
                (ROCK_B + rock.tint[2]).clamp(0.0, 1.0),
            ),
            perceptual_roughness: 0.95,
            ..default()
        });

        // Build compound collider from sub-sphere positions/radii.
        let compound_shapes: Vec<(Vec3, Quat, Collider)> = rock
            .spheres
            .iter()
            .map(|s| (s.offset, Quat::IDENTITY, Collider::sphere(s.radius)))
            .collect();

        let collider = Collider::compound(compound_shapes);

        // Spawn the root entity with physics components.
        let rock_pos = Vec3::new(rock.wx, ry, rock.wz);
        commands
            .spawn((
                Transform::from_translation(rock_pos),
                Visibility::default(),
                RockGardenRock,
                RigidBody::Static,
                collider,
            ))
            .with_children(|parent| {
                // Spawn each sub-sphere as a visible mesh child.
                for sphere in &rock.spheres {
                    let sphere_mesh =
                        meshes.add(Sphere::new(sphere.radius).mesh().ico(2).unwrap());
                    parent.spawn((
                        Mesh3d(sphere_mesh),
                        MeshMaterial3d(mat.clone()),
                        Transform::from_translation(sphere.offset),
                    ));
                }
            });
    }

    // ---- 3. Spawn the sign-post at the entrance ---------------------------

    let sign_x = 110.0_f32;
    let sign_z = 0.0_f32;
    let sign_terrain_y = terrain_height_at(sign_x, sign_z);
    let sign_base_y = sign_terrain_y + SIGN_POST_H * 0.5;

    let sign_mat = materials.add(StandardMaterial {
        base_color: SIGN_COLOR,
        perceptual_roughness: 0.4,
        ..default()
    });

    let post_mesh = meshes.add(Cuboid::new(SIGN_POST_W, SIGN_POST_H, SIGN_POST_D));
    let board_mesh = meshes.add(Cuboid::new(SIGN_BOARD_W, SIGN_BOARD_H, SIGN_BOARD_D));

    // Root post entity — no collider needed, it's just a visual marker.
    commands
        .spawn((
            Transform::from_xyz(sign_x, sign_base_y, sign_z),
            Visibility::default(),
        ))
        .with_children(|parent| {
            // Vertical post.
            parent.spawn((
                Mesh3d(post_mesh),
                MeshMaterial3d(sign_mat.clone()),
                Transform::default(),
            ));
            // Horizontal sign board near the top.
            parent.spawn((
                Mesh3d(board_mesh),
                MeshMaterial3d(sign_mat),
                Transform::from_xyz(0.0, SIGN_POST_H * 0.45, 0.0),
            ));
        });

    // Log how many rocks were placed (visible in dev builds and smoke logs).
    bevy::log::info!(
        "rock_garden: spawned {} boulders in 60×60 m zone at ({}, 0, {})",
        placed.len(),
        ZONE_CX,
        ZONE_CZ
    );
}
