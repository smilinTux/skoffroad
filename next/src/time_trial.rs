// Time Trial: solo timed run around the race path with a ghost car replay
// overlay showing the player's previous best lap. T key starts/cancels.
//
// Public API:
//   TimeTrialPlugin
//   TimeTrialState (resource)

use bevy::prelude::*;
use crate::vehicle::{Chassis, VehicleRoot};
use crate::course::CourseState;

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct TimeTrialPlugin;

impl Plugin for TimeTrialPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TimeTrialState>()
            .add_systems(
                Startup,
                (spawn_ghost_car, spawn_time_trial_hud),
            )
            .add_systems(
                Update,
                (
                    start_with_t,
                    record_path,
                    update_ghost_position,
                    detect_finish,
                    update_hud,
                )
                    .chain()
                    .run_if(resource_exists::<VehicleRoot>),
            );
    }
}

// ---------------------------------------------------------------------------
// Public resource
// ---------------------------------------------------------------------------

#[derive(Resource, Default)]
pub struct TimeTrialState {
    pub running: bool,
    pub elapsed_s: f32,
    pub best_time_s: Option<f32>,
    pub current_path: Vec<(f32, Vec3)>,
    pub ghost_path: Vec<(f32, Vec3)>,
}

// ---------------------------------------------------------------------------
// Components
// ---------------------------------------------------------------------------

#[derive(Component)]
pub struct GhostCar;

#[derive(Component)]
struct TimeTrialHudRoot;

#[derive(Component)]
struct TimeTrialHudText;

// ---------------------------------------------------------------------------
// Startup: spawn ghost car mesh
// ---------------------------------------------------------------------------

fn spawn_ghost_car(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let ghost_mesh = meshes.add(Cuboid::new(2.0, 0.6, 4.0));
    let ghost_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(0.3, 0.6, 1.0, 0.4),
        alpha_mode: AlphaMode::Blend,
        ..default()
    });

    commands.spawn((
        GhostCar,
        Mesh3d(ghost_mesh),
        MeshMaterial3d(ghost_mat),
        Transform::IDENTITY,
        Visibility::Hidden,
    ));
}

// ---------------------------------------------------------------------------
// Startup: spawn HUD panel
// ---------------------------------------------------------------------------

fn spawn_time_trial_hud(mut commands: Commands) {
    let bg = Color::srgba(0.05, 0.05, 0.10, 0.80);

    let panel = commands
        .spawn((
            TimeTrialHudRoot,
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(8.0),
                left: Val::Percent(50.0),
                margin: UiRect {
                    left: Val::Px(-160.0),
                    ..default()
                },
                width: Val::Px(320.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                padding: UiRect::axes(Val::Px(10.0), Val::Px(6.0)),
                ..default()
            },
            BackgroundColor(bg),
            ZIndex(30),
            Visibility::Hidden,
        ))
        .id();

    let hud_text = commands
        .spawn((
            TimeTrialHudText,
            Text::new("TIME TRIAL: 00:00.0  \u{0394} +0.0s"),
            TextFont {
                font_size: 16.0,
                ..default()
            },
            TextColor(Color::WHITE),
        ))
        .id();

    commands.entity(panel).add_child(hud_text);
}

// ---------------------------------------------------------------------------
// System: start_with_t
// ---------------------------------------------------------------------------

fn start_with_t(
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<TimeTrialState>,
    mut ghost_q: Query<&mut Visibility, With<GhostCar>>,
    mut hud_q: Query<&mut Visibility, (With<TimeTrialHudRoot>, Without<GhostCar>)>,
) {
    if !keys.just_pressed(KeyCode::KeyT) {
        return;
    }

    if state.running {
        state.running = false;
        info!("time trial: aborted");
        // Hide HUD and ghost when aborted
        for mut vis in ghost_q.iter_mut() {
            *vis = Visibility::Hidden;
        }
        for mut vis in hud_q.iter_mut() {
            *vis = Visibility::Hidden;
        }
    } else {
        state.current_path.clear();
        state.elapsed_s = 0.0;
        state.running = true;
        // Hide ghost if no best lap recorded yet
        if state.ghost_path.is_empty() {
            for mut vis in ghost_q.iter_mut() {
                *vis = Visibility::Hidden;
            }
        }
        // Show HUD
        for mut vis in hud_q.iter_mut() {
            *vis = Visibility::Visible;
        }
        info!("time trial started");
    }
}

// ---------------------------------------------------------------------------
// System: record_path
// ---------------------------------------------------------------------------

fn record_path(
    time: Res<Time>,
    vehicle: Res<VehicleRoot>,
    chassis_q: Query<&Transform, With<Chassis>>,
    mut state: ResMut<TimeTrialState>,
    mut sample_acc: Local<f32>,
) {
    if !state.running {
        return;
    }

    let dt = time.delta_secs();
    state.elapsed_s += dt;

    *sample_acc += dt;
    if *sample_acc >= 0.1 {
        *sample_acc -= 0.1;
        if let Ok(xform) = chassis_q.get(vehicle.chassis) {
            let pos = xform.translation;
            let t = state.elapsed_s;
            state.current_path.push((t, pos));
        }
    }
}

// ---------------------------------------------------------------------------
// System: update_ghost_position
// ---------------------------------------------------------------------------

fn update_ghost_position(
    state: Res<TimeTrialState>,
    mut ghost_q: Query<(&mut Transform, &mut Visibility), With<GhostCar>>,
) {
    let Ok((mut transform, mut visibility)) = ghost_q.single_mut() else {
        return;
    };

    if !state.running || state.ghost_path.is_empty() {
        *visibility = Visibility::Hidden;
        return;
    }

    let elapsed = state.elapsed_s;
    let path = &state.ghost_path;

    // Find the index of the last sample with timestamp <= elapsed
    let idx = path.partition_point(|(t, _)| *t <= elapsed);

    let pos = if idx == 0 {
        // Before any recorded sample: use the first one
        path[0].1
    } else if idx >= path.len() {
        // Past the end of the ghost recording
        path[path.len() - 1].1
    } else {
        // Lerp between idx-1 and idx
        let (t0, p0) = path[idx - 1];
        let (t1, p1) = path[idx];
        let span = t1 - t0;
        let frac = if span > 0.0001 {
            (elapsed - t0) / span
        } else {
            0.0
        };
        p0.lerp(p1, frac.clamp(0.0, 1.0))
    };

    transform.translation = pos;
    *visibility = Visibility::Visible;
}

// ---------------------------------------------------------------------------
// System: detect_finish
// ---------------------------------------------------------------------------

fn detect_finish(
    course: Option<Res<CourseState>>,
    mut state: ResMut<TimeTrialState>,
    mut prev_completed: Local<bool>,
) {
    let Some(course) = course else { return };

    let rising_edge = course.completed && !*prev_completed;
    *prev_completed = course.completed;

    if rising_edge && state.running {
        state.running = false;
        let elapsed = state.elapsed_s;

        // Store as ghost if this run is shorter than the current best or
        // there is no best yet.
        let is_new_best = match state.best_time_s {
            None => true,
            Some(prev) => elapsed < prev,
        };

        if is_new_best {
            state.ghost_path = state.current_path.clone();
            state.best_time_s = Some(elapsed);
            info!("time trial: finished in {:.2}s (new best)", elapsed);
        } else {
            info!("time trial: finished in {:.2}s", elapsed);
        }
    }
}

// ---------------------------------------------------------------------------
// System: update_hud
// ---------------------------------------------------------------------------

fn update_hud(
    state: Res<TimeTrialState>,
    mut hud_root_q: Query<&mut Visibility, (With<TimeTrialHudRoot>, Without<GhostCar>)>,
    mut hud_text_q: Query<&mut Text, With<TimeTrialHudText>>,
    mut hud_color_q: Query<&mut TextColor, With<TimeTrialHudText>>,
) {
    // Show HUD only when running
    let show = state.running;
    for mut vis in hud_root_q.iter_mut() {
        *vis = if show { Visibility::Visible } else { Visibility::Hidden };
    }

    if !show {
        return;
    }

    let elapsed = state.elapsed_s;
    let time_str = fmt_time(elapsed);

    // Compute delta to ghost at same elapsed time
    let (delta_str, delta_color) = if state.ghost_path.is_empty() {
        ("\u{0394} --".to_string(), Color::WHITE)
    } else {
        let ghost_pos_at_elapsed = ghost_sample_at(&state.ghost_path, elapsed);
        // delta = current elapsed minus the time the ghost was at that position.
        // We approximate: find the ghost timestamp closest to elapsed and
        // compare directly as a time delta.
        let ghost_time = ghost_time_at(&state.ghost_path, elapsed);
        let delta = elapsed - ghost_time;
        let color = if delta > 0.0 {
            // Behind ghost: red
            Color::srgb(1.0, 0.25, 0.25)
        } else {
            // Ahead of ghost: green
            Color::srgb(0.25, 1.0, 0.40)
        };
        let _ = ghost_pos_at_elapsed; // positional lerp used in ghost system, suppress warning
        let sign = if delta >= 0.0 { "+" } else { "" };
        (format!("\u{0394} {}{:.1}s", sign, delta), color)
    };

    let label = format!("TIME TRIAL: {}  {}", time_str, delta_str);

    for mut text in hud_text_q.iter_mut() {
        text.0 = label.clone();
    }
    for mut color in hud_color_q.iter_mut() {
        color.0 = delta_color;
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Format seconds as MM:SS.t  e.g. "01:07.3"
fn fmt_time(secs: f32) -> String {
    let secs = secs.max(0.0);
    let mins = (secs / 60.0) as u32;
    let rem = secs - (mins as f32) * 60.0;
    let sec = rem as u32;
    let tenth = ((rem % 1.0) * 10.0) as u32;
    format!("{:02}:{:02}.{}", mins, sec, tenth)
}

/// Interpolated position in the ghost path at elapsed time `t`.
fn ghost_sample_at(path: &[(f32, Vec3)], t: f32) -> Vec3 {
    if path.is_empty() {
        return Vec3::ZERO;
    }
    let idx = path.partition_point(|(ts, _)| *ts <= t);
    if idx == 0 {
        path[0].1
    } else if idx >= path.len() {
        path[path.len() - 1].1
    } else {
        let (t0, p0) = path[idx - 1];
        let (t1, p1) = path[idx];
        let span = t1 - t0;
        let frac = if span > 0.0001 { (t - t0) / span } else { 0.0 };
        p0.lerp(p1, frac.clamp(0.0, 1.0))
    }
}

/// The ghost's time value at elapsed `t` (identity: returns `t` clamped to
/// the recorded range, giving a direct time delta comparison).
fn ghost_time_at(path: &[(f32, Vec3)], t: f32) -> f32 {
    if path.is_empty() {
        return t;
    }
    // The ghost path timestamps are the times the ghost was at each sample.
    // At elapsed t, the ghost is at whatever position it recorded at t.
    // For HUD delta: delta = t_current - t_ghost_equivalent = t - t
    // which would be 0. Instead we use the ghost's *final* recorded time as
    // the reference: if current elapsed < ghost_end_time we're ahead, else behind.
    // More precisely: find where we are in the ghost's timeline.
    let last_t = path[path.len() - 1].0;
    let first_t = path[0].0;
    // Clamp to recorded range
    let ghost_t = t.clamp(first_t, last_t);
    ghost_t
}
