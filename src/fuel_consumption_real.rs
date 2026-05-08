// Sprint 39 — Real fuel-consumption model.
//
// Replaces the flat burn-rate from fuel.rs with a physics-informed model that
// accounts for throttle input, engine RPM, and chassis pitch (hill-climbing load).
//
// Design:
//   - Maintains an internal `real_fuel_l` tracker (starts at Fuel.current_l).
//   - Each Update frame it computes a multi-factor burn rate and writes the
//     result straight into Fuel.current_l, overriding whatever simple decrement
//     fuel.rs ran beforehand (fuel.rs runs before apply_drive_input, this runs
//     after apply_drive_input — the ordering means our value wins the frame).
//   - When the tank is at zero, drive is NOT zeroed here; fuel.rs already handles
//     that. We just stop decrementing.
//   - A "MPG" HUD node is spawned in the bottom-left corner; it shows a live
//     derived value (mock formula calibrated to give ~17 MPG at highway cruise).
//
// Burn components (all in L/s):
//   base_rate      = 0.0001          (idle trickle)
//   throttle_rate  = |drive| × 0.005 (full throttle ≈ 5 mL/s ≈ 18 L/h ≈ 25 MPG)
//   climbing_rate  = max(0, sin(pitch)) × 0.003
//   rpm_rate       = max(0, rpm - 1000) / 5000 × 0.002

use bevy::prelude::*;
use bevy::math::EulerRot;

use crate::engine_torque::EngineState;
use crate::fuel::Fuel;
use crate::vehicle::{Chassis, DriveInput, VehicleRoot};

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct FuelConsumptionRealPlugin;

impl Plugin for FuelConsumptionRealPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RealFuelState>()
           .add_systems(Startup, spawn_mpg_hud)
           .add_systems(
               Update,
               (
                   real_fuel_burn.after(crate::vehicle::apply_drive_input),
                   update_mpg_hud,
               )
                   .chain()
                   .run_if(resource_exists::<VehicleRoot>),
           );
    }
}

// ---------------------------------------------------------------------------
// Internal state
// ---------------------------------------------------------------------------

/// Tracks the "real" fuel level so that even if fuel.rs writes its own
/// decrement first, we overwrite Fuel.current_l with our authoritative value.
#[derive(Resource)]
struct RealFuelState {
    /// Internal tank tracker (litres). Initialised lazily from Fuel.current_l
    /// on the first frame.
    real_fuel_l: f32,
    /// Initialised flag — false until we've copied the starting level.
    initialised: bool,
    /// Smoothed burn rate for the MPG display (L/s).
    smoothed_burn_l_per_s: f32,
    /// Smoothed speed (m/s) for the MPG display (reserved for future use).
    #[allow(dead_code)]
    smoothed_speed_mps: f32,
}

impl Default for RealFuelState {
    fn default() -> Self {
        Self {
            real_fuel_l:          60.0,
            initialised:          false,
            smoothed_burn_l_per_s: 0.001,
            smoothed_speed_mps:   0.0,
        }
    }
}

// ---------------------------------------------------------------------------
// HUD components
// ---------------------------------------------------------------------------

#[derive(Component)]
struct MpgText;

// ---------------------------------------------------------------------------
// Startup: spawn the MPG indicator
// ---------------------------------------------------------------------------

fn spawn_mpg_hud(mut commands: Commands) {
    // Bottom-left corner, above the edge.
    let panel = commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            left:   Val::Px(12.0),
            bottom: Val::Px(12.0),
            width:  Val::Px(120.0),
            height: Val::Px(26.0),
            padding: UiRect::all(Val::Px(5.0)),
            justify_content: JustifyContent::Center,
            align_items:     AlignItems::Center,
            ..default()
        },
        BackgroundColor(Color::srgba(0.05, 0.05, 0.07, 0.80)),
    )).id();

    let text = commands.spawn((
        MpgText,
        Text::new("MPG: --.-"),
        TextFont { font_size: 12.0, ..default() },
        TextColor(Color::srgb(0.6, 0.9, 0.6)),
    )).id();

    commands.entity(panel).add_child(text);
}

// ---------------------------------------------------------------------------
// System: real_fuel_burn
// ---------------------------------------------------------------------------

/// Multi-factor fuel burn model. Runs after apply_drive_input so that our
/// final write to Fuel.current_l is not overridden this frame.
fn real_fuel_burn(
    time:       Res<Time>,
    engine:     Res<EngineState>,
    input:      Res<DriveInput>,
    vehicle:    Res<VehicleRoot>,
    chassis_q:  Query<&Transform, With<Chassis>>,
    mut fuel:   ResMut<Fuel>,
    mut state:  ResMut<RealFuelState>,
) {
    let dt = time.delta_secs();
    if dt <= 0.0 { return; }

    // Lazily copy the starting level on the very first frame so we don't
    // reset a partially-consumed tank.
    if !state.initialised {
        state.real_fuel_l = fuel.current_l;
        state.initialised = true;
    }

    // If the tank is empty, leave it at zero and don't try to decrement further.
    if state.real_fuel_l <= 0.0 {
        fuel.current_l = 0.0;
        return;
    }

    // --- Read chassis pitch ---------------------------------------------------
    let pitch_rad = if let Ok(transform) = chassis_q.get(vehicle.chassis) {
        // YXZ Euler: index 1 = X rotation = pitch (nose-up/down).
        transform.rotation.to_euler(EulerRot::YXZ).1
    } else {
        0.0
    };

    // --- Compute burn components (all in L/s) --------------------------------
    let throttle       = input.drive.abs();                        // 0..1
    let rpm            = engine.rpm;

    let base_rate      = 0.0001_f32;
    let throttle_rate  = throttle * 0.005;
    let climbing_rate  = pitch_rad.sin().max(0.0) * 0.003;
    let rpm_rate       = (rpm - 1000.0).max(0.0) / 5000.0 * 0.002;

    let total_l_per_s  = base_rate + throttle_rate + climbing_rate + rpm_rate;

    // --- Decrement internal tracker ------------------------------------------
    state.real_fuel_l = (state.real_fuel_l - total_l_per_s * dt).max(0.0);

    // Overwrite the shared resource so fuel.rs's simple decrement is ignored.
    fuel.current_l = state.real_fuel_l;

    // --- Smooth values for HUD -----------------------------------------------
    let alpha = (dt * 2.0).min(1.0); // 2 Hz low-pass
    state.smoothed_burn_l_per_s =
        state.smoothed_burn_l_per_s * (1.0 - alpha) + total_l_per_s * alpha;
}

// ---------------------------------------------------------------------------
// System: update_mpg_hud
// ---------------------------------------------------------------------------

/// Derives a mock MPG figure from the smoothed burn rate and a notional
/// vehicle speed, then refreshes the bottom-left text node.
///
/// Conversion:
///   litres_per_metre = burn_l_per_s / speed_mps       (instantaneous)
///   litres_per_100km = litres_per_metre × 100_000
///   mpg_us           = 235.214 / litres_per_100km
///
/// At 10 m/s (22 mph) and throttle 0.3 the burn ≈ 0.0015 L/s →
/// L/100km ≈ 15 → MPG ≈ 15.7.  At highway cruise (20 m/s, throttle 0.5)
/// ≈ 0.0025 L/s → L/100km ≈ 12.5 → MPG ≈ 18.8.  Idle/very-slow speed
/// falls back to a nominal display.
fn update_mpg_hud(
    state:  Res<RealFuelState>,
    engine: Res<EngineState>,
    mut text_q: Query<&mut Text, With<MpgText>>,
) {
    // Derive approximate speed from RPM (mirrors engine_torque.rs formula in reverse).
    // rpm = speed * 90 + 700  →  speed = (rpm - 700) / 90
    let speed_mps = ((engine.rpm - 700.0) / 90.0).max(0.0);

    let label = if speed_mps > 0.5 && state.smoothed_burn_l_per_s > 1e-6 {
        let litres_per_metre = state.smoothed_burn_l_per_s / speed_mps;
        let l_per_100km = litres_per_metre * 100_000.0;
        let mpg = 235.214 / l_per_100km;
        format!("MPG: {:.1}", mpg.clamp(0.0, 999.0))
    } else {
        "MPG: --.-".to_string()
    };

    for mut text in &mut text_q {
        text.0 = label.clone();
    }
}
