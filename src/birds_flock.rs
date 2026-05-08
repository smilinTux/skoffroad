// Bird flock: 12 boid-style birds drifting overhead in a slow circle.
// Pure decoration — no physics, no collision, just transforms.
// Visible from anywhere on the map; hidden at night.
//
// Public API:
//   BirdsFlockPlugin

use std::f32::consts::PI;

use bevy::prelude::*;

use crate::sky::TimeOfDay;

// ---- Plugin -----------------------------------------------------------------

pub struct BirdsFlockPlugin;

impl Plugin for BirdsFlockPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(FlockState {
               center:  Vec3::new(0.0, 80.0, 0.0),
               heading: 0.0,
           })
           .add_systems(Startup, spawn_birds)
           .add_systems(Update, (
               update_flock_center,
               move_birds,
               hide_birds_at_night,
           ).chain());
    }
}

// ---- Resource ---------------------------------------------------------------

#[derive(Resource)]
struct FlockState {
    center:  Vec3,
    heading: f32,
}

// ---- Component --------------------------------------------------------------

/// Marks a bird entity and stores its relative offset within the flock.
#[derive(Component)]
struct Bird {
    offset: Vec3,
}

// ---- Constants --------------------------------------------------------------

const BIRD_COUNT: usize = 12;

// ---- LCG for deterministic offsets ------------------------------------------

/// Returns a value in [−1, +1) for a given seed.
#[inline]
fn lcg_signed(seed: usize) -> f32 {
    let mut v = (seed as u32).wrapping_mul(1664525).wrapping_add(1013904223);
    v ^= v >> 16;
    v = v.wrapping_mul(0x45d9f3b);
    v ^= v >> 16;
    (v as f32) / (u32::MAX as f32) * 2.0 - 1.0
}

// ---- Systems ----------------------------------------------------------------

fn spawn_birds(
    mut commands:  Commands,
    mut meshes:    ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Small dark cuboid: 0.4 (x) × 0.1 (y) × 0.6 (z) — bird silhouette.
    let mesh_handle = meshes.add(Cuboid::new(0.4, 0.1, 0.6));

    let mat_handle = materials.add(StandardMaterial {
        base_color: Color::srgb(0.10, 0.10, 0.12),
        unlit:      true,
        ..default()
    });

    for i in 0..BIRD_COUNT {
        // Scatter each bird within a ~5 m radius around the flock center.
        let ox = lcg_signed(i * 3 + 0) * 5.0;
        let oy = lcg_signed(i * 3 + 1) * 2.0;
        let oz = lcg_signed(i * 3 + 2) * 5.0;
        let offset = Vec3::new(ox, oy, oz);

        commands.spawn((
            Mesh3d(mesh_handle.clone()),
            MeshMaterial3d(mat_handle.clone()),
            Transform::from_translation(Vec3::new(0.0, 80.0, 0.0) + offset),
            Bird { offset },
        ));
    }
}

/// Advances the flock center along a slow 60 m circle at altitude ~80 m.
fn update_flock_center(
    time:      Res<Time>,
    mut flock: ResMut<FlockState>,
) {
    let dt = time.delta_secs();

    // ~1 revolution every 42 s  (2π / 0.15 ≈ 41.9 s)
    flock.heading += dt * 0.15;

    flock.center.x = flock.heading.cos() * 60.0;
    flock.center.z = flock.heading.sin() * 60.0;
    flock.center.y = 80.0 + (flock.heading * 1.7).sin() * 6.0;
}

/// Positions each bird at flock center + its offset (with a small flapping
/// perturbation) and rotates it to face the direction of motion.
fn move_birds(
    time:  Res<Time>,
    flock: Res<FlockState>,
    mut query: Query<(&Bird, &mut Transform)>,
) {
    let t = time.elapsed_secs();
    let rotation = Quat::from_rotation_y(flock.heading + PI / 2.0);

    for (bird, mut transform) in &mut query {
        // Gentle vertical flutter: sin(time*1.3 + offset.x) * 0.3
        let flutter = (t * 1.3 + bird.offset.x).sin() * 0.3;
        let perturbed = bird.offset + Vec3::new(0.0, flutter, 0.0);

        transform.translation = flock.center + perturbed;
        transform.rotation    = rotation;
    }
}

/// Hides all birds outside the daytime window [0.20, 0.80].
fn hide_birds_at_night(
    tod:       Res<TimeOfDay>,
    mut query: Query<&mut Visibility, With<Bird>>,
) {
    let visible = tod.t >= 0.20 && tod.t <= 0.80;
    let vis = if visible { Visibility::Visible } else { Visibility::Hidden };

    for mut v in &mut query {
        *v = vis;
    }
}
