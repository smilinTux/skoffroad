// Season: cycles through Spring/Summer/Fall/Winter every 5 minutes of
// real time. Tints the global ambient light slightly toward the season
// palette so the world has a sense of cyclical time.
//
// Public API:
//   SeasonPlugin
//   SeasonState (resource)
//   Season enum

use bevy::prelude::*;

pub struct SeasonPlugin;

impl Plugin for SeasonPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SeasonState>()
            .add_systems(Startup, spawn_season_indicator)
            .add_systems(Update, (tick_season, update_indicator, apply_ambient_tint));
    }
}

#[derive(Resource, Default, Clone, Copy)]
pub struct SeasonState {
    pub current: Season,
    pub elapsed_in_season_s: f32,
}

#[derive(Default, Clone, Copy, PartialEq, Eq, Debug)]
pub enum Season {
    #[default]
    Spring,
    Summer,
    Fall,
    Winter,
}

// ---- Constants --------------------------------------------------------------

/// Real-time seconds before the season advances.
const SEASON_DURATION_S: f32 = 300.0;

/// Lerp factor applied each frame to blend ambient color toward target.
const AMBIENT_LERP: f32 = 0.3;

// ---- Internal marker component ----------------------------------------------

/// Marker on the season indicator text node.
#[derive(Component)]
struct SeasonIndicator;

// ---- Startup: spawn HUD indicator -------------------------------------------

fn spawn_season_indicator(mut commands: Commands) {
    // Small panel anchored bottom-right, above any FPS counter that may exist.
    let panel = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(12.0),
                bottom: Val::Px(60.0),
                padding: UiRect::all(Val::Px(6.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.05, 0.05, 0.07, 0.70)),
        ))
        .id();

    let text = commands
        .spawn((
            SeasonIndicator,
            Text::new("Spring"),
            TextFont {
                font_size: 12.0,
                ..default()
            },
            TextColor(season_text_color(Season::Spring)),
        ))
        .id();

    commands.entity(panel).add_children(&[text]);
}

// ---- Systems ----------------------------------------------------------------

/// Advance the elapsed timer; cycle to the next season every 300 s.
fn tick_season(mut state: ResMut<SeasonState>, time: Res<Time>) {
    state.elapsed_in_season_s += time.delta_secs();

    if state.elapsed_in_season_s >= SEASON_DURATION_S {
        state.elapsed_in_season_s = 0.0;
        state.current = next_season(state.current);
        info!("season changed to {:?}", state.current);
    }
}

/// Update the on-screen text label each frame to reflect the current season.
fn update_indicator(
    state: Res<SeasonState>,
    mut query: Query<(&mut Text, &mut TextColor), With<SeasonIndicator>>,
) {
    for (mut text, mut color) in &mut query {
        *text = Text::new(format!("{:?}", state.current));
        *color = TextColor(season_text_color(state.current));
    }
}

/// Lerp the global ambient light color toward the season-specific tint.
fn apply_ambient_tint(
    state: Res<SeasonState>,
    time: Res<Time>,
    mut ambient: ResMut<GlobalAmbientLight>,
) {
    let target = season_ambient_color(state.current);
    let dt = time.delta_secs();
    let factor = (dt * AMBIENT_LERP).clamp(0.0, 1.0);

    // Extract current linear sRGB components, lerp toward target, reassemble.
    let cur = ambient.color.to_srgba();
    let tgt = target.to_srgba();

    let r = lerp(cur.red, tgt.red, factor);
    let g = lerp(cur.green, tgt.green, factor);
    let b = lerp(cur.blue, tgt.blue, factor);

    ambient.color = Color::srgb(r, g, b);
}

// ---- Helpers ----------------------------------------------------------------

fn next_season(s: Season) -> Season {
    match s {
        Season::Spring => Season::Summer,
        Season::Summer => Season::Fall,
        Season::Fall   => Season::Winter,
        Season::Winter => Season::Spring,
    }
}

fn season_text_color(s: Season) -> Color {
    match s {
        Season::Spring => Color::srgb(0.5,  0.95, 0.55),
        Season::Summer => Color::srgb(1.0,  0.95, 0.4),
        Season::Fall   => Color::srgb(0.95, 0.55, 0.20),
        Season::Winter => Color::srgb(0.85, 0.95, 1.0),
    }
}

fn season_ambient_color(s: Season) -> Color {
    match s {
        Season::Spring => Color::srgb(0.95, 1.0,  0.95),
        Season::Summer => Color::srgb(1.05, 1.0,  0.95),
        Season::Fall   => Color::srgb(1.0,  0.85, 0.75),
        Season::Winter => Color::srgb(0.85, 0.92, 1.0),
    }
}

#[inline]
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}
