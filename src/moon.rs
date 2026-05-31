// Moon — a glowing disc/sphere in the night sky, opposite the sun.
//
// Sprint 84 — Dynamic Weather + Richer Night Sky
//
// Strategy:
//   - One emissive sphere (radius 3.5 m, at distance 820 m — just beyond the
//     sky dome) placed in the anti-solar direction each frame.
//   - A soft white PointLight halo (Medium+ only, gate via GraphicsQuality).
//   - Fades in/out via emissive intensity based on "night-ness" derived from
//     TimeOfDay, matching the pattern in stars.rs.
//   - Moon is hidden during deep day (noon) and brightest at midnight.
//
// Anti-solar position:
//   sky.rs uses elevation_rad = (tod.t - 0.25) * 2π and yaw = 0.52 rad.
//   The moon is placed at the same elevation magnitude but opposite direction
//   (negate the XZ vector) so it sits on the opposite side of the sky.
//
// Public API:
//   MoonPlugin

use std::f32::consts::PI;

use bevy::prelude::*;

use crate::graphics_quality::GraphicsQuality;
use crate::sky::TimeOfDay;

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct MoonPlugin;

impl Plugin for MoonPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_moon)
           .add_systems(Update, update_moon);
    }
}

// ---------------------------------------------------------------------------
// Components & Resources
// ---------------------------------------------------------------------------

#[derive(Component)]
struct MoonDisc;

#[derive(Component)]
struct MoonHalo;

#[derive(Resource)]
struct MoonMaterial(Handle<StandardMaterial>);

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const MOON_RADIUS:   f32 = 3.5;
const MOON_DISTANCE: f32 = 820.0;
/// Peak emissive brightness (HDR units) at full night.
const MOON_EMIT:     f32 = 6.0;
/// Halo PointLight range in metres.
const HALO_RANGE:    f32 = 2000.0;
/// Halo intensity at peak (lumens — very faint, just tints the sky sphere).
const HALO_LUMENS:   f32 = 80_000.0;

// ---------------------------------------------------------------------------
// Startup: spawn moon entity
// ---------------------------------------------------------------------------

fn spawn_moon(
    mut commands:  Commands,
    mut meshes:    ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    quality:       Res<GraphicsQuality>,
) {
    // Start invisible; update_moon will position and light it each frame.
    let mat = materials.add(StandardMaterial {
        base_color: Color::srgba(0.95, 0.97, 1.0, 1.0),
        emissive:   LinearRgba::NONE,
        unlit:      true,
        ..default()
    });

    let mesh = meshes.add(Sphere::new(MOON_RADIUS).mesh().ico(1).unwrap());

    commands.spawn((
        MoonDisc,
        Mesh3d(mesh),
        MeshMaterial3d(mat.clone()),
        Transform::from_translation(Vec3::new(0.0, MOON_DISTANCE * 0.6, -MOON_DISTANCE)),
        Visibility::Hidden,
    ));

    commands.insert_resource(MoonMaterial(mat));

    // Halo point-light (Medium+ only).
    if !matches!(*quality, GraphicsQuality::Low) {
        commands.spawn((
            MoonHalo,
            PointLight {
                intensity:       0.0,
                range:           HALO_RANGE,
                color:           Color::srgb(0.90, 0.93, 1.0),
                shadows_enabled: false,
                ..default()
            },
            Transform::from_translation(Vec3::new(0.0, MOON_DISTANCE * 0.6, -MOON_DISTANCE)),
        ));
    }
}

// ---------------------------------------------------------------------------
// Update: position moon + fade emissive
// ---------------------------------------------------------------------------

fn update_moon(
    tod:          Res<TimeOfDay>,
    moon_mat:     Option<Res<MoonMaterial>>,
    mut mats:     ResMut<Assets<StandardMaterial>>,
    mut disc_q:   Query<(&mut Transform, &mut Visibility), (With<MoonDisc>, Without<MoonHalo>)>,
    mut halo_q:   Query<(&mut Transform, &mut PointLight), (With<MoonHalo>, Without<MoonDisc>)>,
) {
    let Some(moon_mat) = moon_mat else { return };

    // ---- Compute night-ness from tod.t --------------------------------
    // cos(2πt): +1 at t=0 (midnight), -1 at t=0.5 (noon).
    let cos_t    = (2.0 * PI * tod.t).cos();
    let t_night  = (cos_t * 0.5 + 0.5).clamp(0.0, 1.0);
    // Smooth-step so the moon doesn't pop.
    let t_smooth = t_night * t_night * (3.0 - 2.0 * t_night);

    // ---- Compute anti-solar position ----------------------------------
    // Matches sky.rs: elevation_rad = (tod.t - 0.25) * 2π, yaw ≈ 0.52.
    let elevation_rad = (tod.t - 0.25) * 2.0 * PI;
    let yaw           = 0.52_f32; // 30° east, same as sun yaw

    // Sun direction vector (not normalised — we rebuild it from angles).
    let sin_el = elevation_rad.sin();
    let cos_el = elevation_rad.cos();
    let sin_yw = yaw.sin();
    let cos_yw = yaw.cos();

    // Sun sits in direction: (cos_yw * cos_el, sin_el, sin_yw * cos_el).
    // Moon is anti-solar: negate the full vector.
    let sun_x =  cos_yw * cos_el;
    let sun_y =  sin_el;
    let sun_z =  sin_yw * cos_el;

    // Anti-solar direction.
    let moon_dir = Vec3::new(-sun_x, -sun_y, -sun_z);

    // Reflect the moon to the upper hemisphere if it dips below — we don't
    // want it underground (it just won't be visible, but let's keep it clean).
    let moon_dir = if moon_dir.y < 0.05 {
        Vec3::new(moon_dir.x, 0.05, moon_dir.z).normalize_or_zero()
    } else {
        moon_dir.normalize_or_zero()
    };

    let moon_pos = moon_dir * MOON_DISTANCE;

    // ---- Moon disc ------------------------------------------------
    for (mut tf, mut vis) in &mut disc_q {
        tf.translation = moon_pos;
        *vis = if t_smooth > 0.01 { Visibility::Visible } else { Visibility::Hidden };
    }

    // ---- Emissive intensity ---------------------------------------
    if let Some(mat) = mats.get_mut(&moon_mat.0) {
        mat.emissive = LinearRgba::WHITE * (MOON_EMIT * t_smooth);
    }

    // ---- Halo PointLight (Medium+) --------------------------------
    for (mut tf, mut light) in &mut halo_q {
        tf.translation = moon_pos;
        light.intensity = HALO_LUMENS * t_smooth;
    }
}
