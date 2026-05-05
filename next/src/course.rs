// Sequential timed course — drives players through the four banner arches.
//
// Gate sequence: START (0) -> CKPT1 (1) -> CKPT2 (2) -> FINISH (3)
// Timer starts when the player crosses START and stops at FINISH.
// Re-crossing START after completion begins a fresh lap.

use bevy::prelude::*;

use crate::terrain::terrain_height_at;
use crate::vehicle::{Chassis, VehicleRoot};

// ---------------------------------------------------------------------------
// Public plugin
// ---------------------------------------------------------------------------

pub struct CoursePlugin;

impl Plugin for CoursePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CourseState>()
            .add_systems(Startup, (init_target, spawn_course_hud))
            .add_systems(
                Update,
                (advance_course, update_course_hud)
                    .run_if(resource_exists::<VehicleRoot>),
            );
    }
}

// ---------------------------------------------------------------------------
// Public resource (read by arrow, trail, pins, tutorial agents)
// ---------------------------------------------------------------------------

/// Public resource: current course target position + status.
/// Other agents (arrow, trail, pins, tutorial) read this.
/// Consumers that run before CoursePlugin is registered should use
/// `Option<Res<CourseState>>`.
#[derive(Resource, Default)]
pub struct CourseState {
    pub current_target: Option<Vec3>, // world position of next gate
    pub current_index: u32,           // 0..=3 (start, ckpt1, ckpt2, finish)
    pub run_time_s: f32,
    pub best_time_s: Option<f32>,
    pub completed: bool,
}

// ---------------------------------------------------------------------------
// Gate definitions (mirrored from banners.rs — cannot import consts there)
// ---------------------------------------------------------------------------

/// XZ positions of each arch in the same order as banners::ARCHES.
const GATES: [(f32, f32); 4] = [
    (5.0, -5.0),    // START  (idx 0)
    (40.0, 30.0),   // CKPT 1 (idx 1)
    (-40.0, 50.0),  // CKPT 2 (idx 2)
    (60.0, -40.0),  // FINISH (idx 3)
];

/// Radius (in XZ metres) within which a gate is considered "passed".
const GATE_RADIUS: f32 = 6.0;

/// Height offset above terrain where `current_target` Y is placed.
const TARGET_Y_OFFSET: f32 = 1.5;

/// Gate labels shown in the HUD.
const GATE_LABELS: [&str; 4] = ["START GATE", "CKPT 1", "CKPT 2", "FINISH"];

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

fn gate_world_pos(index: usize) -> Vec3 {
    let (x, z) = GATES[index];
    Vec3::new(x, terrain_height_at(x, z) + TARGET_Y_OFFSET, z)
}

// ---------------------------------------------------------------------------
// Startup systems
// ---------------------------------------------------------------------------

fn init_target(mut state: ResMut<CourseState>) {
    state.current_target = Some(gate_world_pos(0));
    state.current_index = 0;
}

// ---------------------------------------------------------------------------
// Per-frame course logic
// ---------------------------------------------------------------------------

fn advance_course(
    time: Res<Time>,
    vehicle: Option<Res<VehicleRoot>>,
    chassis_q: Query<&Transform, With<Chassis>>,
    mut state: ResMut<CourseState>,
) {
    let Some(vehicle) = vehicle else { return };
    let Ok(xform) = chassis_q.get(vehicle.chassis) else { return };

    let chassis_pos = xform.translation;
    let dt = time.delta_secs();

    // ----- Completed: only react to re-entering START gate ------------------
    if state.completed {
        let (sx, sz) = GATES[0];
        let dx = chassis_pos.x - sx;
        let dz = chassis_pos.z - sz;
        if (dx * dx + dz * dz).sqrt() < GATE_RADIUS {
            state.current_index = 1;
            state.run_time_s = 0.0;
            state.completed = false;
            state.current_target = Some(gate_world_pos(1));
            info!("course: new lap started — heading to CKPT 1");
        }
        return;
    }

    // ----- Timer (starts after crossing START, i.e. index >= 1) ------------
    if state.current_index >= 1 {
        state.run_time_s += dt;
    }

    // ----- Distance check to current target ---------------------------------
    let Some(target) = state.current_target else { return };
    let dx = chassis_pos.x - target.x;
    let dz = chassis_pos.z - target.z;
    let dist = (dx * dx + dz * dz).sqrt();

    if dist < GATE_RADIUS {
        if state.current_index == 3 {
            // Crossed FINISH
            state.completed = true;
            state.current_target = None;
            let t = state.run_time_s;
            state.best_time_s = Some(match state.best_time_s {
                Some(prev) => prev.min(t),
                None => t,
            });
            info!("course: complete in {:.2} s", t);
        } else {
            state.current_index += 1;
            state.current_target = Some(gate_world_pos(state.current_index as usize));
            info!(
                "course: gate {} passed — heading to {}",
                state.current_index - 1,
                GATE_LABELS[state.current_index as usize],
            );
        }
    }
}

// ---------------------------------------------------------------------------
// HUD components
// ---------------------------------------------------------------------------

#[derive(Component)]
struct CourseHudRoot;

#[derive(Component)]
enum CourseHudText {
    Objective,
    Timer,
}

// ---------------------------------------------------------------------------
// HUD spawn
// ---------------------------------------------------------------------------

fn spawn_course_hud(mut commands: Commands) {
    let bg = Color::srgba(0.05, 0.05, 0.07, 0.75);

    // Panel: top-centre, 280 px wide.
    // left: 50% + negative left margin achieves true horizontal centre.
    let panel = commands
        .spawn((
            CourseHudRoot,
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(14.0),
                left: Val::Percent(50.0),
                margin: UiRect {
                    left: Val::Px(-140.0),
                    ..default()
                },
                width: Val::Px(280.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                padding: UiRect::axes(Val::Px(10.0), Val::Px(8.0)),
                ..default()
            },
            BackgroundColor(bg),
        ))
        .id();

    let obj_text = commands
        .spawn((
            CourseHudText::Objective,
            Text::new("GO TO: START GATE"),
            TextFont {
                font_size: 15.0,
                ..default()
            },
            TextColor(Color::srgb(0.95, 0.95, 0.60)),
        ))
        .id();

    let timer_text = commands
        .spawn((
            CourseHudText::Timer,
            Text::new("00:00.00"),
            TextFont {
                font_size: 20.0,
                ..default()
            },
            TextColor(Color::WHITE),
        ))
        .id();

    commands.entity(panel).add_children(&[obj_text, timer_text]);
}

// ---------------------------------------------------------------------------
// HUD update
// ---------------------------------------------------------------------------

fn update_course_hud(
    state: Res<CourseState>,
    mut texts: Query<(&CourseHudText, &mut Text)>,
) {
    // Format time as MM:SS.cc
    let total = state.run_time_s;
    let mins = (total / 60.0) as u32;
    let secs = (total % 60.0) as u32;
    let cents = ((total % 1.0) * 100.0) as u32;
    let time_str = format!("{:02}:{:02}.{:02}", mins, secs, cents);

    let obj_str = if state.completed {
        "COMPLETE!".to_string()
    } else {
        format!("GO TO: {}", GATE_LABELS[state.current_index as usize])
    };

    for (kind, mut text) in texts.iter_mut() {
        match kind {
            CourseHudText::Objective => text.0 = obj_str.clone(),
            CourseHudText::Timer => text.0 = time_str.clone(),
        }
    }
}
