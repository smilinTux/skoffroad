// Night star field for skoffroad.
//
// 200 small emissive spheres placed on an upper hemisphere of radius 800 m.
// They share a single StandardMaterial whose emissive intensity is scaled each
// frame by "night-ness" derived from TimeOfDay so stars fade out at noon and
// glow fully at midnight.

use bevy::prelude::*;
use crate::sky::TimeOfDay;

// ---- Plugin -----------------------------------------------------------------

pub struct StarsPlugin;

impl Plugin for StarsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_stars)
           .add_systems(Update, fade_stars);
    }
}

// ---- Resource ---------------------------------------------------------------

#[derive(Resource)]
struct StarMaterial(Handle<StandardMaterial>);

// ---- Constants --------------------------------------------------------------

// Reduced 200 → 60 — at 200 stars × shared material write per frame the
// scene rendered at 5 FPS on an Intel iGPU. 60 is still a reasonable
// star field at midnight.
const STAR_COUNT:  usize = 60;
const STAR_RADIUS: f32   = 800.0;
/// Peak emissive intensity (Linear HDR units) when fully visible.
const STAR_EMIT:   f32   = 5.0;

// ---- Fibonacci hemisphere distribution --------------------------------------

/// Returns a point on the upper hemisphere of the given `radius` using an
/// even Fibonacci (golden-ratio) distribution.
///
/// `i` is the star index (0..total), `total` is STAR_COUNT.
fn star_position(i: usize, total: usize, radius: f32) -> Vec3 {
    let phi   = (1.0 + 5.0_f32.sqrt()) / 2.0; // golden ratio
    let theta = 2.0 * std::f32::consts::PI * (i as f32 / phi);
    // y_norm in [0.5, 1.0] → upper hemisphere only
    let y_norm = 0.5 + 0.5 * (i as f32 / total as f32);
    let r = (1.0 - y_norm * y_norm).sqrt();
    Vec3::new(r * theta.cos(), y_norm, r * theta.sin()) * radius
}

// ---- Deterministic LCG for scale variety ------------------------------------

/// Very small linear-congruential generator seeded by index.
/// Returns a value in [0, 1).
#[inline]
fn lcg_f32(seed: usize) -> f32 {
    let mut v = (seed as u32).wrapping_mul(1664525).wrapping_add(1013904223);
    v ^= v >> 16;
    v = v.wrapping_mul(0x45d9f3b);
    v ^= v >> 16;
    (v as f32) / (u32::MAX as f32)
}

/// Maps a [0,1) value to one of three scale variants: 0.5x, 1.0x, or 2.0x.
#[inline]
fn star_scale(rand: f32) -> f32 {
    if rand < 0.33 {
        0.5
    } else if rand < 0.67 {
        1.0
    } else {
        2.0
    }
}

// ---- Systems ----------------------------------------------------------------

fn spawn_stars(
    mut commands:  Commands,
    mut meshes:    ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Shared material — emissive intensity will be mutated each frame.
    let mat_handle = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        emissive:   LinearRgba::WHITE * STAR_EMIT,
        unlit:      true,
        ..default()
    });

    // Low-poly sphere: single icosahedron octahedron subdivision level 0.
    let mesh_handle = meshes.add(
        Sphere::new(0.6).mesh().ico(0).unwrap(),
    );

    for i in 0..STAR_COUNT {
        let pos   = star_position(i, STAR_COUNT, STAR_RADIUS);
        let scale = star_scale(lcg_f32(i));

        commands.spawn((
            Mesh3d(mesh_handle.clone()),
            MeshMaterial3d(mat_handle.clone()),
            Transform::from_translation(pos)
                .with_scale(Vec3::splat(scale)),
        ));
    }

    commands.insert_resource(StarMaterial(mat_handle));
}

/// Each frame, compute "night-ness" from TimeOfDay and scale the shared star
/// material's emissive intensity accordingly.
///
/// t=0.0/1.0 = midnight (night-ness 1.0), t=0.5 = noon (night-ness 0.0).
/// A smooth cosine ramp avoids a hard cut at the day/night boundary.
fn fade_stars(
    tod:       Res<TimeOfDay>,
    star_mat:  Option<Res<StarMaterial>>,
    mut mats:  ResMut<Assets<StandardMaterial>>,
) {
    let Some(star_mat) = star_mat else { return };

    // Map t onto [0, 2π] for a full day cycle.
    // cos(2πt): +1 at t=0 (midnight), −1 at t=0.5 (noon).
    let cos_t    = (2.0 * std::f32::consts::PI * tod.t).cos();
    // Remap [−1, +1] to [0, 1]: night-ness is 1 at midnight, 0 at noon.
    let t_night  = (cos_t * 0.5 + 0.5).clamp(0.0, 1.0);
    // Smooth-step so stars don't pop on/off near dusk/dawn.
    let t_smooth = t_night * t_night * (3.0 - 2.0 * t_night);

    if let Some(mat) = mats.get_mut(&star_mat.0) {
        mat.emissive = LinearRgba::WHITE * (STAR_EMIT * t_smooth);
    }
}
