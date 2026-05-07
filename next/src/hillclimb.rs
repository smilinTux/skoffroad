// Hillclimb mode: state machine + timer + HUD. K key toggles.
// Tracks elevation gained, % grade, time. Persists best time.
//
// Public API:
//   HillclimbPlugin
//   HillclimbState (resource)
//   HillclimbPhase enum

use bevy::prelude::*;
use std::fs;
use std::io::Write as IoWrite;
use std::path::PathBuf;

use crate::notifications::NotificationQueue;
use crate::terrain::terrain_height_at;
use crate::vehicle::{Chassis, VehicleRoot};

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

pub struct HillclimbPlugin;

impl Plugin for HillclimbPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HillclimbState>()
            .add_systems(Startup, (load_best_time, spawn_hillclimb_hud).chain())
            .add_systems(
                Update,
                (
                    toggle_with_k,
                    tick_hillclimb,
                    detect_finish,
                    update_hud,
                    save_on_finish,
                )
                    .chain()
                    .run_if(resource_exists::<VehicleRoot>),
            );
    }
}

#[derive(Resource, Default, Clone)]
pub struct HillclimbState {
    pub phase: HillclimbPhase,
    pub elapsed_s: f32,
    pub start_elevation: f32,
    pub current_elevation: f32,
    pub best_time_s: Option<f32>,
    /// True for exactly one frame after a finish so save_on_finish can act.
    just_finished: bool,
}

#[derive(Default, Clone, Copy, PartialEq, Eq, Debug)]
pub enum HillclimbPhase {
    #[default]
    Idle,
    Active,
    Finished,
}

// ---------------------------------------------------------------------------
// HUD components
// ---------------------------------------------------------------------------

#[derive(Component)]
struct HillclimbHudRoot;

#[derive(Component)]
struct HillclimbHudTitle;

#[derive(Component)]
struct HillclimbHudTime;

#[derive(Component)]
struct HillclimbHudElev;

#[derive(Component)]
struct HillclimbHudBest;

// ---------------------------------------------------------------------------
// Persistence helpers
// ---------------------------------------------------------------------------

fn hillclimb_save_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    let mut p = PathBuf::from(home);
    p.push(".sandk-offroad");
    p.push("hillclimb.json");
    p
}

fn read_best_time() -> Option<f32> {
    let text = fs::read_to_string(hillclimb_save_path()).ok()?;
    let v: serde_json::Value = serde_json::from_str(&text).ok()?;
    let best = v.as_object()?.get("best_time_s")?.as_f64()?;
    Some(best as f32)
}

fn write_best_time(secs: f32) {
    let path = hillclimb_save_path();
    if let Some(parent) = path.parent() {
        if let Err(e) = fs::create_dir_all(parent) {
            warn!("hillclimb: could not create dir {}: {}", parent.display(), e);
            return;
        }
    }
    let json = format!("{{\"best_time_s\": {:.4}}}", secs);
    match fs::File::create(&path) {
        Err(e) => warn!("hillclimb: could not open {} for writing: {}", path.display(), e),
        Ok(mut f) => {
            if let Err(e) = f.write_all(json.as_bytes()) {
                warn!("hillclimb: write failed: {}", e);
            } else {
                info!("hillclimb: saved best time {:.2}s to {}", secs, path.display());
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Startup: load best time
// ---------------------------------------------------------------------------

fn load_best_time(mut state: ResMut<HillclimbState>) {
    if let Some(best) = read_best_time() {
        state.best_time_s = Some(best);
        info!("hillclimb: loaded best time {:.2}s", best);
    } else {
        info!("hillclimb: no saved best time found, starting fresh");
    }
}

// ---------------------------------------------------------------------------
// Startup: spawn HUD panel (hidden by default, top-center 280x60)
// ---------------------------------------------------------------------------

fn spawn_hillclimb_hud(mut commands: Commands) {
    let bg = Color::srgba(0.04, 0.06, 0.12, 0.88);

    let panel = commands
        .spawn((
            HillclimbHudRoot,
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(8.0),
                // Center: offset left by half panel width
                left: Val::Percent(50.0),
                margin: UiRect {
                    left: Val::Px(-140.0),
                    ..default()
                },
                width: Val::Px(280.0),
                min_height: Val::Px(60.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                padding: UiRect::axes(Val::Px(10.0), Val::Px(6.0)),
                row_gap: Val::Px(2.0),
                ..default()
            },
            BackgroundColor(bg),
            ZIndex(40),
            Visibility::Hidden,
        ))
        .id();

    // Row 1: title
    let title = commands
        .spawn((
            HillclimbHudTitle,
            Text::new("HILLCLIMB"),
            TextFont {
                font_size: 13.0,
                ..default()
            },
            TextColor(Color::srgb(1.0, 0.75, 0.15)),
        ))
        .id();

    // Row 2: time MM:SS.cc
    let time_text = commands
        .spawn((
            HillclimbHudTime,
            Text::new("00:00.00"),
            TextFont {
                font_size: 22.0,
                ..default()
            },
            TextColor(Color::WHITE),
        ))
        .id();

    // Row 3: elevation gained
    let elev_text = commands
        .spawn((
            HillclimbHudElev,
            Text::new("Elev: 0.0 m"),
            TextFont {
                font_size: 13.0,
                ..default()
            },
            TextColor(Color::srgb(0.7, 0.9, 1.0)),
        ))
        .id();

    // Row 4: best time (small)
    let best_text = commands
        .spawn((
            HillclimbHudBest,
            Text::new("Best: --"),
            TextFont {
                font_size: 11.0,
                ..default()
            },
            TextColor(Color::srgb(0.6, 0.6, 0.6)),
        ))
        .id();

    commands
        .entity(panel)
        .add_children(&[title, time_text, elev_text, best_text]);
}

// ---------------------------------------------------------------------------
// System: toggle_with_k
// ---------------------------------------------------------------------------

fn toggle_with_k(
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<HillclimbState>,
    vehicle: Res<VehicleRoot>,
    mut chassis_q: Query<&mut Transform, With<Chassis>>,
    mut notifs: ResMut<NotificationQueue>,
) {
    if !keys.just_pressed(KeyCode::KeyK) {
        return;
    }

    match state.phase {
        HillclimbPhase::Idle => {
            // Teleport chassis to track start: (-150, terrain_y + 1, -150)
            let start_x = -150.0_f32;
            let start_z = -150.0_f32;
            let terrain_y = terrain_height_at(start_x, start_z);
            let start_y = terrain_y + 1.0;

            if let Ok(mut xform) = chassis_q.get_mut(vehicle.chassis) {
                xform.translation = Vec3::new(start_x, start_y, start_z);
                // Face roughly up the slope (positive x direction)
                xform.rotation = Quat::IDENTITY;
            }

            state.phase = HillclimbPhase::Active;
            state.elapsed_s = 0.0;
            state.start_elevation = start_y;
            state.current_elevation = start_y;
            state.just_finished = false;

            notifs.push("HILLCLIMB STARTED", Color::srgb(1.0, 0.75, 0.15));
            info!("hillclimb: started at ({}, {}, {})", start_x, start_y, start_z);
        }

        HillclimbPhase::Active => {
            state.phase = HillclimbPhase::Idle;
            notifs.push("HILLCLIMB CANCELLED", Color::srgb(1.0, 0.35, 0.2));
            info!("hillclimb: cancelled");
        }

        HillclimbPhase::Finished => {
            state.phase = HillclimbPhase::Idle;
            info!("hillclimb: reset to idle from finished state");
        }
    }
}

// ---------------------------------------------------------------------------
// System: tick_hillclimb
// ---------------------------------------------------------------------------

fn tick_hillclimb(
    time: Res<Time>,
    mut state: ResMut<HillclimbState>,
    vehicle: Res<VehicleRoot>,
    chassis_q: Query<&Transform, With<Chassis>>,
    mut notifs: ResMut<NotificationQueue>,
) {
    if state.phase != HillclimbPhase::Active {
        return;
    }

    let dt = time.delta_secs();
    state.elapsed_s += dt;

    let Ok(xform) = chassis_q.get(vehicle.chassis) else {
        return;
    };

    state.current_elevation = xform.translation.y;

    // Detect flip: if chassis local-up Y-component is negative
    let chassis_up = xform.rotation * Vec3::Y;
    if chassis_up.y < -0.3 {
        state.phase = HillclimbPhase::Idle;
        notifs.push("HILLCLIMB FAILED — flipped", Color::srgb(1.0, 0.25, 0.1));
        info!("hillclimb: failed — vehicle flipped");
        return;
    }

    // Detect slide below start by >5 m
    if state.current_elevation < state.start_elevation - 5.0 {
        state.phase = HillclimbPhase::Idle;
        notifs.push("HILLCLIMB FAILED — slid back", Color::srgb(1.0, 0.25, 0.1));
        info!("hillclimb: failed — slid below start elevation");
    }
}

// ---------------------------------------------------------------------------
// System: detect_finish
// ---------------------------------------------------------------------------

fn detect_finish(
    mut state: ResMut<HillclimbState>,
    vehicle: Res<VehicleRoot>,
    chassis_q: Query<&Transform, With<Chassis>>,
    finish_gate_q: Query<&Transform, (With<HillclimbFinishGate>, Without<Chassis>)>,
    mut notifs: ResMut<NotificationQueue>,
) {
    if state.phase != HillclimbPhase::Active {
        return;
    }

    let Ok(chassis_xform) = chassis_q.get(vehicle.chassis) else {
        return;
    };

    let chassis_pos = chassis_xform.translation;

    // Must have gained at least 50m in elevation before the finish can trigger
    if chassis_pos.y < state.start_elevation + 50.0 {
        return;
    }

    // Check proximity to any finish gate
    for gate_xform in finish_gate_q.iter() {
        let gate_pos = gate_xform.translation;
        let dx = chassis_pos.x - gate_pos.x;
        let dz = chassis_pos.z - gate_pos.z;
        let xz_dist = (dx * dx + dz * dz).sqrt();

        if xz_dist <= 8.0 {
            let elapsed = state.elapsed_s;
            state.phase = HillclimbPhase::Finished;
            state.just_finished = true;

            let is_new_best = match state.best_time_s {
                None => true,
                Some(prev) => elapsed < prev,
            };

            if is_new_best {
                state.best_time_s = Some(elapsed);
                info!("hillclimb: NEW BEST: {:.2}s", elapsed);
                let msg = format!("FINISHED! {}", fmt_time(elapsed));
                notifs.push(msg, Color::srgb(0.1, 1.0, 0.4));
            } else {
                let best = state.best_time_s.unwrap_or(elapsed);
                let msg = format!("FINISHED! {}  Best: {}", fmt_time(elapsed), fmt_time(best));
                notifs.push(msg, Color::srgb(0.4, 0.85, 1.0));
            }

            info!("hillclimb: finished in {:.2}s", elapsed);
            break;
        }
    }
}

// ---------------------------------------------------------------------------
// System: update_hud
// ---------------------------------------------------------------------------

fn update_hud(
    state: Res<HillclimbState>,
    mut root_q: Query<&mut Visibility, With<HillclimbHudRoot>>,
    mut title_q: Query<(&mut Text, &mut TextColor), (With<HillclimbHudTitle>, Without<HillclimbHudTime>, Without<HillclimbHudElev>, Without<HillclimbHudBest>)>,
    mut time_q: Query<(&mut Text, &mut TextColor), (With<HillclimbHudTime>, Without<HillclimbHudTitle>, Without<HillclimbHudElev>, Without<HillclimbHudBest>)>,
    mut elev_q: Query<&mut Text, (With<HillclimbHudElev>, Without<HillclimbHudTitle>, Without<HillclimbHudTime>, Without<HillclimbHudBest>)>,
    mut best_q: Query<&mut Text, (With<HillclimbHudBest>, Without<HillclimbHudTitle>, Without<HillclimbHudTime>, Without<HillclimbHudElev>)>,
) {
    // Visibility: hidden when Idle
    for mut vis in root_q.iter_mut() {
        *vis = match state.phase {
            HillclimbPhase::Idle => Visibility::Hidden,
            HillclimbPhase::Active | HillclimbPhase::Finished => Visibility::Visible,
        };
    }

    if state.phase == HillclimbPhase::Idle {
        return;
    }

    let elev_gained = (state.current_elevation - state.start_elevation).max(0.0);
    let time_str = fmt_time_centis(state.elapsed_s);

    // Title row
    for (mut text, mut color) in title_q.iter_mut() {
        match state.phase {
            HillclimbPhase::Finished => {
                text.0 = "FINISHED!".to_string();
                color.0 = Color::srgb(0.1, 1.0, 0.4);
            }
            _ => {
                text.0 = "HILLCLIMB".to_string();
                color.0 = Color::srgb(1.0, 0.75, 0.15);
            }
        }
    }

    // Time row
    for (mut text, mut color) in time_q.iter_mut() {
        text.0 = time_str.clone();
        color.0 = match state.phase {
            HillclimbPhase::Finished => Color::srgb(0.1, 1.0, 0.4),
            _ => Color::WHITE,
        };
    }

    // Elevation row
    for mut text in elev_q.iter_mut() {
        text.0 = format!("Elev: +{:.1} m", elev_gained);
    }

    // Best time row
    for mut text in best_q.iter_mut() {
        text.0 = match state.best_time_s {
            Some(b) => format!("Best: {}", fmt_time_centis(b)),
            None => "Best: --".to_string(),
        };
    }
}

// ---------------------------------------------------------------------------
// System: save_on_finish
// ---------------------------------------------------------------------------

fn save_on_finish(mut state: ResMut<HillclimbState>) {
    if !state.just_finished {
        return;
    }
    state.just_finished = false;

    if let Some(best) = state.best_time_s {
        write_best_time(best);
    }
}

// ---------------------------------------------------------------------------
// Finish gate marker (declared here; hillclimb_track.rs will also declare it
// or re-export — the track agent owns that file; we declare our own copy so
// this module compiles standalone and the detect_finish query can reference
// the component type without a circular dependency).
// ---------------------------------------------------------------------------

/// Marker placed on the finish gate entity by HillclimbTrackPlugin.
/// Declared here so hillclimb.rs owns the type; hillclimb_track.rs
/// re-exports or uses `crate::hillclimb::HillclimbFinishGate`.
#[derive(Component, Default)]
pub struct HillclimbFinishGate;

/// Marker placed on the start gate entity (provided for completeness).
#[derive(Component, Default)]
pub struct HillclimbStartGate;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Format seconds as MM:SS.cc  e.g. "01:07.34"
fn fmt_time_centis(secs: f32) -> String {
    let secs = secs.max(0.0);
    let mins = (secs / 60.0) as u32;
    let rem = secs - (mins as f32) * 60.0;
    let sec = rem as u32;
    let centis = ((rem % 1.0) * 100.0) as u32;
    format!("{:02}:{:02}.{:02}", mins, sec, centis)
}

/// Format seconds as MM:SS for notifications
fn fmt_time(secs: f32) -> String {
    let secs = secs.max(0.0);
    let mins = (secs / 60.0) as u32;
    let rem = secs - (mins as f32) * 60.0;
    let sec = rem as u32;
    let centis = ((rem % 1.0) * 100.0) as u32;
    format!("{:02}:{:02}.{:02}", mins, sec, centis)
}
