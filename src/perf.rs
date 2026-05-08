// Detailed performance overlay — toggled with F8 (hidden by default).
// Anchored top-right, below the stats panel (~200 px from top).
// Shows: FPS (current + EMA avg), frame time (current + history max),
//        entity count, and a 32-bar mini-histogram color-coded by frame budget.
//
// Color thresholds (mirror hud.rs): >= 60 fps green, 30..60 yellow, < 30 red.

use bevy::{
    diagnostic::{
        DiagnosticsStore, EntityCountDiagnosticsPlugin, FrameTimeDiagnosticsPlugin,
    },
    prelude::*,
};

pub struct PerfPlugin;

impl Plugin for PerfPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<FrameTimeDiagnosticsPlugin>() {
            app.add_plugins(FrameTimeDiagnosticsPlugin::default());
        }
        if !app.is_plugin_added::<EntityCountDiagnosticsPlugin>() {
            app.add_plugins(EntityCountDiagnosticsPlugin::default());
        }
        app.init_resource::<PerfVisible>()
            .add_systems(Startup, spawn_perf_panel)
            .add_systems(Update, (update_perf_panel, toggle_perf_panel));
    }
}

// ---- Resources & components -------------------------------------------------

#[derive(Resource)]
struct PerfVisible(bool);

impl Default for PerfVisible {
    fn default() -> Self { Self(false) } // hidden until F8
}

#[derive(Component)]
struct PerfRoot;

#[derive(Component)]
enum PerfText { Fps, FrameTime, Entities }

#[derive(Component)]
struct HistBar { index: usize }

// ---- Color helpers ----------------------------------------------------------

const BG: Color = Color::srgba(0.05, 0.05, 0.07, 0.80);
const COLOR_LABEL: Color = Color::srgb(0.75, 0.75, 0.75);

fn fps_color(fps: f32) -> Color {
    if fps >= 60.0      { Color::srgb(0.3,  0.95, 0.3)  }
    else if fps >= 30.0 { Color::srgb(0.95, 0.85, 0.2)  }
    else                { Color::srgb(0.95, 0.2,  0.2)  }
}

// Green < 16.7 ms (60 fps budget), yellow < 33.3 ms, red otherwise.
fn bar_color(ms: f32) -> Color {
    if ms < 16.7        { Color::srgb(0.3,  0.95, 0.3)  }
    else if ms < 33.3   { Color::srgb(0.95, 0.85, 0.2)  }
    else                { Color::srgb(0.95, 0.2,  0.2)  }
}

const HIST_BARS: usize = 32;

// ---- Startup ----------------------------------------------------------------

fn spawn_perf_panel(mut commands: Commands) {
    let root = commands
        .spawn((
            PerfRoot,
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(12.0),
                top: Val::Px(200.0),
                width: Val::Px(240.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(10.0)),
                row_gap: Val::Px(4.0),
                display: Display::None,
                ..default()
            },
            BackgroundColor(BG),
        ))
        .id();

    let fps_text = commands.spawn((
        PerfText::Fps,
        Text::new("FPS: --"),
        TextFont { font_size: 13.0, ..default() },
        TextColor(COLOR_LABEL),
    )).id();

    let ft_text = commands.spawn((
        PerfText::FrameTime,
        Text::new("frame: -- ms"),
        TextFont { font_size: 13.0, ..default() },
        TextColor(COLOR_LABEL),
    )).id();

    let ent_text = commands.spawn((
        PerfText::Entities,
        Text::new("entities: --"),
        TextFont { font_size: 13.0, ..default() },
        TextColor(COLOR_LABEL),
    )).id();

    // Horizontal row of thin bars anchored at their bottom edge.
    let hist_container = commands.spawn(Node {
        flex_direction: FlexDirection::Row,
        align_items: AlignItems::FlexEnd,
        width: Val::Percent(100.0),
        height: Val::Px(28.0),
        column_gap: Val::Px(1.0),
        margin: UiRect::top(Val::Px(6.0)),
        ..default()
    }).id();

    let mut bar_ids = Vec::with_capacity(HIST_BARS);
    for i in 0..HIST_BARS {
        let bar = commands.spawn((
            HistBar { index: i },
            Node { width: Val::Px(5.0), height: Val::Px(4.0), ..default() },
            BackgroundColor(Color::srgb(0.3, 0.95, 0.3)),
        )).id();
        bar_ids.push(bar);
    }

    commands.entity(hist_container).add_children(&bar_ids);
    commands.entity(root).add_children(&[fps_text, ft_text, ent_text, hist_container]);
}

// ---- Update -----------------------------------------------------------------

fn update_perf_panel(
    diagnostics: Res<DiagnosticsStore>,
    visible: Res<PerfVisible>,
    mut texts: Query<(&PerfText, &mut Text, &mut TextColor)>,
    mut bars: Query<(&HistBar, &mut Node, &mut BackgroundColor)>,
) {
    if !visible.0 { return; }

    let fps_diag = diagnostics.get(&FrameTimeDiagnosticsPlugin::FPS);
    let fps_current = fps_diag.and_then(|d| d.value()).unwrap_or(0.0) as f32;
    // .average() is the simple moving average over the stored history window.
    let fps_avg    = fps_diag.and_then(|d| d.average()).unwrap_or(0.0) as f32;

    // FRAME_TIME values are in seconds; multiply by 1000 for ms display.
    let ft_diag = diagnostics.get(&FrameTimeDiagnosticsPlugin::FRAME_TIME);
    let ft_ms   = ft_diag.and_then(|d| d.value()).unwrap_or(0.0) as f32 * 1000.0;

    // .values() returns an opaque forward-only iterator; collect then find max.
    let ft_max_ms = ft_diag
        .map(|d| d.values().copied().fold(0.0_f64, f64::max) as f32 * 1000.0)
        .unwrap_or(0.0);

    let entity_count = diagnostics
        .get(&EntityCountDiagnosticsPlugin::ENTITY_COUNT)
        .and_then(|d| d.value())
        .unwrap_or(0.0) as u64;

    // Collect last HIST_BARS samples (oldest-first) for the histogram.
    let hist: Vec<f32> = ft_diag
        .map(|d| {
            let all: Vec<f32> = d.values().map(|v| *v as f32 * 1000.0).collect();
            let start = all.len().saturating_sub(HIST_BARS);
            all[start..].to_vec()
        })
        .unwrap_or_default();

    for (label, mut text, mut color) in &mut texts {
        match label {
            PerfText::Fps => {
                text.0   = format!("FPS: {:.1} (avg {:.1})", fps_current, fps_avg);
                color.0  = fps_color(fps_current);
            }
            PerfText::FrameTime => {
                text.0   = format!("frame: {:.1} ms (max {:.1})", ft_ms, ft_max_ms);
                color.0  = fps_color(if ft_ms > 0.0 { 1000.0 / ft_ms } else { 999.0 });
            }
            PerfText::Entities => {
                text.0   = format!("entities: {}", entity_count);
                color.0  = COLOR_LABEL;
            }
        }
    }

    // Scale bar heights relative to the worst recent frame (floor: 33.3 ms).
    let scale_max = hist.iter().copied().fold(33.3_f32, f32::max);
    for (bar, mut node, mut bg) in &mut bars {
        let ms = hist.get(bar.index).copied().unwrap_or(0.0);
        node.height = Val::Px(((ms / scale_max) * 24.0).clamp(2.0, 24.0));
        bg.0 = bar_color(ms);
    }
}

// ---- Toggle -----------------------------------------------------------------

fn toggle_perf_panel(
    keys: Res<ButtonInput<KeyCode>>,
    mut visible: ResMut<PerfVisible>,
    mut root_q: Query<&mut Node, With<PerfRoot>>,
) {
    if keys.just_pressed(KeyCode::F8) {
        visible.0 = !visible.0;
        let display = if visible.0 { Display::Flex } else { Display::None };
        for mut node in &mut root_q {
            node.display = display;
        }
    }
}
