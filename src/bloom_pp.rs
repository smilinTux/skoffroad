// Bloom + tonemapping post-process for skoffroad (Sprint 20, tuned Sprint 90).
//
// Attaches Bevy's Bloom + Tonemapping components to the main Camera3d.
// Toggle on/off with the J key (J = "joy"; B is taken by Boost).
// Default: ON.
//
// Sprint 90 realism tuning:
//   - Bloom intensity dropped from 0.20 → 0.06 (was blowing out highlights).
//   - Low-frequency bloom (high_pass_frequency 0.35) so headlights / emissives
//     glow softly without fringing on every lit edge.
//   - Composite mode EnergyConserving keeps scene exposure stable.
//   - Tonemapping stays AgX (set by post_fx.rs pin system) — we no longer
//     fight it by inserting AcesFitted here.  When bloom is disabled we insert
//     Tonemapping::None (post_fx pin restores AgX next PostUpdate).
//   - Tier gate: bloom is only inserted on Medium+ (GraphicsQuality.bloom()).
//     Low gets no Bloom component at all — no hdr blowout on weak hardware.
//
// Public API:
//   BloomPpPlugin
//   BloomPpState (resource)

use bevy::{
    post_process::bloom::{Bloom, BloomCompositeMode},
    prelude::*,
};
use crate::graphics_quality::GraphicsQuality;

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

/// Apply (or remove) Bloom on the main Camera3d, tier-gated to Medium+.
///
/// We do NOT set Tonemapping here anymore — post_fx.rs owns that (AgX).
/// Inserting AcesFitted here then having post_fx pin AgX caused a one-frame
/// flash at startup; now we let post_fx win cleanly.
fn apply_post_fx_on_change(
    state: Res<BloomPpState>,
    quality: Res<GraphicsQuality>,
    mut last: Local<Option<bool>>,
    cam_q: Query<Entity, With<Camera3d>>,
    mut commands: Commands,
) {
    let changed = last.map_or(true, |prev| prev != state.enabled);
    if !changed {
        return;
    }
    *last = Some(state.enabled);

    let Ok(entity) = cam_q.single() else {
        return;
    };
    // Bevy 0.18: Bloom enables HDR implicitly when inserted.

    // Low tier: never show bloom (too expensive / reads as cheap).
    if !quality.bloom() || !state.enabled {
        commands.entity(entity).remove::<Bloom>();
        return;
    }

    // Medium+: restrained physically-based bloom.
    // intensity 0.06  — subtle glow on emissive headlights + sky rim
    // high_pass_frequency 0.35 — only brightest emissive pixels bloom
    // EnergyConserving — bloom energy taken FROM the frame, not added on top
    commands.entity(entity).insert(Bloom {
        intensity: 0.06,
        high_pass_frequency: 0.35,
        composite_mode: BloomCompositeMode::EnergyConserving,
        ..default()
    });
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
