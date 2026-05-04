// Achievement detection and ephemeral toast notifications.
//
// Each Update: evaluate conditions, push newly-earned IDs to pending_toasts.
// spawn_toasts consumes pending_toasts, spawning 280×60 toast panels.
// update_toasts advances each toast through fade-in (0.3 s), hold (4 s), fade-out (0.3 s)
// then despawns them.

use bevy::prelude::*;
use std::collections::{HashSet, VecDeque};

use crate::compass::Waypoint;
use crate::events::{EventLog, GameEvent};
use crate::hud::SessionStats;
use crate::livery::LiveryState;

// ---- Public plugin ----------------------------------------------------------

pub struct AchievementToastPlugin;

impl Plugin for AchievementToastPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<EarnedAchievements>()
            .add_systems(Update, (detect_achievements, spawn_toasts, update_toasts));
    }
}

// ---- Achievement definitions ------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
#[repr(u8)]
pub enum AchievementId {
    SubMileRookie     = 0,
    Marathoner        = 1,
    AirTimePilot      = 2,
    Daredevil         = 3,
    DemolitionDerby   = 4,
    WaypointHunter    = 5,
    LiveryConnoisseur = 6,
    LongHaul          = 7,
}

struct Achievement {
    pub id: AchievementId,
    pub name: &'static str,
    pub description: &'static str,
}

const ACHIEVEMENTS: &[Achievement] = &[
    Achievement {
        id: AchievementId::SubMileRookie,
        name: "Sub-Mile Rookie",
        description: "Drive more than 1 mile (1.6 km).",
    },
    Achievement {
        id: AchievementId::Marathoner,
        name: "Marathoner",
        description: "Drive more than 5 km in a single session.",
    },
    Achievement {
        id: AchievementId::AirTimePilot,
        name: "Air Time Pilot",
        description: "Get airborne for the first time.",
    },
    Achievement {
        id: AchievementId::Daredevil,
        name: "Daredevil",
        description: "Sustain 3 big-tilt warnings.",
    },
    Achievement {
        id: AchievementId::DemolitionDerby,
        name: "Demolition Derby",
        description: "Take 5 hard impacts.",
    },
    Achievement {
        id: AchievementId::WaypointHunter,
        name: "Waypoint Hunter",
        description: "Reach 3 waypoints.",
    },
    Achievement {
        id: AchievementId::LiveryConnoisseur,
        name: "Livery Connoisseur",
        description: "Try all 5 paint liveries.",
    },
    Achievement {
        id: AchievementId::LongHaul,
        name: "Long Haul",
        description: "Play for 10 minutes straight.",
    },
];

// ---- Public resource --------------------------------------------------------

#[derive(Resource, Default)]
pub struct EarnedAchievements {
    /// Set of earned achievement IDs (stored as u8 discriminants).
    pub earned: HashSet<u8>,
    /// Queue of recently earned IDs awaiting toast display.
    pub pending_toasts: VecDeque<u8>,
}

// ---- Detection system -------------------------------------------------------

pub fn detect_achievements(
    stats:   Option<Res<SessionStats>>,
    log:     Option<Res<EventLog>>,
    waypoint: Option<Res<Waypoint>>,
    livery:  Option<Res<LiveryState>>,
    mut earned: ResMut<EarnedAchievements>,
    // Tracks which livery indices have been seen this session.
    mut seen_liveries: Local<HashSet<u8>>,
) {
    // Count event types from the log.
    let (airtime_count, big_tilt_count, hard_impact_count) = if let Some(ref log) = log {
        let mut a = 0u32;
        let mut t = 0u32;
        let mut h = 0u32;
        for (_, ev) in &log.events {
            match ev {
                GameEvent::Airtime { .. }    => a += 1,
                GameEvent::BigTilt { .. }    => t += 1,
                GameEvent::HardImpact { .. } => h += 1,
                _ => {}
            }
        }
        (a, t, h)
    } else {
        (0, 0, 0)
    };

    let distance_m  = stats.as_ref().map(|s| s.distance_m).unwrap_or(0.0);
    let elapsed_s   = stats.as_ref().map(|s| s.elapsed_s).unwrap_or(0.0);
    let reached     = waypoint.as_ref().map(|w| w.reached_count).unwrap_or(0);

    // Track liveries seen; update the Local set.
    if let Some(ref ls) = livery {
        seen_liveries.insert(ls.current);
    }
    let all_liveries_seen = seen_liveries.len() >= 5;

    // Table: (AchievementId discriminant, condition met).
    let checks: &[(u8, bool)] = &[
        (AchievementId::SubMileRookie     as u8, distance_m >= 1_600.0),
        (AchievementId::Marathoner        as u8, distance_m >= 5_000.0),
        (AchievementId::AirTimePilot      as u8, airtime_count >= 1),
        (AchievementId::Daredevil         as u8, big_tilt_count >= 3),
        (AchievementId::DemolitionDerby   as u8, hard_impact_count >= 5),
        (AchievementId::WaypointHunter    as u8, reached >= 3),
        (AchievementId::LiveryConnoisseur as u8, all_liveries_seen),
        (AchievementId::LongHaul          as u8, elapsed_s >= 600.0),
    ];

    for &(id, condition) in checks {
        if condition && !earned.earned.contains(&id) {
            earned.earned.insert(id);
            earned.pending_toasts.push_back(id);
        }
    }
}

// ---- Toast component & constants --------------------------------------------

const TOAST_W: f32       = 280.0;
const TOAST_H: f32       = 60.0;
const TOAST_RIGHT: f32   = 12.0; // resting right-edge offset from screen edge
const FADE_IN_S: f32     = 0.3;
const HOLD_S: f32        = 4.0;
const FADE_OUT_S: f32    = 0.3;
const TOAST_TOTAL: f32   = FADE_IN_S + HOLD_S + FADE_OUT_S;

const TOAST_BG:    Color = Color::srgba(0.05, 0.30, 0.08, 0.92);
const TAG_COLOR:   Color = Color::srgb(0.95, 0.72, 0.20);  // amber
const NAME_COLOR:  Color = Color::WHITE;
const DESC_COLOR:  Color = Color::srgb(0.75, 0.78, 0.82);  // light grey

/// Tracks per-toast lifetime and vertical stack slot.
#[derive(Component)]
struct ToastTimer {
    elapsed: f32,
    slot: usize,
}

/// Marker for the three text spans within a toast.
#[derive(Component, Clone, Copy)]
enum ToastSpan {
    Tag,
    Name,
    Desc,
}

// ---- spawn_toasts -----------------------------------------------------------

pub fn spawn_toasts(
    mut commands:  Commands,
    mut earned:    ResMut<EarnedAchievements>,
    // Count currently alive toasts to assign a vertical stack slot.
    existing_q: Query<&ToastTimer>,
) {
    while let Some(id) = earned.pending_toasts.pop_front() {
        // Find the achievement definition.
        let Some(ach) = ACHIEVEMENTS.iter().find(|a| a.id as u8 == id) else {
            continue;
        };

        // Stack slot: count live toasts; each slot is TOAST_H + 8 px gap.
        let slot = existing_q.iter().count();
        let bottom_offset = 12.0 + slot as f32 * (TOAST_H + 8.0);

        let root = commands.spawn((
            ToastTimer { elapsed: 0.0, slot },
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(TOAST_RIGHT),
                bottom: Val::Px(bottom_offset),
                width: Val::Px(TOAST_W),
                height: Val::Px(TOAST_H),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::SpaceEvenly,
                padding: UiRect::axes(Val::Px(10.0), Val::Px(6.0)),
                ..default()
            },
            BackgroundColor(TOAST_BG),
            ZIndex(300),
        )).id();

        // "ACHIEVEMENT" tag line
        let tag = commands.spawn((
            ToastSpan::Tag,
            Text::new("ACHIEVEMENT"),
            TextFont { font_size: 11.0, ..default() },
            TextColor(TAG_COLOR),
        )).id();

        // Achievement name line
        let name = commands.spawn((
            ToastSpan::Name,
            Text::new(ach.name),
            TextFont { font_size: 16.0, ..default() },
            TextColor(NAME_COLOR),
        )).id();

        // Description line
        let desc = commands.spawn((
            ToastSpan::Desc,
            Text::new(ach.description),
            TextFont { font_size: 12.0, ..default() },
            TextColor(DESC_COLOR),
        )).id();

        commands.entity(root).add_children(&[tag, name, desc]);
    }
}

// ---- update_toasts ----------------------------------------------------------

pub fn update_toasts(
    mut commands: Commands,
    time: Res<Time>,
    mut toast_q: Query<(Entity, &mut ToastTimer, &mut BackgroundColor, &Children)>,
    mut text_q: Query<(&ToastSpan, &mut TextColor)>,
) {
    let dt = time.delta_secs();

    for (entity, mut timer, mut bg, children) in &mut toast_q {
        timer.elapsed += dt;
        let t = timer.elapsed;

        // Alpha curve: fade in → hold at 1 → fade out.
        let alpha = if t < FADE_IN_S {
            t / FADE_IN_S
        } else if t < FADE_IN_S + HOLD_S {
            1.0
        } else {
            let fade_t = t - FADE_IN_S - HOLD_S;
            (1.0 - fade_t / FADE_OUT_S).max(0.0)
        };

        // Apply alpha to background (keep greenish tint).
        bg.0 = Color::srgba(0.05, 0.30, 0.08, 0.92 * alpha);

        // Apply alpha to each text child.
        for child in children.iter() {
            if let Ok((span, mut tc)) = text_q.get_mut(child) {
                let base = match span {
                    ToastSpan::Tag  => TAG_COLOR,
                    ToastSpan::Name => NAME_COLOR,
                    ToastSpan::Desc => DESC_COLOR,
                };
                let lin = base.to_linear();
                tc.0 = Color::linear_rgba(lin.red, lin.green, lin.blue, alpha);
            }
        }

        if t >= TOAST_TOTAL {
            commands.entity(entity).despawn();
        }
    }
}
