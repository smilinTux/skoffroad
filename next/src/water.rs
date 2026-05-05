use bevy::{
    asset::RenderAssetUsages,
    mesh::{Indices, PrimitiveTopology},
    prelude::*,
};
use avian3d::prelude::*;

use crate::vehicle::{Chassis, VehicleRoot};

pub struct WaterPlugin;

impl Plugin for WaterPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_water)
           .add_systems(Update, animate_water)
           .add_systems(PhysicsSchedule,
               buoyancy_system
                   .after(PhysicsStepSystems::NarrowPhase)
                   .before(PhysicsStepSystems::Solver),
           );
    }
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

// Chosen so it sits in the lower ~25% of the terrain height range.
// Terrain uses HEIGHT_SCALE=12, so heights span roughly -12..+12.
// -3.0 sits in the bottom quarter and leaves interesting shoreline detail.
pub const WATER_LEVEL: f32 = -3.0;

// Grid resolution for the water plane. 30x30 quads = 31x31 = 961 verts.
const WATER_GRID: usize = 30;
// World-space extent of the water plane in metres.
const WATER_SIZE: f32 = 200.0;

// Chassis half-height used to normalise submersion depth.
// Mirrors CHASSIS_HALF.y from vehicle.rs without importing it.
const CHASSIS_HALF_Y: f32 = 0.4;
// Total chassis mass (kg) — mirrors CHASSIS_MASS from vehicle.rs.
const CHASSIS_MASS: f32 = 1500.0;
// Standard gravitational acceleration.
const GRAVITY: f32 = 9.81;

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

/// Marks the water mesh entity so the wave-animation system can find it.
#[derive(Component)]
struct WaterMesh;

// ---------------------------------------------------------------------------
// Startup: spawn the water plane
// ---------------------------------------------------------------------------

fn spawn_water(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mesh = build_water_mesh(0.0);
    let handle = meshes.add(mesh);

    let material = materials.add(StandardMaterial {
        base_color: Color::srgba(0.10, 0.30, 0.55, 0.65),
        alpha_mode: AlphaMode::Blend,
        perceptual_roughness: 0.15,
        metallic: 0.1,
        double_sided: true,
        cull_mode: None,
        ..default()
    });

    commands.spawn((
        WaterMesh,
        Mesh3d(handle),
        MeshMaterial3d(material),
        Transform::from_translation(Vec3::new(0.0, WATER_LEVEL, 0.0)),
    ));
}

// ---------------------------------------------------------------------------
// Animated wave displacement
// ---------------------------------------------------------------------------

fn animate_water(
    time: Res<Time>,
    water_q: Query<&Mesh3d, With<WaterMesh>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let Ok(mesh3d) = water_q.single() else { return };
    let Some(mesh) = meshes.get_mut(&mesh3d.0) else { return };

    let t = time.elapsed_secs();
    let vcount = WATER_GRID + 1;

    // Recompute positions in-place.
    let mut positions: Vec<[f32; 3]> = Vec::with_capacity(vcount * vcount);
    for row in 0..vcount {
        for col in 0..vcount {
            let wx = (col as f32 / WATER_GRID as f32 - 0.5) * WATER_SIZE;
            let wz = (row as f32 / WATER_GRID as f32 - 0.5) * WATER_SIZE;
            // Two crossing sine waves give a convincing ripple without heavy math.
            let dy = 0.1 * (wx * 0.5 + t).sin()
                   + 0.1 * (wz * 0.3 + t * 0.8).cos();
            positions.push([wx, dy, wz]);
        }
    }

    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    // Bevy 0.18: compute_flat_normals panics on indexed meshes; the water grid
    // uses indices, so use the smooth-normal variant. Visually it's actually
    // nicer for rolling waves than per-tri faceted shading anyway.
    let _ = mesh.compute_smooth_normals();
}

// ---------------------------------------------------------------------------
// Buoyancy + drag system (runs in PhysicsSchedule before the solver)
// ---------------------------------------------------------------------------

fn buoyancy_system(
    vehicle: Option<Res<VehicleRoot>>,
    mut chassis_q: Query<(Forces, &Transform), With<Chassis>>,
) {
    let Some(vehicle) = vehicle else { return };
    let Ok((mut forces, transform)) = chassis_q.get_mut(vehicle.chassis) else { return };

    let chassis_y = transform.translation.y;
    // How far the chassis centre has sunk below the water surface.
    let submersion = (WATER_LEVEL - chassis_y).max(0.0);
    if submersion <= 0.0 {
        return;
    }

    // --- Buoyancy ---
    // Cap raised to 3.0× weight (was 2.0×). At any meaningful submersion the
    // chassis gets a clear net upward force that overpowers gravity.
    let max_buoy = CHASSIS_MASS * GRAVITY * 3.0;
    let f_buoy = (CHASSIS_MASS * GRAVITY * (submersion / CHASSIS_HALF_Y)).min(max_buoy);
    forces.apply_force(Vec3::new(0.0, f_buoy, 0.0));

    // --- Drag ---
    // 60 N·s/m × clamped submersion. Was 120, then 250 originally — kept
    // dropping it because the user kept getting pinned. 60 is gentle enough
    // that 2 m/s = 120 N drag, well below the 4480 N reverse drive force.
    let vel = forces.linear_velocity();
    let drag_coeff = submersion.min(1.0) * 60.0;
    let drag_force = Vec3::new(
        -vel.x * drag_coeff,
        0.0, // no vertical drag — buoyancy handles up/down motion
        -vel.z * drag_coeff,
    );
    forces.apply_force(drag_force);
}

// ---------------------------------------------------------------------------
// Mesh builder — creates a subdivided XZ plane centred at the origin.
// The Y position is baked in as 0 here; the entity Transform places it at
// WATER_LEVEL, and the animate_water system perturbs Y per vertex each frame.
// ---------------------------------------------------------------------------

fn build_water_mesh(t: f32) -> Mesh {
    let vcount = WATER_GRID + 1;
    let mut positions: Vec<[f32; 3]> = Vec::with_capacity(vcount * vcount);
    let mut normals:   Vec<[f32; 3]> = Vec::with_capacity(vcount * vcount);
    let mut uvs:       Vec<[f32; 2]> = Vec::with_capacity(vcount * vcount);

    for row in 0..vcount {
        for col in 0..vcount {
            let wx = (col as f32 / WATER_GRID as f32 - 0.5) * WATER_SIZE;
            let wz = (row as f32 / WATER_GRID as f32 - 0.5) * WATER_SIZE;
            let dy = 0.1 * (wx * 0.5 + t).sin()
                   + 0.1 * (wz * 0.3 + t * 0.8).cos();
            positions.push([wx, dy, wz]);
            normals.push([0.0, 1.0, 0.0]);
            uvs.push([col as f32 / WATER_GRID as f32, row as f32 / WATER_GRID as f32]);
        }
    }

    let mut indices: Vec<u32> = Vec::with_capacity(WATER_GRID * WATER_GRID * 6);
    for row in 0..WATER_GRID {
        for col in 0..WATER_GRID {
            let tl = (row * vcount + col) as u32;
            let tr = tl + 1;
            let bl = ((row + 1) * vcount + col) as u32;
            let br = bl + 1;
            indices.extend_from_slice(&[tl, bl, tr, tr, bl, br]);
        }
    }

    // MAIN_WORLD | RENDER_WORLD so the CPU can mutate the mesh each frame and
    // the renderer still has access to the updated data.
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
