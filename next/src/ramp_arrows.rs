// Ramp arrows: directional yellow arrow markers above each ramp pointing
// to the safe approach direction. Pulses gently. Helps player line up jumps.
//
// We reconstruct the same 6 ramp XZ positions from ramps.rs by replaying the
// identical LCG (TERRAIN_SEED + 11) and applying the same rejection criteria,
// so the arrows are always aligned with actual ramp locations without needing
// a public API on RampsPlugin.
//
// Public API:
//   RampArrowsPlugin

use bevy::prelude::*;

use crate::terrain::{terrain_height_at, TERRAIN_SEED};

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct RampArrowsPlugin;

impl Plugin for RampArrowsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_ramp_arrows)
           .add_systems(Update, pulse_arrows);
    }
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

/// Marker placed on each arrow root entity.
#[derive(Component)]
pub struct RampArrow {
    pub idx: u32,
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Height above terrain at which each arrow floats.
const ARROW_HEIGHT: f32 = 5.0;

/// Pulse angular frequency (rad/s).
const PULSE_FREQ: f32 = 3.0;

/// Pulse scale amplitude (fraction of normal scale).
const PULSE_AMP: f32 = 0.1;

/// Slow Y-spin speed (rad/s).
const SPIN_SPEED: f32 = 0.4;

/// Phase offset between consecutive arrows.
const PHASE_STEP: f32 = 0.5;

// Shaft dimensions
const SHAFT_W: f32 = 0.3;
const SHAFT_H: f32 = 0.3;
const SHAFT_L: f32 = 1.5;

// Cone geometry — Bevy's Cone: base at -Y, tip at +Y.
const CONE_RADIUS: f32 = 0.5;
const CONE_HEIGHT: f32 = 0.8;

// Tail / feather dimensions
const TAIL_W: f32 = 0.6;
const TAIL_H: f32 = 0.4;
const TAIL_L: f32 = 0.2;

// ---------------------------------------------------------------------------
// LCG — exact copy of the one in ramps.rs so we reproduce ramp positions.
// ---------------------------------------------------------------------------

struct Lcg(u64);

impl Lcg {
    fn new(seed: u32) -> Self {
        Self(seed as u64)
    }

    fn next_f32(&mut self) -> f32 {
        self.0 = self.0.wrapping_mul(1664525).wrapping_add(1013904223) & 0xFFFF_FFFF;
        (self.0 as f32) / (u32::MAX as f32)
    }

    fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + self.next_f32() * (hi - lo)
    }
}

// ---------------------------------------------------------------------------
// Slope helper — mirrors ramps.rs so rejection criteria match exactly.
// ---------------------------------------------------------------------------

const SLOPE_STEP: f32 = 1.0;

fn compute_slope(x: f32, z: f32) -> f32 {
    let h  = terrain_height_at(x, z);
    let hx = terrain_height_at(x + SLOPE_STEP, z);
    let hz = terrain_height_at(x, z + SLOPE_STEP);
    let nxv = Vec3::new(SLOPE_STEP, hx - h, 0.0).normalize();
    let nzv = Vec3::new(0.0, hz - h, SLOPE_STEP).normalize();
    let n   = nxv.cross(nzv).normalize();
    1.0 - n.dot(Vec3::Y).abs().clamp(0.0, 1.0)
}

// ---------------------------------------------------------------------------
// Startup: spawn arrow entities
// ---------------------------------------------------------------------------

fn spawn_ramp_arrows(
    mut commands:  Commands,
    mut meshes:    ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Yellow emissive material shared by all arrow parts.
    let yellow = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.9, 0.2),
        emissive:   LinearRgba::rgb(1.2, 1.0, 0.1),
        unlit:      true,
        ..default()
    });

    // Pre-build shared meshes.
    let shaft_mesh = meshes.add(Cuboid::new(SHAFT_W, SHAFT_H, SHAFT_L));
    let cone_mesh  = meshes.add(Cone { radius: CONE_RADIUS, height: CONE_HEIGHT });
    let tail_mesh  = meshes.add(Cuboid::new(TAIL_W, TAIL_H, TAIL_L));

    // Replay the same LCG used by ramps.rs to reproduce ramp positions.
    let mut lcg  = Lcg::new(TERRAIN_SEED + 11);
    let mut idx  = 0u32;
    let mut attempts = 0u32;

    while idx < 6 && attempts < 2000 {
        attempts += 1;

        let x = lcg.range(-80.0, 80.0);
        let z = lcg.range(-80.0, 80.0);

        // Reject same candidates as ramps.rs.
        if x * x + z * z < 20.0 * 20.0 {
            continue;
        }
        if compute_slope(x, z) > 0.25 {
            continue;
        }

        // Consume the same random calls ramps.rs makes for yaw/width/length/height
        // so the LCG state advances identically for the next iteration.
        let _yaw    = lcg.range(0.0, std::f32::consts::TAU);
        let _width  = lcg.range(4.0, 8.0);
        let _length = lcg.range(6.0, 12.0);
        let _height = lcg.range(1.5, 3.0);

        let ground_y = terrain_height_at(x, z);
        let arrow_y  = ground_y + ARROW_HEIGHT;

        // Arrow root transform: floats above the ramp, points along +Z.
        let root_transform = Transform::from_xyz(x, arrow_y, z);

        // Spawn the root with the RampArrow marker.
        let root = commands.spawn((
            RampArrow { idx },
            root_transform,
            Visibility::Inherited,
        )).id();

        // ----- Shaft child -----
        // Shaft runs from -SHAFT_L/2 to +SHAFT_L/2 along Z.
        // We push it back by half its length so its +Z face is at Z=0 on the
        // root — cone will sit just in front of that.
        let shaft = commands.spawn((
            Mesh3d(shaft_mesh.clone()),
            MeshMaterial3d(yellow.clone()),
            Transform::from_translation(Vec3::new(0.0, 0.0, -SHAFT_L * 0.5)),
        )).id();

        // ----- Cone (arrowhead) child -----
        // Bevy Cone default: base at -Y, tip at +Y.
        // Rotate -90° around X so the tip points along +Z.
        // Place it at Z = 0 + half cone height so its base sits at Z=0.
        let cone_rot = Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2);
        let cone = commands.spawn((
            Mesh3d(cone_mesh.clone()),
            MeshMaterial3d(yellow.clone()),
            Transform::from_translation(Vec3::new(0.0, 0.0, CONE_HEIGHT * 0.5))
                .with_rotation(cone_rot),
        )).id();

        // ----- Tail / feather child -----
        // Placed at the back of the shaft.
        let tail = commands.spawn((
            Mesh3d(tail_mesh.clone()),
            MeshMaterial3d(yellow.clone()),
            Transform::from_translation(Vec3::new(0.0, 0.0, -SHAFT_L)),
        )).id();

        commands.entity(root).add_children(&[shaft, cone, tail]);

        idx += 1;
    }
}

// ---------------------------------------------------------------------------
// Update: pulse scale and spin
// ---------------------------------------------------------------------------

fn pulse_arrows(
    time: Res<Time>,
    mut query: Query<(&mut Transform, &RampArrow)>,
) {
    let t = time.elapsed_secs();

    for (mut tf, arrow) in &mut query {
        let phase  = t * PULSE_FREQ + arrow.idx as f32 * PHASE_STEP;
        let scale  = 1.0 + phase.sin() * PULSE_AMP;
        let spin_y = t * SPIN_SPEED;

        tf.scale    = Vec3::new(1.0, scale, 1.0);
        tf.rotation = Quat::from_rotation_y(spin_y);
    }
}
