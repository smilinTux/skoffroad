// Mountain range: 16 distant peak silhouettes ringing the map at radius
// 400m. Tall low-poly cones tinted faded blue to give a sense of distance.
// Static, no colliders.
//
// Public API:
//   MountainRangePlugin

use bevy::prelude::*;

pub struct MountainRangePlugin;

impl Plugin for MountainRangePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_mountains);
    }
}

// ---- Marker component -------------------------------------------------------

#[derive(Component)]
pub struct MountainPeak;

// ---- Constants --------------------------------------------------------------

const PEAK_COUNT:  usize = 16;
const RING_RADIUS: f32   = 400.0;
const PEAK_Y:      f32   = -10.0;

// Base radial-jitter amplitude (±30 m)
const JITTER_AMP:  f32 = 30.0;

// Default cone dimensions (varied per-peak via LCG)
const BASE_RADIUS_MIN: f32 = 50.0;
const BASE_RADIUS_MAX: f32 = 100.0;
const HEIGHT_MIN:      f32 = 80.0;
const HEIGHT_MAX:      f32 = 160.0;

// Atmospheric blue (faded, desaturated)
const PEAK_COLOR: Color = Color::srgba(0.32, 0.40, 0.55, 1.0);

// ---- Deterministic LCG (seed=33) --------------------------------------------

/// Minimal linear-congruential generator.
/// Returns successive pseudo-random u32 values.
struct Lcg(u32);

impl Lcg {
    fn new(seed: u32) -> Self {
        Self(seed)
    }

    fn next_u32(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(1664525).wrapping_add(1013904223);
        self.0
    }

    /// Returns a value in [0, 1).
    fn next_f32(&mut self) -> f32 {
        (self.next_u32() as f32) / (u32::MAX as f32)
    }

    /// Returns a value in [lo, hi).
    fn next_range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + self.next_f32() * (hi - lo)
    }
}

// ---- Startup system ---------------------------------------------------------

fn spawn_mountains(
    mut commands:  Commands,
    mut meshes:    ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mut rng = Lcg::new(33);

    for i in 0..PEAK_COUNT {
        // Evenly spaced angles around a full circle.
        let angle = (i as f32 / PEAK_COUNT as f32) * std::f32::consts::TAU;

        // ±30 m radial jitter for variety.
        let jitter  = rng.next_range(-JITTER_AMP, JITTER_AMP);
        let r       = RING_RADIUS + jitter;

        let pos = Vec3::new(angle.cos() * r, PEAK_Y, angle.sin() * r);

        // Per-peak cone dimensions via LCG.
        let cone_radius = rng.next_range(BASE_RADIUS_MIN, BASE_RADIUS_MAX);
        let cone_height = rng.next_range(HEIGHT_MIN, HEIGHT_MAX);

        // Cone mesh — Bevy 0.18 exposes `Cone::new(radius, height)`.
        // Resolution kept at 6 for a deliberately low-poly silhouette.
        let mesh = meshes.add(
            Cone::new(cone_radius, cone_height)
                .mesh()
                .resolution(6)
                .build(),
        );

        let mat = materials.add(StandardMaterial {
            base_color:          PEAK_COLOR,
            perceptual_roughness: 0.95,
            ..default()
        });

        commands.spawn((
            MountainPeak,
            Mesh3d(mesh),
            MeshMaterial3d(mat),
            Transform::from_translation(pos),
        ));
    }
}
