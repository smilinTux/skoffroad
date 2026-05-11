// Sponsor billboard scatter.
//
// Reads the ActiveBrandPack at Startup and scatters sponsor-textured roadside
// billboards across the terrain. Placement mirrors scatter.rs's grid scheme
// but uses a separate Perlin salt so sponsors don't collocate with trees or
// rocks. Honors ScatterCfg.density / min_distance_m / slope_band.
//
// For v1, we reuse the box-geometry billboard from billboards.rs (post + panel
// + frame strips + decorative stripes), tinted by each billboard's
// fallback_color. Texture loading + decals follow once asset_manifest.rs is
// wired in.
//
// Public API:
//   SponsorScatterPlugin
//   SponsorBillboard      (component on each placed entity)
//   SponsorAnalytics      (resource — impressions + click counts)

use std::collections::HashMap;

use bevy::prelude::*;
use noise::{NoiseFn, Perlin};

use crate::brand_pack::{ActiveBrandPack, BrandBillboard};
use crate::terrain::{terrain_height_at, TERRAIN_SEED};

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct SponsorScatterPlugin;

impl Plugin for SponsorScatterPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(SponsorAnalytics::default())
            .add_systems(Startup, spawn_sponsors);
    }
}

// ---------------------------------------------------------------------------
// Components / resources
// ---------------------------------------------------------------------------

#[derive(Component, Debug, Clone)]
pub struct SponsorBillboard {
    pub pack_id: String,
    pub billboard_idx: usize,
    pub click_url: String,
}

#[derive(Resource, Default, Debug, Clone)]
pub struct SponsorAnalytics {
    pub impressions: HashMap<String, u32>,
    pub clicks: HashMap<String, u32>,
}

// ---------------------------------------------------------------------------
// Geometry constants — match billboards.rs for visual consistency
// ---------------------------------------------------------------------------

const POST_W: f32 = 0.4;
const POST_H: f32 = 6.0;
const POST_D: f32 = 0.4;

const PANEL_W: f32 = 6.0;
const PANEL_H: f32 = 3.0;
const PANEL_D: f32 = 0.2;

const PANEL_CENTER_Y: f32 = POST_H / 2.0 + PANEL_H / 2.0;

// ---------------------------------------------------------------------------
// Scatter grid (same scheme as scatter.rs)
// ---------------------------------------------------------------------------

const GRID_CELLS: usize = 50;
const WORLD_SIZE: f32 = 200.0;
const CELL_SIZE: f32 = WORLD_SIZE / GRID_CELLS as f32;
const JITTER: f32 = CELL_SIZE * 0.4;

const SLOPE_STEP: f32 = 1.0;
const SPONSOR_SALT: u32 = 0xB1B0_0042;

fn compute_slope(x: f32, z: f32) -> f32 {
    let h  = terrain_height_at(x, z);
    let hx = terrain_height_at(x + SLOPE_STEP, z);
    let hz = terrain_height_at(x, z + SLOPE_STEP);
    let nx_v = Vec3::new(SLOPE_STEP, hx - h, 0.0).normalize();
    let nz_v = Vec3::new(0.0, hz - h, SLOPE_STEP).normalize();
    let n = nx_v.cross(nz_v).normalize();
    1.0 - n.dot(Vec3::Y).abs().clamp(0.0, 1.0)
}

#[inline]
fn hash2(a: i32, b: i32, salt: u32) -> f32 {
    let mut v = (a.wrapping_mul(374761393))
        .wrapping_add(b.wrapping_mul(668265263))
        .wrapping_add(salt as i32);
    v ^= v >> 13;
    v = v.wrapping_mul(1274126177);
    v ^= v >> 16;
    (v as u32) as f32 / u32::MAX as f32
}

// ---------------------------------------------------------------------------
// Spawn
// ---------------------------------------------------------------------------

fn spawn_sponsors(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    active: Res<ActiveBrandPack>,
) {
    let Some(pack) = active.pack.as_ref() else {
        info!("sponsor_scatter: no active brand pack, skipping");
        return;
    };
    if pack.billboards.is_empty() {
        info!("sponsor_scatter: brand pack \"{}\" has no billboards", pack.id);
        return;
    }

    let perlin = Perlin::new(TERRAIN_SEED.wrapping_add(SPONSOR_SALT));
    let density_threshold = 1.0 - pack.scatter.density.clamp(0.0, 1.0);
    let slope_lo = pack.scatter.slope_band[0];
    let slope_hi = pack.scatter.slope_band[1];
    let min_dist_sq = pack.scatter.min_distance_m.powi(2);

    let mesh_post  = meshes.add(Cuboid::new(POST_W,  POST_H,  POST_D));
    let mesh_panel = meshes.add(Cuboid::new(PANEL_W, PANEL_H, PANEL_D));
    let mat_post   = materials.add(StandardMaterial {
        base_color: Color::srgb(0.20, 0.18, 0.16),
        ..default()
    });

    let mut placed: Vec<Vec3> = Vec::new();
    let mut spawned_count = 0;

    for i in 0..GRID_CELLS {
        for j in 0..GRID_CELLS {
            // Centre of the cell in world space (terrain is centred on origin)
            let cx = (i as f32 + 0.5) * CELL_SIZE - WORLD_SIZE * 0.5;
            let cz = (j as f32 + 0.5) * CELL_SIZE - WORLD_SIZE * 0.5;

            // Per-cell Perlin sample for sparse selection
            let n = perlin.get([cx as f64 * 0.05, cz as f64 * 0.05]);
            // Map Perlin's [-1, 1] to [0, 1] for thresholding
            let n01 = (n as f32 * 0.5) + 0.5;
            if n01 < density_threshold { continue; }

            // Jitter within the cell
            let jx = (hash2(i as i32, j as i32, SPONSOR_SALT) - 0.5) * 2.0 * JITTER;
            let jz = (hash2(j as i32, i as i32, SPONSOR_SALT ^ 0x55AA) - 0.5) * 2.0 * JITTER;
            let x = cx + jx;
            let z = cz + jz;

            // Slope eligibility
            let slope = compute_slope(x, z);
            if slope < slope_lo || slope > slope_hi { continue; }

            // Min-distance test against previously placed sponsors
            let y_ground = terrain_height_at(x, z);
            let pos = Vec3::new(x, y_ground, z);
            if placed.iter().any(|p| p.distance_squared(pos) < min_dist_sq) { continue; }

            // Pick a billboard from the pack (weighted)
            let bb_idx = pick_billboard(&pack.billboards, hash2(i as i32, j as i32, 0xDEAD_BEEF));
            let bb = &pack.billboards[bb_idx];

            let panel_color = Color::srgb(
                bb.fallback_color[0], bb.fallback_color[1], bb.fallback_color[2],
            );
            let mat_panel = materials.add(StandardMaterial {
                base_color: panel_color,
                ..default()
            });

            // Face away from origin so a player heading outward sees the panel.
            let look_yaw = (-x).atan2(-z);

            commands
                .spawn((
                    SponsorBillboard {
                        pack_id: pack.id.clone(),
                        billboard_idx: bb_idx,
                        click_url: bb.click_url.clone(),
                    },
                    Transform::from_translation(pos)
                        .with_rotation(Quat::from_rotation_y(look_yaw)),
                    Visibility::Inherited,
                    Name::new(format!("Sponsor[{}#{}]", pack.id, bb_idx)),
                ))
                .with_children(|parent| {
                    parent.spawn((
                        Mesh3d(mesh_post.clone()),
                        MeshMaterial3d(mat_post.clone()),
                        Transform::from_xyz(0.0, POST_H * 0.5, 0.0),
                    ));
                    parent.spawn((
                        Mesh3d(mesh_panel.clone()),
                        MeshMaterial3d(mat_panel.clone()),
                        Transform::from_xyz(0.0, PANEL_CENTER_Y, 0.0),
                    ));
                });

            placed.push(pos);
            spawned_count += 1;
        }
    }

    info!("sponsor_scatter: spawned {} sponsors from pack \"{}\"",
        spawned_count, pack.id);
}

/// Weighted pick over the billboards array. `r` is in [0, 1).
fn pick_billboard(billboards: &[BrandBillboard], r: f32) -> usize {
    let total: f32 = billboards.iter().map(|b| b.weight.max(0.0)).sum();
    if total <= 0.0 { return 0; }
    let target = r.clamp(0.0, 0.9999) * total;
    let mut acc = 0.0;
    for (idx, b) in billboards.iter().enumerate() {
        acc += b.weight.max(0.0);
        if target < acc { return idx; }
    }
    billboards.len() - 1
}
