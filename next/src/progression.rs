// Progression: aggregates XP from XpState, race finishes, collectibles, and
// computes level (1..50). Exposes ProgressionState for unlocks / HUD.
//
// Public API:
//   ProgressionPlugin
//   ProgressionState { level, total_xp, current_level_xp, next_level_xp,
//                      level_up_pending }

use bevy::prelude::*;

use crate::xp::XpState;

// ---- Level curve constants ----------------------------------------------------

/// Maximum player level.
const MAX_LEVEL: u32 = 50;

/// Progress-bar panel geometry.
const PANEL_W: f32 = 320.0;
const PANEL_H: f32 = 56.0;

// ---- Public resources ---------------------------------------------------------

pub struct ProgressionPlugin;

impl Plugin for ProgressionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ProgressionState>()
            .add_systems(Startup, spawn_progression_hud)
            .add_systems(
                Update,
                (recompute_progression, update_progression_hud, flash_level_up_text),
            );
    }
}

#[derive(Resource, Default, Clone)]
pub struct ProgressionState {
    pub level: u32,
    pub total_xp: u64,
    pub current_level_xp: u64,
    pub next_level_xp: u64,
    pub level_up_pending: bool,
}

// ---- Level curve --------------------------------------------------------------

/// Cumulative XP needed to have *reached* level `n`.
/// xp_for_level(0) = 0  (starting state before level 1)
/// xp_for_level(1) = 100
/// xp_for_level(2) = 300
/// xp_for_level(10) = 5500
/// xp_for_level(50) = 127500
#[inline]
fn xp_for_level(n: u32) -> u64 {
    100 * (n as u64) * (n as u64 + 1) / 2
}

/// Given total accumulated XP, return the current level (clamped to MAX_LEVEL).
fn level_from_xp(total: u64) -> u32 {
    // Binary-search for the highest level whose threshold is <= total.
    let mut lo: u32 = 0;
    let mut hi: u32 = MAX_LEVEL;
    while lo < hi {
        let mid = lo + (hi - lo + 1) / 2;
        if xp_for_level(mid) <= total {
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }
    lo.max(1).min(MAX_LEVEL)
}

// ---- Components ---------------------------------------------------------------

#[derive(Component)]
struct ProgressionPanelRoot;

#[derive(Component)]
struct ProgressionLevelText;

#[derive(Component)]
struct ProgressionBarFill;

#[derive(Component)]
struct ProgressionXpText;

#[derive(Component)]
struct LevelUpBanner;

// ---- Startup: spawn HUD -------------------------------------------------------

fn spawn_progression_hud(mut commands: Commands) {
    // Outer panel — bottom-centre, 320 × 56 px, dark translucent bg.
    let panel = commands
        .spawn((
            ProgressionPanelRoot,
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(14.0),
                left: Val::Percent(50.0),
                margin: UiRect {
                    left: Val::Px(-(PANEL_W / 2.0)),
                    ..default()
                },
                width: Val::Px(PANEL_W),
                height: Val::Px(PANEL_H),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                padding: UiRect::axes(Val::Px(8.0), Val::Px(6.0)),
                row_gap: Val::Px(4.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.04, 0.04, 0.06, 0.85)),
        ))
        .id();

    // Top row: "LEVEL N" in bright cyan.
    let level_text = commands
        .spawn((
            ProgressionLevelText,
            Text::new("LEVEL 1"),
            TextFont {
                font_size: 18.0,
                ..default()
            },
            TextColor(Color::srgb(0.4, 0.95, 1.0)),
        ))
        .id();

    // Bar container — full width, fixed height, dark track.
    let bar_track = commands
        .spawn((
            Node {
                width: Val::Px(PANEL_W - 16.0),
                height: Val::Px(10.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.15, 0.15, 0.18, 0.9)),
        ))
        .id();

    // Fill node — width driven by progress fraction.
    let bar_fill = commands
        .spawn((
            ProgressionBarFill,
            Node {
                width: Val::Px(0.0),
                height: Val::Px(10.0),
                ..default()
            },
            BackgroundColor(Color::srgb(0.4, 0.95, 1.0)),
        ))
        .id();

    // XP fraction text — overlaid centred above (or below) bar.
    let xp_text = commands
        .spawn((
            ProgressionXpText,
            Text::new("0/100 XP"),
            TextFont {
                font_size: 12.0,
                ..default()
            },
            TextColor(Color::WHITE),
        ))
        .id();

    commands.entity(bar_track).add_children(&[bar_fill]);
    commands
        .entity(panel)
        .add_children(&[level_text, bar_track, xp_text]);
}

// ---- recompute_progression system ---------------------------------------------

fn recompute_progression(
    xp: Option<Res<XpState>>,
    mut state: ResMut<ProgressionState>,
) {
    let total = match xp {
        Some(ref x) => x.total_xp,
        None => return,
    };

    let new_level = level_from_xp(total);
    let old_level = state.level;

    // XP threshold at the start of current level.
    let level_start = xp_for_level(new_level - 1);
    // XP needed to pass from start-of-level to next level.
    let level_span = xp_for_level(new_level) - level_start;

    state.total_xp = total;
    state.level = new_level;
    state.current_level_xp = total.saturating_sub(level_start);
    state.next_level_xp = level_span;

    if new_level > old_level && old_level != 0 {
        state.level_up_pending = true;
    }
}

// ---- update_progression_hud system --------------------------------------------

fn update_progression_hud(
    state: Res<ProgressionState>,
    mut level_q: Query<&mut Text, (With<ProgressionLevelText>, Without<ProgressionXpText>)>,
    mut xp_q: Query<&mut Text, (With<ProgressionXpText>, Without<ProgressionLevelText>)>,
    mut fill_q: Query<&mut Node, With<ProgressionBarFill>>,
) {
    // Level label.
    for mut text in &mut level_q {
        text.0 = format!("LEVEL {}", state.level);
    }

    // XP fraction text.
    for mut text in &mut xp_q {
        text.0 = format!("{}/{} XP", state.current_level_xp, state.next_level_xp);
    }

    // Progress bar fill width.
    let pct = if state.next_level_xp > 0 {
        (state.current_level_xp as f32 / state.next_level_xp as f32).clamp(0.0, 1.0)
    } else {
        1.0
    };
    for mut node in &mut fill_q {
        node.width = Val::Px(pct * (PANEL_W - 16.0));
    }
}

// ---- flash_level_up_text system -----------------------------------------------

fn flash_level_up_text(
    mut commands: Commands,
    mut state: ResMut<ProgressionState>,
    mut countdown: Local<f32>,
    time: Res<Time>,
    banner_q: Query<Entity, With<LevelUpBanner>>,
    mut text_q: Query<&mut TextColor, With<LevelUpBanner>>,
) {
    let dt = time.delta_secs();

    // Spawn banner when a level-up is detected.
    if state.level_up_pending {
        state.level_up_pending = false;
        *countdown = 2.0;

        // Despawn any existing banner first.
        for e in &banner_q {
            commands.entity(e).despawn();
        }

        // Spawn centred "LEVEL UP!" text in the top-third of the screen.
        commands.spawn((
            LevelUpBanner,
            Text::new("LEVEL UP!"),
            TextFont {
                font_size: 60.0,
                ..default()
            },
            TextColor(Color::srgba(1.0, 0.90, 0.1, 1.0)),
            Node {
                position_type: PositionType::Absolute,
                top: Val::Percent(28.0),
                left: Val::Percent(50.0),
                margin: UiRect {
                    left: Val::Px(-160.0),
                    ..default()
                },
                ..default()
            },
        ));

        return;
    }

    // Tick down and fade.
    if *countdown > 0.0 {
        *countdown = (*countdown - dt).max(0.0);
        let alpha = (*countdown / 2.0).clamp(0.0, 1.0);
        for mut color in &mut text_q {
            color.0 = Color::srgba(1.0, 0.90, 0.1, alpha);
        }

        // Despawn when fully faded.
        if *countdown == 0.0 {
            for e in &banner_q {
                commands.entity(e).despawn();
            }
        }
    }
}
