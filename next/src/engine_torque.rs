// Engine torque: real RPM-based torque curve. Replaces the implicit constant
// drive force feel with a proper engine — peak torque around 2500 RPM, falls
// off above 4500. Drives engine_pro.rs's pitch via shared RPM resource.
//
// Public API:
//   EngineTorquePlugin
//   EngineState (resource — exposes rpm for engine_pro.rs to read)

use bevy::prelude::*;

pub struct EngineTorquePlugin;

impl Plugin for EngineTorquePlugin {
    fn build(&self, _app: &mut App) {
        // populated by sprint-34 agent
    }
}

#[derive(Resource, Default, Clone, Copy)]
pub struct EngineState {
    pub rpm: f32,
    pub torque_mult: f32,
    pub gear: u8,
}
