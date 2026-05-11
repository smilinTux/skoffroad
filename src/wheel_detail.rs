// Wheel detail upgrades: 5 lug nuts, central hub cap, and mud-terrain tread
// added as visual children of each wheel entity. Pure cosmetic — no physics.
// Children inherit the wheel's rotation, so all details spin automatically.
//
// Geometry choices:
//   Lug nuts  — 5 × Cuboid(0.06, 0.06, 0.04), chrome, radius-0.10 circle
//   Hub cap   — Cylinder(r=0.12, h=0.05), bright chrome, wheel-axis face
//   Tread     — 16 × Cuboid(~0.11, ~0.07, ~0.14), near-black, on tire OD
//   Biters    — 8 × small Cuboid on each shoulder
//
// Coordinate convention (wheel local space):
//   The wheel mesh is Cylinder with axis Y. The wheel entity's transform has
//   Quat::from_rotation_z(FRAC_PI_2) baked in, which maps the cylinder's +Y
//   axis to chassis -X. Children positions are expressed in this same
//   wheel-entity local frame. We place outward-face details at local +Y =
//   ±WHEEL_HALF_WIDTH, and tread blocks at radial offset TREAD_RADIAL in the
//   XZ plane (just beyond the tire OD).
//
// Tire-size scaling:
//   When the player switches to 35" or 37" tires the wheel CYLINDER mesh
//   grows but the detail children must also grow or they'll be buried inside
//   the larger tire. We read the active VehicleModsState::tire_size.radius()
//   and compute radius_ratio = current_radius / WHEEL_RADIUS (stock). Every
//   radial offset is multiplied by radius_ratio; face offsets (hub/lug
//   positions along the local-Y axis) are also multiplied so the details feel
//   proportionally chunkier on bigger tires.
//
// Respawn guard:
//   Uses a WheelDetailAttached marker component instead of Local<bool> so the
//   system correctly re-attaches after a chassis respawn (the old done=true
//   guard would skip re-attachment on the new wheel entities).
//
// Public API:
//   WheelDetailPlugin
//   WheelDetailAttached  (marker component — exported so variants.rs can query it)

use std::f32::consts::TAU;
use bevy::prelude::*;
use crate::vehicle::{VehicleRoot, Wheel};
use crate::vehicle_mods::VehicleModsState;

// ---- Stock (base) dimensions ----
// These match the stock WHEEL_RADIUS = 0.35 defined in vehicle.rs. All
// placement math below is expressed in these base units and then multiplied
// by radius_ratio at runtime.

const WHEEL_HALF_WIDTH: f32 = 0.18;
const WHEEL_RADIUS: f32     = 0.35;

// Lug-nut ring radius and count.
const LUG_RING_RADIUS: f32 = 0.10;
const LUG_COUNT: usize     = 5;

// Hub-cap cylinder dimensions.
const HUB_RADIUS: f32  = 0.12;
const HUB_HEIGHT: f32  = 0.05;

// Mud-terrain tread: 16 chunky blocks around the OD in a zig-zag offset
// pattern, plus 8 sidewall biters on each side.
const TREAD_COUNT: usize    = 16;
/// Base radial distance for tread blocks (stock radius + small clearance).
const TREAD_RADIAL_BASE: f32 = WHEEL_RADIUS + 0.015;
const TREAD_W: f32          = 0.11;   // axial (across tire width)
const TREAD_H: f32          = 0.07;   // radial (depth into ground)
const TREAD_D: f32          = 0.14;   // tangential (along rolling direction)
/// How far blocks alternate inward/outward across the tire face (zig-zag look).
const TREAD_AXIAL_OFFSET: f32 = 0.06;
/// Sidewall biters — small chunks on the shoulder of the tire.
const BITER_COUNT: usize    = 8;
const BITER_W: f32          = 0.04;
const BITER_H: f32          = 0.05;
const BITER_D: f32          = 0.09;

// ---- Marker component ----

/// Added to a Wheel entity once its detail children have been spawned.
/// Checked each frame so the system skips wheels that already have details
/// and correctly re-attaches after a chassis respawn (new Wheel entities
/// won't carry this marker).
#[derive(Component)]
pub struct WheelDetailAttached;

// ---- Plugin ----

pub struct WheelDetailPlugin;

impl Plugin for WheelDetailPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, attach_wheel_detail);
    }
}

// ---- System ----

/// Runs every frame. For each Wheel entity that does NOT yet have the
/// WheelDetailAttached marker, spawns the full detail child set and inserts
/// the marker. This fires once per wheel on first spawn, and again on any
/// chassis respawn that creates fresh Wheel entities (because the new entities
/// won't have the marker).
fn attach_wheel_detail(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    vehicle: Option<Res<VehicleRoot>>,
    wheel_q: Query<Entity, (With<Wheel>, Without<WheelDetailAttached>)>,
    mods_opt: Option<Res<VehicleModsState>>,
) {
    // Wait until VehicleRoot exists (spawned by vehicle.rs Startup system).
    let Some(_vehicle) = vehicle else { return };

    // Determine the active tire radius and compute the scale ratio vs stock.
    let current_radius = mods_opt
        .as_deref()
        .map(|m| m.tire_size.radius())
        .unwrap_or(WHEEL_RADIUS);
    let r = current_radius / WHEEL_RADIUS; // radius_ratio; 1.0 for stock tires

    // Build shared mesh / material handles. Mesh dimensions are pre-scaled
    // where the geometry itself should grow (tread block sizes), or kept
    // constant for small fastener-level details (lug nuts, hub cap) whose
    // absolute size doesn't change much with tire size.
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

    // Tread and biter meshes scale with the tire: wider tread on bigger tires
    // looks proportionally correct and avoids the "tiny bumps on huge tire" look.
    let tread_mesh = meshes.add(Cuboid::new(TREAD_W * r, TREAD_H, TREAD_D * r));
    let tread_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.05, 0.05, 0.05),
        perceptual_roughness: 0.95,
        ..default()
    });
    let biter_mesh = meshes.add(Cuboid::new(BITER_W * r, BITER_H, BITER_D * r));

    // Radial offset for tread blocks scaled to current tire OD.
    let tread_radial = TREAD_RADIAL_BASE * r;
    // Face offset (along wheel local Y): how far out hub/lug details sit.
    let face_offset = WHEEL_HALF_WIDTH * r;

    for wheel_entity in wheel_q.iter() {
        let mut children: Vec<Entity> = Vec::with_capacity(2 * (LUG_COUNT + 1) + TREAD_COUNT);

        // ---- A. Hub caps on BOTH faces ----
        // Cylinder in wheel local space has its axis along Y. Both end faces sit
        // at ±face_offset along local Y (scaled from ±WHEEL_HALF_WIDTH for the
        // stock tire). Rotate 90° around X so the flat hub-cap face is flush
        // with the wheel face.
        for face_sign in [1.0_f32, -1.0_f32] {
            let hub = commands.spawn((
                Mesh3d(hub_mesh.clone()),
                MeshMaterial3d(hub_mat.clone()),
                Transform::from_translation(Vec3::new(0.0, face_sign * face_offset, 0.0))
                    .with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
            )).id();
            children.push(hub);

            // ---- B. Lug nuts — 5 around a circle of radius LUG_RING_RADIUS ----
            // The lug ring radius is not scaled (lug nuts are fixed hardware, not
            // proportionally bigger on larger tires), but the face Y offset is
            // scaled so they sit on the actual tire face.
            for i in 0..LUG_COUNT {
                let angle = i as f32 * TAU / LUG_COUNT as f32;
                let ring_offset = Vec3::new(
                    LUG_RING_RADIUS * angle.sin(),
                    0.0,
                    LUG_RING_RADIUS * angle.cos(),
                );
                let pos = Vec3::new(
                    0.0,
                    face_sign * (face_offset + HUB_HEIGHT * 0.5),
                    0.0,
                ) + ring_offset;
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
        // positioned at tread_radial distance and rotated around Y so its depth
        // faces radially outward. Alternating blocks shift axially (along the
        // tire width) creating the staggered MT look.
        for i in 0..TREAD_COUNT {
            let angle = i as f32 * TAU / TREAD_COUNT as f32;
            // Zig-zag: even blocks offset to +Y face, odd to -Y face.
            // The axial offset is also scaled so it's proportional on big tires.
            let axial = if i % 2 == 0 {
                TREAD_AXIAL_OFFSET * r
            } else {
                -TREAD_AXIAL_OFFSET * r
            };
            let pos = Vec3::new(
                tread_radial * angle.sin(),
                axial,
                tread_radial * angle.cos(),
            );
            let rot = Quat::from_rotation_y(angle);
            children.push(commands.spawn((
                Mesh3d(tread_mesh.clone()),
                MeshMaterial3d(tread_mat.clone()),
                Transform::from_translation(pos).with_rotation(rot),
            )).id());
        }

        // ---- D. Sidewall biters — small chunks on both shoulders for the
        // aggressive mud-terrain look. Positioned just inside the tread band on
        // each face of the cylinder at the scaled face_offset.
        for face_sign in [1.0_f32, -1.0_f32] {
            for i in 0..BITER_COUNT {
                // Rotate half-step so biters sit between main tread blocks.
                let angle = (i as f32 + 0.5) * TAU / BITER_COUNT as f32;
                let pos = Vec3::new(
                    (tread_radial - 0.04 * r) * angle.sin(),
                    face_sign * (face_offset + 0.005),
                    (tread_radial - 0.04 * r) * angle.cos(),
                );
                let rot = Quat::from_rotation_y(angle);
                children.push(commands.spawn((
                    Mesh3d(biter_mesh.clone()),
                    MeshMaterial3d(tread_mat.clone()),
                    Transform::from_translation(pos).with_rotation(rot),
                )).id());
            }
        }

        commands.entity(wheel_entity)
            .add_children(&children)
            .insert(WheelDetailAttached);
    }
}
