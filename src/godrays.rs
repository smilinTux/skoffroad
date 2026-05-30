// God rays / sun shafts: spawns radial billboards at the sun direction
// when the sun is low (sunrise/sunset). Subtle additive layer faked with
// alpha quads — no shader pipeline.
//
// Improvements (Sprint 83):
//   - Intensity and quad scale both ramp by sun elevation: long bright rays
//     near the horizon, no rays at noon or at night.
//   - Warm golden tint at low sun angles; cooler white-amber at higher angles.
//   - Gated to Medium+ quality (hidden entirely on Low).
//
// Public API:
//   GodraysPlugin

use std::f32::consts::PI;

use bevy::prelude::*;

use crate::graphics_quality::GraphicsQuality;
use crate::sky::TimeOfDay;

// ---- Plugin -----------------------------------------------------------------

pub struct GodraysPlugin;

impl Plugin for GodraysPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_godrays)
           .add_systems(Update, (position_godrays_at_sun, update_godray_intensity));
    }
}

// ---- Components -------------------------------------------------------------

/// Marker on the single root entity that holds all godray quads as children.
#[derive(Component)]
struct GodrayRoot;

/// Marker on each individual quad child.
#[derive(Component)]
struct GodrayQuad;

// ---- Constants --------------------------------------------------------------

const QUAD_COUNT:    usize = 10;
/// Quad size used for all ray billboards.
const QUAD_SIZE_MAX: f32   = 14.0;
/// How far (m) in front of the camera we place the root.
const OFFSET_M:      f32   = 10.0;
/// Peak alpha at maximum effect (sun_y = 0).
const BASE_ALPHA:    f32 = 0.22;
/// Rays are visible when 0 <= sun_y <= SUN_Y_MAX.
const SUN_Y_MAX:     f32 = 0.40;

// Colour at golden hour (warm orange-amber).
const COLOR_GOLDEN: [f32; 3] = [1.00, 0.72, 0.35];
// Colour at slightly higher sun (softer warm-white).
const COLOR_DAY:    [f32; 3] = [1.00, 0.92, 0.75];

// ---- Startup ----------------------------------------------------------------

fn spawn_godrays(
    mut commands:  Commands,
    mut meshes:    ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    quality:       Res<GraphicsQuality>,
) {
    // Low quality: skip entirely — no god rays.
    if *quality == GraphicsQuality::Low {
        return;
    }

    // Shared unlit additive material.
    // Alpha and colour are overwritten each frame by `update_godray_intensity`.
    let mat = materials.add(StandardMaterial {
        base_color:   Color::srgba(
            COLOR_GOLDEN[0], COLOR_GOLDEN[1], COLOR_GOLDEN[2], BASE_ALPHA,
        ),
        unlit:        true,
        alpha_mode:   AlphaMode::Add,
        double_sided: true,
        cull_mode:    None,
        ..default()
    });

    let mesh = meshes.add(
        Plane3d::default().mesh().size(QUAD_SIZE_MAX, QUAD_SIZE_MAX)
    );

    // Spawn the root — initially hidden until the sky check runs.
    let root = commands.spawn((
        GodrayRoot,
        Transform::default(),
        Visibility::Hidden,
        InheritedVisibility::default(),
        ViewVisibility::default(),
    )).id();

    // Fan of quads around the Z axis.
    for i in 0..QUAD_COUNT {
        let angle = i as f32 * PI / QUAD_COUNT as f32;
        let rotation = Quat::from_rotation_z(angle);

        let quad = commands.spawn((
            GodrayQuad,
            Mesh3d(mesh.clone()),
            MeshMaterial3d(mat.clone()),
            Transform::from_rotation(rotation),
        )).id();

        commands.entity(root).add_child(quad);
    }
}

// ---- Systems ----------------------------------------------------------------

/// Position the GodrayRoot slightly in front of the camera in the sun direction.
fn position_godrays_at_sun(
    tod:      Res<TimeOfDay>,
    quality:  Res<GraphicsQuality>,
    cam_q:    Query<&Transform, With<Camera3d>>,
    mut root: Query<&mut Transform, (With<GodrayRoot>, Without<Camera3d>)>,
) {
    // Low quality spawned no root — nothing to do.
    if *quality == GraphicsQuality::Low {
        return;
    }

    let Ok(cam) = cam_q.single() else { return };
    let Ok(mut root_tf) = root.single_mut() else { return };

    let angle   = (tod.t - 0.25) * 2.0 * PI;
    let sun_dir = Vec3::new(angle.cos(), angle.sin(), 0.0).normalize();

    // Place root OFFSET_M ahead of the camera in the sun direction.
    let target_pos = cam.translation + sun_dir * OFFSET_M;
    root_tf.translation = target_pos;

    // Rotate root so its local +Y faces the camera (billboard the fan).
    let to_cam = (cam.translation - target_pos).normalize_or_zero();
    if to_cam.length_squared() > 0.0 {
        root_tf.look_at(cam.translation, Vec3::Y);
    }
}

/// Show/hide god rays and scale intensity + colour by sun elevation.
///
/// Behaviour:
///   sun_y < 0          -> hidden (night)
///   sun_y in [0, 0.40] -> visible; strongest + warmest at sun_y=0
///   sun_y > 0.40       -> hidden (mid-day)
fn update_godray_intensity(
    tod:           Res<TimeOfDay>,
    quality:       Res<GraphicsQuality>,
    mut root_q:    Query<&mut Visibility, With<GodrayRoot>>,
    quad_q:        Query<&MeshMaterial3d<StandardMaterial>, With<GodrayQuad>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if *quality == GraphicsQuality::Low {
        return;
    }

    let angle = (tod.t - 0.25) * 2.0 * PI;
    let sun_y = angle.sin();

    let Ok(mut vis) = root_q.single_mut() else { return };

    if sun_y > SUN_Y_MAX || sun_y < 0.0 {
        *vis = Visibility::Hidden;
        return;
    }

    *vis = Visibility::Visible;

    // t_horizon: 1 at sun_y=0 (horizon), 0 at sun_y=SUN_Y_MAX.
    let t_horizon = smooth_step((SUN_Y_MAX - sun_y) / SUN_Y_MAX);

    // Alpha: peaks at BASE_ALPHA when sun is at the horizon.
    let alpha = t_horizon * BASE_ALPHA;

    // Colour: warm golden at horizon, cooler warm-white as sun rises.
    let r = COLOR_GOLDEN[0] + (COLOR_DAY[0] - COLOR_GOLDEN[0]) * (1.0 - t_horizon);
    let g = COLOR_GOLDEN[1] + (COLOR_DAY[1] - COLOR_GOLDEN[1]) * (1.0 - t_horizon);
    let b = COLOR_GOLDEN[2] + (COLOR_DAY[2] - COLOR_GOLDEN[2]) * (1.0 - t_horizon);

    for mat_handle in &quad_q {
        if let Some(mat) = materials.get_mut(mat_handle) {
            mat.base_color = Color::srgba(r, g, b, alpha);
        }
    }
}

// ---- Helpers ----------------------------------------------------------------

#[inline]
fn smooth_step(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}
