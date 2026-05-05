// Distance markers forming a "+" grid along X=0 and Z=0 axes.
//
// Markers are placed every 25 m from -100 to +100, skipping the origin.
// Each marker is a thin white vertical pole with a red stripe at mid-height.
// Materials are created once and cloned (handle-copy) across all 16 markers.

use bevy::prelude::*;

use crate::terrain::terrain_height_at;

pub struct MarkersPlugin;

impl Plugin for MarkersPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_markers);
    }
}

// Pole geometry.
const POLE_RADIUS: f32 = 0.08;
const POLE_HEIGHT: f32 = 1.5;

// Red stripe ring — slightly wider radius, thinner height, centred at pole mid.
const STRIPE_RADIUS: f32 = 0.13;
const STRIPE_HEIGHT: f32 = 0.18;

// Grid parameters.
const STEP: f32 = 25.0;
const EXTENT: i32 = 4; // positions: -4*25 … +4*25 = -100 … +100

fn spawn_markers(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // One mesh + material for the pole, one for the stripe — shared via handle.
    let pole_mesh = meshes.add(Cylinder::new(POLE_RADIUS, POLE_HEIGHT));
    let stripe_mesh = meshes.add(Cylinder::new(STRIPE_RADIUS, STRIPE_HEIGHT));

    let pole_mat = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        emissive: LinearRgba::rgb(0.8, 0.8, 0.8),
        perceptual_roughness: 0.5,
        ..default()
    });

    let stripe_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.9, 0.05, 0.05),
        emissive: LinearRgba::rgb(0.6, 0.0, 0.0),
        perceptual_roughness: 0.4,
        ..default()
    });

    // Collect all (x, z) positions along the two axes.
    let mut positions: Vec<(f32, f32)> = Vec::new();

    for i in -EXTENT..=EXTENT {
        if i == 0 {
            continue; // skip origin
        }
        let coord = i as f32 * STEP;
        // Along X axis: z = 0, x varies.
        positions.push((coord, 0.0));
        // Along Z axis: x = 0, z varies.
        positions.push((0.0, coord));
    }

    for (x, z) in positions {
        let ground_y = terrain_height_at(x, z);

        // Pole centre: half-height above ground so bottom sits on terrain.
        let pole_y = ground_y + POLE_HEIGHT * 0.5;

        // Stripe sits at pole mid-height (POLE_HEIGHT/2 above pole centre).
        let stripe_y = ground_y + POLE_HEIGHT * 0.5;

        // Spawn the pole.
        commands.spawn((
            Mesh3d(pole_mesh.clone()),
            MeshMaterial3d(pole_mat.clone()),
            Transform::from_xyz(x, pole_y, z),
        ));

        // Spawn the stripe ring at the same Y as pole centre (mid-height of pole).
        commands.spawn((
            Mesh3d(stripe_mesh.clone()),
            MeshMaterial3d(stripe_mat.clone()),
            Transform::from_xyz(x, stripe_y, z),
        ));
    }
}
