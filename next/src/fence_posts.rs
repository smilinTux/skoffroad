// Fence posts: 3 procedural fence rows along approach paths to landmarks
// (rock garden, lighthouse, hillclimb). Each post = wood cuboid + 3 horizontal
// rail cuboids spanning to the next post.
//
// Sprint 35 — fully populated.
//
// Public API:
//   FencePostsPlugin
//   FencePost  (component)

use bevy::prelude::*;
use avian3d::prelude::*;

use crate::terrain::terrain_height_at;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

pub struct FencePostsPlugin;

/// Marker component placed on every fence-post entity.
#[derive(Component)]
pub struct FencePost;

impl Plugin for FencePostsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_fence_rows);
    }
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

// Post geometry (half-extents for collider; full for mesh)
const POST_W: f32 = 0.15;
const POST_H: f32 = 1.5;
const POST_D: f32 = 0.15;

const POST_HALF_W: f32 = POST_W * 0.5;
const POST_HALF_H: f32 = POST_H * 0.5;
const POST_HALF_D: f32 = POST_D * 0.5;

// Rail geometry
const RAIL_H: f32 = 0.08;
const RAIL_D: f32 = 0.04;

// Rail Y-offsets measured from the base of the post (terrain_y)
const RAIL_Y_OFFSETS: [f32; 3] = [0.4, 0.8, 1.2];

// Wood colour / material
const WOOD_COLOR: Srgba = Srgba::new(0.50, 0.38, 0.25, 1.0);
const WOOD_ROUGHNESS: f32 = 0.95;

// ---------------------------------------------------------------------------
// Row definitions  (start, end, post_count)
// ---------------------------------------------------------------------------

struct FenceRow {
    start: Vec3,
    end: Vec3,
    count: usize,
}

const FENCE_ROWS: [FenceRow; 3] = [
    // Row 1 — toward rock garden
    FenceRow {
        start: Vec3::new(40.0, 0.0, 0.0),
        end: Vec3::new(110.0, 0.0, 0.0),
        count: 12,
    },
    // Row 2 — toward lighthouse
    FenceRow {
        start: Vec3::new(35.0, 0.0, 60.0),
        end: Vec3::new(80.0, 0.0, 90.0),
        count: 12,
    },
    // Row 3 — toward hillclimb
    FenceRow {
        start: Vec3::new(-40.0, 0.0, -50.0),
        end: Vec3::new(-130.0, 0.0, -130.0),
        count: 12,
    },
];

// ---------------------------------------------------------------------------
// Startup system
// ---------------------------------------------------------------------------

fn spawn_fence_rows(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // One shared wood material for the entire fence set.
    let wood_mat = materials.add(StandardMaterial {
        base_color: WOOD_COLOR.into(),
        perceptual_roughness: WOOD_ROUGHNESS,
        ..default()
    });

    // Shared post mesh.
    let post_mesh = meshes.add(Cuboid::new(POST_W, POST_H, POST_D));

    for row in &FENCE_ROWS {
        spawn_row(
            &mut commands,
            &mut meshes,
            &wood_mat,
            &post_mesh,
            row,
        );
    }
}

// ---------------------------------------------------------------------------
// Per-row spawner
// ---------------------------------------------------------------------------

fn spawn_row(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    wood_mat: &Handle<StandardMaterial>,
    post_mesh: &Handle<Mesh>,
    row: &FenceRow,
) {
    // Build the N post positions along the segment.
    let positions: Vec<Vec3> = post_positions(row);

    for (i, &pos) in positions.iter().enumerate() {
        let terrain_y = terrain_height_at(pos.x, pos.z);
        let post_center_y = terrain_y + POST_HALF_H; // centre of the 1.5 m post

        // Spawn the post entity.
        commands.spawn((
            FencePost,
            Mesh3d(post_mesh.clone()),
            MeshMaterial3d(wood_mat.clone()),
            Transform::from_translation(Vec3::new(pos.x, post_center_y, pos.z)),
            RigidBody::Static,
            Collider::cuboid(POST_HALF_W, POST_HALF_H, POST_HALF_D),
        ));

        // Spawn rails to the NEXT post (no rail after the last post).
        if i + 1 < positions.len() {
            let next = positions[i + 1];
            spawn_rails(commands, meshes, wood_mat, pos, terrain_y, next);
        }
    }
}

// ---------------------------------------------------------------------------
// Rail spawner for a single gap
// ---------------------------------------------------------------------------

fn spawn_rails(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    wood_mat: &Handle<StandardMaterial>,
    post_xz: Vec3,   // current post (x, _, z)
    post_base_y: f32, // terrain_y under current post
    next_xz: Vec3,   // next post (x, _, z)
) {
    let dx = next_xz.x - post_xz.x;
    let dz = next_xz.z - post_xz.z;
    let gap = (dx * dx + dz * dz).sqrt();

    if gap < 1e-4 {
        return; // degenerate gap — skip
    }

    // Rail mesh: full gap length × rail cross-section.
    let rail_mesh = meshes.add(Cuboid::new(gap, RAIL_H, RAIL_D));

    // Direction angle (rotation around Y so the rail aligns with the fence row).
    let yaw = (-dz).atan2(dx); // angle of the gap vector in XZ

    // Midpoint in XZ between the two posts.
    let mid_x = post_xz.x + dx * 0.5;
    let mid_z = post_xz.z + dz * 0.5;

    for &y_offset in &RAIL_Y_OFFSETS {
        let rail_y = post_base_y + y_offset;

        commands.spawn((
            Mesh3d(rail_mesh.clone()),
            MeshMaterial3d(wood_mat.clone()),
            Transform::from_translation(Vec3::new(mid_x, rail_y, mid_z))
                .with_rotation(Quat::from_rotation_y(yaw)),
        ));
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Compute the N evenly-spaced post positions along a straight segment.
fn post_positions(row: &FenceRow) -> Vec<Vec3> {
    let n = row.count;
    if n == 0 {
        return vec![];
    }
    if n == 1 {
        return vec![row.start];
    }
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let t = i as f32 / (n - 1) as f32;
        out.push(Vec3::new(
            row.start.x + t * (row.end.x - row.start.x),
            0.0, // Y resolved per-post from terrain
            row.start.z + t * (row.end.z - row.start.z),
        ));
    }
    out
}
