// Map transitions: black fade overlay covering the screen. Other code triggers
// a TransitionRequest event; this plugin animates a 0→1 alpha rise, swaps
// ActiveMap to the requested kind at peak, then fades 1→0 back to gameplay.
//
// Public API:
//   TransitionPlugin
//   TransitionRequest (event)
//   TransitionState (resource)

use bevy::prelude::*;

use crate::maps::{ActiveMap, MapKind};

pub struct TransitionPlugin;

impl Plugin for TransitionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TransitionState>()
            .add_systems(Startup, spawn_overlay)
            .add_systems(
                Update,
                (tick_transition, update_overlay_alpha).chain(),
            )
            .add_observer(consume_request);
    }
}

#[derive(Event, Clone, Copy, Debug)]
pub struct TransitionRequest {
    pub target: MapKind,
}

#[derive(Resource, Default, Clone, Copy, Debug)]
pub struct TransitionState {
    /// 0..1 progress of the current transition; 0 = no transition active.
    pub alpha: f32,
    pub direction: TransitionDirection,
    pub target: Option<MapKind>,
}

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransitionDirection {
    #[default]
    Idle,
    FadeIn,
    FadeOut,
}

// ---- Marker component -------------------------------------------------------

#[derive(Component)]
struct TransitionOverlay;

// ---- Systems ----------------------------------------------------------------

fn spawn_overlay(mut commands: Commands) {
    commands.spawn((
        TransitionOverlay,
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
        ZIndex(9999),
    ));
}

fn consume_request(
    trigger: On<TransitionRequest>,
    mut state: ResMut<TransitionState>,
) {
    if state.direction != TransitionDirection::Idle {
        // Ignore requests while a transition is already running.
        return;
    }

    state.direction = TransitionDirection::FadeOut;
    state.target = Some(trigger.event().target);
    state.alpha = 0.0;
}

fn tick_transition(
    time: Res<Time>,
    mut state: ResMut<TransitionState>,
    mut active_map: ResMut<ActiveMap>,
) {
    let dt = time.delta_secs();

    match state.direction {
        TransitionDirection::FadeOut => {
            state.alpha += dt / 0.5;
            if state.alpha >= 1.0 {
                state.alpha = 1.0;
                // Swap the active map at peak opacity — screen is fully black.
                if let Some(target) = state.target {
                    active_map.0 = target;
                }
                state.direction = TransitionDirection::FadeIn;
            }
        }
        TransitionDirection::FadeIn => {
            state.alpha -= dt / 0.5;
            if state.alpha <= 0.0 {
                state.alpha = 0.0;
                state.direction = TransitionDirection::Idle;
                state.target = None;
            }
        }
        TransitionDirection::Idle => {}
    }
}

fn update_overlay_alpha(
    state: Res<TransitionState>,
    mut query: Query<&mut BackgroundColor, With<TransitionOverlay>>,
) {
    for mut bg in query.iter_mut() {
        bg.0 = Color::srgba(0.0, 0.0, 0.0, state.alpha);
    }
}
