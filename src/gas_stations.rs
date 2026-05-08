// Gas stations: 3 procedural fuel pumps placed at scenic positions. When
// the chassis enters the refill zone (radius 5 m) and stops (speed < 1 m/s),
// fuel slowly refills at 10 % per second of tank capacity.
//
// Public API:
//   GasStationsPlugin

use bevy::prelude::*;
use avian3d::prelude::{Collider, RigidBody, LinearVelocity};

use crate::fuel::Fuel;
use crate::terrain::terrain_height_at;
use crate::vehicle::{Chassis, VehicleRoot};

// ── Public plugin ─────────────────────────────────────────────────────────────

pub struct GasStationsPlugin;

impl Plugin for GasStationsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_gas_stations)
           .add_systems(Update, (
               tick_refill.run_if(resource_exists::<VehicleRoot>),
               pulse_pump_lights,
           ));
    }
}

// ── Fixed pump positions (XZ only; Y is resolved from terrain) ──────────────

const PUMP_XZ: [(f32, f32); 3] = [
    ( 20.0,  35.0),
    (-30.0, -25.0),
    ( 60.0, -65.0),
];

// ── Components ────────────────────────────────────────────────────────────────

/// Root entity for each gas-station pump.
#[derive(Component)]
struct GasPump;

/// Marker for the small indicator light cylinder on each pump.
#[derive(Component)]
pub struct PumpLight;

/// Tracks per-pump "first time near" state so we only log once per approach.
#[derive(Component)]
struct PumpRefillState {
    /// True while the chassis is currently inside the refill radius.
    was_near: bool,
}

// ── Startup: spawn pumps ──────────────────────────────────────────────────────

fn spawn_gas_stations(
    mut commands: Commands,
    mut meshes:   ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Shared mesh handles (reused across all 3 pumps).
    let base_mesh  = meshes.add(Cylinder::new(1.0, 0.4));
    let body_mesh  = meshes.add(Cuboid::new(0.8, 1.4, 0.6));
    let hose_mesh  = meshes.add(Cylinder::new(0.05, 0.6));
    let sign_mesh  = meshes.add(Cuboid::new(1.2, 0.3, 0.2));
    let light_mesh = meshes.add(Cylinder::new(0.15, 0.05));

    // Shared material handles.
    let base_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.2, 0.2, 0.22),
        perceptual_roughness: 0.8,
        ..default()
    });
    let body_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.85, 0.80, 0.65),
        perceptual_roughness: 0.6,
        ..default()
    });
    let hose_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.15, 0.15, 0.15),
        perceptual_roughness: 0.9,
        ..default()
    });
    let sign_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.6, 0.2),
        perceptual_roughness: 0.5,
        ..default()
    });
    // Light starts unlit; pulse_pump_lights modulates the emissive each frame.
    let light_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.3, 1.0, 0.5),
        emissive:   LinearRgba::new(0.0, 0.0, 0.0, 1.0),
        perceptual_roughness: 0.3,
        ..default()
    });

    for &(px, pz) in &PUMP_XZ {
        let ground_y = terrain_height_at(px, pz);

        // Base cylinder sits on the terrain surface; its centre is at +0.2 Y
        // (half of 0.4 height).
        let base_y  = ground_y + 0.2;
        // Body cuboid sits on top of the base; its centre is at base_top + 0.7.
        let body_y  = ground_y + 0.4 + 0.7;   // base_top + body_half_height
        // Hose: offset to the right side of the body, same height as body centre.
        let hose_y  = body_y;
        // Sign above the body.
        let sign_y  = body_y + 0.7 + 0.15;    // body_top + sign_half_height
        // Light on top of the sign.
        let light_y = sign_y + 0.15 + 0.025;  // sign_top + light_half_height

        // Root entity: RigidBody::Static + small cuboid collider so the chassis
        // can bump into the pump without tunnelling through.
        let root = commands.spawn((
            GasPump,
            PumpRefillState { was_near: false },
            Transform::from_xyz(px, ground_y, pz),
            Visibility::default(),
            RigidBody::Static,
            Collider::cuboid(0.4, 1.1, 0.3),
        )).id();

        // Base
        let base = commands.spawn((
            Mesh3d(base_mesh.clone()),
            MeshMaterial3d(base_mat.clone()),
            Transform::from_xyz(0.0, base_y - ground_y, 0.0),
        )).id();

        // Body
        let body = commands.spawn((
            Mesh3d(body_mesh.clone()),
            MeshMaterial3d(body_mat.clone()),
            Transform::from_xyz(0.0, body_y - ground_y, 0.0),
        )).id();

        // Hose: offset +0.5 on X side
        let hose = commands.spawn((
            Mesh3d(hose_mesh.clone()),
            MeshMaterial3d(hose_mat.clone()),
            Transform::from_xyz(0.5, hose_y - ground_y, 0.0),
        )).id();

        // Sign
        let sign = commands.spawn((
            Mesh3d(sign_mesh.clone()),
            MeshMaterial3d(sign_mat.clone()),
            Transform::from_xyz(0.0, sign_y - ground_y, 0.0),
        )).id();

        // Indicator light
        let light = commands.spawn((
            PumpLight,
            Mesh3d(light_mesh.clone()),
            MeshMaterial3d(light_mat.clone()),
            Transform::from_xyz(0.0, light_y - ground_y, 0.0),
        )).id();

        commands.entity(root).add_children(&[base, body, hose, sign, light]);
    }
}

// ── Update: refill chassis fuel when near a pump and nearly stopped ───────────

fn tick_refill(
    time:      Res<Time>,
    vehicle:   Res<VehicleRoot>,
    chassis_q: Query<(&Transform, &LinearVelocity), With<Chassis>>,
    mut pumps: Query<(&Transform, &mut PumpRefillState), With<GasPump>>,
    mut fuel:  ResMut<Fuel>,
) {
    let Ok((chassis_tf, lin_vel)) = chassis_q.get(vehicle.chassis) else { return };

    let chassis_pos = chassis_tf.translation;
    let speed       = Vec3::new(lin_vel.x, lin_vel.y, lin_vel.z).length();
    let dt          = time.delta_secs();

    for (pump_tf, mut state) in pumps.iter_mut() {
        let dist = chassis_pos.distance(pump_tf.translation);

        if dist < 5.0 && speed < 1.0 && fuel.current_l < fuel.capacity_l {
            if !state.was_near {
                info!("refilling at gas station");
                state.was_near = true;
            }
            // 10 % of capacity per second.
            let refill_rate = fuel.capacity_l * 0.10;
            fuel.current_l = (fuel.current_l + refill_rate * dt).min(fuel.capacity_l);
        } else {
            // Reset latch when the vehicle leaves the zone so it fires again on re-entry.
            if dist >= 5.0 {
                state.was_near = false;
            }
        }
    }
}

// ── Update: pulse the indicator lights ───────────────────────────────────────

fn pulse_pump_lights(
    time:        Res<Time>,
    lights_q:    Query<&MeshMaterial3d<StandardMaterial>, With<PumpLight>>,
    mut mats:    ResMut<Assets<StandardMaterial>>,
) {
    let t       = time.elapsed_secs();
    let alpha   = 0.5 + (t * 4.0).sin() * 0.5; // 0..1

    for mat_handle in lights_q.iter() {
        if let Some(mat) = mats.get_mut(mat_handle.id()) {
            mat.emissive = LinearRgba::new(0.3 * alpha, 1.0 * alpha, 0.5 * alpha, 1.0);
        }
    }
}
