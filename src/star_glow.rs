// Star glow enhancement layer — Sprint 84.
//
// Complements stars.rs without touching it. Adds:
//   1. A second star field (150 stars) at slightly different positions and
//      scales, adding depth / density at deep night.
//   2. Moon-fade: both this layer and the primary stars.rs layer will fade
//      on their own from TimeOfDay.  This file also reads a MoonBrightness
//      resource (written by moon.rs) if present to further dim stars.
//   3. Brightness scaling: stars get 20% brighter at deep night (tod near
//      midnight) via a boosted emissive.
//
// Public API:
//   StarGlowPlugin
//   MoonBrightness (resource)

use std::f32::consts::PI;

use bevy::prelude::*;

use crate::sky::TimeOfDay;

// ---------------------------------------------------------------------------
// Public resource written by moon.rs (indirectly via this module)
// ---------------------------------------------------------------------------

/// 0.0 = no moon, 1.0 = full moon. Written here based on TimeOfDay.
/// Other files may read this to dim effects during full moon.
#[derive(Resource, Default, Clone, Copy)]
pub struct MoonBrightness(pub f32);

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct StarGlowPlugin;

impl Plugin for StarGlowPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MoonBrightness>()
           .add_systems(Startup, spawn_glow_stars)
           .add_systems(Update, (update_moon_brightness, fade_glow_stars));
    }
}

// ---------------------------------------------------------------------------
// Components & Resources
// ---------------------------------------------------------------------------

#[derive(Component)]
struct GlowStar;

#[derive(Resource)]
struct GlowStarMaterial(Handle<StandardMaterial>);

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const GLOW_STAR_COUNT:  usize = 150;
const GLOW_STAR_RADIUS: f32   = 810.0;  // slightly further than primary 800 m
/// Peak emissive at deepest night (on top of base stars.rs stars).
const GLOW_EMIT_BASE:   f32   = 3.5;
/// Bonus emissive at deep midnight (tod.t near 0 or 1).
const GLOW_EMIT_BOOST:  f32   = 2.0;

// ---------------------------------------------------------------------------
// Fibonacci hemisphere — offset from stars.rs by π so positions differ
// ---------------------------------------------------------------------------

fn glow_star_position(i: usize, total: usize, radius: f32) -> Vec3 {
    let phi   = (1.0 + 5.0_f32.sqrt()) / 2.0;
    // Offset by 0.37 to avoid identical positions as stars.rs
    let theta = 2.0 * PI * ((i as f32 + 0.37) / phi);
    // y_norm in [0.3, 0.9] — denser mid-sky, fewer right at zenith
    let y_norm = 0.3 + 0.6 * (i as f32 / total as f32);
    let r = (1.0 - y_norm * y_norm).sqrt();
    Vec3::new(r * theta.cos(), y_norm, r * theta.sin()) * radius
}

#[inline]
fn lcg_f32(seed: usize) -> f32 {
    let mut v = (seed as u32).wrapping_mul(1664525).wrapping_add(1013904223);
    v ^= v >> 16;
    v = v.wrapping_mul(0x45d9f3b);
    v ^= v >> 16;
    (v as f32) / (u32::MAX as f32)
}

// ---------------------------------------------------------------------------
// Startup: spawn glow star field
// ---------------------------------------------------------------------------

fn spawn_glow_stars(
    mut commands:  Commands,
    mut meshes:    ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mat = materials.add(StandardMaterial {
        base_color: Color::srgba(0.90, 0.92, 1.0, 1.0),
        emissive:   LinearRgba::NONE,
        unlit:      true,
        ..default()
    });

    // Smaller mesh variant (ico level 0, radius 0.45) for a finer sparkle.
    let mesh = meshes.add(Sphere::new(0.45).mesh().ico(0).unwrap());

    for i in 0..GLOW_STAR_COUNT {
        let pos   = glow_star_position(i, GLOW_STAR_COUNT, GLOW_STAR_RADIUS);
        // Scale varies 0.5x-1.5x by index.
        let rand  = lcg_f32(i + 9999);
        let scale = 0.5 + rand;  // [0.5, 1.5)

        commands.spawn((
            GlowStar,
            Mesh3d(mesh.clone()),
            MeshMaterial3d(mat.clone()),
            Transform::from_translation(pos).with_scale(Vec3::splat(scale)),
        ));
    }

    commands.insert_resource(GlowStarMaterial(mat));
}

// ---------------------------------------------------------------------------
// System: compute MoonBrightness from TimeOfDay
// ---------------------------------------------------------------------------

fn update_moon_brightness(
    tod:  Res<TimeOfDay>,
    mut moon: ResMut<MoonBrightness>,
) {
    // Same night-ness calculation as stars.rs.
    let cos_t   = (2.0 * PI * tod.t).cos();
    let t_night = (cos_t * 0.5 + 0.5).clamp(0.0, 1.0);
    let smooth  = t_night * t_night * (3.0 - 2.0 * t_night);
    // Moon is brightest at midnight (t_night = 1.0).
    moon.0 = smooth;
}

// ---------------------------------------------------------------------------
// System: fade glow stars
// ---------------------------------------------------------------------------

fn fade_glow_stars(
    tod:      Res<TimeOfDay>,
    glow_mat: Option<Res<GlowStarMaterial>>,
    moon:     Res<MoonBrightness>,
    mut mats: ResMut<Assets<StandardMaterial>>,
) {
    let Some(glow_mat) = glow_mat else { return };

    // Night-ness base (mirrors stars.rs pattern).
    let cos_t    = (2.0 * PI * tod.t).cos();
    let t_night  = (cos_t * 0.5 + 0.5).clamp(0.0, 1.0);
    let t_smooth = t_night * t_night * (3.0 - 2.0 * t_night);

    // Deep-night boost: extra brightness when very near midnight.
    // moon.0 peaks at midnight, so we re-use it.
    let deep_boost = moon.0 * moon.0; // quadratic: sharply peaks at midnight

    let emit = (GLOW_EMIT_BASE + GLOW_EMIT_BOOST * deep_boost) * t_smooth;

    if let Some(mat) = mats.get_mut(&glow_mat.0) {
        mat.emissive = LinearRgba::WHITE * emit;
    }
}
