// Forza-style ground trail: 30 pre-spawned sphere markers forming a glowing
// cyan line from the chassis toward the current course target.
//
// Design: fixed pool (no per-frame spawn/despawn). Each frame the markers are
// repositioned along the chassis→target straight line, sitting on the terrain.
// When there is no target, or the chassis is within 6 m, all markers are hidden.
// When the target is more than 100 m away only every other marker is shown
// (15 visible) to avoid a wall of dots at long range.

use bevy::prelude::*;

use crate::course::CourseState;
use crate::terrain::terrain_height_at;
use crate::vehicle::{Chassis, VehicleRoot};

// ---- Constants ----------------------------------------------------------------

const POOL_SIZE: usize = 30;
const MARKER_RADIUS: f32 = 0.25;
const HOVER_HEIGHT: f32 = 0.4;
const HIDE_DIST: f32 = 6.0;
const SPARSE_DIST: f32 = 100.0;

// ---- Component ----------------------------------------------------------------

#[derive(Component)]
pub struct TrailMarker {
    pub index: u32,
}

// ---- Plugin -------------------------------------------------------------------

pub struct TrailPlugin;

impl Plugin for TrailPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_trail_pool)
           .add_systems(Update, update_trail);
    }
}

// ---- Startup: pre-spawn pool --------------------------------------------------

fn spawn_trail_pool(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mesh = meshes.add(Sphere::new(MARKER_RADIUS).mesh().ico(1).unwrap());
    let material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.2, 1.0, 1.0),
        emissive: LinearRgba::rgb(0.0, 2.5, 2.5),
        ..default()
    });

    for i in 0..POOL_SIZE {
        commands.spawn((
            TrailMarker { index: i as u32 },
            Mesh3d(mesh.clone()),
            MeshMaterial3d(material.clone()),
            Transform::from_translation(Vec3::ZERO),
            Visibility::Hidden,
        ));
    }
}

// ---- Update: position & show/hide markers each frame -------------------------

fn update_trail(
    vehicle: Option<Res<VehicleRoot>>,
    course: Option<Res<CourseState>>,
    chassis_q: Query<&Transform, With<Chassis>>,
    mut marker_q: Query<(&TrailMarker, &mut Transform, &mut Visibility), Without<Chassis>>,
) {
    // Resolve chassis position.
    let chassis_pos = {
        let Some(vehicle) = vehicle else {
            hide_all(&mut marker_q);
            return;
        };
        let Ok(transform) = chassis_q.get(vehicle.chassis) else {
            hide_all(&mut marker_q);
            return;
        };
        transform.translation
    };

    // Resolve course target.
    let target = {
        let Some(course) = course else {
            hide_all(&mut marker_q);
            return;
        };
        let Some(t) = course.current_target else {
            hide_all(&mut marker_q);
            return;
        };
        t
    };

    let chassis_xz = Vec2::new(chassis_pos.x, chassis_pos.z);
    let target_xz  = Vec2::new(target.x, target.z);
    let dist       = chassis_xz.distance(target_xz);

    // Hide all markers if too close to the target.
    if dist < HIDE_DIST {
        hide_all(&mut marker_q);
        return;
    }

    let sparse = dist > SPARSE_DIST;

    for (marker, mut transform, mut visibility) in &mut marker_q {
        let i = marker.index as usize;

        // In sparse mode only show even-indexed markers.
        if sparse && (i % 2 != 0) {
            *visibility = Visibility::Hidden;
            continue;
        }

        let t   = (i + 1) as f32 / (POOL_SIZE + 1) as f32;
        let xz  = chassis_xz.lerp(target_xz, t);
        let y   = terrain_height_at(xz.x, xz.y) + HOVER_HEIGHT;

        transform.translation = Vec3::new(xz.x, y, xz.y);
        *visibility = Visibility::Inherited;
    }
}

// ---- Helper ------------------------------------------------------------------

fn hide_all(
    marker_q: &mut Query<(&TrailMarker, &mut Transform, &mut Visibility), Without<Chassis>>,
) {
    for (_marker, _transform, mut visibility) in marker_q.iter_mut() {
        *visibility = Visibility::Hidden;
    }
}
