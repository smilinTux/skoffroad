// Sunrise / sunset horizon gradient band.
//
// Spawns a large flat ring mesh around the scene origin at a low altitude that
// serves as a warm glow band near the horizon during golden hour. The ring
// fades in and out based on the TimeOfDay resource and is tinted orange-pink
// at dawn/dusk, invisible at noon and night. It complements sky.rs (which
// drives the sky dome colour) without touching sky.rs itself.
//
// Implementation:
//   A torus-like flat ring built from a triangle fan: inner radius 400 m,
//   outer radius 900 m (just inside the sky dome at 900 m), height = 1 m.
//   AlphaMode::Blend + unlit material. The mesh is entirely procedural.
//   Headless-safe: all work is CPU-only mesh + material creation.
//
// Public API:
//   SunsetGradientPlugin

use std::f32::consts::PI;

use bevy::{
    asset::RenderAssetUsages,
    mesh::{Indices, PrimitiveTopology},
    prelude::*,
};

use crate::sky::TimeOfDay;

// ---- Plugin -----------------------------------------------------------------

pub struct SunsetGradientPlugin;

impl Plugin for SunsetGradientPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_horizon_band)
           .add_systems(Update, update_horizon_band);
    }
}

// ---- Component --------------------------------------------------------------

#[derive(Component)]
struct HorizonBand;

// ---- Constants --------------------------------------------------------------

/// Inner radius of the horizon ring (m). Should be well beyond the playfield.
const INNER_R: f32 = 400.0;
/// Outer radius — just inside the sky dome (sky dome radius = 900 m).
const OUTER_R: f32 = 880.0;
/// Y position: just at terrain level so it shows above the horizon line.
const BAND_Y:  f32 = 2.0;
/// Segments around the ring.
const SEGS:    u32 = 64;

// Colour at peak golden hour.
const PEAK_COLOR: [f32; 4] = [1.00, 0.48, 0.22, 0.55];
// Sun is above the horizon but still low (sin_el ~ 0.12): pale amber.
const LOW_COLOR:  [f32; 4] = [1.00, 0.70, 0.45, 0.20];

// Sun elevation thresholds (sin_el values):
//   Band is fully opaque between -0.06 and +0.06 (straddling the horizon).
//   Fades to zero beyond ±0.18 above and at/below the night threshold.
const FADE_IN_START:  f32 = -0.18; // begin appearing (pre-dawn / deep dusk)
const PEAK_START:     f32 = -0.06;
const PEAK_END:       f32 =  0.06;
const FADE_OUT_END:   f32 =  0.22;

// ---- Mesh builder -----------------------------------------------------------

fn build_ring_mesh() -> Mesh {
    // Flat ring in XZ plane (Y = 0). We'll position the entity at BAND_Y.
    let n = SEGS as usize;
    let mut positions: Vec<[f32; 3]> = Vec::with_capacity(n * 2);
    let mut normals:   Vec<[f32; 3]> = Vec::with_capacity(n * 2);
    let mut uvs:       Vec<[f32; 2]> = Vec::with_capacity(n * 2);
    let mut indices:   Vec<u32>      = Vec::new();

    for i in 0..n {
        let theta = 2.0 * PI * i as f32 / n as f32;
        let cos_t = theta.cos();
        let sin_t = theta.sin();

        // Inner vertex (UV u=0 = inner edge, fully transparent)
        positions.push([INNER_R * cos_t, 0.0, INNER_R * sin_t]);
        normals.push([0.0, 1.0, 0.0]);
        uvs.push([0.0, i as f32 / n as f32]);

        // Outer vertex (UV u=1 = outer edge, also fades to transparent)
        positions.push([OUTER_R * cos_t, 0.0, OUTER_R * sin_t]);
        normals.push([0.0, 1.0, 0.0]);
        uvs.push([1.0, i as f32 / n as f32]);
    }

    // Triangle strip wrapping around.
    for i in 0..n as u32 {
        let next = (i + 1) % n as u32;
        let a = i * 2;
        let b = i * 2 + 1;
        let c = next * 2;
        let d = next * 2 + 1;
        indices.extend_from_slice(&[a, b, c, b, d, c]);
    }

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL,   normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0,     uvs);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

// ---- Startup ----------------------------------------------------------------

fn spawn_horizon_band(
    mut commands:  Commands,
    mut meshes:    ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mat = materials.add(StandardMaterial {
        base_color: Color::srgba(0.0, 0.0, 0.0, 0.0), // fully transparent at start
        alpha_mode: AlphaMode::Blend,
        unlit:      true,
        cull_mode:  None, // visible from both above and below
        double_sided: true,
        ..default()
    });

    commands.spawn((
        HorizonBand,
        Mesh3d(meshes.add(build_ring_mesh())),
        MeshMaterial3d(mat),
        Transform::from_translation(Vec3::new(0.0, BAND_Y, 0.0)),
    ));
}

// ---- Update -----------------------------------------------------------------

fn update_horizon_band(
    tod:      Res<TimeOfDay>,
    band_q:   Query<&MeshMaterial3d<StandardMaterial>, With<HorizonBand>>,
    mut mats: ResMut<Assets<StandardMaterial>>,
) {
    let angle  = (tod.t - 0.25) * 2.0 * PI;
    let sin_el = angle.sin(); // 1 = noon, -1 = midnight

    // Compute blend factor: 0 outside active range, 1 in peak window.
    let alpha_scale = if sin_el < FADE_IN_START || sin_el > FADE_OUT_END {
        0.0_f32
    } else if sin_el < PEAK_START {
        // Fade in from FADE_IN_START to PEAK_START.
        smooth_step((sin_el - FADE_IN_START) / (PEAK_START - FADE_IN_START))
    } else if sin_el <= PEAK_END {
        1.0_f32
    } else {
        // Fade out from PEAK_END to FADE_OUT_END.
        smooth_step(1.0 - (sin_el - PEAK_END) / (FADE_OUT_END - PEAK_END))
    };

    if alpha_scale <= 0.001 {
        // Fully hidden — write transparent and return early.
        for mat_handle in &band_q {
            if let Some(mat) = mats.get_mut(mat_handle) {
                mat.base_color = Color::srgba(0.0, 0.0, 0.0, 0.0);
            }
        }
        return;
    }

    // Within the peak range, also tint: at sun_y = 0 it's PEAK, at 0.06 blend LOW.
    let inner_t = ((sin_el - PEAK_START) / (PEAK_END - PEAK_START)).clamp(0.0, 1.0);
    let r = PEAK_COLOR[0] + (LOW_COLOR[0] - PEAK_COLOR[0]) * inner_t;
    let g = PEAK_COLOR[1] + (LOW_COLOR[1] - PEAK_COLOR[1]) * inner_t;
    let b = PEAK_COLOR[2] + (LOW_COLOR[2] - PEAK_COLOR[2]) * inner_t;
    let a = (PEAK_COLOR[3] + (LOW_COLOR[3] - PEAK_COLOR[3]) * inner_t) * alpha_scale;

    let color = Color::srgba(r, g, b, a);

    for mat_handle in &band_q {
        if let Some(mat) = mats.get_mut(mat_handle) {
            mat.base_color = color;
        }
    }
}

// ---- Helpers ----------------------------------------------------------------

#[inline]
fn smooth_step(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}
