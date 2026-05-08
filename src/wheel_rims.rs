// Wheel rims: 5-spoke offroad rim design for each wheel. Each rim consists of
// 5 spoke cuboids radiating from a central hub in the wheel-local XZ rolling
// plane (the plane perpendicular to the wheel axle). Spokes sit between the
// hub cap (radius 0.12) and the tread blocks (radius 0.36) added by
// wheel_detail.rs, occupying the radial band 0.05–0.25.
//
// Coordinate convention (wheel-local space):
//   The wheel entity has Quat::from_rotation_z(FRAC_PI_2) baked in, mapping
//   the cylinder's +Y axis to chassis -X (lateral). The XZ plane in wheel
//   local space is therefore the rolling plane. Spokes are placed in this XZ
//   plane and offset slightly along +Y to sit on the outward face.
//
// Public API:
//   WheelRimsPlugin
//   WheelRim   (marker component on each spoke entity)

use std::f32::consts::TAU;
use bevy::prelude::*;
use crate::vehicle::{VehicleRoot, Wheel};

// ---- Mirror vehicle.rs ----
const WHEEL_HALF_WIDTH: f32 = 0.18;

// Spoke count and geometry.
const SPOKE_COUNT: usize = 5;

// Cuboid half-extents: 0.04 × 0.04 × 0.20 (cross-section × cross-section × radial length).
const SPOKE_W: f32 = 0.04;
const SPOKE_H: f32 = 0.04;
const SPOKE_D: f32 = 0.20;

// Spokes are placed with their centre at radius 0.15 in the XZ plane:
//   inner tip ≈ 0.05, outer tip ≈ 0.25 — between hub (0.12) and tread (0.36).
const SPOKE_CENTRE_RADIUS: f32 = 0.15;

// Small phase offset so no spoke aligns with lug-nut positions (which are at
// multiples of TAU/5 with no offset in wheel_detail.rs).
const PHASE_OFFSET: f32 = TAU / 10.0; // 36 ° offset → halfway between lug nuts

// Y offset along wheel face: 70% of half-width keeps spokes visible from the
// wheel-cam without clipping into the rim cylinder already rendered by vehicle.rs.
const SPOKE_Y_OFFSET: f32 = WHEEL_HALF_WIDTH * 0.7;

// ---- Marker component ----

#[derive(Component)]
pub struct WheelRim;

// ---- Plugin ----

pub struct WheelRimsPlugin;

impl Plugin for WheelRimsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, attach_rims_once);
    }
}

// ---- System ----

/// Runs every frame until VehicleRoot is available and all Wheel entities
/// exist, then attaches 5 spoke cuboids as children of each wheel entity.
/// A `Local<bool>` guard ensures it fires exactly once.
fn attach_rims_once(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    vehicle: Option<Res<VehicleRoot>>,
    wheel_q: Query<Entity, With<Wheel>>,
    mut done: Local<bool>,
) {
    if *done { return; }
    // Wait until VehicleRoot is present (inserted by vehicle.rs Startup system).
    let Some(_vehicle) = vehicle else { return };

    // Build shared mesh and material once; clone handles for each spoke.
    let spoke_mesh = meshes.add(Cuboid::new(SPOKE_W, SPOKE_H, SPOKE_D));
    let spoke_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.30, 0.30, 0.32),
        perceptual_roughness: 0.5,
        metallic: 0.6,
        ..default()
    });

    for wheel_entity in wheel_q.iter() {
        let mut children: Vec<Entity> = Vec::with_capacity(SPOKE_COUNT);

        for i in 0..SPOKE_COUNT {
            let angle = i as f32 * TAU / SPOKE_COUNT as f32 + PHASE_OFFSET;

            // Position: wheel-local XZ rolling plane, offset along +Y (outward face).
            let pos = Vec3::new(
                SPOKE_CENTRE_RADIUS * angle.sin(),
                SPOKE_Y_OFFSET,
                SPOKE_CENTRE_RADIUS * angle.cos(),
            );

            // Rotate the spoke so its longest dimension (Z = SPOKE_D) points radially
            // outward from the hub. from_rotation_y(angle) does exactly that because
            // the cuboid's +Z aligns with the radius direction at `angle` in XZ.
            let rot = Quat::from_rotation_y(angle);

            let spoke = commands.spawn((
                WheelRim,
                Mesh3d(spoke_mesh.clone()),
                MeshMaterial3d(spoke_mat.clone()),
                Transform::from_translation(pos).with_rotation(rot),
            )).id();
            children.push(spoke);
        }

        commands.entity(wheel_entity).add_children(&children);
    }

    *done = true;
}
