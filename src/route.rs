// Procedural trail route: 12 bright cyan emissive pillars arranged as a noisy
// circle around the spawn point. The player drives through them freely — no
// colliders, no scoring, just clear visual breadcrumbs marking the loop.

use bevy::prelude::*;

use crate::terrain::{terrain_height_at, TERRAIN_SEED};

pub struct RoutePlugin;

impl Plugin for RoutePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_route);
    }
}

// ---------------------------------------------------------------------------
// LCG — same style as ramps.rs so the route is fully deterministic.
// ---------------------------------------------------------------------------

struct Lcg(u64);

impl Lcg {
    fn new(seed: u32) -> Self {
        Self(seed as u64)
    }

    /// Next float in [0, 1).
    fn next_f32(&mut self) -> f32 {
        self.0 = self.0.wrapping_mul(1664525).wrapping_add(1013904223) & 0xFFFF_FFFF;
        (self.0 as f32) / (u32::MAX as f32)
    }

    /// Float in [-1, 1).
    fn signed(&mut self) -> f32 {
        self.next_f32() * 2.0 - 1.0
    }
}

// ---------------------------------------------------------------------------
// Waypoint geometry constants
// ---------------------------------------------------------------------------

const WAYPOINTS: usize = 12;
const BASE_RADIUS: f32 = 50.0;
const RADIUS_JITTER: f32 = 15.0;
// The pillar base sits this many metres above the terrain surface.
const PILLAR_LIFT: f32 = 1.5;
// Cylinder dimensions: radius 0.25 m, height 4 m.
const PILLAR_RADIUS: f32 = 0.25;
const PILLAR_HEIGHT: f32 = 4.0;

// ---------------------------------------------------------------------------
// Spawn
// ---------------------------------------------------------------------------

fn spawn_route(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Seed is offset from TERRAIN_SEED by 23 so the route positions are
    // independent of scatter, ramp, and heightmap noise sequences.
    let mut lcg = Lcg::new(TERRAIN_SEED + 23);

    let pillar_mesh = meshes.add(Cylinder::new(PILLAR_RADIUS, PILLAR_HEIGHT));

    let pillar_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.0, 1.0, 1.0),
        emissive: LinearRgba::rgb(0.0, 2.0, 2.0),
        // Unlit-ish: high emissive, but keep PBR so it picks up bloom if present.
        perceptual_roughness: 0.4,
        ..default()
    });

    for i in 0..WAYPOINTS {
        let angle = i as f32 * std::f32::consts::TAU / WAYPOINTS as f32;
        let r = BASE_RADIUS + lcg.signed() * RADIUS_JITTER;
        let x = r * angle.cos();
        let z = r * angle.sin();
        // Terrain height at this XZ, then lift so the pillar base clears the ground.
        // Bevy's Cylinder is centred at its own origin, so we add half the height
        // plus the requested clearance to place the bottom edge correctly.
        let y = terrain_height_at(x, z) + PILLAR_LIFT + PILLAR_HEIGHT * 0.5;

        commands.spawn((
            Mesh3d(pillar_mesh.clone()),
            MeshMaterial3d(pillar_mat.clone()),
            Transform::from_translation(Vec3::new(x, y, z)),
        ));
    }
}
