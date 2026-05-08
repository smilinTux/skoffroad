// winch_cable_physics.rs — Sprint 39
//
// Visualizes the winch cable as a catenary-curved line of 8 segments using
// Bevy Gizmos.  This is a pure visual effect; actual winch-pull physics live
// in winch.rs.  The parabolic sag formula gives a convincing heavy-cable droop
// without requiring any rope-physics simulation.
//
// Public API:
//   WinchCablePhysicsPlugin

use bevy::prelude::*;

use crate::winch::WinchState;
use crate::vehicle::{Chassis, VehicleRoot};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Number of segments in the cable visual (produces N+1 points).
const SEGMENTS: usize = 8;

/// Cable color: orange-ish steel.
const CABLE_COLOR: Color = Color::srgb(0.8, 0.4, 0.1);

/// Forward offset from chassis origin to the front-bumper attachment point.
const ATTACH_FWD: f32 = 1.4;

/// Vertical offset for the attachment point.
const ATTACH_Y: f32 = 0.2;

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct WinchCablePhysicsPlugin;

impl Plugin for WinchCablePhysicsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, draw_winch_cable);
    }
}

// ---------------------------------------------------------------------------
// System
// ---------------------------------------------------------------------------

fn draw_winch_cable(
    mut gizmos: Gizmos,
    winch: Option<Res<WinchState>>,
    vehicle: Option<Res<VehicleRoot>>,
    chassis_q: Query<&Transform, With<Chassis>>,
) {
    // Guard: need both resources to be present.
    let Some(winch) = winch else { return };
    let Some(vehicle) = vehicle else { return };

    // Guard: winch must be attached (anchor present).
    let Some(anchor) = winch.anchor_pos else { return };

    // Guard: chassis transform must be available.
    let Ok(chassis_transform) = chassis_q.get(vehicle.chassis) else { return };

    // Compute front-bumper attachment point in world space.
    let chassis_pos = chassis_transform.translation;
    let chassis_fwd = (chassis_transform.rotation * Vec3::NEG_Z).normalize();
    let start = chassis_pos + chassis_fwd * ATTACH_FWD + Vec3::Y * ATTACH_Y;
    let end = anchor;

    // Tension: use 1.0 when spooling (actively pulling), 0.5 otherwise.
    // sag > 0 means drooping downward (negative Y contribution).
    let tension = if winch.spooling { 1.0_f32 } else { 0.5_f32 };
    let sag = -0.3 + tension * 0.25; // range: -0.3 (slack) … -0.05 (taut)

    // Build N+1 points along the parabolic cable.
    let mut points = [Vec3::ZERO; SEGMENTS + 1];
    for i in 0..=SEGMENTS {
        let t = i as f32 / SEGMENTS as f32;

        // Linear interpolation between start and end.
        let base = start.lerp(end, t);

        // Parabolic sag applied in world Y (cable droops under gravity).
        // y(t) = y_lerp + sag * 4 * t * (1 - t)
        // The factor of 4 ensures the midpoint deflection equals `sag` exactly.
        let y_offset = sag * 4.0 * t * (1.0 - t);

        points[i] = base + Vec3::new(0.0, y_offset, 0.0);
    }

    // Draw the segments.
    for i in 0..SEGMENTS {
        gizmos.line(points[i], points[i + 1], CABLE_COLOR);
    }
}
