// Tire pressure / airdown: T key cycles 5/15/35 psi.
//   5 psi  — max grip on rocks, lower top speed, faster bump-stop hits
//   15 psi — balanced
//   35 psi — stiff, fast on hardpan, less grip on rocks
//
// Public API:
//   TirePressurePlugin
//   TirePressureState (resource)
//
// Sprint 33 — owns this file exclusively.

use bevy::prelude::*;
use avian3d::prelude::*;

use crate::vehicle::{Chassis, VehicleRoot, Wheel};

// Physics tuning knobs
// At 5 psi lateral damping multiplier (more grip on rocks).
const LOW_PSI_LATERAL_MUL:  f32 = 1.4;
// At 5 psi top speed cap (m/s): drag kicks in above this.
const LOW_PSI_TOP_SPEED:     f32 = 12.0;
// Drag coefficient applied when over the speed cap (scaled to chassis mass).
const LOW_PSI_DRAG_COEFF:    f32 = 6_000.0;
// Base lateral grip constant matching vehicle.rs LATERAL_GRIP.
const LATERAL_GRIP:          f32 = 8_000.0;
// At 35 psi lateral grip multiplier (less grip).
const HIGH_PSI_LATERAL_MUL:  f32 = 0.8;

// Wheel squash constants
// Threshold below which baseline squash applies.
const SQUASH_PSI_THRESHOLD:  f32 = 10.0;
// Baseline scale.z reduction at <= 10 psi.
const SQUASH_BASELINE:       f32 = 0.07;
// Additional reduction at 5 psi (total = SQUASH_BASELINE + this).
const SQUASH_5_PSI:          f32 = 0.07;

// ---- Public plugin ----------------------------------------------------------

pub struct TirePressurePlugin;

impl Plugin for TirePressurePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TirePressureState>()
            .add_systems(Startup, spawn_psi_indicator)
            .add_systems(
                Update,
                (
                    cycle_with_t,
                    apply_baseline_squash,
                    update_indicator,
                ),
            )
            .add_systems(
                PhysicsSchedule,
                apply_pressure_effects
                    .after(PhysicsStepSystems::NarrowPhase)
                    .before(PhysicsStepSystems::Solver),
            );
    }
}

// ---- Resources & components ------------------------------------------------

/// Tire pressure state shared across the session.
#[derive(Resource, Clone, Copy)]
pub struct TirePressureState {
    pub psi: f32,
}

impl Default for TirePressureState {
    fn default() -> Self {
        Self { psi: 15.0 }
    }
}

/// Marker on the PSI HUD text node.
#[derive(Component)]
struct PsiIndicatorText;

// ---- Startup: spawn HUD indicator (top-right, below the session stats panel) ----

fn spawn_psi_indicator(mut commands: Commands) {
    // Small panel anchored top-right. We nudge it down slightly so it sits
    // below hud.rs's stats panel (which is ~140 px tall at 12 px top).
    let panel = commands
        .spawn(Node {
            position_type: PositionType::Absolute,
            right: Val::Px(12.0),
            top: Val::Px(165.0),
            width: Val::Px(110.0),
            height: Val::Px(30.0),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        })
        .insert(BackgroundColor(Color::srgba(0.05, 0.05, 0.07, 0.80)))
        .id();

    let label = commands
        .spawn((
            PsiIndicatorText,
            Text::new("PSI: 15"),
            TextFont {
                font_size: 12.0,
                ..default()
            },
            TextColor(Color::srgb(0.95, 0.85, 0.2)), // yellow = balanced (15)
        ))
        .id();

    commands.entity(panel).add_child(label);
}

// ---- Update: cycle psi with T key ------------------------------------------

fn cycle_with_t(
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<TirePressureState>,
) {
    if !keys.just_pressed(KeyCode::KeyT) {
        return;
    }

    // Advance through cycle: 15 → 5 → 35 → 15 …
    let next = match state.psi as u32 {
        15 => 5.0,
        5  => 35.0,
        _  => 15.0, // covers 35 and any unexpected value
    };
    state.psi = next;
    info!("tire pressure: {} psi", state.psi);
}

// ---- PhysicsSchedule: apply extra lateral/drag forces ----------------------
//
// This system adds supplemental forces on top of whatever vehicle.rs already
// applied.  It does NOT modify vehicle.rs constants.
//
//  5 psi:  +40% lateral grip (more grip on rocks) + top-speed cap drag
// 15 psi:  no extra forces
// 35 psi:  −20% lateral grip (less grip on rocks)

fn apply_pressure_effects(
    state: Res<TirePressureState>,
    vehicle: Option<Res<VehicleRoot>>,
    mut chassis_q: Query<(Forces, &Transform), With<Chassis>>,
) {
    // 15 psi: nothing to do.
    if (state.psi - 15.0).abs() < 0.1 {
        return;
    }

    let Some(vehicle) = vehicle else { return };
    let Ok((mut forces, transform)) = chassis_q.get_mut(vehicle.chassis) else { return };

    let vel = forces.linear_velocity();

    // Compute lateral direction from chassis orientation.
    // Bevy convention: local +X is right, local -Z is forward.
    let right = (transform.rotation * Vec3::X).normalize();

    let lateral_v = vel.dot(right); // signed lateral speed

    if state.psi < 10.0 {
        // ---- 5 psi: extra lateral grip + top-speed cap ----

        // Additional lateral damping: 0.4 × LATERAL_GRIP × v_lateral
        // (on top of the 1.0× already applied by vehicle.rs → net 1.4×).
        let extra_grip = LOW_PSI_LATERAL_MUL - 1.0; // 0.4
        let f_lat = -extra_grip * LATERAL_GRIP * lateral_v;
        forces.apply_force(right * f_lat);

        // Top-speed cap: quadratic drag once speed exceeds LOW_PSI_TOP_SPEED.
        let speed = vel.length();
        if speed > LOW_PSI_TOP_SPEED {
            let excess = speed - LOW_PSI_TOP_SPEED;
            // Drag opposes velocity direction, proportional to excess speed.
            let drag = -vel.normalize_or_zero() * LOW_PSI_DRAG_COEFF * excess;
            forces.apply_force(drag);
        }
    } else {
        // ---- 35 psi: reduced lateral grip ----

        // Subtract 0.2 × LATERAL_GRIP × v_lateral from lateral force,
        // i.e. apply a positive restoring force in the lateral slip direction
        // (counteracts part of vehicle.rs's grip force → net 0.8×).
        let reduced = 1.0 - HIGH_PSI_LATERAL_MUL; // 0.2 — fraction to *remove*
        let f_lat = reduced * LATERAL_GRIP * lateral_v; // intentionally positive (opposing the grip)
        forces.apply_force(right * f_lat);
    }
}

// ---- Update: apply baseline wheel squash at low psi -------------------------
//
// tire_squash.rs writes scale every frame via apply_squash (also in Update,
// registered before this system's plugin initialises). We run after it in
// natural Update ordering: tire_squash writes first, then we multiply scale.z.
//
// At <= 10 psi: scale.z *= (1 - SQUASH_BASELINE)  → 0.93 baseline
// At  5 psi:   scale.z *= (1 - SQUASH_5_PSI)      → additional 7% (net 0.93*0.93≈0.865)
// At 35 psi:   no change.

fn apply_baseline_squash(
    state: Res<TirePressureState>,
    mut wheel_q: Query<&mut Transform, With<Wheel>>,
) {
    if state.psi > SQUASH_PSI_THRESHOLD {
        return;
    }

    let baseline_factor = 1.0 - SQUASH_BASELINE; // 0.93
    let extra_factor = if state.psi < 10.0 {
        1.0 - SQUASH_5_PSI // 0.93 at 5 psi
    } else {
        1.0
    };

    for mut transform in wheel_q.iter_mut() {
        transform.scale.z *= baseline_factor * extra_factor;
    }
}

// ---- Update: refresh PSI indicator text and colour -------------------------

fn update_indicator(
    state: Res<TirePressureState>,
    mut text_q: Query<(&mut Text, &mut TextColor), With<PsiIndicatorText>>,
) {
    let Ok((mut text, mut color)) = text_q.single_mut() else { return };

    text.0 = format!("PSI: {}", state.psi as u32);

    // Green = 5 (rock grip), Yellow = 15 (balanced), Red = 35 (stiff/speed)
    color.0 = if state.psi < 10.0 {
        Color::srgb(0.2, 0.9, 0.2)   // green
    } else if state.psi < 25.0 {
        Color::srgb(0.95, 0.85, 0.2) // yellow
    } else {
        Color::srgb(0.95, 0.25, 0.2) // red
    };
}
