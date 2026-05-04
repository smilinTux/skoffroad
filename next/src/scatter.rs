// Procedural tree and rock placement across the 200x200 m terrain.
//
// Placement strategy:
//   - 50x50 candidate grid = 4 m cell spacing
//   - Trees on flat/gentle slopes (slope < 0.20, noise > 0.4)
//   - Rocks on medium-to-steep slopes (slope > 0.30, or slope > 0.15 and noise > 0.6)
//   - Per-instance jitter in position, Y-rotation, and scale for visual variety

use bevy::prelude::*;
use avian3d::prelude::*;
use noise::{NoiseFn, Perlin};

use crate::terrain::{terrain_height_at, TERRAIN_SEED};

pub struct ScatterPlugin;

impl Plugin for ScatterPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_scatter);
    }
}

const GRID_CELLS: usize = 50;
const WORLD_SIZE: f32 = 200.0;
// Width of one grid cell in world space.
const CELL_SIZE: f32 = WORLD_SIZE / GRID_CELLS as f32;
// Maximum random offset within a cell (±0.7 * cell size).
const JITTER: f32 = CELL_SIZE * 0.7;

// Finite-difference step for slope estimation — matches terrain.rs GRID resolution.
const SLOPE_STEP: f32 = 1.0;

fn compute_slope(x: f32, z: f32) -> f32 {
    let h  = terrain_height_at(x, z);
    let hx = terrain_height_at(x + SLOPE_STEP, z);
    let hz = terrain_height_at(x, z + SLOPE_STEP);
    let nx_v = Vec3::new(SLOPE_STEP, hx - h, 0.0).normalize();
    let nz_v = Vec3::new(0.0, hz - h, SLOPE_STEP).normalize();
    let n = nx_v.cross(nz_v).normalize();
    // slope = 0 on flat ground, 1 on a vertical face.
    1.0 - n.dot(Vec3::Y).clamp(0.0, 1.0)
}

// Simple deterministic hash → float in [0, 1) for per-cell jitter and scale.
// We don't need Perlin for these; a fast integer hash is sufficient.
#[inline]
fn hash2(a: i32, b: i32, salt: u32) -> f32 {
    let mut v = (a.wrapping_mul(374761393))
        .wrapping_add(b.wrapping_mul(668265263))
        .wrapping_add(salt as i32);
    v ^= v >> 13;
    v = v.wrapping_mul(1274126177);
    v ^= v >> 16;
    (v as u32) as f32 / u32::MAX as f32
}

fn spawn_scatter(
    mut commands: Commands,
    mut meshes:   ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Separate Perlin instances seeded off TERRAIN_SEED so trees and rocks
    // have independent spatial patterns that differ from the heightmap noise.
    let tree_noise  = Perlin::new(TERRAIN_SEED + 1);
    let rock_noise  = Perlin::new(TERRAIN_SEED + 2);

    // --- shared materials ---
    let trunk_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.35, 0.25, 0.15),
        perceptual_roughness: 0.9,
        ..default()
    });
    let foliage_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.18, 0.40, 0.18),
        perceptual_roughness: 0.85,
        ..default()
    });
    let rock_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.42, 0.42, 0.45),
        perceptual_roughness: 0.95,
        ..default()
    });

    // --- shared meshes ---
    let trunk_mesh   = meshes.add(Cylinder::new(0.15, 1.5));
    let foliage_mesh = meshes.add(Cone { radius: 0.8, height: 1.8 });
    // Sphere with a small radius; scale applied per-instance to make it look organic.
    let rock_mesh    = meshes.add(Sphere::new(0.5).mesh().ico(1).unwrap());

    let half = WORLD_SIZE * 0.5;

    for gz in 0..GRID_CELLS {
        for gx in 0..GRID_CELLS {
            // Cell centre in world space.
            let cx = -half + (gx as f32 + 0.5) * CELL_SIZE;
            let cz = -half + (gz as f32 + 0.5) * CELL_SIZE;

            // Noise coordinates in [0, 1] across the terrain.
            let nx = (cx / WORLD_SIZE + 0.5) as f64;
            let nz = (cz / WORLD_SIZE + 0.5) as f64;

            let t_val = tree_noise.get([nx * 6.0, nz * 6.0]) as f32 * 0.5 + 0.5;
            let r_val = rock_noise.get([nx * 7.0, nz * 7.0]) as f32 * 0.5 + 0.5;

            let slope = compute_slope(cx, cz);

            // ---- tree placement ----
            if slope < 0.20 && t_val > 0.4 {
                let jx  = (hash2(gx as i32, gz as i32, 10) * 2.0 - 1.0) * JITTER;
                let jz  = (hash2(gx as i32, gz as i32, 20) * 2.0 - 1.0) * JITTER;
                let rot = hash2(gx as i32, gz as i32, 30) * std::f32::consts::TAU;
                let scale = 0.7 + hash2(gx as i32, gz as i32, 40) * 0.7;

                let wx = cx + jx;
                let wz = cz + jz;
                let h  = terrain_height_at(wx, wz);

                let parent = commands.spawn((
                    Transform::from_translation(Vec3::new(wx, h, wz))
                        .with_rotation(Quat::from_rotation_y(rot))
                        .with_scale(Vec3::splat(scale)),
                    Visibility::default(),
                    RigidBody::Static,
                    Collider::cuboid(0.3, 1.5, 0.3),
                )).id();

                // Trunk: half-height above ground.
                let trunk = commands.spawn((
                    Mesh3d(trunk_mesh.clone()),
                    MeshMaterial3d(trunk_mat.clone()),
                    // Cylinder is centred at origin; lift by half its height (0.75).
                    Transform::from_translation(Vec3::new(0.0, 0.75, 0.0)),
                )).id();

                // Foliage cone: sits above the trunk (trunk top at 1.5, cone half-height 0.9).
                let foliage = commands.spawn((
                    Mesh3d(foliage_mesh.clone()),
                    MeshMaterial3d(foliage_mat.clone()),
                    Transform::from_translation(Vec3::new(0.0, 1.5 + 0.9, 0.0)),
                )).id();

                commands.entity(parent).add_children(&[trunk, foliage]);
            }

            // ---- rock placement ----
            let place_rock = slope > 0.30 || (slope > 0.15 && r_val > 0.6);
            if place_rock {
                let jx  = (hash2(gx as i32, gz as i32, 50) * 2.0 - 1.0) * JITTER;
                let jz  = (hash2(gx as i32, gz as i32, 60) * 2.0 - 1.0) * JITTER;
                let rot = hash2(gx as i32, gz as i32, 70) * std::f32::consts::TAU;

                // Non-uniform scale for organic squashed/stretched look.
                let sx = 0.4 + hash2(gx as i32, gz as i32, 80) * 0.8;
                let sy = 0.4 + hash2(gx as i32, gz as i32, 90) * 0.8;
                let sz = 0.4 + hash2(gx as i32, gz as i32, 100) * 0.8;

                let wx = cx + jx;
                let wz = cz + jz;
                let h  = terrain_height_at(wx, wz);

                // Collider radius approximated as the mean of the three scale axes
                // (the base mesh sphere has radius 0.5, so effective radius = mean * 0.5).
                let cr = (sx + sy + sz) / 3.0 * 0.5;

                commands.spawn((
                    Mesh3d(rock_mesh.clone()),
                    MeshMaterial3d(rock_mat.clone()),
                    Transform::from_translation(Vec3::new(wx, h, wz))
                        .with_rotation(Quat::from_rotation_y(rot))
                        .with_scale(Vec3::new(sx, sy, sz)),
                    RigidBody::Static,
                    Collider::sphere(cr),
                ));
            }
        }
    }
}
