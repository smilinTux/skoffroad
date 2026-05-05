// Rally course arch banners — START / CHECKPOINT / FINISH.
//
// Each arch is an inverted-U built from:
//   - Two vertical Cylinder pillars (r=0.4, h=8.0) separated by 7 m
//   - One horizontal Cuboid top beam (8.0 x 0.8 x 0.8) joining the pillar tops
//
// Color coding:
//   START      — bright green emissive
//   CHECKPOINT — yellow emissive
//   FINISH     — red emissive
//
// Arch orientations face toward the next arch in sequence so the course
// direction is implied by the geometry.

use bevy::prelude::*;
use avian3d::prelude::*;

use crate::terrain::terrain_height_at;

// ---------------------------------------------------------------------------
// Public plugin
// ---------------------------------------------------------------------------

pub struct BannersPlugin;

impl Plugin for BannersPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_banners);
    }
}

// ---------------------------------------------------------------------------
// Arch geometry constants
// ---------------------------------------------------------------------------

const PILLAR_RADIUS: f32 = 0.4;
const PILLAR_HEIGHT: f32 = 8.0;
/// Half-span: each pillar is placed +/- HALF_SPAN from the arch center.
const HALF_SPAN: f32 = 3.5;
/// Beam spans the full gap between pillar outer edges.
const BEAM_WIDTH: f32 = HALF_SPAN * 2.0 + PILLAR_RADIUS * 2.0; // ~7.8 m
const BEAM_HEIGHT: f32 = 0.8;
const BEAM_DEPTH: f32 = 0.8;

// ---------------------------------------------------------------------------
// Arch definitions
// ---------------------------------------------------------------------------

/// Fixed world XZ positions for each arch.  Y is queried from the heightmap.
struct ArchDef {
    xz: [f32; 2],
    color: LinearRgba,
}

const ARCHES: [ArchDef; 4] = [
    // START — near spawn, green
    ArchDef { xz: [5.0, -5.0],   color: LinearRgba::rgb(0.0, 3.0, 0.0) },
    // CHECKPOINT 1 — yellow
    ArchDef { xz: [40.0, 30.0],  color: LinearRgba::rgb(3.0, 2.5, 0.0) },
    // CHECKPOINT 2 — yellow
    ArchDef { xz: [-40.0, 50.0], color: LinearRgba::rgb(3.0, 2.5, 0.0) },
    // FINISH — red
    ArchDef { xz: [60.0, -40.0], color: LinearRgba::rgb(3.0, 0.0, 0.0) },
];

// ---------------------------------------------------------------------------
// Startup system
// ---------------------------------------------------------------------------

fn spawn_banners(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let pillar_mesh = meshes.add(Cylinder::new(PILLAR_RADIUS, PILLAR_HEIGHT));
    let beam_mesh   = meshes.add(Cuboid::new(BEAM_WIDTH, BEAM_HEIGHT, BEAM_DEPTH));

    let num_arches = ARCHES.len();

    for (i, arch) in ARCHES.iter().enumerate() {
        let [ax, az] = arch.xz;
        let ground_y = terrain_height_at(ax, az);

        // Orientation: face toward the next arch in sequence (wrap at end).
        let next_i = (i + 1) % num_arches;
        let [nx, nz] = ARCHES[next_i].xz;
        let dir = Vec2::new(nx - ax, nz - az);
        // yaw so that local +X points toward the next arch.
        // atan2(dx, dz) gives the angle in the XZ plane.
        let yaw = dir.x.atan2(dir.y);
        let rotation = Quat::from_rotation_y(yaw);

        let mat = materials.add(StandardMaterial {
            base_color: Color::WHITE,
            emissive: arch.color,
            perceptual_roughness: 0.4,
            ..default()
        });

        // Arch root: centred at the midpoint between the pillars, elevated so
        // pillars sit on the terrain.  Cylinder origin is at its centre, so we
        // raise the root by half the pillar height.
        let root_y = ground_y + PILLAR_HEIGHT * 0.5;

        let root = commands.spawn((
            Transform::from_xyz(ax, root_y, az).with_rotation(rotation),
            Visibility::default(),
        )).id();

        // Left pillar (local -X from centre, oriented along arch span).
        let left_pillar = commands.spawn((
            Mesh3d(pillar_mesh.clone()),
            MeshMaterial3d(mat.clone()),
            Transform::from_xyz(-HALF_SPAN, 0.0, 0.0),
            RigidBody::Static,
            Collider::cylinder(PILLAR_RADIUS, PILLAR_HEIGHT),
        )).id();

        // Right pillar.
        let right_pillar = commands.spawn((
            Mesh3d(pillar_mesh.clone()),
            MeshMaterial3d(mat.clone()),
            Transform::from_xyz(HALF_SPAN, 0.0, 0.0),
            RigidBody::Static,
            Collider::cylinder(PILLAR_RADIUS, PILLAR_HEIGHT),
        )).id();

        // Top beam: centred horizontally, raised to just above the pillar tops.
        // Pillar top is at +PILLAR_HEIGHT/2 relative to the root.  Place beam
        // centre at that height plus half the beam thickness.
        let beam_y = PILLAR_HEIGHT * 0.5 + BEAM_HEIGHT * 0.5;
        let top_beam = commands.spawn((
            Mesh3d(beam_mesh.clone()),
            MeshMaterial3d(mat.clone()),
            Transform::from_xyz(0.0, beam_y, 0.0),
            RigidBody::Static,
            Collider::cuboid(BEAM_WIDTH * 0.5, BEAM_HEIGHT * 0.5, BEAM_DEPTH * 0.5),
        )).id();

        commands.entity(root).add_children(&[left_pillar, right_pillar, top_beam]);
    }
}
