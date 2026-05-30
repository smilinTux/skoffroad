// Wheel rims: 5-spoke offroad rim design for each wheel. Each rim consists of
// 5 spoke cuboids radiating from a central hub in the wheel-local XZ rolling
// plane (the plane perpendicular to the wheel axle). Spokes sit between the
// hub cap (radius 0.12) and the tread blocks (radius 0.36) added by
// wheel_detail.rs, occupying the radial band 0.05–0.25.
//
// Sprint 81: quality-gated materials.
//   Medium+ → polished chrome (metallic 0.95, roughness 0.08, reflectance 0.95)
//              + a thin outer rim-ring accent (darker chrome barrel) for depth.
//   Low     → brushed aluminium (metallic 0.5, roughness 0.55).
//
// Coordinate convention (wheel-local space):
//   The wheel entity has Quat::from_rotation_z(FRAC_PI_2) baked in, mapping
//   the cylinder's +Y axis to chassis -X (lateral). The XZ plane in wheel
//   local space is therefore the rolling plane. Spokes are placed in this XZ
//   plane and offset slightly along +Y to sit on the outward face.
//
// Respawn guard:
//   Uses WheelRimAttached marker component (same pattern as wheel_detail.rs's
//   WheelDetailAttached) instead of Local<bool>. This means the system
//   correctly re-attaches spokes to fresh Wheel entities after a chassis
//   RespawnRequest without the Local<bool> guard blocking it.
//
// Public API:
//   WheelRimsPlugin
//   WheelRim          (marker component on each spoke entity)
//   WheelRimAttached  (marker placed on the Wheel entity once rims are attached)

use std::f32::consts::TAU;
use bevy::prelude::*;
use crate::vehicle::{VehicleRoot, Wheel};
use crate::graphics_quality::GraphicsQuality;

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

// Outer rim-ring accent: thin flat ring (Cylinder) at the tire barrel edge.
// Radius 0.26 (just inside the tread OD), height 0.02.
const RIM_RING_RADIUS: f32 = 0.26;
const RIM_RING_HEIGHT: f32 = 0.02;

// ---- Marker components ----

/// Placed on each individual spoke entity.
#[derive(Component)]
pub struct WheelRim;

/// Placed on the Wheel entity once its rim spokes have been attached.
/// Checked each frame so the system skips wheels that already have rims
/// and correctly re-attaches after a chassis respawn (new Wheel entities
/// won't carry this marker).
#[derive(Component)]
pub struct WheelRimAttached;

// ---- Plugin ----

pub struct WheelRimsPlugin;

impl Plugin for WheelRimsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, attach_rims);
    }
}

// ---- System ----

/// Runs every frame. For each Wheel entity that does NOT yet carry
/// `WheelRimAttached`, spawns 5 spoke cuboids + a rim-ring accent and
/// attaches them as children. Inserts WheelRimAttached afterward so
/// subsequent frames skip that wheel. Re-fires automatically on chassis
/// respawn because fresh Wheel entities won't carry the marker.
fn attach_rims(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    vehicle: Option<Res<VehicleRoot>>,
    quality: Res<GraphicsQuality>,
    wheel_q: Query<Entity, (With<Wheel>, Without<WheelRimAttached>)>,
) {
    // Wait until VehicleRoot is present (inserted by vehicle.rs Startup system).
    let Some(_vehicle) = vehicle else { return };

    // Build quality-gated shared materials.
    //
    // Medium+ → polished chrome: high metallic, very low roughness, high
    // reflectance — emulates a mirror-polished forged-aluminium centre-cap rim.
    // Low     → brushed aluminium: moderate metallic, rougher surface — cheaper
    //           look consistent with the Low matte-body-paint tier.
    let (spoke_mat, ring_mat) = if quality.vehicle_clearcoat() {
        // Polished chrome spokes.
        let s = materials.add(StandardMaterial {
            base_color: Color::srgb(0.82, 0.83, 0.88),
            perceptual_roughness: 0.08,
            metallic: 0.95,
            reflectance: 0.95,
            ..default()
        });
        // Outer rim barrel ring — slightly darker, same chrome finish.
        let r = materials.add(StandardMaterial {
            base_color: Color::srgb(0.70, 0.72, 0.78),
            perceptual_roughness: 0.12,
            metallic: 0.95,
            reflectance: 0.90,
            ..default()
        });
        (s, r)
    } else {
        // Brushed aluminium.
        let s = materials.add(StandardMaterial {
            base_color: Color::srgb(0.55, 0.55, 0.58),
            perceptual_roughness: 0.55,
            metallic: 0.50,
            ..default()
        });
        let r = materials.add(StandardMaterial {
            base_color: Color::srgb(0.45, 0.45, 0.48),
            perceptual_roughness: 0.60,
            metallic: 0.45,
            ..default()
        });
        (s, r)
    };

    let spoke_mesh    = meshes.add(Cuboid::new(SPOKE_W, SPOKE_H, SPOKE_D));
    // Rim-ring: Cylinder axis is Y; rotate 90° around Z to orient the flat
    // face toward +Y (outward) so it sits flush on the wheel face.
    let rim_ring_mesh = meshes.add(Cylinder::new(RIM_RING_RADIUS, RIM_RING_HEIGHT));

    for wheel_entity in wheel_q.iter() {
        let mut children: Vec<Entity> = Vec::with_capacity(SPOKE_COUNT + 1);

        // ---- Spokes ----
        for i in 0..SPOKE_COUNT {
            let angle = i as f32 * TAU / SPOKE_COUNT as f32 + PHASE_OFFSET;

            // Position: wheel-local XZ rolling plane, offset along +Y (outward face).
            let pos = Vec3::new(
                SPOKE_CENTRE_RADIUS * angle.sin(),
                SPOKE_Y_OFFSET,
                SPOKE_CENTRE_RADIUS * angle.cos(),
            );

            // Rotate the spoke so its longest dimension (Z = SPOKE_D) points radially
            // outward from the hub.
            let rot = Quat::from_rotation_y(angle);

            let spoke = commands.spawn((
                WheelRim,
                Mesh3d(spoke_mesh.clone()),
                MeshMaterial3d(spoke_mat.clone()),
                Transform::from_translation(pos).with_rotation(rot),
            )).id();
            children.push(spoke);
        }

        // ---- Outer rim-ring accent ----
        // Sits at SPOKE_Y_OFFSET, slightly outward, as a subtle depth break between
        // the spokes and the tread. The Cylinder's native axis is Y, which in
        // wheel-local space already points outward — no rotation needed.
        let ring = commands.spawn((
            WheelRim,
            Mesh3d(rim_ring_mesh.clone()),
            MeshMaterial3d(ring_mat.clone()),
            Transform::from_translation(Vec3::new(0.0, SPOKE_Y_OFFSET + 0.01, 0.0)),
        )).id();
        children.push(ring);

        commands.entity(wheel_entity)
            .add_children(&children)
            .insert(WheelRimAttached);
    }
}
