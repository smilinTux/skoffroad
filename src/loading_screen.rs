// Loading screen: black overlay with title text shown for first ~1.5s of
// app run, then fades to reveal gameplay.
//
// Public API:
//   LoadingScreenPlugin

use bevy::prelude::*;

// ---------------------------------------------------------------------------
// Public plugin
// ---------------------------------------------------------------------------

pub struct LoadingScreenPlugin;

impl Plugin for LoadingScreenPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LoadingScreenState>()
            .add_systems(Startup, spawn_loading_screen)
            .add_systems(
                Update,
                (tick_loading_screen, update_overlay_alpha, dismiss_on_input).chain(),
            );
    }
}

// ---------------------------------------------------------------------------
// Internal state
// ---------------------------------------------------------------------------

#[derive(Default, Clone, Copy, PartialEq, Eq, Debug)]
enum Phase {
    #[default]
    Visible,
    FadingOut,
    Done,
}

#[derive(Resource)]
struct LoadingScreenState {
    phase: Phase,
    /// Accumulates elapsed seconds since startup.
    t: f32,
    /// Current overlay opacity (0.0 = transparent, 1.0 = opaque).
    alpha: f32,
}

impl Default for LoadingScreenState {
    fn default() -> Self {
        Self {
            phase: Phase::Visible,
            t: 0.0,
            alpha: 1.0,
        }
    }
}

// ---------------------------------------------------------------------------
// Component markers
// ---------------------------------------------------------------------------

/// Root entity for the loading screen overlay.
#[derive(Component)]
struct LoadingScreenRoot;

/// Marker for the subtitle text node — used for sine-pulse animation.
#[derive(Component)]
struct LoadingSubtitle;

// ---------------------------------------------------------------------------
// Colour / size constants
// ---------------------------------------------------------------------------

const TITLE_COLOR:   Color = Color::srgb(1.0, 0.9, 0.3);
const TAGLINE_COLOR: Color = Color::srgb(0.40, 0.40, 0.40);
/// Base RGB for the subtitle — alpha is driven dynamically.
const SUB_R: f32 = 0.75;
const SUB_G: f32 = 0.75;
const SUB_B: f32 = 0.75;

const TITLE_SIZE:   f32 = 64.0;
const SUB_SIZE:     f32 = 18.0;
const TAGLINE_SIZE: f32 = 12.0;

/// After this many seconds the overlay accepts key-press dismissal.
const HOLD_SECS: f32 = 1.5;
/// After this many total seconds the fade starts automatically.
const AUTO_FADE_SECS: f32 = 4.0;
/// Duration of the fade-out in seconds.
const FADE_DURATION: f32 = 0.6;
/// Subtitle pulse: alpha cycles between these bounds (sine wave).
const PULSE_MIN: f32 = 0.6;
const PULSE_MAX: f32 = 1.0;
/// Pulse speed in radians per second.
const PULSE_SPEED: f32 = std::f32::consts::TAU * 0.8;

// ---------------------------------------------------------------------------
// Startup: build loading-screen tree
// ---------------------------------------------------------------------------

fn spawn_loading_screen(mut commands: Commands) {
    // Full-screen opaque black overlay.  ZIndex(99999) ensures it covers
    // everything including the HUD and any other UI spawned at startup.
    let root = commands
        .spawn((
            LoadingScreenRoot,
            Node {
                width:           Val::Percent(100.0),
                height:          Val::Percent(100.0),
                position_type:   PositionType::Absolute,
                flex_direction:  FlexDirection::Column,
                align_items:     AlignItems::Center,
                // Push the title block to roughly 35% from the top.
                padding:         UiRect::top(Val::Percent(35.0)),
                row_gap:         Val::Px(16.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 1.0)),
            ZIndex(99999),
        ))
        .id();

    // Large game title — lowercase wordmark.
    let title = commands
        .spawn((
            Text::new("skoffroad"),
            TextFont { font_size: TITLE_SIZE, ..default() },
            TextColor(TITLE_COLOR),
        ))
        .id();

    // Brand subtitle.
    let brand = commands
        .spawn((
            Text::new("S&K  OFFROAD"),
            TextFont { font_size: 18.0, ..default() },
            TextColor(Color::srgb(0.85, 0.78, 0.55)),
        ))
        .id();

    // "Press any key to start" subtitle (pulsed once input is accepted).
    let subtitle = commands
        .spawn((
            LoadingSubtitle,
            Text::new("Press any key to start"),
            TextFont { font_size: SUB_SIZE, ..default() },
            // Start invisible — alpha will be set each frame once visible.
            TextColor(Color::srgba(SUB_R, SUB_G, SUB_B, 0.0)),
        ))
        .id();

    // Small tagline — auto-pulled from CARGO_PKG_VERSION at build time.
    let tagline = commands
        .spawn((
            Text::new(format!("v{}  \u{2014}  procedural off-road sandbox", env!("CARGO_PKG_VERSION"))),
            TextFont { font_size: TAGLINE_SIZE, ..default() },
            TextColor(TAGLINE_COLOR),
        ))
        .id();

    commands
        .entity(root)
        .add_children(&[title, brand, subtitle, tagline]);
}

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

/// Advance the phase timer and manage phase transitions.
fn tick_loading_screen(
    time:  Res<Time>,
    mut state: ResMut<LoadingScreenState>,
) {
    if state.phase == Phase::Done {
        return;
    }

    state.t += time.delta_secs();

    match state.phase {
        Phase::Visible => {
            state.alpha = 1.0;

            // Auto-start fade after AUTO_FADE_SECS even without a key press.
            if state.t >= AUTO_FADE_SECS {
                state.phase = Phase::FadingOut;
            }
        }
        Phase::FadingOut => {
            // Decrease alpha linearly over FADE_DURATION seconds.
            state.alpha -= time.delta_secs() / FADE_DURATION;
            if state.alpha <= 0.0 {
                state.alpha = 0.0;
                state.phase = Phase::Done;
            }
        }
        Phase::Done => {}
    }
}

/// Sync the overlay BackgroundColor alpha and despawn when done.
fn update_overlay_alpha(
    mut commands: Commands,
    state: Res<LoadingScreenState>,
    mut overlay_q: Query<(Entity, &mut BackgroundColor), With<LoadingScreenRoot>>,
    mut subtitle_q: Query<&mut TextColor, With<LoadingSubtitle>>,
) {
    for (entity, mut bg) in overlay_q.iter_mut() {
        if state.phase == Phase::Done {
            // Overlay fully transparent — remove the entity tree.
            commands.entity(entity).despawn();
            continue;
        }

        bg.0 = Color::srgba(0.0, 0.0, 0.0, state.alpha);
    }

    // Pulse the subtitle alpha only after the hold period has elapsed and
    // while the overlay is still on screen.
    for mut color in subtitle_q.iter_mut() {
        if state.t < HOLD_SECS || state.phase == Phase::Done {
            // Keep subtitle hidden until input is accepted.
            color.0 = Color::srgba(SUB_R, SUB_G, SUB_B, 0.0);
        } else {
            let pulse = (state.t * PULSE_SPEED).sin() * 0.5 + 0.5; // 0..1
            let a = PULSE_MIN + pulse * (PULSE_MAX - PULSE_MIN);
            color.0 = Color::srgba(SUB_R, SUB_G, SUB_B, a);
        }
    }
}

/// Detect any key press and begin the fade after the hold period.
fn dismiss_on_input(
    keys:      Res<ButtonInput<KeyCode>>,
    mut state: ResMut<LoadingScreenState>,
) {
    // Only accept input after the initial hold period.
    if state.phase != Phase::Visible || state.t < HOLD_SECS {
        return;
    }

    if keys.get_just_pressed().next().is_some() {
        state.phase = Phase::FadingOut;
    }
}
