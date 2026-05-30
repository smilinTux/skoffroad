// Weather clouds: layered cumulus cloud meshes (composed of stacked sphere
// puffs at varied altitudes and scales). They slowly drift in the wind
// direction and tint their colour by time of day (warm at golden hour,
// cool/grey at midday, dark at night). Cloud count is gated by GraphicsQuality.
//
// Public API:
//   WeatherCloudsPlugin

use std::f32::consts::PI;

use bevy::prelude::*;

use crate::graphics_quality::GraphicsQuality;
use crate::sky::TimeOfDay;
use crate::wind::WindState;

// ---- Constants ---------------------------------------------------------------

/// Cloud counts per quality tier.
const CLOUD_COUNT_HIGH:   usize = 14;
const CLOUD_COUNT_MEDIUM: usize = 9;
const CLOUD_COUNT_LOW:    usize = 4;

/// Clouds move at this fraction of wind speed (dimensionless scale factor).
const WIND_SPEED_FACTOR: f32 = 0.3;

/// Fallback wind direction when WindState is absent.
const FALLBACK_WIND_DIR: Vec3 = Vec3::new(0.5, 0.0, -0.866);

/// Clouds wrap when their X or Z drifts outside this limit.
const WRAP_LIMIT: f32 = 250.0;

// Time-of-day cloud colour palette (RGBA).
/// Golden hour: warm orange-white.
const COLOR_GOLDEN: [f32; 4] = [1.00, 0.82, 0.60, 0.88];
/// Midday: bright cool white.
const COLOR_DAY:    [f32; 4] = [0.96, 0.96, 0.98, 0.82];
/// Dusk/dawn band just at the horizon: deeper warm pink.
const COLOR_DUSK:   [f32; 4] = [0.90, 0.65, 0.50, 0.80];
/// Night: dark blue-grey.
const COLOR_NIGHT:  [f32; 4] = [0.18, 0.20, 0.28, 0.75];

// ---- Plugin ------------------------------------------------------------------

pub struct WeatherCloudsPlugin;

impl Plugin for WeatherCloudsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_clouds)
           .add_systems(Update, (drift_clouds, wrap_clouds, tint_clouds_by_tod));
    }
}

// ---- Components --------------------------------------------------------------

/// Marks a cloud parent entity.
#[derive(Component)]
pub struct CloudParent;

/// Marks each puff child mesh so the tint system can find them.
#[derive(Component)]
struct CloudPuff;

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

// ---- Colour helpers ----------------------------------------------------------

#[inline]
fn lerp4(a: [f32; 4], b: [f32; 4], t: f32) -> [f32; 4] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
        a[3] + (b[3] - a[3]) * t,
    ]
}

#[inline]
fn smooth_step(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Compute the cloud tint colour for the current TimeOfDay.
fn cloud_color_for_tod(tod: &TimeOfDay) -> Color {
    let angle  = (tod.t - 0.25) * 2.0 * PI;
    let sin_el = angle.sin();      // 1 at noon, -1 at midnight
    let above  = sin_el.max(0.0); // 0 when at/below horizon

    // golden factor: 1 when sun near horizon (|sin_el| < 0.18).
    let golden = smooth_step((1.0 - (sin_el.abs() / 0.18).min(1.0)).max(0.0));

    // 1. Lerp night -> day by sun elevation.
    let base = lerp4(COLOR_NIGHT, COLOR_DAY, smooth_step(above));
    // 2. Blend toward golden at low sun angles.
    let with_golden = lerp4(base, COLOR_GOLDEN, golden);
    // 3. Extra dusk/dawn tint when sun is just below the horizon.
    let dusk_t = smooth_step(
        ((-sin_el).clamp(0.0, 0.08) / 0.08).min(1.0)
    );
    let final_c = lerp4(with_golden, COLOR_DUSK, dusk_t);

    Color::srgba(final_c[0], final_c[1], final_c[2], final_c[3])
}

// ---- Spawn system ------------------------------------------------------------

fn spawn_clouds(
    mut commands:  Commands,
    mut meshes:    ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    quality:       Res<GraphicsQuality>,
) {
    let cloud_count = match *quality {
        GraphicsQuality::Low    => CLOUD_COUNT_LOW,
        GraphicsQuality::Medium => CLOUD_COUNT_MEDIUM,
        GraphicsQuality::High   => CLOUD_COUNT_HIGH,
    };

    // Start with midday colour; tint_clouds_by_tod updates every frame.
    let start_color = Color::srgba(
        COLOR_DAY[0], COLOR_DAY[1], COLOR_DAY[2], COLOR_DAY[3],
    );

    let cloud_mat = materials.add(StandardMaterial {
        base_color: start_color,
        alpha_mode: AlphaMode::Blend,
        unlit:      true,
        ..default()
    });

    let mut lcg = Lcg::new(11);

    for i in 0..cloud_count {
        // Three altitude bands for layered depth:
        //   band 0 - low cumulus  (55-75 m)
        //   band 1 - mid alto     (80-110 m)
        //   band 2 - high cirrus  (120-160 m)
        let band = i % 3;
        let (py_lo, py_hi): (f32, f32) = match band {
            0 => (55.0, 75.0),
            1 => (80.0, 110.0),
            _ => (120.0, 160.0),
        };

        let px = lcg.range(-200.0, 200.0);
        let py = lcg.range(py_lo, py_hi);
        let pz = lcg.range(-200.0, 200.0);

        // High-altitude clouds are wider and flatter.
        let scale_xz = lcg.range(0.8, 1.8);
        let scale_y: f32 = match band {
            2 => lcg.range(0.3, 0.6), // cirrus: flat
            _ => lcg.range(0.6, 1.2),
        };

        let parent = commands.spawn((
            CloudParent,
            Transform {
                translation: Vec3::new(px, py, pz),
                scale: Vec3::new(scale_xz, scale_y, scale_xz),
                ..default()
            },
            Visibility::default(),
        )).id();

        // 4-8 sphere puffs per cloud; cap per tier.
        let max_puffs: usize = match *quality {
            GraphicsQuality::Low    => 4,
            GraphicsQuality::Medium => 6,
            GraphicsQuality::High   => 8,
        };
        let extra = (lcg.next_f32() * (max_puffs - 4 + 1) as f32) as usize;
        let puff_count = 4 + extra;

        for _ in 0..puff_count {
            let radius = lcg.range(2.5, 6.0);
            let ox = lcg.range(-5.0, 5.0);
            let oy = lcg.range(-0.5, 1.5); // slight vertical stack
            let oz = lcg.range(-4.0, 4.0);

            let mesh_handle = meshes.add(Sphere::new(radius).mesh().ico(1).unwrap());

            let child = commands.spawn((
                CloudPuff,
                Mesh3d(mesh_handle),
                MeshMaterial3d(cloud_mat.clone()),
                Transform::from_translation(Vec3::new(ox, oy, oz)),
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

// ---- Time-of-day tint system -------------------------------------------------

/// Re-colour every cloud puff material each frame based on the time of day.
fn tint_clouds_by_tod(
    tod:      Res<TimeOfDay>,
    puff_q:   Query<&MeshMaterial3d<StandardMaterial>, With<CloudPuff>>,
    mut mats: ResMut<Assets<StandardMaterial>>,
) {
    let color = cloud_color_for_tod(&tod);
    for mat_handle in &puff_q {
        if let Some(mat) = mats.get_mut(mat_handle) {
            mat.base_color = color;
        }
    }
}
