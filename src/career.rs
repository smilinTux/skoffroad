// Career mode: linear sequence of objectives. Player completes them
// one at a time; completion triggers XP award + advancement.
//
// Public API:
//   CareerPlugin
//   CareerState { objectives, current_idx, all_complete }
//   CareerObjective { name, description, kind: ObjectiveKind, completed }
//   ObjectiveKind enum

use bevy::prelude::*;

use crate::airtime::AirtimeStats;
use crate::collectibles::CollectibleCount;
use crate::course::CourseState;
use crate::hud::SessionStats;
use crate::progression::ProgressionState;
use crate::race::{RacePhase, RaceState};
use crate::xp::XpState;

// ---------------------------------------------------------------------------
// Public plugin
// ---------------------------------------------------------------------------

pub struct CareerPlugin;

impl Plugin for CareerPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(CareerState::with_objectives())
            .add_systems(Startup, spawn_career_hud)
            .add_systems(Update, (evaluate_career, update_career_hud).chain());
    }
}

// ---------------------------------------------------------------------------
// Public resources
// ---------------------------------------------------------------------------

#[derive(Resource)]
pub struct CareerState {
    pub objectives: Vec<CareerObjective>,
    pub current_idx: usize,
    pub all_complete: bool,
}

impl Default for CareerState {
    fn default() -> Self {
        Self::with_objectives()
    }
}

impl CareerState {
    fn with_objectives() -> Self {
        let objectives = vec![
            CareerObjective {
                name: "FIRST DRIVE".to_string(),
                description: "Reach progression level 2".to_string(),
                kind: ObjectiveKind::ReachLevel { level: 2 },
                completed: false,
                xp_reward: 100,
            },
            CareerObjective {
                name: "QUICK COURSE".to_string(),
                description: "Finish the course in under 90 seconds".to_string(),
                kind: ObjectiveKind::CourseUnder { seconds: 90 },
                completed: false,
                xp_reward: 200,
            },
            CareerObjective {
                name: "COLLECTOR".to_string(),
                description: "Collect 5 gems".to_string(),
                kind: ObjectiveKind::CollectGems { count: 5 },
                completed: false,
                xp_reward: 200,
            },
            CareerObjective {
                name: "BREAK 50 MPH".to_string(),
                description: "Hit a top speed of 50 mph".to_string(),
                kind: ObjectiveKind::TopSpeed { mph: 50 },
                completed: false,
                xp_reward: 250,
            },
            CareerObjective {
                name: "BIG AIR".to_string(),
                description: "Get airborne for 2 seconds in a single jump".to_string(),
                kind: ObjectiveKind::Airtime { seconds: 2 },
                completed: false,
                xp_reward: 300,
            },
            CareerObjective {
                name: "FAST COURSE".to_string(),
                description: "Finish the course in under 60 seconds".to_string(),
                kind: ObjectiveKind::CourseUnder { seconds: 60 },
                completed: false,
                xp_reward: 400,
            },
            CareerObjective {
                name: "BEAT THE RIVALS".to_string(),
                description: "Win a race against rivals".to_string(),
                kind: ObjectiveKind::WinRace,
                completed: false,
                xp_reward: 500,
            },
            CareerObjective {
                name: "RANK UP".to_string(),
                description: "Reach progression level 10".to_string(),
                kind: ObjectiveKind::ReachLevel { level: 10 },
                completed: false,
                xp_reward: 800,
            },
        ];

        Self {
            objectives,
            current_idx: 0,
            all_complete: false,
        }
    }
}

#[derive(Clone, Debug)]
pub struct CareerObjective {
    pub name: String,
    pub description: String,
    pub kind: ObjectiveKind,
    pub completed: bool,
    pub xp_reward: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObjectiveKind {
    /// Finish the course in under N seconds.
    CourseUnder { seconds: u32 },
    /// Win a race against rivals.
    WinRace,
    /// Collect at least N gems.
    CollectGems { count: u32 },
    /// Reach a top speed >= N mph.
    TopSpeed { mph: u32 },
    /// Get airtime >= N seconds in a single jump.
    Airtime { seconds: u32 },
    /// Reach progression level N.
    ReachLevel { level: u32 },
}

// ---------------------------------------------------------------------------
// HUD components
// ---------------------------------------------------------------------------

#[derive(Component)]
struct CareerHudRoot;

#[derive(Component)]
enum CareerHudText {
    Header,
    Objective,
    Description,
}

// ---------------------------------------------------------------------------
// Startup: spawn career HUD panel
// ---------------------------------------------------------------------------

fn spawn_career_hud(mut commands: Commands) {
    let bg = Color::srgba(0.05, 0.05, 0.07, 0.80);

    let panel = commands
        .spawn((
            CareerHudRoot,
            Node {
                position_type: PositionType::Absolute,
                top: Val::Percent(40.0),
                left: Val::Px(14.0),
                width: Val::Px(240.0),
                height: Val::Px(80.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::FlexStart,
                padding: UiRect::axes(Val::Px(10.0), Val::Px(8.0)),
                row_gap: Val::Px(2.0),
                ..default()
            },
            BackgroundColor(bg),
        ))
        .id();

    let header_text = commands
        .spawn((
            CareerHudText::Header,
            Text::new("CAREER"),
            TextFont {
                font_size: 12.0,
                ..default()
            },
            TextColor(Color::srgb(0.6, 0.6, 0.6)),
        ))
        .id();

    let objective_text = commands
        .spawn((
            CareerHudText::Objective,
            Text::new("1/8: FIRST DRIVE"),
            TextFont {
                font_size: 16.0,
                ..default()
            },
            TextColor(Color::srgb(1.0, 0.9, 0.1)),
        ))
        .id();

    let desc_text = commands
        .spawn((
            CareerHudText::Description,
            Text::new("Reach progression level 2"),
            TextFont {
                font_size: 11.0,
                ..default()
            },
            TextColor(Color::WHITE),
        ))
        .id();

    commands
        .entity(panel)
        .add_children(&[header_text, objective_text, desc_text]);
}

// ---------------------------------------------------------------------------
// Update: evaluate current objective
// ---------------------------------------------------------------------------

fn evaluate_career(
    mut career: ResMut<CareerState>,
    mut xp: ResMut<XpState>,
    progression: Option<Res<ProgressionState>>,
    course: Option<Res<CourseState>>,
    race: Option<Res<RaceState>>,
    collectibles: Option<Res<CollectibleCount>>,
    session_stats: Option<Res<SessionStats>>,
    airtime: Option<Res<AirtimeStats>>,
    // Rising-edge locals: track previous completed state for CourseUnder and WinRace.
    mut prev_course_completed: Local<bool>,
    mut prev_race_finished: Local<bool>,
) {
    if career.all_complete {
        return;
    }

    let idx = career.current_idx;
    if idx >= career.objectives.len() {
        career.all_complete = true;
        return;
    }

    let kind = career.objectives[idx].kind;
    let xp_reward = career.objectives[idx].xp_reward;
    let name = career.objectives[idx].name.clone();

    let met = match kind {
        ObjectiveKind::ReachLevel { level } => {
            progression
                .as_ref()
                .map(|p| p.level >= level)
                .unwrap_or(false)
        }

        ObjectiveKind::CourseUnder { seconds } => {
            let currently_completed = course
                .as_ref()
                .map(|c| c.completed && c.run_time_s < seconds as f32)
                .unwrap_or(false);

            // Rising edge: only trigger on a fresh completion (was not completed last frame).
            let rising = currently_completed && !*prev_course_completed;
            *prev_course_completed = course.as_ref().map(|c| c.completed).unwrap_or(false);
            rising
        }

        ObjectiveKind::WinRace => {
            let currently_won = race.as_ref().map(|r| {
                r.phase == RacePhase::Finished
                    && r.leaderboard.first().map(|e| e.is_player).unwrap_or(false)
            }).unwrap_or(false);

            let rising = currently_won && !*prev_race_finished;
            *prev_race_finished = race.as_ref().map(|r| r.phase == RacePhase::Finished).unwrap_or(false);
            rising
        }

        ObjectiveKind::CollectGems { count } => {
            collectibles
                .as_ref()
                .map(|c| c.collected >= count)
                .unwrap_or(false)
        }

        ObjectiveKind::TopSpeed { mph } => {
            session_stats
                .as_ref()
                .map(|s| (s.max_speed_mps * 2.237) as u32 >= mph)
                .unwrap_or(false)
        }

        ObjectiveKind::Airtime { seconds } => {
            airtime
                .as_ref()
                .map(|a| a.max_air_s >= seconds as f32)
                .unwrap_or(false)
        }
    };

    if met {
        career.objectives[idx].completed = true;
        xp.total_xp = xp.total_xp.saturating_add(xp_reward as u64);
        xp.session_xp = xp.session_xp.saturating_add(xp_reward as u64);
        career.current_idx += 1;

        info!(
            "CAREER: {} complete (+{} XP)",
            name, xp_reward
        );

        if career.current_idx >= career.objectives.len() {
            career.all_complete = true;
            info!("CAREER: all objectives complete!");
        }
    }
}

// ---------------------------------------------------------------------------
// Update: refresh career HUD
// ---------------------------------------------------------------------------

fn update_career_hud(
    career: Res<CareerState>,
    mut texts: Query<(&CareerHudText, &mut Text, &mut TextColor)>,
) {
    let total = career.objectives.len();

    for (kind, mut text, mut color) in texts.iter_mut() {
        if career.all_complete {
            match kind {
                CareerHudText::Header => {
                    text.0 = "CAREER".to_string();
                    color.0 = Color::srgb(0.6, 0.6, 0.6);
                }
                CareerHudText::Objective => {
                    text.0 = "CAREER COMPLETE!".to_string();
                    color.0 = Color::srgb(1.0, 0.85, 0.2);
                }
                CareerHudText::Description => {
                    text.0 = String::new();
                    color.0 = Color::NONE;
                }
            }
        } else {
            let idx = career.current_idx.min(total - 1);
            let obj = &career.objectives[idx];
            match kind {
                CareerHudText::Header => {
                    text.0 = "CAREER".to_string();
                    color.0 = Color::srgb(0.6, 0.6, 0.6);
                }
                CareerHudText::Objective => {
                    text.0 = format!("{}/{}: {}", idx + 1, total, obj.name);
                    color.0 = Color::srgb(1.0, 0.9, 0.1);
                }
                CareerHudText::Description => {
                    text.0 = obj.description.clone();
                    color.0 = Color::WHITE;
                }
            }
        }
    }
}
