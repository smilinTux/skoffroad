// Interior 3D: cockpit details visible from FirstPerson camera mode —
// steering wheel that turns with input, dashboard with speedometer arc,
// 2 bucket seats. All attached as chassis children.
//
// Public API:
//   Interior3dPlugin

use bevy::prelude::*;

pub struct Interior3dPlugin;

impl Plugin for Interior3dPlugin {
    fn build(&self, _app: &mut App) {
        // populated by sprint-34 agent
    }
}
