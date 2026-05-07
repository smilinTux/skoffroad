// HDR sky dome for SandK Offroad — Sprint 40.
//
// Spawns a giant inverted icosphere (radius 2000 m) whose inner surface carries
// a baked per-vertex gradient:
//   zenith  → sRGB(0.20, 0.35, 0.65)  (deep blue)
//   horizon → sRGB(0.85, 0.75, 0.55)  (warm sand)
//   below   → sRGB(0.30, 0.25, 0.20)  (ground tint)
//
// The material is unlit + emissive so the dome stays bright regardless of the
// scene's directional light or ambient setting.  The dome follows the camera
// each frame so the player never reaches its edge.
//
// This file is distinct from sky.rs (which owns the sun, fog, time-of-day, and
// the smaller 900 m UV-sphere) and from stars.rs (which owns the star quads).

use bevy::{
    asset::RenderAssetUsages,
    mesh::VertexAttributeValues,
    prelude::*,
};

// ---- Plugin -----------------------------------------------------------------

pub struct HdrSkyboxPlugin;

impl Plugin for HdrSkyboxPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_hdr_skybox)
           .add_systems(Update, attach_to_camera);
    }
}

// ---- Marker component -------------------------------------------------------

/// Marks the HDR sky-dome entity so `attach_to_camera` can find it.
#[derive(Component)]
pub struct HdrSkybox;

// ---- Constants --------------------------------------------------------------

const DOME_RADIUS: f32 = 2000.0;

/// sRGB warm horizon colour at y = 0.
const HORIZON: [f32; 4] = [0.85, 0.75, 0.55, 1.0];
/// sRGB zenith colour at y = 1.
const ZENITH:  [f32; 4] = [0.20, 0.35, 0.65, 1.0];
/// sRGB ground tint for vertices below the horizon (y < 0).
const GROUND:  [f32; 4] = [0.30, 0.25, 0.20, 1.0];

// ---- Startup: spawn the dome ------------------------------------------------

fn spawn_hdr_skybox(
    mut commands:  Commands,
    mut meshes:    ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Build a subdivision-3 icosphere and then mutate it in place.
    let mut mesh: Mesh = Sphere::new(DOME_RADIUS).mesh().ico(3).unwrap();

    // --- 1. Flip normals so the inside surface is rendered -------------------
    //
    // Bevy's icosphere has outward-pointing normals.  We negate them so the
    // inside of the sphere faces the camera (which sits at the centre).
    if let Some(VertexAttributeValues::Float32x3(normals)) =
        mesh.attribute_mut(Mesh::ATTRIBUTE_NORMAL)
    {
        for n in normals.iter_mut() {
            n[0] = -n[0];
            n[1] = -n[1];
            n[2] = -n[2];
        }
    }

    // --- 2. Compute per-vertex gradient colours ------------------------------
    //
    // Read positions, derive y_norm, lerp colours, inject as ATTRIBUTE_COLOR.
    let colors: Vec<[f32; 4]> = {
        // Borrow positions immutably first, then drop before inserting colors.
        let positions: Vec<[f32; 3]> = match mesh.attribute(Mesh::ATTRIBUTE_POSITION) {
            Some(VertexAttributeValues::Float32x3(v)) => v.clone(),
            _ => Vec::new(),
        };

        positions
            .iter()
            .map(|&[_x, y, _z]| {
                let y_norm = (y / DOME_RADIUS).clamp(-1.0, 1.0);
                if y_norm >= 0.0 {
                    // Above horizon: lerp horizon → zenith.
                    lerp_color4(&HORIZON, &ZENITH, y_norm)
                } else {
                    // Below horizon: lerp horizon → ground.
                    lerp_color4(&HORIZON, &GROUND, -y_norm)
                }
            })
            .collect()
    };

    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);

    // Ensure the mesh is visible on both sides (cull_mode = None handles this
    // in the material, but RenderAssetUsages::default() is still needed).
    let _ = RenderAssetUsages::default(); // already default on Sphere::mesh()

    let mat = materials.add(StandardMaterial {
        // base_color tints the vertex colors; WHITE = identity.
        base_color: Color::WHITE,
        // Emissive = base_color so the dome glows at full intensity.
        emissive: LinearRgba::WHITE,
        // Unlit: ignore all scene lighting.
        unlit: true,
        // Render from inside the sphere (no back-face culling).
        cull_mode: None,
        ..default()
    });

    commands.spawn((
        HdrSkybox,
        Mesh3d(meshes.add(mesh)),
        MeshMaterial3d(mat),
        Transform::default(),
    ));
}

// ---- Update: keep dome centred on the camera --------------------------------

/// Each frame, copy the active Camera3d's XYZ translation to the sky dome so
/// the player can never fly "outside" the gradient sphere.
fn attach_to_camera(
    camera_q: Query<&Transform, With<Camera3d>>,
    mut dome_q: Query<&mut Transform, (With<HdrSkybox>, Without<Camera3d>)>,
) {
    let Ok(cam_tf) = camera_q.single() else { return };
    for mut dome_tf in &mut dome_q {
        dome_tf.translation = cam_tf.translation;
    }
}

// ---- Helpers ----------------------------------------------------------------

/// Component-wise linear interpolation between two sRGBA colours.
#[inline]
fn lerp_color4(a: &[f32; 4], b: &[f32; 4], t: f32) -> [f32; 4] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
        1.0,
    ]
}
