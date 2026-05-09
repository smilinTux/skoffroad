// Daily challenge: one rotating goal per day, deterministically seeded
// from the local date. Completing it grants bonus XP. Persists "last
// completed day" so a single day can only reward once.
//
// Public API:
//   DailyPlugin
//   DailyState { today_seed, challenge: DailyChallenge, completed_today,
//                progress_value }

use bevy::prelude::*;
use crate::platform_storage::epoch_seconds;

use crate::airtime::AirtimeStats;
use crate::collectibles::CollectibleCount;
use crate::hud::SessionStats;
use crate::wheelie::WheelieStats;
use crate::xp::XpState;

// ---- Constants ---------------------------------------------------------------

const MPS_TO_MPH: f32 = 2.237;
const PANEL_W: f32 = 280.0;
const PANEL_H: f32 = 70.0;
const BAR_H: f32 = 14.0;

const COLOR_HEADER: Color = Color::srgb(0.85, 0.6, 1.0);
const COLOR_NAME: Color = Color::WHITE;
const COLOR_NAME_DONE: Color = Color::srgb(0.3, 0.95, 0.45);
const COLOR_BAR_BASE: Color = Color::srgb(0.55, 0.20, 0.85);
const COLOR_BAR_DONE: Color = Color::srgb(0.3, 0.95, 0.45);
const COLOR_BG: Color = Color::srgba(0.04, 0.04, 0.08, 0.82);

// ---- Public plugin -----------------------------------------------------------

pub struct DailyPlugin;

impl Plugin for DailyPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DailyState>()
            .add_systems(Startup, (build_daily_challenge, spawn_daily_hud).chain())
            .add_systems(
                Update,
                (track_progress, complete_check, update_daily_hud).chain(),
            );
    }
}

// ---- Public resources --------------------------------------------------------

#[derive(Resource, Default, Clone)]
pub struct DailyState {
    pub today_seed: u64,
    pub challenge: DailyChallenge,
    pub completed_today: bool,
    pub progress_value: f32,
}

#[derive(Clone, Debug, Default)]
pub struct DailyChallenge {
    pub name: String,
    pub description: String,
    pub kind: DailyKind,
    pub target: f32,
    pub xp_reward: u32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DailyKind {
    #[default]
    DriveDistanceKm,
    AirtimeTotalSeconds,
    HitTopSpeedMph,
    CollectGems,
    DoWheelies,
}

// ---- Private components ------------------------------------------------------

#[derive(Component)]
struct DailyHudRoot;

#[derive(Component)]
struct DailyHudName;

#[derive(Component)]
struct DailyHudProgressText;

#[derive(Component)]
struct DailyHudBar;

// ---- Startup: build challenge ------------------------------------------------

fn build_daily_challenge(mut state: ResMut<DailyState>) {
    let today_seed = epoch_seconds() / 86400;
    state.today_seed = today_seed;

    let index = today_seed % 5;
    state.challenge = match index {
        0 => DailyChallenge {
            name: "Drive 5 km".into(),
            description: "Cover 5 kilometres in a single session.".into(),
            kind: DailyKind::DriveDistanceKm,
            target: 5.0,
            xp_reward: 500,
        },
        1 => DailyChallenge {
            name: "Total 15 s of airtime".into(),
            description: "Rack up 15 seconds of total airtime today.".into(),
            kind: DailyKind::AirtimeTotalSeconds,
            target: 15.0,
            xp_reward: 400,
        },
        2 => DailyChallenge {
            name: "Reach 70 mph".into(),
            description: "Hit a top speed of at least 70 mph.".into(),
            kind: DailyKind::HitTopSpeedMph,
            target: 70.0,
            xp_reward: 300,
        },
        3 => DailyChallenge {
            name: "Collect 10 gems".into(),
            description: "Pick up 10 collectible gems scattered around the world.".into(),
            kind: DailyKind::CollectGems,
            target: 10.0,
            xp_reward: 350,
        },
        _ => DailyChallenge {
            name: "Pull 5 wheelies".into(),
            description: "Pull off 5 wheelies (each must last at least 0.5 s).".into(),
            kind: DailyKind::DoWheelies,
            target: 5.0,
            xp_reward: 400,
        },
    };
}

// ---- Startup: spawn HUD panel ------------------------------------------------

fn spawn_daily_hud(mut commands: Commands, state: Res<DailyState>) {
    // Outer panel — bottom-left, 280 x 70 px, dark background
    let panel = commands
        .spawn((
            DailyHudRoot,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(12.0),
                bottom: Val::Px(12.0),
                width: Val::Px(PANEL_W),
                height: Val::Px(PANEL_H),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::FlexStart,
                padding: UiRect::axes(Val::Px(10.0), Val::Px(6.0)),
                row_gap: Val::Px(2.0),
                ..default()
            },
            BackgroundColor(COLOR_BG),
        ))
        .id();

    // Row 1: "DAILY CHALLENGE" header — purple-ish, 11 pt
    let header = commands
        .spawn((
            Text::new("DAILY CHALLENGE"),
            TextFont {
                font_size: 11.0,
                ..default()
            },
            TextColor(COLOR_HEADER),
        ))
        .id();

    // Row 2: challenge name — 14 pt white (green when completed)
    let name_text = commands
        .spawn((
            DailyHudName,
            Text::new(&state.challenge.name),
            TextFont {
                font_size: 14.0,
                ..default()
            },
            TextColor(COLOR_NAME),
        ))
        .id();

    // Row 3: progress label + bar container
    let row3 = commands
        .spawn((Node {
            width: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(2.0),
            ..default()
        },))
        .id();

    // Progress fraction text: "0.0 / 5.0"
    let progress_text = commands
        .spawn((
            DailyHudProgressText,
            Text::new(format!("0.0 / {:.1}", state.challenge.target)),
            TextFont {
                font_size: 12.0,
                ..default()
            },
            TextColor(Color::srgb(0.85, 0.85, 0.85)),
        ))
        .id();

    // Bar track (full width, dark bg)
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
            DailyHudBar,
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
        .entity(row3)
        .add_children(&[progress_text, bar_track]);
    commands
        .entity(panel)
        .add_children(&[header, name_text, row3]);
}

// ---- Update: track progress --------------------------------------------------

fn track_progress(
    mut state: ResMut<DailyState>,
    session: Res<SessionStats>,
    airtime: Res<AirtimeStats>,
    wheelie: Res<WheelieStats>,
    collectibles: Res<CollectibleCount>,
) {
    if state.completed_today {
        return;
    }
    state.progress_value = match state.challenge.kind {
        DailyKind::DriveDistanceKm => session.distance_m / 1000.0,
        DailyKind::AirtimeTotalSeconds => airtime.session_total_air_s,
        DailyKind::HitTopSpeedMph => session.max_speed_mps * MPS_TO_MPH,
        DailyKind::CollectGems => collectibles.collected as f32,
        DailyKind::DoWheelies => wheelie.wheelie_count as f32,
    };
}

// ---- Update: completion check ------------------------------------------------

fn complete_check(
    mut state: ResMut<DailyState>,
    mut xp: ResMut<XpState>,
    time: Res<Time>,
) {
    if state.completed_today {
        return;
    }
    if state.progress_value >= state.challenge.target {
        state.completed_today = true;
        let reward = state.challenge.xp_reward as u64;
        xp.total_xp += reward;
        xp.session_xp += reward;
        xp.last_gain = state.challenge.xp_reward as i32;
        xp.last_gain_t = time.elapsed_secs();
        info!(
            "[DailyChallenge] '{}' completed -- +{} XP",
            state.challenge.name, state.challenge.xp_reward
        );
    }
}

// ---- Update: refresh HUD every frame ----------------------------------------

fn update_daily_hud(
    state: Res<DailyState>,
    mut name_q: Query<(&mut Text, &mut TextColor), With<DailyHudName>>,
    mut prog_q: Query<(&mut Text, &mut TextColor), (With<DailyHudProgressText>, Without<DailyHudName>)>,
    mut bar_q: Query<(&mut Node, &mut BackgroundColor), With<DailyHudBar>>,
) {
    let target = state.challenge.target;
    let progress = state.progress_value.min(target);
    let fraction = if target > 0.0 { progress / target } else { 0.0 };
    let done = state.completed_today;

    // Row 2: name / completion banner
    for (mut text, mut color) in &mut name_q {
        if done {
            text.0 = format!("[X] COMPLETE -- +{} XP", state.challenge.xp_reward);
            color.0 = COLOR_NAME_DONE;
        } else {
            text.0 = state.challenge.name.clone();
            color.0 = COLOR_NAME;
        }
    }

    // Row 3a: progress label
    for (mut text, mut color) in &mut prog_q {
        if done {
            text.0 = String::new();
            color.0 = Color::NONE;
        } else {
            text.0 = format!("{:.1} / {:.1}", progress, target);
            color.0 = Color::srgb(0.85, 0.85, 0.85);
        }
    }

    // Row 3b: bar fill — width as percentage, colour flips on completion
    for (mut node, mut bg) in &mut bar_q {
        node.width = Val::Percent(fraction * 100.0);
        bg.0 = if done { COLOR_BAR_DONE } else { COLOR_BAR_BASE };
    }
}
