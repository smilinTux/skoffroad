// Procedural jump ramp placement across the 200x200 m terrain.
//
// Approach: rotated Cuboid mesh + matching Cuboid collider.
// Each ramp is a box tilted ~18 degrees on the X axis so its top surface
// slopes upward from front to back. The collider matches the box exactly.
// This avoids custom wedge mesh math while still launching the chassis airborne
// when it hits at speed — the slanted top face does the work.
//
// Six ramps are placed at Startup using an LCG seeded from TERRAIN_SEED + 11.
// Candidates outside [-80, 80] XZ, within 20 m of origin, or on terrain with
// slope > 0.25 are rejected and resampled.

use bevy::prelude::*;
use avian3d::prelude::*;

use crate::terrain::{terrain_height_at, TERRAIN_SEED};

pub struct RampsPlugin;

impl Plugin for RampsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_ramps);
    }
}

// ---------------------------------------------------------------------------
// LCG — same style as scatter.rs hash2 but as a stateful iterator
// ---------------------------------------------------------------------------

struct Lcg(u64);

impl Lcg {
    fn new(seed: u32) -> Self {
        Self(seed as u64)
    }

    /// Next float in [0, 1).
    fn next_f32(&mut self) -> f32 {
        // Classic LCG parameters (Numerical Recipes).
        self.0 = self.0.wrapping_mul(1664525).wrapping_add(1013904223) & 0xFFFF_FFFF;
        (self.0 as f32) / (u32::MAX as f32)
    }

    /// Float in [lo, hi).
    fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + self.next_f32() * (hi - lo)
    }
}

// ---------------------------------------------------------------------------
// Slope helper (mirrors scatter.rs)
// ---------------------------------------------------------------------------

const SLOPE_STEP: f32 = 1.0;

fn compute_slope(x: f32, z: f32) -> f32 {
    let h  = terrain_height_at(x, z);
    let hx = terrain_height_at(x + SLOPE_STEP, z);
    let hz = terrain_height_at(x, z + SLOPE_STEP);
    let nxv = Vec3::new(SLOPE_STEP, hx - h, 0.0).normalize();
    let nzv = Vec3::new(0.0, hz - h, SLOPE_STEP).normalize();
    let n = nxv.cross(nzv).normalize();
    // .abs() guards against the -Y normal that nxv.cross(nzv) produces on
    // flat ground — see scatter.rs comment.
    1.0 - n.dot(Vec3::Y).abs().clamp(0.0, 1.0)
}

// ---------------------------------------------------------------------------
// Spawn
// ---------------------------------------------------------------------------

// Tilt (radians) applied to the box so the top surface slopes upward.
// 18 degrees gives a noticeable launch angle without being impossible to hit.
const RAMP_TILT_RAD: f32 = 0.314_159; // ~18 deg

fn spawn_ramps(
    mut commands: Commands,
    mut meshes:   ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let ramp_mat = materials.add(StandardMaterial {
        base_color:          Color::srgb(1.0, 0.5, 0.1),
        perceptual_roughness: 0.85,
        emissive:            LinearRgba::rgb(0.4, 0.15, 0.0),
        ..default()
    });

    let mut lcg = Lcg::new(TERRAIN_SEED + 11);

    let mut placed = 0u32;
    let mut attempts = 0u32;

    while placed < 6 && attempts < 2000 {
        attempts += 1;

        // Random XZ in [-80, 80].
        let x = lcg.range(-80.0, 80.0);
        let z = lcg.range(-80.0, 80.0);

        // Skip too-close-to-origin.
        if x * x + z * z < 20.0 * 20.0 {
            continue;
        }

        // Skip steep terrain.
        if compute_slope(x, z) > 0.25 {
            continue;
        }

        // Random orientation (full 360°).
        let yaw = lcg.range(0.0, std::f32::consts::TAU);

        // Random dimensions: width 4–8 m, length 6–12 m, height 1.5–3.0 m.
        let width  = lcg.range(4.0, 8.0);
        let length = lcg.range(6.0, 12.0);
        let height = lcg.range(1.5, 3.0);

        let ground_y = terrain_height_at(x, z);

        // The box is centred at its own origin. When tilted, the front edge
        // dips below centre and the back rises. We lift the whole assembly so
        // the lowest point (front-bottom edge) sits flush with the terrain.
        //
        // With a tilt of θ around X, the front-bottom edge drops by
        //   (length/2)*sin(θ) + (height/2)*(1 - cos(θ))
        // approximated simply as:  height/2 + (length/2)*sin(θ).
        let lift = height * 0.5 + (length * 0.5) * RAMP_TILT_RAD.sin();

        let transform = Transform {
            translation: Vec3::new(x, ground_y + lift, z),
            rotation: Quat::from_rotation_y(yaw)
                    * Quat::from_rotation_x(-RAMP_TILT_RAD),
            scale: Vec3::ONE,
        };

        let mesh   = meshes.add(Cuboid::new(width, height, length));
        let half_w = width  * 0.5;
        let half_h = height * 0.5;
        let half_l = length * 0.5;

        commands.spawn((
            Mesh3d(mesh),
            MeshMaterial3d(ramp_mat.clone()),
            transform,
            RigidBody::Static,
            Collider::cuboid(half_w, half_h, half_l),
        ));

        placed += 1;
    }
}
