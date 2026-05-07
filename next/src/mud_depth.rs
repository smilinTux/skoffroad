// Mud-depth deformation: each MudPuddle gets its own 16×16 deformable mesh.
// When a wheel passes through, vertices within 0.5 m of the contact point are
// pushed downward, creating persistent tyre-impression ruts in the mud surface.
// Unlike ground_ruts.rs (decal quads), this modifies actual mesh geometry.
//
// Architecture:
//   1. replace_puddles_with_deformable_mesh (one-shot, Local<bool> guard)
//      – Replaces every flat Plane3d MudPuddle mesh with a hand-built 16×16
//        grid mesh using RenderAssetUsages::MAIN_WORLD | RENDER_WORLD so it
//        can be mutated on the CPU each frame.
//      – Attaches DeformablePuddleMesh { mesh_handle } to the entity.
//      – Initialises per-puddle vertex depths in PuddleDepths.
//
//   2. deform_under_wheels (every frame)
//      – For each grounded wheel inside a puddle's radius, sinks vertices
//        within 0.5 m of the wheel contact. Max depth capped at -0.5 m.
//      – Rewrites ATTRIBUTE_POSITION via meshes.get_mut. Also recomputes
//        smooth normals so lighting updates correctly.
//
// Public API:
//   MudDepthPlugin

use bevy::{
    asset::RenderAssetUsages,
    mesh::{Indices, PrimitiveTopology},
    prelude::*,
};

use crate::mud_puddles::MudPuddle;
use crate::vehicle::{Chassis, Wheel, VehicleRoot};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Vertex resolution of the deformable grid (N×N vertices → (N-1)×(N-1) quads).
const GRID_N: usize = 16;

/// Horizontal influence radius (metres): vertices within this XZ distance of
/// the wheel contact point are deformed.
const DEFORM_RADIUS: f32 = 0.5;

/// Rate of deformation per second (metres/sec downward).
const DEFORM_RATE: f32 = 0.05;

/// Maximum allowed downward displacement (positive value; Y is negated).
const MAX_DEPTH: f32 = 0.5;

/// Number of vertices in one deformable puddle mesh.
const VERT_COUNT: usize = GRID_N * GRID_N;

// ---------------------------------------------------------------------------
// Components / Resources
// ---------------------------------------------------------------------------

/// Attached to a MudPuddle entity once its mesh has been replaced with the
/// deformable grid.  Holds the mesh handle so deform_under_wheels can look it
/// up directly without re-querying Assets<Mesh>.
#[derive(Component)]
pub struct DeformablePuddleMesh {
    pub mesh_handle: Handle<Mesh>,
}

/// Per-puddle, per-vertex depth values (metres, non-negative; applied as -depth
/// to Y during mesh updates).  Indexed as [puddle_idx][vertex_idx].
///
/// This resource is the authoritative "persistence layer"; the Mesh asset holds
/// only the rendered snapshot.
#[derive(Resource, Default)]
pub struct PuddleDepths {
    /// `entries[i]` holds the Vec<f32> of depths for the i-th puddle (indexed
    /// by insertion order via PuddleSlot).
    pub entries: Vec<Vec<f32>>,
}

/// Internal slot index assigned to each puddle entity so PuddleDepths can be
/// accessed by position.
#[derive(Component)]
struct PuddleSlot(usize);

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct MudDepthPlugin;

impl Plugin for MudDepthPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PuddleDepths>()
           .add_systems(Update, (
               replace_puddles_with_deformable_mesh,
               deform_under_wheels,
           ));
    }
}

// ---------------------------------------------------------------------------
// System 1 — replace_puddles_with_deformable_mesh
// ---------------------------------------------------------------------------
//
// Runs every frame but short-circuits on the first success via Local<bool>.
// Waits until at least one MudPuddle entity is present (MudPuddlesPlugin
// spawns them in Startup, so this system will find them on the first Update).
//
// For each puddle: builds a 16×16 subdivided grid mesh sized to the puddle's
// radius, then replaces the entity's Mesh3d handle.
// ---------------------------------------------------------------------------

fn replace_puddles_with_deformable_mesh(
    mut commands:   Commands,
    mut meshes:     ResMut<Assets<Mesh>>,
    mut depths:     ResMut<PuddleDepths>,
    puddle_q:       Query<(Entity, &MudPuddle), Without<DeformablePuddleMesh>>,
    mut done:       Local<bool>,
) {
    // Already ran successfully — skip every subsequent frame.
    if *done {
        return;
    }

    // Collect puddles that still need replacement.
    let pending: Vec<(Entity, Vec3, f32)> = puddle_q
        .iter()
        .map(|(e, p)| (e, p.center, p.radius))
        .collect();

    if pending.is_empty() {
        // MudPuddlesPlugin hasn't run yet; try again next frame.
        return;
    }

    for (entity, center, radius) in pending {
        let slot = depths.entries.len();
        depths.entries.push(vec![0.0_f32; VERT_COUNT]);

        let mesh = build_deformable_mesh(center, radius);
        let handle = meshes.add(mesh);

        commands.entity(entity)
            .insert(Mesh3d(handle.clone()))
            .insert(DeformablePuddleMesh { mesh_handle: handle })
            .insert(PuddleSlot(slot));
    }

    *done = true;
}

// ---------------------------------------------------------------------------
// System 2 — deform_under_wheels
// ---------------------------------------------------------------------------
//
// Each frame: find every grounded wheel that lies within a puddle's XZ radius.
// For matching puddles, sink the vertices closest to the wheel contact.
// Writes the updated positions back into the Mesh asset.
// ---------------------------------------------------------------------------

fn deform_under_wheels(
    vehicle:    Option<Res<VehicleRoot>>,
    chassis_q:  Query<&Transform, With<Chassis>>,
    wheel_q:    Query<&Wheel, Without<Chassis>>,
    puddle_q:   Query<(&MudPuddle, &DeformablePuddleMesh, &PuddleSlot, &Transform)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut depths: ResMut<PuddleDepths>,
    time:       Res<Time>,
) {
    let Some(vehicle) = vehicle else { return };
    let Ok(chassis_tf) = chassis_q.get(vehicle.chassis) else { return };

    let dt = time.delta_secs();
    let chassis_pos = chassis_tf.translation;
    let chassis_rot = chassis_tf.rotation;

    // Collect world positions of grounded wheels (mirrors splash_particles.rs).
    let grounded: Vec<Vec3> = wheel_q
        .iter()
        .filter(|w| w.is_grounded)
        .map(|w| chassis_pos + chassis_rot * wheel_local_offset(w.index))
        .collect();

    if grounded.is_empty() {
        return;
    }

    for (puddle, deformable, slot, _puddle_tf) in puddle_q.iter() {
        let radius = puddle.radius;

        // Check whether any grounded wheel lies inside the puddle's XZ footprint.
        let hitting_wheels: Vec<Vec3> = grounded
            .iter()
            .copied()
            .filter(|wp| {
                let dx = wp.x - puddle.center.x;
                let dz = wp.z - puddle.center.z;
                (dx * dx + dz * dz).sqrt() <= radius
            })
            .collect();

        if hitting_wheels.is_empty() {
            continue;
        }

        let Some(mesh) = meshes.get_mut(&deformable.mesh_handle) else { continue };
        let entry = &mut depths.entries[slot.0];

        // Extract positions via the public as_float3() accessor.
        let mut positions: Vec<[f32; 3]> = match mesh
            .attribute(Mesh::ATTRIBUTE_POSITION)
            .and_then(|a| a.as_float3())
        {
            Some(v) => v.to_vec(),
            None => continue,
        };

        // Deform vertices near each hitting wheel.
        for wheel_pos in &hitting_wheels {
            for vi in 0..VERT_COUNT {
                let [vx, _vy, vz] = positions[vi];
                // Vertex position is in puddle-local space (centered at origin).
                // Convert to world XZ for distance test.
                let world_vx = vx + puddle.center.x;
                let world_vz = vz + puddle.center.z;

                let dx = world_vx - wheel_pos.x;
                let dz = world_vz - wheel_pos.z;
                let dist = (dx * dx + dz * dz).sqrt();

                if dist > DEFORM_RADIUS {
                    continue;
                }

                // Smooth falloff: vertices closer to wheel centre sink more.
                let falloff = 1.0 - (dist / DEFORM_RADIUS);
                let delta = DEFORM_RATE * falloff * dt;

                let new_depth = (entry[vi] + delta).min(MAX_DEPTH);
                entry[vi] = new_depth;

                // Y in mesh-local space: starts at 0 (flat plane), sinks negative.
                positions[vi][1] = -new_depth;
            }
        }

        // Write back updated positions.
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        // Recompute normals so lighting reflects the deformed shape.
        let _ = mesh.compute_smooth_normals();
    }
}

// ---------------------------------------------------------------------------
// Mesh builder
// ---------------------------------------------------------------------------
//
// Produces a GRID_N × GRID_N vertex grid, flat at Y=0, spanning [-radius,
// +radius] on both X and Z axes in LOCAL space.  The entity's Transform places
// it at the puddle's world center; only Y deformation happens at runtime.
// ---------------------------------------------------------------------------

fn build_deformable_mesh(_center: Vec3, radius: f32) -> Mesh {
    let n = GRID_N;
    let mut positions: Vec<[f32; 3]> = Vec::with_capacity(n * n);
    let mut normals:   Vec<[f32; 3]> = Vec::with_capacity(n * n);
    let mut uvs:       Vec<[f32; 2]> = Vec::with_capacity(n * n);

    // The entity Transform is set at `center`, so vertex positions are
    // relative to that origin (local space).  X and Z span [-radius, +radius].
    let step = (2.0 * radius) / (n - 1) as f32;

    for row in 0..n {
        for col in 0..n {
            let lx = -radius + col as f32 * step;
            let lz = -radius + row as f32 * step;
            positions.push([lx, 0.0, lz]);
            normals.push([0.0, 1.0, 0.0]);
            uvs.push([col as f32 / (n - 1) as f32, row as f32 / (n - 1) as f32]);
        }
    }

    // Pre-bake very subtle dimples (optional cosmetic touch that shows the
    // mesh is deformable even before any wheels arrive).
    // Each vertex gets a tiny concave offset based on its distance from the
    // local origin, so the centre of the puddle sits very slightly lower.
    for vi in 0..(n * n) {
        let [lx, _, lz] = positions[vi];
        let dist_norm = ((lx * lx + lz * lz).sqrt() / radius).min(1.0);
        // Inward bowl: deepest at centre (dist_norm=0), zero at rim (1.0).
        // Maximum pre-bake depth 0.01 m (barely perceptible).
        positions[vi][1] = -(1.0 - dist_norm) * 0.01;
    }

    let mut indices: Vec<u32> = Vec::with_capacity((n - 1) * (n - 1) * 6);
    for row in 0..(n - 1) {
        for col in 0..(n - 1) {
            let tl = (row * n + col) as u32;
            let tr = tl + 1;
            let bl = ((row + 1) * n + col) as u32;
            let br = bl + 1;
            indices.extend_from_slice(&[tl, bl, tr, tr, bl, br]);
        }
    }

    // MAIN_WORLD | RENDER_WORLD: required so CPU-side mutations each frame
    // are uploaded to the GPU (same pattern as water.rs).
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

// ---------------------------------------------------------------------------
// Helper: wheel-anchor offsets in chassis-local space
// ---------------------------------------------------------------------------
//
// Mirrors WHEEL_OFFSETS from vehicle.rs without importing the private constant.
// FL=0, FR=1, RL=2, RR=3.
// ---------------------------------------------------------------------------

#[inline]
fn wheel_local_offset(index: usize) -> Vec3 {
    match index {
        0 => Vec3::new(-1.1, -0.35, -1.4),
        1 => Vec3::new( 1.1, -0.35, -1.4),
        2 => Vec3::new(-1.1, -0.35,  1.4),
        _ => Vec3::new( 1.1, -0.35,  1.4),
    }
}
