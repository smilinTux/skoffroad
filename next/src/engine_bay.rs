// Engine bay: procedural V8 engine block visible through the grille of
// every variant. Composed of cuboid block + 8 cylinder lifters + air
// intake + alternator + valve covers. Visible from front-facing camera
// modes and through the windshield.
//
// Public API:
//   EngineBayPlugin

use bevy::prelude::*;

pub struct EngineBayPlugin;

impl Plugin for EngineBayPlugin {
    fn build(&self, _app: &mut App) {
        // populated by sprint-34 agent
    }
}
