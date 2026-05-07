// Differential lock: J key toggles locker. When locked, both wheels on an
// axle spin at the same rate — no slip-induced torque loss when one wheel
// is in the air. Visible HUD indicator + cosmetic glow on driveline.
//
// Public API:
//   DiffLockPlugin
//   DiffLockState (resource)

use bevy::prelude::*;

pub struct DiffLockPlugin;

impl Plugin for DiffLockPlugin {
    fn build(&self, _app: &mut App) {
        // populated by sprint-34 agent
    }
}

#[derive(Resource, Default, Clone, Copy, PartialEq, Eq)]
pub struct DiffLockState {
    pub front_locked: bool,
    pub rear_locked: bool,
}
