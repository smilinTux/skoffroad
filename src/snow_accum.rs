// Snow accumulation: two thin panel meshes (hood + roof) that grow taller as
// snow falls and shrink when the vehicle moves fast or drives through water.
//
// Sprint 70 — Effect 2
//
// Trigger:
//   Activated while SeasonState.current == Season::Winter (same gate as snow.rs).
//   Depth accumulates at 0.05/s.  Fast driving (> 15 m/s) blows it off at
//   0.20/s.  Driving through water resets depth to 0 instantly (checked via
//   a simple Y-position heuristic: if chassis Y ≤ water_level, it's in water).
//
// Public API:
//   SnowAccumPlugin
//
// Tuning knobs:
//   ACCUM_RATE      — depth gain per second while snowing
//   SHED_RATE       — depth loss per second when driving fast
//   FAST_THRESHOLD  — speed (m/s) above which shedding activates
//   WATER_LEVEL     — world Y below which the chassis is considered submerged
//   PANEL_MAX_Y     — max panel Y-scale (metres of snow depth, visual cap)

use bevy::prelude::*;
use avian3d::prelude::LinearVelocity;

use crate::season::{Season, SeasonState};
use crate::vehicle::{Chassis, VehicleRoot};

// ---- Public API ---------------------------------------------------------------

pub struct SnowAccumPlugin;

impl Plugin for SnowAccumPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<VehicleSnowDepth>()
           .add_systems(Update, (
               attach_snow_panels,
               update_snow_depth,
               apply_snow_panels,
           ));
    }
}

// ---- Resource -----------------------------------------------------------------

/// Tracks the accumulated snow depth on the vehicle, in [0.0, 1.0].
#[derive(Resource, Default)]
pub struct VehicleSnowDepth(pub f32);

// ---- Components ---------------------------------------------------------------

/// Marks the thin snow panel on the hood.
#[derive(Component)]
struct SnowPanelHood;

/// Marks the thin snow panel on the roof.
#[derive(Component)]
struct SnowPanelRoof;

// ---- Constants ---------------------------------------------------------------

const ACCUM_RATE: f32 = 0.05;       // depth/s while snowing
const SHED_RATE:  f32 = 0.20;       // depth/s while fast
const FAST_THRESHOLD: f32 = 15.0;   // m/s → shedding kicks in
const WATER_LEVEL:    f32 = 0.3;    // world Y (m) — below this = submerged

/// Maximum visual panel Y extension (metres).
const PANEL_MAX_Y: f32 = 0.06;

/// Panel base thickness (so it's never completely invisible at depth 0).
const PANEL_BASE_Y: f32 = 0.002;

// Chassis-local positions of the snow panels.
const HOOD_LOCAL_POS:  Vec3 = Vec3::new(0.0, 0.55, -1.2);
const ROOF_LOCAL_POS:  Vec3 = Vec3::new(0.0, 0.92,  0.2);

// Panel footprint half-extents (X, Z) — matches typical truck hood / roof.
const HOOD_W: f32 = 0.7;
const HOOD_D: f32 = 0.8;
const ROOF_W: f32 = 0.65;
const ROOF_D: f32 = 0.9;

// ---- System: attach panels once -----------------------------------------------

fn attach_snow_panels(
    mut commands:  Commands,
    mut meshes:    ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    vehicle:       Option<Res<VehicleRoot>>,
    chassis_q:     Query<Entity, With<Chassis>>,
    hood_q:        Query<(), With<SnowPanelHood>>,
    mut attached:  Local<bool>,
) {
    if *attached { return; }
    let Some(ref vr) = vehicle else { return };
    let Ok(chassis_entity) = chassis_q.get(vr.chassis) else { return };

    let mat = materials.add(StandardMaterial {
        base_color:   Color::srgba(0.92, 0.95, 1.0, 0.88),
        alpha_mode:   AlphaMode::Blend,
        unlit:        true,
        double_sided: false,
        cull_mode:    Some(bevy::render::render_resource::Face::Back),
        perceptual_roughness: 0.9,
        metallic: 0.0,
        ..default()
    });

    // Hood panel
    let hood_mesh = meshes.add(Cuboid::new(HOOD_W, PANEL_BASE_Y, HOOD_D));
    let hood = commands.spawn((
        SnowPanelHood,
        Mesh3d(hood_mesh),
        MeshMaterial3d(mat.clone()),
        Transform::from_translation(HOOD_LOCAL_POS),
    )).id();

    // Roof panel
    let roof_mesh = meshes.add(Cuboid::new(ROOF_W, PANEL_BASE_Y, ROOF_D));
    let roof = commands.spawn((
        SnowPanelRoof,
        Mesh3d(roof_mesh),
        MeshMaterial3d(mat),
        Transform::from_translation(ROOF_LOCAL_POS),
    )).id();

    commands.entity(chassis_entity).add_children(&[hood, roof]);

    *attached = true;
    let _ = hood_q; // suppress unused warning
}

// ---- System: update snow_depth resource --------------------------------------

fn update_snow_depth(
    time:      Res<Time>,
    season:    Option<Res<SeasonState>>,
    vehicle:   Option<Res<VehicleRoot>>,
    chassis_q: Query<(&Transform, &LinearVelocity), With<Chassis>>,
    mut depth: ResMut<VehicleSnowDepth>,
) {
    let is_winter = season
        .as_ref()
        .map(|s| s.current == Season::Winter)
        .unwrap_or(false);

    let dt = time.delta_secs();

    // Get chassis state.
    let (chassis_y, speed_mps) = if let Some(ref vr) = vehicle {
        if let Ok((tf, vel)) = chassis_q.get(vr.chassis) {
            let spd = Vec3::new(vel.x, 0.0, vel.z).length();
            (tf.translation.y, spd)
        } else {
            (0.0, 0.0)
        }
    } else {
        (0.0, 0.0)
    };

    // Submerged in water → instant reset.
    if chassis_y <= WATER_LEVEL {
        depth.0 = 0.0;
        return;
    }

    if is_winter {
        depth.0 += ACCUM_RATE * dt;
    }

    if speed_mps > FAST_THRESHOLD {
        depth.0 -= SHED_RATE * dt;
    }

    depth.0 = depth.0.clamp(0.0, 1.0);
}

// ---- System: apply depth to panel transforms ---------------------------------

fn apply_snow_panels(
    depth:     Res<VehicleSnowDepth>,
    mut hood_q: Query<&mut Transform, (With<SnowPanelHood>, Without<SnowPanelRoof>)>,
    mut roof_q: Query<&mut Transform, (With<SnowPanelRoof>, Without<SnowPanelHood>)>,
) {
    let panel_y = PANEL_BASE_Y + PANEL_MAX_Y * depth.0;

    for mut tf in &mut hood_q {
        tf.scale.y = panel_y / PANEL_BASE_Y;
    }
    for mut tf in &mut roof_q {
        tf.scale.y = panel_y / PANEL_BASE_Y;
    }
}
