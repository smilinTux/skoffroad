// Snow: white sphere particles falling during Winter season.
// Activated when SeasonState.current == Season::Winter (~150 flakes).
// Each flake carries its own velocity and is despawned when it hits the
// terrain, then refilled by manage_snow_particles on the next frame.
//
// Public API:
//   SnowPlugin

use bevy::prelude::*;

use crate::season::{Season, SeasonState};
use crate::terrain::terrain_height_at;

// ---- Public API ---------------------------------------------------------------

pub struct SnowPlugin;

impl Plugin for SnowPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (manage_snow_particles, move_snow));
    }
}

// ---- Internal components ------------------------------------------------------

/// Marker component for a snowflake particle entity.
#[derive(Component)]
pub struct Snowflake {
    pub vel: Vec3,
}

// ---- Constants ----------------------------------------------------------------

/// Target live snowflake count while Winter is active.
const FLAKE_COUNT: usize = 150;

/// Radius of each sphere mesh (m).
const FLAKE_RADIUS: f32 = 0.08;

/// Half-side of the horizontal spawn box around the camera (m).
const SPAWN_RADIUS: f32 = 50.0;

/// Minimum height above the camera to spawn a flake.
const SPAWN_Y_MIN: f32 = 10.0;

/// Maximum height above the camera to spawn a flake.
const SPAWN_Y_MAX: f32 = 30.0;

/// Horizontal drift magnitude (m/s).
const DRIFT: f32 = 0.3;

/// Base downward speed (m/s).
const FALL_SPEED: f32 = 1.5;

// ---- Systems ------------------------------------------------------------------

/// Spawn flakes up to the cap while in Winter; despawn all when leaving Winter.
pub fn manage_snow_particles(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    season: Option<Res<SeasonState>>,
    camera_q: Query<&Transform, With<Camera3d>>,
    flakes: Query<Entity, With<Snowflake>>,
) {
    // If no SeasonState resource or not Winter: despawn everything and bail.
    let is_winter = season
        .as_ref()
        .map(|s| s.current == Season::Winter)
        .unwrap_or(false);

    if !is_winter {
        for entity in &flakes {
            commands.entity(entity).despawn();
        }
        return;
    }

    let Ok(cam_tf) = camera_q.single() else { return };
    let cam_pos = cam_tf.translation;

    let existing = flakes.iter().count();
    if existing >= FLAKE_COUNT {
        return;
    }

    let to_spawn = FLAKE_COUNT - existing;

    // Shared mesh + material (handles are cloned so the GPU resource is shared).
    let mesh = meshes.add(Sphere::new(FLAKE_RADIUS));
    let mat = materials.add(StandardMaterial {
        base_color: Color::srgba(0.95, 0.95, 1.0, 0.85),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        double_sided: true,
        cull_mode: None,
        ..default()
    });

    // Cheap deterministic LCG seeded from camera position + existing count.
    let mut seed: u32 = (cam_pos.x.to_bits() ^ cam_pos.z.to_bits())
        .wrapping_add(existing as u32)
        .wrapping_mul(1_664_525)
        .wrapping_add(1_013_904_223);

    for _ in 0..to_spawn {
        let rx = lcg_signed(&mut seed) * SPAWN_RADIUS;
        let rz = lcg_signed(&mut seed) * SPAWN_RADIUS;
        // Scatter vertically so the initial cloud looks like ongoing snowfall.
        let ry = lcg_next(&mut seed) * (SPAWN_Y_MAX - SPAWN_Y_MIN) + SPAWN_Y_MIN;

        let pos = Vec3::new(cam_pos.x + rx, cam_pos.y + ry, cam_pos.z + rz);

        let vx = lcg_signed(&mut seed) * DRIFT;
        let vz = lcg_signed(&mut seed) * DRIFT;
        let vel = Vec3::new(vx, -FALL_SPEED, vz);

        commands.spawn((
            Snowflake { vel },
            Mesh3d(mesh.clone()),
            MeshMaterial3d(mat.clone()),
            Transform::from_translation(pos),
        ));
    }
}

/// Integrate snowflake positions; despawn any that reach terrain level.
pub fn move_snow(
    mut commands: Commands,
    time: Res<Time>,
    season: Option<Res<SeasonState>>,
    camera_q: Query<&Transform, With<Camera3d>>,
    mut flakes: Query<
        (Entity, &mut Transform, &Snowflake),
        (With<Snowflake>, Without<crate::storm::RainDrop>, Without<Camera3d>),
    >,
) {
    let is_winter = season
        .as_ref()
        .map(|s| s.current == Season::Winter)
        .unwrap_or(false);

    if !is_winter {
        return;
    }

    let dt = time.delta_secs();
    let Ok(cam_tf) = camera_q.single() else { return };
    let cam_pos = cam_tf.translation;

    for (entity, mut tf, flake) in &mut flakes {
        tf.translation += flake.vel * dt;

        let x = tf.translation.x;
        let z = tf.translation.z;
        let terrain_y = terrain_height_at(x, z);

        // Despawn if the flake has landed on terrain or drifted far out of range.
        let dx = (x - cam_pos.x).abs();
        let dz = (z - cam_pos.z).abs();
        let out_of_range = dx > SPAWN_RADIUS * 1.5 || dz > SPAWN_RADIUS * 1.5;

        if tf.translation.y < terrain_y || out_of_range {
            // manage_snow_particles will refill the pool next frame.
            commands.entity(entity).despawn();
        }
    }
}

// ---- LCG helpers --------------------------------------------------------------

#[inline]
fn lcg_next(seed: &mut u32) -> f32 {
    *seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    *seed as f32 / u32::MAX as f32
}

#[inline]
fn lcg_signed(seed: &mut u32) -> f32 {
    lcg_next(seed) * 2.0 - 1.0
}
