// Boats: 2 small drifting boats on water lakes. Each is a hull cuboid +
// cabin + stripe + mast + flag. Slowly drift in a circle, bobbing with a
// simple sin wave. Pure decoration — no physics.
//
// Public API:
//   BoatsPlugin

use std::f32::consts::PI;

use bevy::prelude::*;

use crate::water::WATER_LEVEL;

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct BoatsPlugin;

impl Plugin for BoatsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_boats)
           .add_systems(Update, drift_boats);
    }
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Base Y for a boat: sits just above the water surface.
const BOAT_BASE_Y: f32 = WATER_LEVEL + 1.0;

/// Circular drift radius (metres).
const DRIFT_RADIUS: f32 = 8.0;

/// Angular drift speed (radians per second — one full circle ≈ 314 s).
const DRIFT_SPEED: f32 = 0.02;

/// Hull dimensions: width × height × length.
const HULL_W: f32 = 1.5;
const HULL_H: f32 = 0.4;
const HULL_L: f32 = 4.0;

/// Cabin dimensions.
const CABIN_W: f32 = 1.0;
const CABIN_H: f32 = 0.8;
const CABIN_L: f32 = 1.5;

/// Decorative stripe: slightly wider/longer than hull, very thin.
const STRIPE_W: f32 = 1.6;
const STRIPE_H: f32 = 0.1;
const STRIPE_L: f32 = 4.1;

/// Mast cylinder.
const MAST_RADIUS: f32 = 0.05;
const MAST_HEIGHT: f32 = 2.5;

/// Tiny flag cuboid.
const FLAG_W: f32 = 0.4;
const FLAG_H: f32 = 0.2;
const FLAG_L: f32 = 0.05;

// World-space (x, z) centres for the two boat drift circles.
const BOAT_POSITIONS: [(f32, f32); 2] = [
    (-50.0, 80.0),
    ( 70.0, -55.0),
];

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

/// State component on the root boat entity.
#[derive(Component)]
pub struct Boat {
    /// Unique identifier for phase-shifting the bobbing animation.
    pub id: u32,
    /// Radius of the circular drift path (metres).
    pub drift_radius: f32,
    /// Angular speed of the drift (radians per second).
    pub drift_speed: f32,
    /// XZ centre of the drift circle (stored as Vec2 for convenience).
    pub base_pos: Vec2,
}

// ---------------------------------------------------------------------------
// Startup: spawn both boats
// ---------------------------------------------------------------------------

fn spawn_boats(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // ---- shared meshes ----
    let hull_mesh   = meshes.add(Cuboid::new(HULL_W, HULL_H, HULL_L));
    let cabin_mesh  = meshes.add(Cuboid::new(CABIN_W, CABIN_H, CABIN_L));
    let stripe_mesh = meshes.add(Cuboid::new(STRIPE_W, STRIPE_H, STRIPE_L));
    let mast_mesh   = meshes.add(Cylinder::new(MAST_RADIUS, MAST_HEIGHT));
    let flag_mesh   = meshes.add(Cuboid::new(FLAG_W, FLAG_H, FLAG_L));

    // ---- shared materials ----
    let hull_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.9, 0.9, 0.85),
        perceptual_roughness: 0.7,
        ..default()
    });
    let cabin_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.2, 0.4, 0.7),
        perceptual_roughness: 0.6,
        ..default()
    });
    let stripe_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.85, 0.20, 0.20),
        perceptual_roughness: 0.6,
        ..default()
    });
    let mast_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.6, 0.5, 0.35),
        perceptual_roughness: 0.85,
        ..default()
    });
    let flag_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.85, 0.1),
        perceptual_roughness: 0.5,
        ..default()
    });

    for (boat_id, &(bx, bz)) in BOAT_POSITIONS.iter().enumerate() {
        let start_pos = Vec3::new(bx, BOAT_BASE_Y, bz);

        // ---- parent (root) entity ----
        let parent = commands.spawn((
            Boat {
                id: boat_id as u32,
                drift_radius: DRIFT_RADIUS,
                drift_speed: DRIFT_SPEED,
                base_pos: Vec2::new(bx, bz),
            },
            Transform::from_translation(start_pos),
            Visibility::default(),
        )).id();

        // ---- hull: centred at parent origin ----
        let hull = commands.spawn((
            Mesh3d(hull_mesh.clone()),
            MeshMaterial3d(hull_mat.clone()),
            Transform::from_translation(Vec3::ZERO),
        )).id();
        commands.entity(parent).add_child(hull);

        // ---- cabin: on top of hull, shifted slightly toward bow ----
        // Hull half-height = HULL_H / 2; cabin sits on top so its bottom
        // aligns with the hull top → cabin centre at HULL_H/2 + CABIN_H/2.
        let cabin_y = HULL_H * 0.5 + CABIN_H * 0.5;
        let cabin = commands.spawn((
            Mesh3d(cabin_mesh.clone()),
            MeshMaterial3d(cabin_mat.clone()),
            Transform::from_translation(Vec3::new(0.0, cabin_y, -0.5)),
        )).id();
        commands.entity(parent).add_child(cabin);

        // ---- decorative stripe: at the waterline on the hull side ----
        // Positioned at hull mid-height, slightly outside hull width.
        let stripe_y = 0.0;  // hull centre height
        let stripe = commands.spawn((
            Mesh3d(stripe_mesh.clone()),
            MeshMaterial3d(stripe_mat.clone()),
            Transform::from_translation(Vec3::new(0.0, stripe_y, 0.0)),
        )).id();
        commands.entity(parent).add_child(stripe);

        // ---- mast: rising from hull top near the bow ----
        let mast_y = HULL_H * 0.5 + MAST_HEIGHT * 0.5;
        let mast = commands.spawn((
            Mesh3d(mast_mesh.clone()),
            MeshMaterial3d(mast_mat.clone()),
            Transform::from_translation(Vec3::new(0.0, mast_y, 1.0)),
        )).id();
        commands.entity(parent).add_child(mast);

        // ---- flag: at mast top ----
        let flag_y = HULL_H * 0.5 + MAST_HEIGHT + FLAG_H * 0.5;
        let flag = commands.spawn((
            Mesh3d(flag_mesh.clone()),
            MeshMaterial3d(flag_mat.clone()),
            Transform::from_translation(Vec3::new(FLAG_W * 0.5, flag_y, 1.0)),
        )).id();
        commands.entity(parent).add_child(flag);
    }
}

// ---------------------------------------------------------------------------
// Update: drift_boats
// ---------------------------------------------------------------------------

fn drift_boats(
    time: Res<Time>,
    mut boats: Query<(&mut Transform, &Boat)>,
) {
    let t = time.elapsed_secs();

    for (mut transform, boat) in boats.iter_mut() {
        let angle = t * boat.drift_speed;

        // Circular drift in XZ.
        transform.translation.x = boat.base_pos.x + angle.cos() * boat.drift_radius;
        transform.translation.z = boat.base_pos.y + angle.sin() * boat.drift_radius;

        // Bobbing in Y: gentle sin wave, phase-shifted per boat.
        transform.translation.y =
            BOAT_BASE_Y + (t * 1.2 + boat.id as f32 * 1.5).sin() * 0.15;

        // Face the direction of motion: velocity direction is tangent to the
        // circle, pointing in the direction of increasing angle.
        // tangent direction = (-sin(angle), 0, cos(angle)), but we want the
        // boat's bow (+Z local) to face that direction, so:
        //   heading = -angle - PI/2
        transform.rotation = Quat::from_rotation_y(-angle - PI / 2.0);
    }
}
