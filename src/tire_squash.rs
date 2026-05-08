// Tire squash: visual deformation of the wheel mesh proportional to
// suspension load. Subtle effect (max ~15% squash, slight bulge) so the
// player can see tires deform on rocks during wheel-cam (M1).
//
// Axis convention (wheel local space):
//   The wheel mesh is Cylinder with axis Y. The wheel entity's spawn
//   transform bakes in Quat::from_rotation_z(FRAC_PI_2), which maps the
//   cylinder's +Y axis to chassis -X (lateral). The cylinder's circular
//   cross-section lives in the local XZ plane, which after rotation
//   occupies the world YZ plane — i.e. "vertical" in world space aligns
//   with local Z (and X). We therefore apply vertical squash on scale.z
//   and lateral bulge on scale.x, leaving scale.y (the wheel width axis,
//   mapping to chassis lateral) unchanged at 1.0.
//
//   Summary:
//     scale.x → radial bulge  (world X, tire width direction after rotation)
//     scale.y → lateral / rim width  (1.0 — untouched)
//     scale.z → vertical squash  (world Y, "up")
//
// Public API:
//   TireSquashPlugin

use bevy::prelude::*;
use crate::vehicle::Wheel;

// ---- Constants ----

/// Maximum suspension travel, mirrors vehicle.rs SUSPENSION_LEN.
const SUSPENSION_LEN: f32 = 0.60;

/// Maximum fraction by which the tire shrinks vertically under full load.
const MAX_SQUASH: f32 = 0.15;

/// Maximum fraction by which the tire bulges laterally under full load.
const MAX_BULGE: f32 = 0.05;

// ---- Plugin ----

pub struct TireSquashPlugin;

impl Plugin for TireSquashPlugin {
    fn build(&self, app: &mut App) {
        // Runs in Update. vehicle.rs's update_wheel_visuals (also Update) writes
        // transform.translation and transform.rotation but never scale, so
        // there is no write conflict regardless of system order. We rely on
        // that non-overlap — scale is owned exclusively by this system.
        app.add_systems(Update, apply_squash);
    }
}

// ---- System ----

/// Scales each wheel mesh to simulate pneumatic tire deformation under load.
///
/// - `load_pct` = current_compression / SUSPENSION_LEN, clamped [0, 1].
/// - Vertical axis (local Z after bake-in rotation): scaled down by up to 15%.
/// - Lateral bulge (local X): scaled up by up to 5%.
/// - Width axis (local Y): unchanged (1.0).
///
/// To diagnose which visual axis is correct in-engine, an `info!` log fires
/// for wheel index 0 once per second.
fn apply_squash(mut wheel_q: Query<(&mut Transform, &Wheel)>) {
    for (mut transform, wheel) in wheel_q.iter_mut() {
        let load_pct = (wheel.current_compression / SUSPENSION_LEN).clamp(0.0, 1.0);
        let squash_scale = 1.0 - MAX_SQUASH * load_pct;
        let bulge_scale  = 1.0 + MAX_BULGE  * load_pct;
        transform.scale = Vec3::new(bulge_scale, 1.0, squash_scale);
    }
}
