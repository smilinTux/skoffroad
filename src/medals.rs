// Medals: per-challenge bronze/silver/gold awarded based on completion
// times or scores. Persists best medal per challenge for the session
// (and via SavePlugin for persistence between runs).
//
// Public API:
//   MedalsPlugin
//   MedalsState { best: HashMap<MedalChallenge, Medal> }
//   Medal enum (None < Bronze < Silver < Gold)
//   MedalChallenge enum

use bevy::prelude::*;
use std::collections::HashMap;

use crate::airtime::AirtimeStats;
use crate::collectibles::CollectibleCount;
use crate::course::CourseState;
use crate::hud::SessionStats;
use crate::race::{RacePhase, RaceState};

// ---------------------------------------------------------------------------
// Public plugin
// ---------------------------------------------------------------------------

pub struct MedalsPlugin;

impl Plugin for MedalsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MedalsState>()
            .init_resource::<MedalPopupQueue>()
            .init_resource::<CabinetVisible>()
            .add_systems(Startup, (spawn_medal_popup, spawn_cabinet_panel))
            .add_systems(
                Update,
                (
                    check_course_medal,
                    check_race_medal,
                    check_score_medals,
                    update_medal_popup,
                    toggle_cabinet_with_m,
                    update_cabinet,
                ),
            );
    }
}

// ---------------------------------------------------------------------------
// Public resources
// ---------------------------------------------------------------------------

#[derive(Resource, Default, Clone)]
pub struct MedalsState {
    pub best: HashMap<MedalChallenge, Medal>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum Medal {
    #[default]
    None,
    Bronze,
    Silver,
    Gold,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MedalChallenge {
    CourseTime,
    RaceVsRivals,
    GemCollector,
    Airtime,
    TopSpeed,
}

// ---------------------------------------------------------------------------
// Threshold helpers
// ---------------------------------------------------------------------------

fn course_time_medal(run_time_s: f32) -> Medal {
    if run_time_s <= 50.0 {
        Medal::Gold
    } else if run_time_s <= 70.0 {
        Medal::Silver
    } else if run_time_s <= 90.0 {
        Medal::Bronze
    } else {
        Medal::None
    }
}

fn race_position_medal(position: usize) -> Medal {
    match position {
        1 => Medal::Gold,
        2 => Medal::Silver,
        3 => Medal::Bronze,
        _ => Medal::None,
    }
}

fn gem_medal(collected: u32) -> Medal {
    if collected >= 15 {
        Medal::Gold
    } else if collected >= 10 {
        Medal::Silver
    } else if collected >= 5 {
        Medal::Bronze
    } else {
        Medal::None
    }
}

fn airtime_medal(max_air_s: f32) -> Medal {
    if max_air_s >= 3.0 {
        Medal::Gold
    } else if max_air_s >= 2.0 {
        Medal::Silver
    } else if max_air_s >= 1.0 {
        Medal::Bronze
    } else {
        Medal::None
    }
}

fn speed_medal(top_speed_mph: f32) -> Medal {
    if top_speed_mph >= 80.0 {
        Medal::Gold
    } else if top_speed_mph >= 65.0 {
        Medal::Silver
    } else if top_speed_mph >= 50.0 {
        Medal::Bronze
    } else {
        Medal::None
    }
}

fn challenge_display_name(challenge: MedalChallenge) -> &'static str {
    match challenge {
        MedalChallenge::CourseTime   => "Course Time",
        MedalChallenge::RaceVsRivals => "Race vs Rivals",
        MedalChallenge::GemCollector => "Gem Collector",
        MedalChallenge::Airtime      => "Airtime",
        MedalChallenge::TopSpeed     => "Top Speed",
    }
}

fn medal_prefix(medal: Medal) -> &'static str {
    match medal {
        Medal::Gold   => "** GOLD **",
        Medal::Silver => "** SILVER **",
        Medal::Bronze => "** BRONZE **",
        Medal::None   => "",
    }
}

fn medal_color(medal: Medal) -> Color {
    match medal {
        Medal::Gold   => Color::srgb(1.0, 0.85, 0.2),
        Medal::Silver => Color::srgb(0.85, 0.85, 0.9),
        Medal::Bronze => Color::srgb(0.85, 0.55, 0.3),
        Medal::None   => Color::WHITE,
    }
}

// ---------------------------------------------------------------------------
// Internal resources & components
// ---------------------------------------------------------------------------

/// Queue of medals to pop up (at most one visible at a time).
#[derive(Resource, Default)]
struct MedalPopupQueue {
    pending: std::collections::VecDeque<(MedalChallenge, Medal)>,
}

/// Marker on the popup root node.
#[derive(Component)]
struct MedalPopupRoot;

/// Text node inside the popup.
#[derive(Component)]
struct MedalPopupText;

/// Tracks the popup display timer.
#[derive(Component)]
struct MedalPopupTimer {
    elapsed: f32,
}

/// Marker on the cabinet root node.
#[derive(Component)]
struct MedalCabinetRoot;

/// Marker on cabinet row text nodes (indexed by MedalChallenge order).
#[derive(Component)]
struct CabinetRowText(usize);

/// Tracks whether the cabinet is open.
#[derive(Resource, Default)]
struct CabinetVisible(bool);

// ---------------------------------------------------------------------------
// Startup systems
// ---------------------------------------------------------------------------

fn spawn_medal_popup(mut commands: Commands) {
    // Hidden popup container — centred mid-screen.
    let root = commands
        .spawn((
            MedalPopupRoot,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Percent(50.0),
                top: Val::Percent(40.0),
                margin: UiRect {
                    left: Val::Px(-240.0),
                    ..default()
                },
                width: Val::Px(480.0),
                height: Val::Px(70.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
            Visibility::Hidden,
        ))
        .id();

    let text = commands
        .spawn((
            MedalPopupText,
            Text::new(""),
            TextFont {
                font_size: 30.0,
                ..default()
            },
            TextColor(Color::WHITE),
        ))
        .id();

    commands.entity(root).add_child(text);
}

fn spawn_cabinet_panel(mut commands: Commands) {
    const PANEL_W: f32 = 400.0;
    const PANEL_H: f32 = 340.0;

    let root = commands
        .spawn((
            MedalCabinetRoot,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Percent(50.0),
                top: Val::Percent(50.0),
                margin: UiRect {
                    left: Val::Px(-PANEL_W / 2.0),
                    top: Val::Px(-PANEL_H / 2.0),
                    ..default()
                },
                width: Val::Px(PANEL_W),
                height: Val::Px(PANEL_H),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(20.0)),
                row_gap: Val::Px(10.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.05, 0.05, 0.10, 0.95)),
            Visibility::Hidden,
            ZIndex(400),
        ))
        .id();

    // Title row
    let title = commands
        .spawn((
            Text::new("MEDAL CABINET"),
            TextFont {
                font_size: 22.0,
                ..default()
            },
            TextColor(Color::srgb(0.95, 0.92, 0.80)),
        ))
        .id();

    commands.entity(root).add_child(title);

    // One row per challenge
    let challenges = [
        MedalChallenge::CourseTime,
        MedalChallenge::RaceVsRivals,
        MedalChallenge::GemCollector,
        MedalChallenge::Airtime,
        MedalChallenge::TopSpeed,
    ];

    for (i, _) in challenges.iter().enumerate() {
        let row = commands
            .spawn((
                CabinetRowText(i),
                Text::new(""),
                TextFont {
                    font_size: 16.0,
                    ..default()
                },
                TextColor(Color::srgb(0.85, 0.85, 0.85)),
            ))
            .id();
        commands.entity(root).add_child(row);
    }

    // Hint row
    let hint = commands
        .spawn((
            Text::new("Press M to close"),
            TextFont {
                font_size: 12.0,
                ..default()
            },
            TextColor(Color::srgb(0.55, 0.55, 0.60)),
        ))
        .id();

    commands.entity(root).add_child(hint);
}

// ---------------------------------------------------------------------------
// Award helper — returns true if the medal is new best and queues a popup.
// ---------------------------------------------------------------------------

fn maybe_award(
    challenge: MedalChallenge,
    new_medal: Medal,
    state: &mut MedalsState,
    queue: &mut MedalPopupQueue,
) {
    if new_medal == Medal::None {
        return;
    }
    let current_best = state.best.get(&challenge).copied().unwrap_or(Medal::None);
    if new_medal > current_best {
        state.best.insert(challenge, new_medal);
        queue.pending.push_back((challenge, new_medal));
        info!(
            "medals: {} earned {:?} for {:?}",
            medal_prefix(new_medal),
            new_medal,
            challenge
        );
    }
}

// ---------------------------------------------------------------------------
// Edge-detection systems
// ---------------------------------------------------------------------------

fn check_course_medal(
    course: Option<Res<CourseState>>,
    mut state: ResMut<MedalsState>,
    mut queue: ResMut<MedalPopupQueue>,
    mut seen_completed: Local<bool>,
) {
    let Some(course) = course else { return };

    // Rising-edge detection: only award when `completed` flips from false→true.
    if course.completed && !*seen_completed {
        *seen_completed = true;
        let medal = course_time_medal(course.run_time_s);
        maybe_award(MedalChallenge::CourseTime, medal, &mut state, &mut queue);
    }

    // Reset the seen flag when course resets (completed goes false).
    if !course.completed {
        *seen_completed = false;
    }
}

fn check_race_medal(
    race: Option<Res<RaceState>>,
    mut state: ResMut<MedalsState>,
    mut queue: ResMut<MedalPopupQueue>,
    mut seen_finished: Local<bool>,
) {
    let Some(race) = race else { return };

    if race.phase == RacePhase::Finished && !*seen_finished {
        *seen_finished = true;

        // Find the player's finishing position (1-based rank in sorted leaderboard).
        let position = race
            .leaderboard
            .iter()
            .position(|e| e.is_player)
            .map(|idx| idx + 1)
            .unwrap_or(usize::MAX);

        let medal = race_position_medal(position);
        maybe_award(MedalChallenge::RaceVsRivals, medal, &mut state, &mut queue);
    }

    // Reset when race goes back to Lobby for a rematch.
    if race.phase == RacePhase::Lobby {
        *seen_finished = false;
    }
}

fn check_score_medals(
    gems: Option<Res<CollectibleCount>>,
    air: Option<Res<AirtimeStats>>,
    stats: Option<Res<SessionStats>>,
    mut state: ResMut<MedalsState>,
    mut queue: ResMut<MedalPopupQueue>,
) {
    if let Some(gems) = gems {
        let medal = gem_medal(gems.collected);
        maybe_award(MedalChallenge::GemCollector, medal, &mut state, &mut queue);
    }

    if let Some(air) = air {
        let medal = airtime_medal(air.max_air_s);
        maybe_award(MedalChallenge::Airtime, medal, &mut state, &mut queue);
    }

    if let Some(stats) = stats {
        // SessionStats stores max_speed_mps; convert to mph for threshold.
        let top_mph = stats.max_speed_mps * 2.237;
        let medal = speed_medal(top_mph);
        maybe_award(MedalChallenge::TopSpeed, medal, &mut state, &mut queue);
    }
}

// ---------------------------------------------------------------------------
// Popup system
// ---------------------------------------------------------------------------

fn update_medal_popup(
    mut commands: Commands,
    time: Res<Time>,
    mut queue: ResMut<MedalPopupQueue>,
    // Popup root: visibility + bg alpha
    mut root_q: Query<
        (Entity, &mut Visibility, &mut BackgroundColor, Option<&mut MedalPopupTimer>),
        With<MedalPopupRoot>,
    >,
    // Text node inside popup
    mut text_q: Query<(&mut Text, &mut TextColor), With<MedalPopupText>>,
    // Children of popup root (to grab the text entity for the timer)
    children_q: Query<&Children, With<MedalPopupRoot>>,
) {
    const POPUP_DURATION: f32 = 3.0;
    let dt = time.delta_secs();

    let Ok((root_entity, mut vis, mut bg, maybe_timer)) = root_q.single_mut() else {
        return;
    };

    match maybe_timer {
        // Popup currently active — advance timer.
        Some(mut timer) => {
            timer.elapsed += dt;
            let t = timer.elapsed;
            // Fade: first 0.3 s in, last 0.5 s out.
            let alpha = if t < 0.3 {
                t / 0.3
            } else if t < POPUP_DURATION - 0.5 {
                1.0
            } else {
                let fade_t = t - (POPUP_DURATION - 0.5);
                (1.0 - fade_t / 0.5).max(0.0)
            };

            bg.0 = Color::srgba(0.0, 0.0, 0.0, 0.70 * alpha);

            // Update text alpha via the text child.
            if let Ok(children) = children_q.get(root_entity) {
                for child in children.iter() {
                    if let Ok((_, mut tc)) = text_q.get_mut(child) {
                        // Re-read medal color with alpha.
                        // We stored the medal color implicitly; read it back.
                        let base = tc.0.to_linear();
                        tc.0 = Color::linear_rgba(base.red, base.green, base.blue, alpha);
                    }
                }
            }

            if t >= POPUP_DURATION {
                // Dismiss.
                commands.entity(root_entity).remove::<MedalPopupTimer>();
                *vis = Visibility::Hidden;
            }
        }

        // No active popup — try to dequeue one.
        None => {
            if let Some((challenge, medal)) = queue.pending.pop_front() {
                // Set text content.
                let label = challenge_display_name(challenge);
                let prefix = medal_prefix(medal);
                let msg = format!("{} {}", prefix, label);

                // Find the text child and update it.
                if let Ok(children) = children_q.get(root_entity) {
                    for child in children.iter() {
                        if let Ok((mut text, mut tc)) = text_q.get_mut(child) {
                            text.0 = msg.clone();
                            tc.0 = medal_color(medal);
                        }
                    }
                }

                *vis = Visibility::Inherited;
                bg.0 = Color::srgba(0.0, 0.0, 0.0, 0.0);
                commands
                    .entity(root_entity)
                    .insert(MedalPopupTimer { elapsed: 0.0 });
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Cabinet toggle (M key)
// ---------------------------------------------------------------------------

fn toggle_cabinet_with_m(
    keys: Res<ButtonInput<KeyCode>>,
    mut visible: ResMut<CabinetVisible>,
    mut root_q: Query<&mut Visibility, With<MedalCabinetRoot>>,
) {
    if keys.just_pressed(KeyCode::KeyM) {
        visible.0 = !visible.0;
        for mut vis in &mut root_q {
            *vis = if visible.0 {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
        }
    }
}

// ---------------------------------------------------------------------------
// Cabinet content update
// ---------------------------------------------------------------------------

fn update_cabinet(
    state: Res<MedalsState>,
    mut row_q: Query<(&CabinetRowText, &mut Text, &mut TextColor)>,
) {
    const CHALLENGES: [MedalChallenge; 5] = [
        MedalChallenge::CourseTime,
        MedalChallenge::RaceVsRivals,
        MedalChallenge::GemCollector,
        MedalChallenge::Airtime,
        MedalChallenge::TopSpeed,
    ];

    for (row, mut text, mut color) in &mut row_q {
        let challenge = CHALLENGES[row.0];
        let best = state.best.get(&challenge).copied().unwrap_or(Medal::None);
        let name = challenge_display_name(challenge);

        let medal_str = match best {
            Medal::Gold   => "GOLD",
            Medal::Silver => "SILVER",
            Medal::Bronze => "BRONZE",
            Medal::None   => "-",
        };

        // Pad with dots to align medal word at column ~30.
        let dots_needed = 30usize.saturating_sub(name.len() + medal_str.len());
        let dots: String = ".".repeat(dots_needed);
        text.0 = format!("{}{}{}", name, dots, medal_str);
        color.0 = medal_color(best);
    }
}
