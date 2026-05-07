// Stunt score: tracks score per trick. Each completed wheelie/jump/drift
// adds points multiplied by ComboState.multiplier. HUD shows total stunt
// score bottom-right.
//
// Public API:
//   StuntScorePlugin
//   StuntScoreState (resource)

use bevy::prelude::*;

use crate::airtime::AirtimeStats;
use crate::combo::ComboState;
use crate::drift_meter::DriftMeterState;
use crate::wheelie::WheelieStats;

// ---------------------------------------------------------------------------
// Public resource
// ---------------------------------------------------------------------------

#[derive(Resource, Default, Clone, Copy)]
pub struct StuntScoreState {
    pub total: u32,
    pub last_added_t: f32,
}

// ---------------------------------------------------------------------------
// Private components
// ---------------------------------------------------------------------------

#[derive(Component)]
struct StuntScoreHudRoot;

#[derive(Component)]
struct StuntScoreHudText;

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct StuntScorePlugin;

impl Plugin for StuntScorePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<StuntScoreState>()
            .add_systems(Startup, spawn_stunt_score_hud)
            .add_systems(
                Update,
                (track_wheelies, track_jumps, track_drifts, update_hud),
            );
    }
}

// ---------------------------------------------------------------------------
// Startup: spawn HUD
// ---------------------------------------------------------------------------

fn spawn_stunt_score_hud(mut commands: Commands) {
    let root = commands
        .spawn((
            StuntScoreHudRoot,
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(90.0),
                right: Val::Px(14.0),
                padding: UiRect::all(Val::Px(6.0)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.05, 0.05, 0.05, 0.75)),
        ))
        .id();

    let label = commands
        .spawn((
            StuntScoreHudText,
            Text::new("STUNT: 0"),
            TextFont {
                font_size: 16.0,
                ..default()
            },
            TextColor(Color::srgb(1.0, 1.0, 0.0)), // yellow
        ))
        .id();

    commands.entity(root).add_child(label);
}

// ---------------------------------------------------------------------------
// System: track_wheelies
// Detects rising edge of wheelie_count (a wheelie just completed).
// Guards that the completed wheelie was meaningful (longest_wheelie_s > 0.5 s).
// ---------------------------------------------------------------------------

fn track_wheelies(
    wheelie: Option<Res<WheelieStats>>,
    combo: Res<ComboState>,
    time: Res<Time>,
    mut score: ResMut<StuntScoreState>,
    mut last_count: Local<u32>,
) {
    let Some(wheelie) = wheelie else { return };

    let current_count = wheelie.wheelie_count;

    if current_count > *last_count && wheelie.longest_wheelie_s > 0.5 {
        let pts = 50u32.saturating_mul(combo.multiplier);
        score.total = score.total.saturating_add(pts);
        score.last_added_t = time.elapsed_secs();
        info!("StuntScore +{pts} (wheelie) => {}", score.total);
    }

    *last_count = current_count;
}

// ---------------------------------------------------------------------------
// System: track_jumps
// Tracks peak airtime during a flight; awards on landing edge.
// ---------------------------------------------------------------------------

fn track_jumps(
    airtime: Option<Res<AirtimeStats>>,
    combo: Res<ComboState>,
    time: Res<Time>,
    mut score: ResMut<StuntScoreState>,
    mut was_airborne: Local<bool>,
    mut peak_air: Local<f32>,
) {
    let Some(airtime) = airtime else { return };

    let currently_airborne = airtime.airborne;

    // While airborne, keep updating peak.
    if currently_airborne {
        if !*was_airborne {
            // Fresh takeoff — reset peak.
            *peak_air = 0.0;
        }
        *peak_air = peak_air.max(airtime.current_air_s);
    }

    // Falling edge: airborne -> grounded.
    if *was_airborne && !currently_airborne {
        let peak = *peak_air;
        if peak > 0.4 {
            let base = 50u32.saturating_add((peak * 100.0) as u32);
            let pts = base.saturating_mul(combo.multiplier);
            score.total = score.total.saturating_add(pts);
            score.last_added_t = time.elapsed_secs();
            info!("StuntScore +{pts} (jump {peak:.2}s) => {}", score.total);
        }
        *peak_air = 0.0;
    }

    *was_airborne = currently_airborne;
}

// ---------------------------------------------------------------------------
// System: track_drifts
// Detects when DriftMeterState.current_score drops to 0 from a non-zero value.
// That falling edge means the drift just completed and the score was cashed out.
// ---------------------------------------------------------------------------

fn track_drifts(
    drift: Option<Res<DriftMeterState>>,
    combo: Res<ComboState>,
    time: Res<Time>,
    mut score: ResMut<StuntScoreState>,
    mut last_drift_score: Local<u32>,
) {
    let Some(drift) = drift else { return };

    let current = drift.current_score;

    // Falling edge: score was > 0 last frame and is now 0 → drift completed.
    if *last_drift_score > 50 && current == 0 {
        let pts = last_drift_score.saturating_mul(combo.multiplier);
        score.total = score.total.saturating_add(pts);
        score.last_added_t = time.elapsed_secs();
        info!(
            "StuntScore +{pts} (drift {}) => {}",
            *last_drift_score,
            score.total
        );
    }

    *last_drift_score = current;
}

// ---------------------------------------------------------------------------
// System: update_hud
// Refreshes the score text. Pulses text scale briefly when score changes.
// ---------------------------------------------------------------------------

fn update_hud(
    score: Res<StuntScoreState>,
    time: Res<Time>,
    mut text_q: Query<(&mut Text, &mut Transform), With<StuntScoreHudText>>,
) {
    for (mut text, mut transform) in &mut text_q {
        if score.is_changed() {
            text.0 = format!("STUNT: {}", score.total);
        }

        // Pulse: scale bounces for ~0.5 s after a point award.
        let elapsed_since = time.elapsed_secs() - score.last_added_t;
        let scale = if elapsed_since < 0.5 {
            // Decay from 1.3 back to 1.0 over 0.5 s.
            let t = elapsed_since / 0.5;
            1.0 + 0.3 * (1.0 - t)
        } else {
            1.0
        };
        transform.scale = Vec3::splat(scale);
    }
}
