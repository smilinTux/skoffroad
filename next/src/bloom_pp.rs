// Bloom + tonemapping post-process for SandK Offroad (Sprint 20).
//
// Attaches Bevy's Bloom + Tonemapping components to the main Camera3d.
// Toggle on/off with the J key (J = "joy"; B is taken by Boost).
// Default: ON.
//
// Public API:
//   BloomPpPlugin
//   BloomPpState (resource)

use bevy::{
    core_pipeline::tonemapping::Tonemapping,
    post_process::bloom::Bloom,
    prelude::*,
};

// ── Public types ────────────────────────────────────────────────────────────

pub struct BloomPpPlugin;

#[derive(Resource, Clone, Copy)]
pub struct BloomPpState {
    pub enabled: bool,
}

impl Default for BloomPpState {
    fn default() -> Self {
        Self { enabled: true }
    }
}

// ── Plugin ───────────────────────────────────────────────────────────────────

impl Plugin for BloomPpPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(BloomPpState::default())
            .add_systems(Update, apply_post_fx_on_change)
            .add_systems(Update, toggle_with_j);
    }
}

// ── Systems ──────────────────────────────────────────────────────────────────

/// Apply (or remove) Bloom + Tonemapping on the main Camera3d.
///
/// A `Local<bool>` tracks the last-seen enabled value.  Initialised to
/// `false`, so the first frame always looks like a change (state starts
/// `true`) and the effect is installed at startup without an extra
/// `Startup` system.
fn apply_post_fx_on_change(
    state: Res<BloomPpState>,
    mut last: Local<Option<bool>>,
    mut cam_q: Query<(Entity, &mut Camera), With<Camera3d>>,
    mut commands: Commands,
) {
    let changed = last.map_or(true, |prev| prev != state.enabled);
    if !changed {
        return;
    }
    *last = Some(state.enabled);

    let Ok((entity, mut camera)) = cam_q.single_mut() else {
        return;
    };

    // Note: In Bevy 0.18 Camera no longer has an `hdr` field;
    // Bloom automatically enables HDR when inserted as a component.
    let _ = camera; // still need the query to confirm a camera exists

    if state.enabled {
        commands
            .entity(entity)
            .insert(Bloom {
                intensity: 0.20,
                ..default()
            })
            .insert(Tonemapping::AcesFitted);
    } else {
        commands
            .entity(entity)
            .remove::<Bloom>()
            .insert(Tonemapping::None);
    }
}

/// Toggle bloom with the J key (plain press, no modifiers).
fn toggle_with_j(
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<BloomPpState>,
) {
    if keys.just_pressed(KeyCode::KeyJ)
        && !keys.pressed(KeyCode::ShiftLeft)
        && !keys.pressed(KeyCode::ShiftRight)
        && !keys.pressed(KeyCode::ControlLeft)
        && !keys.pressed(KeyCode::ControlRight)
    {
        state.enabled = !state.enabled;
        info!("bloom: {}", state.enabled);
    }
}
