// Bite-sized challenges: 30-second mini-tasks (e.g. "reach 60 mph",
// "stay airborne 1.5s", "collect 3 gems"). C key starts a random one.
// Reward: bonus XP. Distinct from Daily (per-session, multi-completion).
//
// Public API:
//   ChallengesPlugin
//   ChallengesState (resource)

use bevy::prelude::*;

use crate::airtime::AirtimeStats;
use crate::hud::SessionStats;
use crate::wheelie::WheelieStats;
use crate::xp::XpState;

// ---- Constants ---------------------------------------------------------------

const MPS_TO_MPH: f32 = 2.237;
const CHALLENGE_DURATION_S: f32 = 30.0;
const CHALLENGE_REWARD_XP: u64 = 150;

const PANEL_W: f32 = 320.0;
const PANEL_H: f32 = 70.0;
const BAR_H: f32 = 14.0;

const COLOR_HEADER: Color = Color::srgb(1.0, 0.70, 0.0);   // amber
const COLOR_NAME: Color = Color::WHITE;
const COLOR_BAR_BASE: Color = Color::srgb(0.85, 0.55, 0.0);  // amber-ish
const COLOR_BAR_DONE: Color = Color::srgb(0.3, 0.95, 0.45);  // green
const COLOR_BG: Color = Color::srgba(0.04, 0.04, 0.06, 0.82);

// ---- Public plugin -----------------------------------------------------------

pub struct ChallengesPlugin;

impl Plugin for ChallengesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ChallengesState>()
            .add_systems(Startup, spawn_challenge_hud)
            .add_systems(
                Update,
                (start_with_c, track_challenge, update_hud).chain(),
            );
    }
}

// ---- Public resource ---------------------------------------------------------

#[derive(Resource, Default)]
pub struct ChallengesState {
    pub active: bool,
    pub elapsed_s: f32,
    pub current_kind: u32,
    pub progress: f32,
    pub target: f32,
    pub completed: bool,
}

// ---- Private components ------------------------------------------------------

#[derive(Component)]
struct ChallengeHudRoot;

#[derive(Component)]
struct ChallengeHudHeader;

#[derive(Component)]
struct ChallengeHudDesc;

#[derive(Component)]
struct ChallengeHudBar;

// ---- Description helper ------------------------------------------------------

fn kind_description(kind: u32) -> &'static str {
    match kind {
        0 => "Reach 60 mph",
        1 => "1.5s air in one jump",
        2 => "Drive 500m",
        3 => "3 wheelies",
        4 => "Hit 80 mph",
        _ => "Unknown",
    }
}

fn kind_target(kind: u32) -> f32 {
    match kind {
        0 => 60.0,
        1 => 1.5,
        2 => 500.0,
        3 => 3.0,
        4 => 80.0,
        _ => 1.0,
    }
}

// ---- Startup: spawn HUD panel ------------------------------------------------

fn spawn_challenge_hud(mut commands: Commands) {
    // Outer panel — top-centre, 320 x 70 px, hidden by default
    let panel = commands
        .spawn((
            ChallengeHudRoot,
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(12.0),
                left: Val::Percent(50.0),
                margin: UiRect {
                    left: Val::Px(-(PANEL_W / 2.0)),
                    ..default()
                },
                width: Val::Px(PANEL_W),
                height: Val::Px(PANEL_H),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::FlexStart,
                padding: UiRect::axes(Val::Px(10.0), Val::Px(6.0)),
                row_gap: Val::Px(2.0),
                display: Display::None, // hidden until active
                ..default()
            },
            BackgroundColor(COLOR_BG),
        ))
        .id();

    // Row 1: "CHALLENGE — Xs" header — amber, 11 pt
    let header = commands
        .spawn((
            ChallengeHudHeader,
            Text::new("CHALLENGE — 30s"),
            TextFont {
                font_size: 11.0,
                ..default()
            },
            TextColor(COLOR_HEADER),
        ))
        .id();

    // Row 2: challenge description — 14 pt white
    let desc_text = commands
        .spawn((
            ChallengeHudDesc,
            Text::new(""),
            TextFont {
                font_size: 14.0,
                ..default()
            },
            TextColor(COLOR_NAME),
        ))
        .id();

    // Row 3: progress bar container
    let bar_track = commands
        .spawn((Node {
            width: Val::Percent(100.0),
            height: Val::Px(BAR_H),
            ..default()
        },))
        .id();

    // Bar fill (width updated every frame)
    let bar_fill = commands
        .spawn((
            ChallengeHudBar,
            Node {
                width: Val::Percent(0.0),
                height: Val::Percent(100.0),
                ..default()
            },
            BackgroundColor(COLOR_BAR_BASE),
        ))
        .id();

    commands.entity(bar_track).add_children(&[bar_fill]);
    commands
        .entity(panel)
        .add_children(&[header, desc_text, bar_track]);
}

// ---- System: start challenge with C key -------------------------------------

fn start_with_c(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut state: ResMut<ChallengesState>,
    session: Res<SessionStats>,
    wheelie: Res<WheelieStats>,
    mut dist_baseline: Local<f32>,
    mut wheelie_baseline: Local<u32>,
) {
    if !keys.just_pressed(KeyCode::KeyC) {
        return;
    }

    // Pick kind using time-based pseudo-random
    let kind = (time.elapsed_secs() as u32 % 5) as u32;

    // Snapshot baselines for delta-based challenges
    *dist_baseline = session.distance_m;
    *wheelie_baseline = wheelie.wheelie_count;

    state.active = true;
    state.elapsed_s = 0.0;
    state.current_kind = kind;
    state.target = kind_target(kind);
    state.progress = 0.0;
    state.completed = false;
}

// ---- System: track active challenge -----------------------------------------

fn track_challenge(
    time: Res<Time>,
    mut state: ResMut<ChallengesState>,
    mut xp: ResMut<XpState>,
    session: Res<SessionStats>,
    airtime: Res<AirtimeStats>,
    wheelie: Res<WheelieStats>,
    dist_baseline: Local<f32>,
    wheelie_baseline: Local<u32>,
) {
    if !state.active {
        return;
    }

    let dt = time.delta_secs();
    state.elapsed_s += dt;

    // Compute current progress based on challenge kind
    let progress = match state.current_kind {
        0 => session.max_speed_mps * MPS_TO_MPH,
        1 => airtime.max_air_s,
        2 => (session.distance_m - *dist_baseline).max(0.0),
        3 => (wheelie.wheelie_count.saturating_sub(*wheelie_baseline)) as f32,
        4 => session.max_speed_mps * MPS_TO_MPH,
        _ => 0.0,
    };
    state.progress = progress;

    // Check completion
    if !state.completed && state.progress >= state.target {
        state.completed = true;
        xp.total_xp = xp.total_xp.saturating_add(CHALLENGE_REWARD_XP);
        xp.session_xp = xp.session_xp.saturating_add(CHALLENGE_REWARD_XP);
        xp.last_gain = CHALLENGE_REWARD_XP as i32;
        xp.last_gain_t = time.elapsed_secs();
        info!("challenge complete!");
    }

    // Timeout after 30 seconds
    if state.elapsed_s >= CHALLENGE_DURATION_S {
        if !state.completed {
            info!("challenge failed.");
        }
        state.active = false;
    }
}

// ---- System: update HUD ------------------------------------------------------

fn update_hud(
    state: Res<ChallengesState>,
    mut root_q: Query<(&mut Node, &mut BackgroundColor), (With<ChallengeHudRoot>, Without<ChallengeHudBar>)>,
    mut header_q: Query<&mut Text, (With<ChallengeHudHeader>, Without<ChallengeHudDesc>)>,
    mut desc_q: Query<&mut Text, (With<ChallengeHudDesc>, Without<ChallengeHudHeader>)>,
    mut bar_q: Query<(&mut Node, &mut BackgroundColor), (With<ChallengeHudBar>, Without<ChallengeHudRoot>)>,
) {
    // Show/hide root panel
    for (mut node, _bg) in &mut root_q {
        node.display = if state.active {
            Display::Flex
        } else {
            Display::None
        };
    }

    if !state.active {
        return;
    }

    let time_left = (CHALLENGE_DURATION_S - state.elapsed_s).max(0.0).ceil() as u32;
    let fraction = if state.target > 0.0 {
        (state.progress / state.target).clamp(0.0, 1.0)
    } else {
        0.0
    };

    // Row 1: header with time remaining
    for mut text in &mut header_q {
        text.0 = format!("CHALLENGE \u{2014} {}s", time_left);
    }

    // Row 2: challenge description
    for mut text in &mut desc_q {
        text.0 = kind_description(state.current_kind).to_string();
    }

    // Row 3: progress bar
    for (mut node, mut bg) in &mut bar_q {
        node.width = Val::Percent(fraction * 100.0);
        bg.0 = if state.completed {
            COLOR_BAR_DONE
        } else {
            COLOR_BAR_BASE
        };
    }
}
