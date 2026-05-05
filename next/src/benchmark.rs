// Benchmark mode: F12 starts a 30-second FPS logger that writes results to
// ~/.sandk-offroad/benchmark-{ts}.txt with min/max/avg/p1/p99 frame times.
//
// Public API:
//   BenchmarkPlugin
//   BenchmarkState (resource)

use std::{
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use bevy::prelude::*;

// ---- Public plugin ----------------------------------------------------------

pub struct BenchmarkPlugin;

impl Plugin for BenchmarkPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BenchmarkState>()
            .add_systems(Startup, spawn_bench_hud)
            .add_systems(
                Update,
                (
                    start_on_f12,
                    record_frames,
                    update_indicator,
                    finalize_and_write,
                ),
            );
    }
}

// ---- Resources & components -------------------------------------------------

/// Persistent benchmark state — reset each time a run starts.
#[derive(Resource, Default)]
pub struct BenchmarkState {
    pub running:     bool,
    pub elapsed_s:   f32,
    pub frame_times: Vec<f32>,
}

/// Marker for the root container node (visibility toggle).
#[derive(Component)]
struct BenchHudRoot;

/// Marker for the countdown text inside the container.
#[derive(Component)]
struct BenchHudText;

// ---- Constants --------------------------------------------------------------

const BENCH_DURATION_S: f32 = 30.0;

const COLOR_BENCH: Color = Color::srgb(0.95, 0.15, 0.15); // red
const BG: Color = Color::srgba(0.05, 0.05, 0.07, 0.80);

// ---- Startup: spawn HUD indicator -------------------------------------------

fn spawn_bench_hud(mut commands: Commands) {
    // Outer container — top-right corner, above the perf panel.
    commands
        .spawn((
            BenchHudRoot,
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(12.0),
                top:   Val::Px(170.0), // sits just below the stats panel
                padding: UiRect::axes(Val::Px(10.0), Val::Px(6.0)),
                ..default()
            },
            BackgroundColor(BG),
            Visibility::Hidden,
        ))
        .with_children(|parent| {
            parent.spawn((
                BenchHudText,
                Text::new("BENCH 00:30"),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(COLOR_BENCH),
            ));
        });
}

// ---- Systems ----------------------------------------------------------------

/// Press F12 to start a benchmark run (ignored while one is already running).
fn start_on_f12(
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<BenchmarkState>,
) {
    if keys.just_pressed(KeyCode::F12) && !state.running {
        state.running     = true;
        state.elapsed_s   = 0.0;
        state.frame_times.clear();
        info!("benchmark: started — recording for {BENCH_DURATION_S} s");
    }
}

/// Accumulate elapsed time and push one frame-time sample (ms) per frame.
fn record_frames(time: Res<Time>, mut state: ResMut<BenchmarkState>) {
    if !state.running {
        return;
    }
    let dt = time.delta_secs();
    state.elapsed_s  += dt;
    state.frame_times.push(dt * 1000.0);
}

/// Update the countdown text and show/hide the HUD container.
fn update_indicator(
    state: Res<BenchmarkState>,
    mut root_q: Query<&mut Visibility, With<BenchHudRoot>>,
    mut text_q: Query<&mut Text, With<BenchHudText>>,
) {
    let target_vis = if state.running {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };

    for mut vis in &mut root_q {
        *vis = target_vis;
    }

    if state.running {
        let remaining = (BENCH_DURATION_S - state.elapsed_s).max(0.0).ceil() as u32;
        let secs = remaining % 60;
        let label = format!("BENCH 00:{:02}", secs);
        for mut text in &mut text_q {
            text.0 = label.clone();
        }
    }
}

/// When the run completes, compute statistics and write the report file.
fn finalize_and_write(mut state: ResMut<BenchmarkState>) {
    if !state.running || state.elapsed_s < BENCH_DURATION_S {
        return;
    }

    state.running = false;

    // --- Compute statistics --------------------------------------------------

    let mut samples = state.frame_times.clone();
    let n = samples.len();

    if n == 0 {
        warn!("benchmark: no frames recorded — skipping report");
        return;
    }

    let avg: f32 = samples.iter().sum::<f32>() / n as f32;
    let fps_avg = 1000.0 / avg;

    // Sort for percentile calculations.
    samples.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let min = samples[0];
    let max = samples[n - 1];

    // p1 / p99 — floor-index, clamped to valid range.
    let p1_idx  = ((n as f32 * 0.01) as usize).min(n - 1);
    let p99_idx = ((n as f32 * 0.99) as usize).min(n - 1);
    let p1  = samples[p1_idx];
    let p99 = samples[p99_idx];

    // --- Build report string -------------------------------------------------

    let report = format!(
        "SandK Offroad Benchmark\n\
         Duration: {:.1} s\n\
         Total frames: {}\n\
         Frame time avg: {:.2} ms ({:.1} FPS)\n\
         Frame time min: {:.2} ms\n\
         Frame time max: {:.2} ms\n\
         Frame time p1:  {:.2} ms\n\
         Frame time p99: {:.2} ms\n",
        BENCH_DURATION_S,
        n,
        avg,
        fps_avg,
        min,
        max,
        p1,
        p99,
    );

    // --- Resolve output path -------------------------------------------------

    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let path = bench_report_path(ts);

    // Create parent directory if it does not exist.
    if let Some(parent) = path.parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            warn!("benchmark: could not create directory {:?}: {}", parent, e);
        }
    }

    match std::fs::write(&path, &report) {
        Ok(()) => {
            let path_str = path.to_string_lossy();
            info!("benchmark: results written to {}", path_str);
        }
        Err(e) => {
            warn!("benchmark: failed to write report to {:?}: {}", path, e);
        }
    }
}

// ---- Path helpers -----------------------------------------------------------

fn bench_report_path(unix_ts: u64) -> PathBuf {
    let filename = format!("benchmark-{}.txt", unix_ts);
    // Always write to ~/.sandk-offroad/ regardless of platform.
    match std::env::var("HOME").ok().map(PathBuf::from) {
        Some(home) => home.join(".sandk-offroad").join(filename),
        None => {
            warn!("benchmark: $HOME not set; writing to current directory");
            PathBuf::from(filename)
        }
    }
}
