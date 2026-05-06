// Impact flash: brief red screen overlay when a HardImpact event fires.
// Flash alpha rises to 0.3 instantly, decays to 0 over 0.4s. Reads
// EventLog watermark like mixer.rs and combo.rs.
//
// Public API:
//   ImpactFlashPlugin

use bevy::prelude::*;

use crate::events::{EventLog, GameEvent};

// ---- Public API ----------------------------------------------------------------

pub struct ImpactFlashPlugin;

impl Plugin for ImpactFlashPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ImpactFlashState>()
           .add_systems(Startup, spawn_overlay)
           .add_systems(Update, (
               detect_impact,
               tick_alpha,
               update_overlay,
           ).chain());
    }
}

// ---- Resource ------------------------------------------------------------------

#[derive(Resource, Default)]
pub struct ImpactFlashState {
    pub alpha: f32,
}

// ---- Marker component ----------------------------------------------------------

#[derive(Component)]
pub struct ImpactFlashOverlay;

// ---- Constants -----------------------------------------------------------------

/// Peak alpha applied instantly on HardImpact.
const FLASH_PEAK: f32 = 0.3;

/// Duration (seconds) over which the flash fully decays to 0.
const FLASH_DECAY_S: f32 = 0.4;

/// ZIndex: above gameplay HUD (800), below loading screen (900+).
const ZINDEX: i32 = 850;

// ---- Startup -------------------------------------------------------------------

fn spawn_overlay(mut commands: Commands) {
    commands.spawn((
        ImpactFlashOverlay,
        Node {
            width:         Val::Percent(100.0),
            height:        Val::Percent(100.0),
            position_type: PositionType::Absolute,
            ..default()
        },
        BackgroundColor(Color::srgba(0.9, 0.10, 0.10, 0.0)),
        ZIndex(ZINDEX),
    ));
}

// ---- Systems -------------------------------------------------------------------

/// Walk EventLog for new HardImpact events (watermark pattern from mixer.rs).
fn detect_impact(
    event_log: Option<Res<EventLog>>,
    mut state: ResMut<ImpactFlashState>,
    mut last_seen: Local<f32>,
) {
    let Some(event_log) = event_log else { return };

    let mut newest_ts = *last_seen;

    for (ts, ev) in &event_log.events {
        if *ts <= *last_seen {
            continue;
        }
        if *ts > newest_ts {
            newest_ts = *ts;
        }
        if let GameEvent::HardImpact { .. } = ev {
            state.alpha = FLASH_PEAK;
        }
    }

    *last_seen = newest_ts;
}

/// Decay alpha toward 0 at a rate of FLASH_PEAK / FLASH_DECAY_S per second.
fn tick_alpha(
    time:      Res<Time>,
    mut state: ResMut<ImpactFlashState>,
) {
    let dt = time.delta_secs();
    state.alpha = (state.alpha - dt / FLASH_DECAY_S).max(0.0);
}

/// Push the current alpha into the overlay node's BackgroundColor.
fn update_overlay(
    state:        Res<ImpactFlashState>,
    mut overlays: Query<&mut BackgroundColor, With<ImpactFlashOverlay>>,
) {
    for mut bg in &mut overlays {
        bg.0 = Color::srgba(0.9, 0.10, 0.10, state.alpha);
    }
}
