// Chassis headlights: two SpotLights parented to the chassis entity.
// Auto-on at night (TimeOfDay.t < 0.25 || t > 0.75); Y toggles manual override.

use bevy::prelude::*;
use crate::sky::TimeOfDay;
use crate::vehicle::VehicleRoot;

// ---- Plugin -----------------------------------------------------------------

pub struct HeadlightsPlugin;

impl Plugin for HeadlightsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HeadlightState>()
           .add_systems(PostStartup, spawn_headlights
               .run_if(resource_exists::<VehicleRoot>))
           .add_systems(Update, (toggle_headlights, update_headlight_intensity));
    }
}

// ---- Resource ---------------------------------------------------------------

#[derive(Resource)]
pub struct HeadlightState {
    pub on:   bool,
    pub auto: bool,
}

impl Default for HeadlightState {
    fn default() -> Self {
        Self { on: false, auto: true }
    }
}

// ---- Marker component -------------------------------------------------------

#[derive(Component)]
struct Headlight;

// ---- Constants --------------------------------------------------------------

// Bevy 0.18 SpotLight intensity is in lumens but the HDR pipeline needs
// substantially more to read as bright headlights against a dark scene.
// 30000 was barely visible; 200000 gives a clear cone that lights terrain.
const HL_INTENSITY:   f32 = 200_000.0;
const HL_RANGE:       f32 = 80.0;
const HL_OUTER_ANGLE: f32 = 0.436_332; // 25°
const HL_INNER_ANGLE: f32 = 0.261_799; // 15°

// ---- Systems ----------------------------------------------------------------

fn spawn_headlights(
    mut commands: Commands,
    vehicle: Res<VehicleRoot>,
) {
    let color = Color::srgb(1.0, 0.95, 0.85);

    for x in [-0.7_f32, 0.7_f32] {
        let pos = Vec3::new(x, 0.0, -1.95);
        // looking_at points -Z; target is directly forward in chassis space.
        let transform = Transform::from_translation(pos)
            .looking_at(pos + Vec3::NEG_Z, Vec3::Y);

        let light = SpotLight {
            intensity:       HL_INTENSITY,
            range:           HL_RANGE,
            color,
            outer_angle:     HL_OUTER_ANGLE,
            inner_angle:     HL_INNER_ANGLE,
            shadows_enabled: false,
            ..default()
        };

        let light_id = commands.spawn((Headlight, light, transform)).id();
        commands.entity(vehicle.chassis).add_child(light_id);
    }
}

/// Y  → manual override: disable auto, toggle on/off.
/// Shift+Y → return to auto mode.
fn toggle_headlights(
    keys:  Res<ButtonInput<KeyCode>>,
    mut state: ResMut<HeadlightState>,
) {
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);

    if keys.just_pressed(KeyCode::KeyY) {
        if shift {
            // Shift+Y: back to auto.
            state.auto = true;
        } else {
            // Y: manual override.
            state.auto = false;
            state.on   = !state.on;
        }
    }
}

/// Drives SpotLight intensity from HeadlightState + TimeOfDay each frame.
fn update_headlight_intensity(
    state: Res<HeadlightState>,
    tod:   Res<TimeOfDay>,
    mut lights: Query<&mut SpotLight, With<Headlight>>,
) {
    let is_night = tod.t < 0.25 || tod.t > 0.75;
    let active   = if state.auto { is_night } else { state.on };
    let target   = if active { HL_INTENSITY } else { 0.0 };

    for mut light in &mut lights {
        light.intensity = target;
    }
}
