// Drive mode: `4` key toggles 2WD (rear-wheel drive only) and 4WD (all
// wheels driven). 4WD is the current default behavior; 2WD mode reduces
// drive force on the front 2 wheels to zero. HUD indicator shows mode.
//
// Public API:
//   DriveModePlugin
//   DriveModeState (resource)

use bevy::prelude::*;

pub struct DriveModePlugin;

impl Plugin for DriveModePlugin {
    fn build(&self, _app: &mut App) {
        // populated by sprint-34 agent
    }
}

#[derive(Resource, Clone, Copy, PartialEq, Eq)]
pub struct DriveModeState {
    pub four_wheel_drive: bool,
}

impl Default for DriveModeState {
    fn default() -> Self {
        Self { four_wheel_drive: true }
    }
}
