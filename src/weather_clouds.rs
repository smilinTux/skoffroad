// Weather clouds: 8 drifting cumulus-style cloud meshes (composed of
// stacked white cuboids) overhead. They slowly drift in the wind direction.
// Pure decoration — no shadows, no physics.
//
// Public API:
//   WeatherCloudsPlugin

use bevy::prelude::*;

use crate::wind::WindState;

// ---- Constants ---------------------------------------------------------------

const CLOUD_COUNT: usize = 8;

/// Clouds move at this fraction of wind speed (dimensionless scale factor).
const WIND_SPEED_FACTOR: f32 = 0.3;

/// Fallback wind direction when WindState is absent.
const FALLBACK_WIND_DIR: Vec3 = Vec3::new(0.5, 0.0, -0.866);

/// Clouds wrap when their X or Z drifts outside ±WRAP_LIMIT.
const WRAP_LIMIT: f32 = 200.0;

// ---- Plugin ------------------------------------------------------------------

pub struct WeatherCloudsPlugin;

impl Plugin for WeatherCloudsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_clouds)
           .add_systems(Update, (drift_clouds, wrap_clouds));
    }
}

// ---- Component ---------------------------------------------------------------

/// Marks a cloud parent entity.
#[derive(Component)]
pub struct CloudParent;

// ---- LCG ---------------------------------------------------------------------

struct Lcg(u64);

impl Lcg {
    fn new(seed: u32) -> Self {
        Self(seed as u64)
    }

    /// Returns the next value in [0, 1).
    fn next_f32(&mut self) -> f32 {
        self.0 = self.0
            .wrapping_mul(1_664_525)
            .wrapping_add(1_013_904_223)
            & 0xFFFF_FFFF;
        (self.0 as f32) / (u32::MAX as f32)
    }

    /// Returns the next value in [lo, hi).
    fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + self.next_f32() * (hi - lo)
    }
}

// ---- Spawn system ------------------------------------------------------------

fn spawn_clouds(
    mut commands:  Commands,
    mut meshes:    ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let cloud_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(0.95, 0.95, 0.97, 0.85),
        alpha_mode: AlphaMode::Blend,
        unlit:      true,
        ..default()
    });

    let mut lcg = Lcg::new(11);

    for _ in 0..CLOUD_COUNT {
        // Deterministic parent position.
        let px = lcg.range(-150.0, 150.0);
        let py = lcg.range(60.0, 90.0);
        let pz = lcg.range(-150.0, 150.0);

        let parent = commands.spawn((
            CloudParent,
            Transform::from_translation(Vec3::new(px, py, pz)),
            Visibility::default(),
        )).id();

        // 4–5 cuboid puffs per cloud (draw once for count).
        let puff_count = 4 + (lcg.next_f32() * 2.0) as usize; // 4 or 5

        for _ in 0..puff_count {
            let w = lcg.range(3.0, 7.0);
            let h = lcg.range(1.0, 2.0);
            let d = lcg.range(3.0, 7.0);

            let ox = lcg.range(-3.0, 3.0);
            let oz = lcg.range(-3.0, 3.0);

            let mesh_handle = meshes.add(Cuboid::new(w, h, d));

            let child = commands.spawn((
                Mesh3d(mesh_handle),
                MeshMaterial3d(cloud_mat.clone()),
                Transform::from_translation(Vec3::new(ox, 0.0, oz)),
            )).id();

            commands.entity(parent).add_child(child);
        }
    }
}

// ---- Drift system ------------------------------------------------------------

fn drift_clouds(
    time:      Res<Time>,
    wind:      Option<Res<WindState>>,
    mut query: Query<&mut Transform, With<CloudParent>>,
) {
    let dt = time.delta_secs();

    let (wind_dir, wind_speed) = if let Some(w) = wind {
        (w.direction.normalize_or_zero(), w.speed_mps)
    } else {
        (FALLBACK_WIND_DIR.normalize(), 3.0)
    };

    let delta = wind_dir * wind_speed * WIND_SPEED_FACTOR * dt;

    for mut transform in &mut query {
        transform.translation += delta;
    }
}

// ---- Wrap system -------------------------------------------------------------

fn wrap_clouds(
    mut query: Query<&mut Transform, With<CloudParent>>,
) {
    for mut transform in &mut query {
        if transform.translation.x > WRAP_LIMIT {
            transform.translation.x = -WRAP_LIMIT;
        } else if transform.translation.x < -WRAP_LIMIT {
            transform.translation.x = WRAP_LIMIT;
        }

        if transform.translation.z > WRAP_LIMIT {
            transform.translation.z = -WRAP_LIMIT;
        } else if transform.translation.z < -WRAP_LIMIT {
            transform.translation.z = WRAP_LIMIT;
        }
    }
}
