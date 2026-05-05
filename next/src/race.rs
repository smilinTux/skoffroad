// Race state machine + leaderboard.
//
// Phases:
//   Lobby     — pre-race; rivals visible but stationary.
//   Countdown — 3-2-1-GO (3 seconds). Rivals frozen.
//   Active    — racing; lap counting + leaderboard updates each frame.
//   Finished  — all racers crossed the finish line; results table.
//
// Public API:
//   RacePlugin
//   RaceState { phase, leaderboard, total_laps, countdown_remaining }
//   RacePhase enum
//   RaceEntry  — one row in the leaderboard

use bevy::prelude::*;
use avian3d::prelude::{LinearVelocity, AngularVelocity};

use crate::ai_path::PathFollower;
use crate::rival::{Rival, RivalChassis};
use crate::vehicle::{Chassis, VehicleRoot};

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct RacePlugin;

impl Plugin for RacePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(RaceState {
                total_laps: 2,
                countdown_remaining: 3.0,
                ..default()
            })
            .add_systems(Startup, spawn_race_hud)
            .add_systems(
                Update,
                (
                    start_race_input,
                    tick_race_phase,
                    update_leaderboard,
                    freeze_rivals,
                    update_race_hud,
                )
                    .chain(),
            );
    }
}

// ---------------------------------------------------------------------------
// Resources / components
// ---------------------------------------------------------------------------

#[derive(Resource, Default)]
pub struct RaceState {
    pub phase: RacePhase,
    pub countdown_remaining: f32,
    pub leaderboard: Vec<RaceEntry>,
    pub total_laps: u32,
    /// Seconds elapsed since Active phase began.
    pub elapsed_s: f32,
}

#[derive(Default, Clone, Copy, PartialEq, Eq, Debug)]
pub enum RacePhase {
    #[default]
    Lobby,
    Countdown,
    Active,
    Finished,
}

#[derive(Clone, Debug)]
pub struct RaceEntry {
    pub entity: Entity,
    pub name: String,
    pub is_player: bool,
    pub progress: f32,
    pub lap: u32,
    pub finished: bool,
    pub finish_time_s: Option<f32>,
}

/// Marker on the centre-screen countdown / status Text node.
#[derive(Component)]
pub struct RaceCenterMessage;

// ---------------------------------------------------------------------------
// Startup: spawn the HUD text node
// ---------------------------------------------------------------------------

fn spawn_race_hud(mut commands: Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                // Centre horizontally and sit near the top third of the screen.
                left: Val::Percent(50.0),
                top: Val::Percent(35.0),
                ..default()
            },
        ))
        .with_children(|parent| {
            parent.spawn((
                RaceCenterMessage,
                Text::new("PRESS R TO START RACE"),
                TextFont { font_size: 48.0, ..default() },
                TextColor(Color::srgba(1.0, 1.0, 0.2, 0.95)),
                // Shift left by roughly half the text width so it looks centred.
                Node {
                    left: Val::Px(-280.0),
                    ..default()
                },
            ));
        });
}

// ---------------------------------------------------------------------------
// System: handle R key to transition Lobby → Countdown and Finished → Lobby
// ---------------------------------------------------------------------------

pub fn start_race_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<RaceState>,
) {
    if keys.just_pressed(KeyCode::KeyR) {
        match state.phase {
            RacePhase::Lobby => {
                state.phase = RacePhase::Countdown;
                state.countdown_remaining = 3.0;
                state.elapsed_s = 0.0;
                info!("Race countdown started!");
            }
            RacePhase::Finished => {
                // Reset to Lobby for a rematch.
                state.phase = RacePhase::Lobby;
                state.leaderboard.clear();
                state.elapsed_s = 0.0;
                info!("Returning to lobby for rematch.");
            }
            _ => {}
        }
    }
}

// ---------------------------------------------------------------------------
// System: drive the phase state machine
// ---------------------------------------------------------------------------

pub fn tick_race_phase(
    time: Res<Time>,
    mut state: ResMut<RaceState>,
    mut follower_q: Query<&mut PathFollower>,
    vehicle: Option<Res<VehicleRoot>>,
) {
    let dt = time.delta_secs();

    match state.phase {
        RacePhase::Lobby | RacePhase::Finished => {}

        RacePhase::Countdown => {
            state.countdown_remaining -= dt;

            if state.countdown_remaining <= 0.0 {
                state.phase = RacePhase::Active;
                state.elapsed_s = 0.0;

                // Reset PathFollower progress for every racer (player + rivals).
                for mut follower in &mut follower_q {
                    follower.current_idx = 0;
                    follower.lap = 0;
                    follower.total_distance = 0.0;
                }

                // Also reset the player's PathFollower if the VehicleRoot exists and
                // the chassis itself carries one.
                if let Some(vr) = vehicle.as_ref() {
                    if let Ok(mut pf) = follower_q.get_mut(vr.chassis) {
                        pf.current_idx = 0;
                        pf.lap = 0;
                        pf.total_distance = 0.0;
                    }
                }

                info!("Race started — GO!");
            }
        }

        RacePhase::Active => {
            state.elapsed_s += dt;

            // Check finish conditions from the leaderboard entries (built last
            // frame by update_leaderboard which runs before us in the chain).
            let total_laps = state.total_laps;
            let elapsed = state.elapsed_s;
            for entry in &mut state.leaderboard {
                if !entry.finished && entry.lap >= total_laps {
                    entry.finished = true;
                    entry.finish_time_s = Some(elapsed);
                    info!("{} finished in {:.2}s", entry.name, elapsed);
                }
            }

            let all_finished = !state.leaderboard.is_empty()
                && state.leaderboard.iter().all(|e| e.finished);

            if all_finished {
                state.phase = RacePhase::Finished;
                info!("Race complete!");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// System: build the leaderboard every frame from live PathFollower data
// ---------------------------------------------------------------------------

pub fn update_leaderboard(
    mut state: ResMut<RaceState>,
    vehicle: Option<Res<VehicleRoot>>,
    // Player chassis carries Chassis marker.
    player_q: Query<(Entity, &PathFollower), With<Chassis>>,
    // Rivals carry both Rival and RivalChassis markers.
    rival_q: Query<(Entity, &PathFollower, &Rival), With<RivalChassis>>,
) {
    // Collect current per-entity finish info keyed by entity so we can
    // preserve finished state across frame rebuilds.
    let mut prev: std::collections::HashMap<Entity, (bool, Option<f32>)> =
        state.leaderboard.iter().map(|e| (e.entity, (e.finished, e.finish_time_s))).collect();

    let mut entries: Vec<RaceEntry> = Vec::new();

    // --- Player ---
    if let Some(vr) = &vehicle {
        // Try the chassis entity directly first.
        if let Ok((entity, follower)) = player_q.get(vr.chassis) {
            let (finished, finish_time_s) = prev.remove(&entity).unwrap_or((false, None));
            entries.push(RaceEntry {
                entity,
                name: "YOU".to_string(),
                is_player: true,
                progress: follower.total_distance,
                lap: follower.lap,
                finished,
                finish_time_s,
            });
        } else {
            // Fallback: iterate in case the query yields a single player entity.
            for (entity, follower) in &player_q {
                let (finished, finish_time_s) = prev.remove(&entity).unwrap_or((false, None));
                entries.push(RaceEntry {
                    entity,
                    name: "YOU".to_string(),
                    is_player: true,
                    progress: follower.total_distance,
                    lap: follower.lap,
                    finished,
                    finish_time_s,
                });
                break; // only one player
            }
        }
    }

    // --- Rivals ---
    for (entity, follower, rival) in &rival_q {
        let (finished, finish_time_s) = prev.remove(&entity).unwrap_or((false, None));
        entries.push(RaceEntry {
            entity,
            name: rival.name.clone(),
            is_player: false,
            progress: follower.total_distance,
            lap: follower.lap,
            finished,
            finish_time_s,
        });
    }

    // Sort: finished entries first (by finish_time ascending), then unfinished
    // by progress descending.
    entries.sort_by(|a, b| {
        match (a.finished, b.finished) {
            (true, true) => {
                let ta = a.finish_time_s.unwrap_or(f32::MAX);
                let tb = b.finish_time_s.unwrap_or(f32::MAX);
                ta.partial_cmp(&tb).unwrap_or(std::cmp::Ordering::Equal)
            }
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            (false, false) => b
                .progress
                .partial_cmp(&a.progress)
                .unwrap_or(std::cmp::Ordering::Equal),
        }
    });

    state.leaderboard = entries;
}

// ---------------------------------------------------------------------------
// System: freeze rival physics during Lobby and Countdown
// ---------------------------------------------------------------------------

fn freeze_rivals(
    state: Res<RaceState>,
    mut rival_chassis_q: Query<
        (&mut LinearVelocity, &mut AngularVelocity),
        With<RivalChassis>,
    >,
) {
    if matches!(state.phase, RacePhase::Lobby | RacePhase::Countdown) {
        for (mut lv, mut av) in &mut rival_chassis_q {
            lv.0 = bevy::math::Vec3::ZERO;
            av.0 = bevy::math::Vec3::ZERO;
        }
    }
}

// ---------------------------------------------------------------------------
// System: update the centre-screen HUD text
// ---------------------------------------------------------------------------

fn update_race_hud(
    state: Res<RaceState>,
    mut msg_q: Query<(&mut Text, &mut TextColor), With<RaceCenterMessage>>,
) {
    let Ok((mut text, mut color)) = msg_q.single_mut() else { return };

    match state.phase {
        RacePhase::Lobby => {
            text.0 = "PRESS R TO START RACE".to_string();
            color.0 = Color::srgba(1.0, 1.0, 0.2, 0.95);
        }
        RacePhase::Countdown => {
            let remaining = state.countdown_remaining;
            if remaining < 0.5 {
                text.0 = "GO!".to_string();
                color.0 = Color::srgba(0.2, 1.0, 0.2, 1.0);
            } else {
                let count = remaining.ceil() as u32;
                text.0 = format!("{}", count);
                color.0 = Color::srgba(1.0, 0.6, 0.1, 1.0);
            }
        }
        RacePhase::Active => {
            text.0 = String::new();
        }
        RacePhase::Finished => {
            text.0 = "RACE COMPLETE \u{2014} PRESS R FOR REMATCH".to_string();
            color.0 = Color::srgba(0.4, 0.9, 1.0, 0.95);
        }
    }
}
