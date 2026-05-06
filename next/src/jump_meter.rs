// Jump meter: on takeoff, records chassis position. On landing, computes
// horizontal distance (XZ) and air time, then flashes big center-screen text
// "JUMP: 8.3m  AIR: 1.2s" for 2 seconds with a fade-out in the last 0.5 s.
//
// Public API:
//   JumpMeterPlugin
//   JumpMeterState (resource)

use bevy::prelude::*;

use crate::airtime::AirtimeStats;
use crate::vehicle::{Chassis, VehicleRoot};

// ---- Minimum air time required to trigger the flash (s) ---------------------
const MIN_FLASH_AIR_S: f32 = 0.4;
// ---- How long the flash stays on screen (s) ---------------------------------
const FLASH_DURATION: f32 = 2.0;
// ---- Fade starts when flash_remaining drops below this (s) ------------------
const FADE_WINDOW_S: f32 = 0.5;

// ---- Public resource ---------------------------------------------------------

#[derive(Resource, Default, Clone, Copy)]
pub struct JumpMeterState {
    pub takeoff_pos:     Vec3,
    pub last_dist_m:     f32,
    pub last_air_s:      f32,
    pub flash_remaining: f32,
}

// ---- Components --------------------------------------------------------------

#[derive(Component)]
struct JumpMeterHudRoot;

#[derive(Component)]
struct JumpMeterHudText;

// ---- Plugin ------------------------------------------------------------------

pub struct JumpMeterPlugin;

impl Plugin for JumpMeterPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<JumpMeterState>()
           .add_systems(Startup, spawn_jump_meter_hud)
           .add_systems(Update, (
               detect_takeoff_landing.run_if(resource_exists::<VehicleRoot>),
               update_flash_text,
           ));
    }
}

// ---- Startup: spawn hidden centered flash text -------------------------------

fn spawn_jump_meter_hud(mut commands: Commands) {
    // Root node: absolutely positioned, top 25%, left 50%, shifted left by 200 px
    // to center the 400 px-wide text.
    let root = commands.spawn((
        JumpMeterHudRoot,
        Node {
            position_type: PositionType::Absolute,
            top:           Val::Percent(25.0),
            left:          Val::Percent(50.0),
            margin:        UiRect { left: Val::Px(-200.0), ..default() },
            width:         Val::Px(400.0),
            justify_content: JustifyContent::Center,
            align_items:   AlignItems::Center,
            display:       Display::None, // hidden until a qualifying jump lands
            ..default()
        },
    )).id();

    let label = commands.spawn((
        JumpMeterHudText,
        Text::new("JUMP: 0.0m  AIR: 0.0s"),
        TextFont { font_size: 32.0, ..default() },
        TextColor(Color::srgba(1.0, 0.95, 0.3, 1.0)),
    )).id();

    commands.entity(root).add_child(label);
}

// ---- Update: detect takeoff / landing transitions ---------------------------

fn detect_takeoff_landing(
    vehicle:    Res<VehicleRoot>,
    chassis_q:  Query<&Transform, With<Chassis>>,
    stats:      Res<AirtimeStats>,
    time:       Res<Time>,
    mut state:  ResMut<JumpMeterState>,
    mut was_airborne: Local<bool>,
    mut peak_air_s:   Local<f32>,
) {
    let _ = time; // not needed here — flash timer is in update_flash_text

    let chassis_pos = if let Ok(t) = chassis_q.get(vehicle.chassis) {
        t.translation
    } else {
        return;
    };

    let is_airborne = stats.airborne;

    // Track the highest current_air_s seen this flight so we have it at landing,
    // because detect_airtime in airtime.rs resets current_air_s = 0 on landing.
    if is_airborne {
        if stats.current_air_s > *peak_air_s {
            *peak_air_s = stats.current_air_s;
        }
    }

    match (*was_airborne, is_airborne) {
        // Transition: grounded → airborne (takeoff)
        (false, true) => {
            state.takeoff_pos = chassis_pos;
            *peak_air_s = 0.0;
            info!("JumpMeter: takeoff at ({:.1}, {:.1}, {:.1})",
                chassis_pos.x, chassis_pos.y, chassis_pos.z);
        }

        // Transition: airborne → grounded (landing)
        (true, false) => {
            // Use peak_air_s captured during flight; fall back to max_air_s if
            // it happens to be higher (e.g. system ordering put airtime first).
            let air_s = peak_air_s.max(stats.current_air_s);

            if air_s >= MIN_FLASH_AIR_S {
                let dx = chassis_pos.x - state.takeoff_pos.x;
                let dz = chassis_pos.z - state.takeoff_pos.z;
                let dist = (dx * dx + dz * dz).sqrt();

                state.last_dist_m     = dist;
                state.last_air_s      = air_s;
                state.flash_remaining = FLASH_DURATION;

                info!("JUMP! dist={:.1}m air={:.1}s", dist, air_s);
            }

            *peak_air_s = 0.0;
        }

        // Sustained states: nothing to do
        _ => {}
    }

    *was_airborne = is_airborne;
}

// ---- Update: refresh flash text visibility and content ----------------------

fn update_flash_text(
    time:      Res<Time>,
    mut state: ResMut<JumpMeterState>,
    mut root_q: Query<&mut Node, With<JumpMeterHudRoot>>,
    mut text_q: Query<(&mut Text, &mut TextColor), With<JumpMeterHudText>>,
) {
    let dt = time.delta_secs();

    // Advance timer (clamp at zero).
    if state.flash_remaining > 0.0 {
        state.flash_remaining = (state.flash_remaining - dt).max(0.0);
    }

    let showing = state.flash_remaining > 0.0;

    // Show/hide root node.
    for mut node in &mut root_q {
        node.display = if showing { Display::Flex } else { Display::None };
    }

    if !showing {
        return;
    }

    // Alpha: 1.0 while remaining > FADE_WINDOW_S, then linearly fades to 0.
    let alpha = (state.flash_remaining / FADE_WINDOW_S).min(1.0);

    let content = format!(
        "JUMP: {:.1}m  AIR: {:.1}s",
        state.last_dist_m, state.last_air_s
    );

    for (mut text, mut color) in &mut text_q {
        text.0  = content.clone();
        color.0 = Color::srgba(1.0, 0.95, 0.3, alpha);
    }
}
