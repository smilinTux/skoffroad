// Climb assist: applies subtle extra downforce on the chassis when the
// terrain under it is sloped > 15 deg, improving wheel-ground contact for
// uphill traction. Always-on by default; toggle with Shift+Y.
//
// Architecture:
//   update_slope  (Update)         — sample terrain, store slope in state
//   toggle_with_shift_y (Update)   — flip enabled flag
//   update_indicator    (Update)   — refresh HUD text/colour
//   apply_climb_downforce (PhysicsSchedule, before Solver) — push chassis down
//
// Public API:
//   ClimbAssistPlugin
//   ClimbAssistState (resource)

use bevy::prelude::*;
use avian3d::prelude::*;

use crate::terrain::terrain_height_at;
use crate::vehicle::{Chassis, VehicleRoot};

// ---- Constants ---------------------------------------------------------------

/// Slope threshold (degrees) below which no downforce is applied.
const SLOPE_THRESHOLD_DEG: f32 = 15.0;

/// Maximum downforce (N) applied when slope >= threshold + 30 deg.
const MAX_DOWNFORCE_N: f32 = 8000.0;

/// Finite-difference step (m) used to estimate terrain normal.
const FD_STEP: f32 = 0.5;

// ---- HUD layout --------------------------------------------------------------

const PANEL_W: f32   = 200.0;
/// top: 280 px — below the assists panel per spec.
const PANEL_TOP: f32 = 280.0;

// ---- Colors ------------------------------------------------------------------

const GREEN:  Color = Color::srgb(0.2, 0.95, 0.35);
const YELLOW: Color = Color::srgb(0.95, 0.85, 0.2);
const RED:    Color = Color::srgb(0.95, 0.25, 0.2);
const GREY:   Color = Color::srgb(0.5, 0.5, 0.5);

// ---- Public resource ---------------------------------------------------------

#[derive(Resource, Clone, Copy)]
pub struct ClimbAssistState {
    pub enabled: bool,
    /// Last sampled terrain slope directly under the chassis (degrees).
    pub last_slope_deg: f32,
}

impl Default for ClimbAssistState {
    fn default() -> Self {
        Self { enabled: true, last_slope_deg: 0.0 }
    }
}

// ---- HUD components ----------------------------------------------------------

#[derive(Component)] struct ClimbHudRoot;
#[derive(Component)] struct ClimbHudText;

// ---- Plugin ------------------------------------------------------------------

pub struct ClimbAssistPlugin;

impl Plugin for ClimbAssistPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ClimbAssistState>()
           .add_systems(Startup, spawn_climb_indicator)
           .add_systems(
               Update,
               (
                   toggle_with_shift_y,
                   update_slope
                       .run_if(resource_exists::<VehicleRoot>),
                   update_indicator,
               ),
           )
           .add_systems(
               PhysicsSchedule,
               apply_climb_downforce
                   .after(PhysicsStepSystems::NarrowPhase)
                   .before(PhysicsStepSystems::Solver),
           );
    }
}

// ---- Startup: spawn HUD indicator --------------------------------------------

fn spawn_climb_indicator(mut commands: Commands) {
    let root = commands.spawn((
        ClimbHudRoot,
        Node {
            position_type: PositionType::Absolute,
            right:         Val::Px(12.0),
            top:           Val::Px(PANEL_TOP),
            width:         Val::Px(PANEL_W),
            height:        Val::Px(30.0),
            align_items:   AlignItems::Center,
            padding: UiRect {
                left:   Val::Px(8.0),
                right:  Val::Px(8.0),
                top:    Val::Px(4.0),
                bottom: Val::Px(4.0),
            },
            ..default()
        },
        BackgroundColor(Color::srgba(0.05, 0.05, 0.07, 0.80)),
    )).id();

    let label = commands.spawn((
        ClimbHudText,
        Text::new("[CLIMB] ON  ( 0 deg)"),
        TextFont { font_size: 12.0, ..default() },
        TextColor(GREEN),
    )).id();

    commands.entity(root).add_child(label);
}

// ---- Helper: compute slope under a world-space XZ position -------------------

/// Returns slope in degrees at (x, z) via 3-point finite difference.
/// Sample center, center+FD_STEP in X, center+FD_STEP in Z.
/// Two tangent vectors are crossed to get the terrain normal.
fn slope_deg_at(x: f32, z: f32) -> f32 {
    let h_c  = terrain_height_at(x,           z          );
    let h_px = terrain_height_at(x + FD_STEP, z          );
    let h_pz = terrain_height_at(x,           z + FD_STEP);

    // Tangent along X axis: move FD_STEP in X, height changes by (h_px - h_c).
    let tx = Vec3::new(FD_STEP, h_px - h_c, 0.0);
    // Tangent along Z axis.
    let tz = Vec3::new(0.0,     h_pz - h_c, FD_STEP);

    // Normal = cross product (right-hand rule gives upward normal for typical terrain).
    let normal = tx.cross(tz).normalize_or_zero();

    // Slope = angle between terrain normal and world-up axis.
    // acos(|n.y|) so that inverted normals give the same reading.
    normal.y.abs().acos().to_degrees()
}

// ---- System: sample terrain slope each Update tick ---------------------------

fn update_slope(
    vehicle:       Res<VehicleRoot>,
    chassis_q:     Query<&Transform, With<Chassis>>,
    mut state:     ResMut<ClimbAssistState>,
) {
    let Ok(transform) = chassis_q.get(vehicle.chassis) else { return };
    let pos = transform.translation;
    state.last_slope_deg = slope_deg_at(pos.x, pos.z);
}

// ---- System: toggle Shift+Y --------------------------------------------------

fn toggle_with_shift_y(
    keys:      Res<ButtonInput<KeyCode>>,
    mut state: ResMut<ClimbAssistState>,
) {
    if keys.just_pressed(KeyCode::KeyY) && keys.pressed(KeyCode::ShiftLeft) {
        state.enabled = !state.enabled;
        info!("climb assist: {}", state.enabled);
    }
}

// ---- System: apply downforce (PhysicsSchedule) -------------------------------

fn apply_climb_downforce(
    state:         Res<ClimbAssistState>,
    vehicle:       Option<Res<VehicleRoot>>,
    mut chassis_q: Query<Forces, With<Chassis>>,
) {
    if !state.enabled { return; }

    let slope_deg = state.last_slope_deg;
    if slope_deg <= SLOPE_THRESHOLD_DEG { return; }

    let Some(vehicle) = vehicle else { return };
    let Ok(mut forces) = chassis_q.get_mut(vehicle.chassis) else { return };

    let slope_factor = ((slope_deg - SLOPE_THRESHOLD_DEG) / 30.0).clamp(0.0, 1.0);
    let downforce_n  = slope_factor * MAX_DOWNFORCE_N;
    forces.apply_force(Vec3::new(0.0, -downforce_n, 0.0));
}

// ---- System: update HUD indicator --------------------------------------------

fn update_indicator(
    state:      Res<ClimbAssistState>,
    mut text_q: Query<(&mut Text, &mut TextColor), With<ClimbHudText>>,
) {
    let slope = state.last_slope_deg;

    let color = if !state.enabled {
        GREY
    } else if slope < SLOPE_THRESHOLD_DEG {
        GREEN
    } else if slope < 30.0 {
        YELLOW
    } else {
        RED
    };

    let label = if state.enabled {
        format!("[CLIMB] ON  ({:.0} deg)", slope)
    } else {
        format!("[CLIMB] OFF ({:.0} deg)", slope)
    };

    for (mut text, mut fg) in &mut text_q {
        text.0 = label.clone();
        fg.0   = color;
    }
}
