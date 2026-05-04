// Camera shake on hard impacts.
//
// Reads HardImpact events from EventLog and applies trauma-based shake to the
// Camera3d transform each frame, running after camera.rs's update systems.

use bevy::prelude::*;
use crate::events::{EventLog, GameEvent as GE};

// ---- Public types -----------------------------------------------------------

#[derive(Resource, Default)]
pub struct ShakeState {
    pub trauma: f32,       // 0..=1; decays over time
    pub last_event_t: f32, // highest timestamp already processed
}

// ---- Plugin -----------------------------------------------------------------

pub struct ShakePlugin;

impl Plugin for ShakePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ShakeState>()
           .add_systems(Update, (update_shake_state, apply_shake));
    }
}

// ---- LCG helpers ------------------------------------------------------------

/// Returns two pseudo-random f32 values in [-1, 1] from a u64 seed.
fn rng_pair(seed: u64) -> (f32, f32) {
    let s = seed
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    let a = (s >> 33) as u32 as f32 / u32::MAX as f32 * 2.0 - 1.0;
    let s2 = s
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    let b = (s2 >> 33) as u32 as f32 / u32::MAX as f32 * 2.0 - 1.0;
    (a, b)
}

// ---- Systems ----------------------------------------------------------------

/// Reads new HardImpact events from the EventLog and updates trauma.
fn update_shake_state(
    log: Res<EventLog>,
    mut state: ResMut<ShakeState>,
    time: Res<Time>,
) {
    let dt = time.delta_secs();

    // Scan events newer than what we have already processed.
    let mut newest_t = state.last_event_t;
    for (ts, ev) in &log.events {
        if *ts <= state.last_event_t {
            continue;
        }
        if let GE::HardImpact { v } = ev {
            let added = (v.abs() / 20.0).min(0.6);
            state.trauma = (state.trauma + added).min(1.0);
        }
        if *ts > newest_t {
            newest_t = *ts;
        }
    }
    state.last_event_t = newest_t;

    // Decay trauma.
    state.trauma = (state.trauma - 1.0 * dt).max(0.0);
}

/// Perturbs the Camera3d transform each frame while trauma > 0.
fn apply_shake(
    state: Res<ShakeState>,
    mut cam_q: Query<&mut Transform, With<Camera3d>>,
    time: Res<Time>,
) {
    if state.trauma <= 0.0 {
        return;
    }

    let Ok(mut cam) = cam_q.single_mut() else { return };

    let seed = (time.elapsed_secs() * 1000.0) as u64;
    let (rx, ry) = rng_pair(seed);
    let (rz, _) = rng_pair(seed.wrapping_add(7919));

    let t2 = state.trauma * state.trauma;

    let pos_offset = Vec3::new(rx, ry, rz) * t2 * 0.3;
    let rot_offset = Quat::from_rotation_z(rz * t2 * 0.05);

    cam.translation += pos_offset;
    cam.rotation = cam.rotation * rot_offset;
}
