// Wheel detail upgrades: 5 lug nuts, central hub cap, and 8 tread blocks added
// as visual children of each wheel entity. Pure cosmetic — no physics interaction.
// Children inherit the wheel's rotation, so all details spin automatically.
//
// Geometry choices:
//   Lug nuts  — 5 × Cuboid(0.06, 0.06, 0.04), chrome, radius-0.10 circle
//   Hub cap   — Cylinder(r=0.12, h=0.05), bright chrome, wheel-axis face
//   Tread     — 8 × Cuboid(0.08, 0.05, 0.10), near-black, on tire OD
//
// Coordinate convention (wheel local space):
//   The wheel mesh is Cylinder with axis Y. The wheel entity's transform has
//   Quat::from_rotation_z(FRAC_PI_2) baked in, which maps the cylinder's +Y
//   axis to chassis -X. Children positions are expressed in this same
//   wheel-entity local frame. We place outward-face details at local +Y = +0.18
//   (one half-width out), and tread blocks at radial offset 0.36 in the XZ
//   plane (just beyond WHEEL_RADIUS = 0.35).
//
// Public API:
//   WheelDetailPlugin

use std::f32::consts::TAU;
use bevy::prelude::*;
use crate::vehicle::{VehicleRoot, Wheel};

// ---- Mirrors vehicle.rs ----
const WHEEL_HALF_WIDTH: f32 = 0.18;
const WHEEL_RADIUS: f32     = 0.35;

// Lug-nut ring radius and count.
const LUG_RING_RADIUS: f32 = 0.10;
const LUG_COUNT: usize     = 5;

// Hub-cap cylinder dimensions.
const HUB_RADIUS: f32  = 0.12;
const HUB_HEIGHT: f32  = 0.05;

// Mud-terrain tread: 16 chunky blocks around the OD in a zig-zag offset
// pattern, plus 8 sidewall biters on each side. Larger and more aggressive
// than the original 8 smooth blocks.
const TREAD_COUNT: usize    = 16;
const TREAD_RADIAL: f32     = WHEEL_RADIUS + 0.015;
const TREAD_W: f32          = 0.11;   // axial (across tire width)
const TREAD_H: f32          = 0.07;   // radial (depth into ground)
const TREAD_D: f32          = 0.14;   // tangential (along rolling direction)
/// How far blocks alternate inward/outward across the tire face. Creates the
/// zig-zag mud-terrain look (vs the original centred 8-block row).
const TREAD_AXIAL_OFFSET: f32 = 0.06;
/// Sidewall biters — small chunks on the shoulder of the tire.
const BITER_COUNT: usize    = 8;
const BITER_W: f32          = 0.04;
const BITER_H: f32          = 0.05;
const BITER_D: f32          = 0.09;

// ---- Plugin ----

pub struct WheelDetailPlugin;

impl Plugin for WheelDetailPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, attach_wheel_detail_once);
    }
}

// ---- System ----

/// Runs every frame; attaches detail meshes once VehicleRoot is available and
/// all Wheel entities exist. A `Local<bool>` guard ensures it runs exactly once.
fn attach_wheel_detail_once(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    vehicle: Option<Res<VehicleRoot>>,
    wheel_q: Query<Entity, With<Wheel>>,
    mut done: Local<bool>,
) {
    if *done { return; }
    // Wait until VehicleRoot exists (spawned by vehicle.rs Startup system).
    let Some(_vehicle) = vehicle else { return };

    // Build shared mesh / material handles once, reuse across all 4 wheels.
    let lug_mesh = meshes.add(Cuboid::new(0.06, 0.06, 0.04));
    let lug_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.70, 0.70, 0.75),
        perceptual_roughness: 0.2,
        metallic: 0.8,
        ..default()
    });

    let hub_mesh = meshes.add(Cylinder::new(HUB_RADIUS, HUB_HEIGHT));
    let hub_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.85, 0.85, 0.90),
        perceptual_roughness: 0.15,
        metallic: 0.9,
        ..default()
    });

    let tread_mesh = meshes.add(Cuboid::new(TREAD_W, TREAD_H, TREAD_D));
    let tread_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.05, 0.05, 0.05),
        perceptual_roughness: 0.95,
        ..default()
    });

    for wheel_entity in wheel_q.iter() {
        let mut children: Vec<Entity> = Vec::with_capacity(2 * (LUG_COUNT + 1) + TREAD_COUNT);

        // ---- A. Hub caps on BOTH faces ----
        // Cylinder in wheel local space has its axis along Y. Both end faces sit
        // at ±WHEEL_HALF_WIDTH along local Y. Mirror the hub-cap+lug ring onto
        // each face so the wheel looks correct regardless of which side the
        // camera is on (previously only +Y had a hub, leaving the opposite side
        // of the truck looking unfinished). Rotate 90° around X so the flat
        // hub-cap face is flush with the wheel face.
        for face_sign in [1.0_f32, -1.0_f32] {
            let hub = commands.spawn((
                Mesh3d(hub_mesh.clone()),
                MeshMaterial3d(hub_mat.clone()),
                Transform::from_translation(Vec3::new(0.0, face_sign * WHEEL_HALF_WIDTH, 0.0))
                    .with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
            )).id();
            children.push(hub);

            // ---- B. Lug nuts — 5 around a circle of radius LUG_RING_RADIUS, mirrored to each face ----
            for i in 0..LUG_COUNT {
                let angle = i as f32 * TAU / LUG_COUNT as f32;
                let ring_offset = Vec3::new(LUG_RING_RADIUS * angle.sin(), 0.0, LUG_RING_RADIUS * angle.cos());
                let pos = Vec3::new(0.0, face_sign * (WHEEL_HALF_WIDTH + HUB_HEIGHT * 0.5), 0.0) + ring_offset;
                let lug = commands.spawn((
                    Mesh3d(lug_mesh.clone()),
                    MeshMaterial3d(lug_mat.clone()),
                    Transform::from_translation(pos),
                )).id();
                children.push(lug);
            }
        }

        // ---- C. Mud-terrain tread — 16 chunky blocks in a zig-zag pattern ----
        // The XZ plane is the rolling plane (cylinder axis = Y). Every block is
        // positioned at TREAD_RADIAL distance and rotated around Y so its depth
        // faces radially outward. Alternating blocks shift axially (along the
        // tire width) creating the staggered MT look.
        let biter_mesh = meshes.add(Cuboid::new(BITER_W, BITER_H, BITER_D));
        for i in 0..TREAD_COUNT {
            let angle = i as f32 * TAU / TREAD_COUNT as f32;
            // Zig-zag: even blocks offset to +Y face, odd to -Y face.
            let axial = if i % 2 == 0 { TREAD_AXIAL_OFFSET } else { -TREAD_AXIAL_OFFSET };
            let pos = Vec3::new(
                TREAD_RADIAL * angle.sin(),
                axial,
                TREAD_RADIAL * angle.cos(),
            );
            let rot = Quat::from_rotation_y(angle);
            children.push(commands.spawn((
                Mesh3d(tread_mesh.clone()),
                MeshMaterial3d(tread_mat.clone()),
                Transform::from_translation(pos).with_rotation(rot),
            )).id());
        }

        // ---- D. Sidewall biters — small chunks on both shoulders for the
        // aggressive mud-terrain look. Positioned just outside the tread band
        // on each face of the cylinder.
        for face_sign in [1.0_f32, -1.0_f32] {
            for i in 0..BITER_COUNT {
                let angle = (i as f32 + 0.5) * TAU / BITER_COUNT as f32; // rotate half-step so they sit between main treads
                let pos = Vec3::new(
                    (TREAD_RADIAL - 0.04) * angle.sin(),
                    face_sign * (WHEEL_HALF_WIDTH + 0.005),
                    (TREAD_RADIAL - 0.04) * angle.cos(),
                );
                let rot = Quat::from_rotation_y(angle);
                children.push(commands.spawn((
                    Mesh3d(biter_mesh.clone()),
                    MeshMaterial3d(tread_mat.clone()),
                    Transform::from_translation(pos).with_rotation(rot),
                )).id());
            }
        }

        commands.entity(wheel_entity).add_children(&children);
    }

    *done = true;
}
