// Procedural obstacle placement across the 200x200 m terrain.
//
// Three obstacle types scattered using an LCG seeded from TERRAIN_SEED + 19:
//   Barrel   — upright orange cylinder, rammable
//   Log      — brown cylinder on its side, driveable-over
//   LowWall  — concrete cuboid, blocks or hops
//
// Placement: 30x30 candidate grid (cell ~6.67 m).  For each cell an LCG draw
// decides whether anything spawns (>0.92 threshold → ~7 % density, ~63 total).
// Slope and origin-exclusion zones filter unsuitable locations.

use bevy::prelude::*;
use avian3d::prelude::*;

use crate::terrain::{terrain_height_at, TERRAIN_SEED};

pub struct ObstaclesPlugin;

impl Plugin for ObstaclesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_obstacles);
    }
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const GRID_CELLS: usize = 30;
const WORLD_SIZE: f32 = 200.0;
const CELL_SIZE: f32 = WORLD_SIZE / GRID_CELLS as f32; // ≈ 6.667 m

// Exclusion radius around the origin (spawn point).
const ORIGIN_RADIUS: f32 = 12.0;
// Maximum terrain slope to allow obstacle placement.
const MAX_SLOPE: f32 = 0.30;
// LCG draw threshold — only spawn when draw > this value (~7 % of cells).
const SPAWN_THRESHOLD: f32 = 0.92;

// ---------------------------------------------------------------------------
// Obstacle geometry constants
// ---------------------------------------------------------------------------

// Barrel: upright cylinder
const BARREL_RADIUS: f32 = 0.5;
const BARREL_HEIGHT: f32 = 1.2;
const BARREL_HALF_H: f32 = BARREL_HEIGHT * 0.5;

// Log: same cylinder, lying on its side (rotated 90° around Z)
const LOG_RADIUS: f32 = 0.35;
const LOG_LENGTH: f32 = 4.0;
const LOG_HALF_L: f32 = LOG_LENGTH * 0.5;

// Low wall: cuboid
const WALL_W: f32 = 3.0;
const WALL_H: f32 = 0.8;
const WALL_D: f32 = 0.4;
const WALL_HALF_H: f32 = WALL_H * 0.5;

// ---------------------------------------------------------------------------
// LCG — same style used in ramps.rs
// ---------------------------------------------------------------------------

struct Lcg(u64);

impl Lcg {
    fn new(seed: u32) -> Self {
        Self(seed as u64)
    }

    /// Next float in [0, 1).
    fn next_f32(&mut self) -> f32 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223) & 0xFFFF_FFFF;
        (self.0 as f32) / (u32::MAX as f32)
    }
}

// ---------------------------------------------------------------------------
// Slope helper (mirrors scatter.rs / ramps.rs)
// ---------------------------------------------------------------------------

const SLOPE_STEP: f32 = 1.0;

fn compute_slope(x: f32, z: f32) -> f32 {
    let h  = terrain_height_at(x, z);
    let hx = terrain_height_at(x + SLOPE_STEP, z);
    let hz = terrain_height_at(x, z + SLOPE_STEP);
    let nxv = Vec3::new(SLOPE_STEP, hx - h, 0.0).normalize();
    let nzv = Vec3::new(0.0, hz - h, SLOPE_STEP).normalize();
    let n = nxv.cross(nzv).normalize();
    1.0 - n.dot(Vec3::Y).clamp(0.0, 1.0)
}

// ---------------------------------------------------------------------------
// Obstacle type
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
enum ObstacleKind {
    Barrel,
    Log,
    LowWall,
}

impl ObstacleKind {
    fn from_index(i: u32) -> Self {
        match i % 3 {
            0 => Self::Barrel,
            1 => Self::Log,
            _ => Self::LowWall,
        }
    }

    /// Half-height offset so the obstacle sits on the ground surface.
    fn ground_lift(self) -> f32 {
        match self {
            Self::Barrel  => BARREL_HALF_H,
            // Log is on its side: its radius becomes the vertical extent.
            Self::Log     => LOG_RADIUS,
            Self::LowWall => WALL_HALF_H,
        }
    }
}

// ---------------------------------------------------------------------------
// Spawn system
// ---------------------------------------------------------------------------

fn spawn_obstacles(
    mut commands: Commands,
    mut meshes:   ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // --- shared materials ---
    let barrel_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.4, 0.1),
        perceptual_roughness: 0.7,
        ..default()
    });
    // Red stripe band — a second, narrower cylinder at mid-height.
    let stripe_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.8, 0.05, 0.05),
        perceptual_roughness: 0.6,
        ..default()
    });
    let log_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.32, 0.20, 0.10),
        perceptual_roughness: 0.95,
        ..default()
    });
    let wall_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.55, 0.55, 0.58),
        perceptual_roughness: 0.9,
        ..default()
    });

    // --- shared meshes ---
    let barrel_mesh = meshes.add(Cylinder::new(BARREL_RADIUS, BARREL_HEIGHT));
    // Thin cylinder for the red stripe (slightly larger radius to sit on top of barrel shell).
    let stripe_mesh = meshes.add(Cylinder::new(BARREL_RADIUS + 0.005, 0.15));
    let log_mesh    = meshes.add(Cylinder::new(LOG_RADIUS, LOG_LENGTH));
    let wall_mesh   = meshes.add(Cuboid::new(WALL_W, WALL_H, WALL_D));

    let mut lcg = Lcg::new(TERRAIN_SEED + 19);

    let half = WORLD_SIZE * 0.5;

    let mut kind_counter = [0u32; 3]; // [barrels, logs, walls]
    let mut kind_index = 0u32;

    for gz in 0..GRID_CELLS {
        for gx in 0..GRID_CELLS {
            // 1. Spawn gate — draw must exceed threshold.
            let gate = lcg.next_f32();
            if gate <= SPAWN_THRESHOLD {
                // Still advance the other draws to keep the sequence deterministic.
                let _ = lcg.next_f32(); // jitter x
                let _ = lcg.next_f32(); // jitter z
                let _ = lcg.next_f32(); // y rotation
                continue;
            }

            // 2. Cell centre.
            let cx = -half + (gx as f32 + 0.5) * CELL_SIZE;
            let cz = -half + (gz as f32 + 0.5) * CELL_SIZE;

            // 3. Random XZ position within the cell.
            let jx = (lcg.next_f32() * 2.0 - 1.0) * (CELL_SIZE * 0.45);
            let jz = (lcg.next_f32() * 2.0 - 1.0) * (CELL_SIZE * 0.45);
            let wx = cx + jx;
            let wz = cz + jz;

            // 4. Skip if within the origin exclusion radius.
            if wx * wx + wz * wz < ORIGIN_RADIUS * ORIGIN_RADIUS {
                let _ = lcg.next_f32(); // y rotation (consume)
                continue;
            }

            // 5. Skip on steep terrain.
            if compute_slope(wx, wz) > MAX_SLOPE {
                let _ = lcg.next_f32(); // y rotation (consume)
                continue;
            }

            // 6. Y rotation.
            let yaw = lcg.next_f32() * std::f32::consts::TAU;

            // 7. Obstacle kind, cycling through enum variants.
            let kind = ObstacleKind::from_index(kind_index);
            kind_index = kind_index.wrapping_add(1);

            // 8. Ground height + lift so obstacle sits on the surface.
            let ground_y = terrain_height_at(wx, wz);
            let y = ground_y + kind.ground_lift();

            match kind {
                ObstacleKind::Barrel => {
                    let parent = commands.spawn((
                        Transform::from_translation(Vec3::new(wx, y, wz))
                            .with_rotation(Quat::from_rotation_y(yaw)),
                        Visibility::default(),
                        RigidBody::Static,
                        Collider::cylinder(BARREL_RADIUS, BARREL_HEIGHT),
                    )).id();

                    // Main barrel body.
                    let body = commands.spawn((
                        Mesh3d(barrel_mesh.clone()),
                        MeshMaterial3d(barrel_mat.clone()),
                        Transform::IDENTITY,
                    )).id();

                    // Red horizontal stripe near mid-height.
                    let stripe = commands.spawn((
                        Mesh3d(stripe_mesh.clone()),
                        MeshMaterial3d(stripe_mat.clone()),
                        Transform::from_translation(Vec3::new(0.0, 0.15, 0.0)),
                    )).id();

                    commands.entity(parent).add_children(&[body, stripe]);
                    kind_counter[0] += 1;
                }

                ObstacleKind::Log => {
                    // Cylinder lies on its side: rotate 90° around Z so its axis is X.
                    let rotation = Quat::from_rotation_y(yaw)
                        * Quat::from_rotation_z(std::f32::consts::FRAC_PI_2);

                    commands.spawn((
                        Mesh3d(log_mesh.clone()),
                        MeshMaterial3d(log_mat.clone()),
                        Transform::from_translation(Vec3::new(wx, y, wz))
                            .with_rotation(rotation),
                        RigidBody::Static,
                        Collider::cylinder(LOG_RADIUS, LOG_LENGTH),
                    ));
                    kind_counter[1] += 1;
                }

                ObstacleKind::LowWall => {
                    commands.spawn((
                        Mesh3d(wall_mesh.clone()),
                        MeshMaterial3d(wall_mat.clone()),
                        Transform::from_translation(Vec3::new(wx, y, wz))
                            .with_rotation(Quat::from_rotation_y(yaw)),
                        RigidBody::Static,
                        Collider::cuboid(WALL_W * 0.5, WALL_H * 0.5, WALL_D * 0.5),
                    ));
                    kind_counter[2] += 1;
                }
            }
        }
    }

    // Log a quick tally (visible only in dev / debug builds).
    bevy::log::info!(
        "obstacles: {} barrels, {} logs, {} walls ({} total)",
        kind_counter[0],
        kind_counter[1],
        kind_counter[2],
        kind_counter[0] + kind_counter[1] + kind_counter[2],
    );
}
