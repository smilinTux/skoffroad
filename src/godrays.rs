// God rays / sun shafts: spawns radial billboards at the sun direction
// when the sun is low (sunrise/sunset). Subtle additive layer faked with
// alpha quads — no shader pipeline.
//
// Public API:
//   GodraysPlugin

use std::f32::consts::PI;

use bevy::prelude::*;

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

const QUAD_COUNT: usize = 8;
/// Each quad is 8×8 m in its local plane.
const QUAD_SIZE:  f32   = 8.0;
/// How far (m) in front of the camera we place the root.
const OFFSET_M:   f32   = 10.0;
/// Base alpha at maximum effect (sun_y = 0.0).
const BASE_ALPHA: f32   = 0.15;
/// Above this sun_y, god rays are hidden.
const SUN_Y_MAX:  f32   = 0.4;

// ---- Startup ----------------------------------------------------------------

fn spawn_godrays(
    mut commands:  Commands,
    mut meshes:    ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Shared unlit additive material — amber glow.
    // Alpha is overwritten each frame by `update_godray_intensity`.
    let mat = materials.add(StandardMaterial {
        base_color:  Color::srgba(1.0, 0.85, 0.5, BASE_ALPHA),
        unlit:       true,
        alpha_mode:  AlphaMode::Add,
        double_sided: true,
        cull_mode:   None,
        ..default()
    });

    let mesh = meshes.add(Plane3d::default().mesh().size(QUAD_SIZE, QUAD_SIZE));

    // Spawn the root — initially hidden until the sky check runs.
    let root = commands.spawn((
        GodrayRoot,
        Transform::default(),
        Visibility::Hidden,
        InheritedVisibility::default(),
        ViewVisibility::default(),
    )).id();

    // Eight quads fanned around the Z axis.
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
    cam_q:    Query<&Transform, With<Camera3d>>,
    mut root: Query<&mut Transform, (With<GodrayRoot>, Without<Camera3d>)>,
) {
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

/// Show / hide godrays and adjust intensity based on sun elevation.
fn update_godray_intensity(
    tod:          Res<TimeOfDay>,
    mut root_q:   Query<&mut Visibility, With<GodrayRoot>>,
    quad_q:       Query<&MeshMaterial3d<StandardMaterial>, With<GodrayQuad>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let angle = (tod.t - 0.25) * 2.0 * PI;
    let sun_y = angle.sin();

    let Ok(mut vis) = root_q.single_mut() else { return };

    if sun_y > SUN_Y_MAX || sun_y < 0.0 {
        *vis = Visibility::Hidden;
        return;
    }

    *vis = Visibility::Visible;

    // Alpha ramps from 0.15 at sun_y=0 down to 0.0 at sun_y=0.4.
    let alpha = (SUN_Y_MAX - sun_y) / SUN_Y_MAX * BASE_ALPHA;

    for mat_handle in &quad_q {
        if let Some(mat) = materials.get_mut(mat_handle) {
            mat.base_color = Color::srgba(1.0, 0.85, 0.5, alpha);
        }
    }
}
