// Time-trial mode — stopwatch that times each waypoint run.
//
// Piggybacks on Waypoint::reached_count from compass.rs.  No file
// persistence: best time resets on session exit.  A single float in a text
// file would be trivial, but the save-slot system (save.rs) is the proper
// home for that and another agent owns it — skipping to stay in bounds.

use bevy::prelude::*;

// ---- Plugin -----------------------------------------------------------------

pub struct TrialPlugin;

impl Plugin for TrialPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TrialState>()
            .add_systems(Startup, spawn_trial_hud)
            .add_systems(
                Update,
                (
                    update_trial,
                    update_trial_hud.run_if(resource_exists::<crate::compass::Waypoint>),
                ),
            );
    }
}

// ---- Resource ---------------------------------------------------------------

#[derive(Resource, Default)]
pub struct TrialState {
    /// Wall-clock elapsed seconds at the start of the current run.
    pub start_time_s: f32,
    /// Last reached_count we observed — detects waypoint transitions.
    pub last_reached: u32,
    /// Time of the most recently completed run, in seconds.
    pub last_run_s: f32,
    /// Best run time this session; None until at least one run completes.
    pub best_run_s: Option<f32>,
    /// True for finish_flash_ticks frames after a waypoint is reached.
    pub just_finished: bool,
    /// Countdown to clearing just_finished.
    pub finish_flash_ticks: u32,
    /// Total waypoints reached this session (run counter for "run N" display).
    pub total_runs: u32,
    /// Which run number holds the best time.
    pub best_run_number: u32,
    /// Whether the HUD panel is visible.
    pub visible: bool,
    /// Set to true once we've latched the initial start time.
    started: bool,
}

// ---- Components -------------------------------------------------------------

#[derive(Component)]
struct TrialHudRoot;

#[derive(Component)]
enum TrialText {
    Current,
    Best,
}

// ---- Color constants ---------------------------------------------------------

const BG_NORMAL: Color   = Color::srgba(0.05, 0.05, 0.07, 0.75);
const BG_FLASH: Color    = Color::srgba(0.05, 0.30, 0.07, 0.85);
const COLOR_LABEL: Color = Color::srgb(0.85, 0.85, 0.55);

// ---- Startup: spawn HUD -----------------------------------------------------

fn spawn_trial_hud(mut commands: Commands) {
    // Panel: top-center, 280 px wide, just below the 30 px compass strip.
    // compass strip is at top:8, height:30 — so top of this panel is 8+30+4 = 42.
    let root = commands
        .spawn((
            TrialHudRoot,
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(42.0),
                width: Val::Px(280.0),
                margin: UiRect {
                    left: Val::Auto,
                    right: Val::Auto,
                    ..default()
                },
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(8.0)),
                ..default()
            },
            BackgroundColor(BG_NORMAL),
            ZIndex(20),
        ))
        .id();

    let current_text = commands
        .spawn((
            TrialText::Current,
            Text::new("CURRENT: 0:00.00"),
            TextFont {
                font_size: 13.0,
                ..default()
            },
            TextColor(COLOR_LABEL),
        ))
        .id();

    let best_text = commands
        .spawn((
            TrialText::Best,
            Text::new("BEST:    --"),
            TextFont {
                font_size: 13.0,
                ..default()
            },
            TextColor(COLOR_LABEL),
        ))
        .id();

    commands.entity(root).add_children(&[current_text, best_text]);
}

// ---- Update: trial logic ----------------------------------------------------

fn update_trial(
    waypoint: Option<Res<crate::compass::Waypoint>>,
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<TrialState>,
) {
    // Toggle visibility with L.
    if keys.just_pressed(KeyCode::KeyL) {
        state.visible = !state.visible;
    }

    let Some(waypoint) = waypoint else { return };
    let elapsed = time.elapsed_secs();

    // Latch the initial start time on the very first frame where the waypoint
    // resource exists and we haven't started yet.
    if !state.started {
        state.start_time_s = elapsed;
        state.started = true;
    }

    // Detect a new waypoint being reached.
    if waypoint.reached_count > state.last_reached {
        let run_s = elapsed - state.start_time_s;

        state.total_runs += 1;
        state.last_run_s = run_s;

        state.best_run_s = Some(match state.best_run_s {
            None => {
                state.best_run_number = state.total_runs;
                run_s
            }
            Some(prev) if run_s < prev => {
                state.best_run_number = state.total_runs;
                run_s
            }
            Some(prev) => prev,
        });

        // Reset timer for the next run.
        state.start_time_s = elapsed;
        state.last_reached = waypoint.reached_count;

        state.just_finished = true;
        state.finish_flash_ticks = 60;
    }

    // Decrement the flash counter.
    if state.finish_flash_ticks > 0 {
        state.finish_flash_ticks -= 1;
        if state.finish_flash_ticks == 0 {
            state.just_finished = false;
        }
    }
}

// ---- Update: HUD display ----------------------------------------------------

fn update_trial_hud(
    state: Res<TrialState>,
    time: Res<Time>,
    mut root_q: Query<(&mut Node, &mut BackgroundColor), With<TrialHudRoot>>,
    mut texts: Query<(&TrialText, &mut Text)>,
) {
    let display = if state.visible {
        Display::Flex
    } else {
        Display::None
    };

    for (mut node, mut bg) in &mut root_q {
        node.display = display;
        bg.0 = if state.just_finished { BG_FLASH } else { BG_NORMAL };
    }

    if !state.visible {
        return;
    }

    let elapsed = time.elapsed_secs();
    let run_elapsed = elapsed - state.start_time_s;
    let current_str = fmt_time(run_elapsed);

    let best_str = match state.best_run_s {
        None => "BEST:    --".to_string(),
        Some(b) => {
            let runs_ago = state.total_runs - state.best_run_number;
            if runs_ago == 0 {
                // Best was just set this run.
                format!("BEST:    {} (this run)", fmt_time(b))
            } else {
                format!(
                    "BEST:    {} (run {})",
                    fmt_time(b),
                    state.best_run_number
                )
            }
        }
    };

    for (label, mut text) in &mut texts {
        match label {
            TrialText::Current => {
                text.0 = format!("CURRENT: {}", current_str);
            }
            TrialText::Best => {
                text.0 = best_str.clone();
            }
        }
    }
}

// ---- Helpers ----------------------------------------------------------------

/// Format seconds as M:SS.ss  e.g. "1:07.42"
fn fmt_time(secs: f32) -> String {
    let secs = secs.max(0.0);
    let minutes = (secs / 60.0) as u32;
    let remainder = secs - (minutes as f32) * 60.0;
    format!("{:01}:{:05.2}", minutes, remainder)
}
