// Sprint 76 — Prop LOD / distance culling
//
// PropLodPlugin adds a coarse visibility-culling system that hides (`Visibility::Hidden`)
// any entity carrying the `PropLod` component when it is more than `LOD_DISTANCE` metres
// from the chassis (or camera, if no chassis is present).
//
// Design decisions:
//  - The check runs every LOD_CHECK_INTERVAL frames (default: every 8 frames) rather than
//    every frame, so the per-frame cost is roughly 1/8th of a per-frame scan.
//  - Colliders are NOT toggled — they remain active at all distances.  This avoids
//    physics artifacts (a vehicle driving into a prop that suddenly loses its collider)
//    and keeps the implementation simple.  The GPU rendering cost (draw-call for hidden
//    meshes) is zero because Bevy skips visibility-culled entities in the render graph.
//  - The component is separate from `MapProp` so it can be used by any spawn system.

use bevy::prelude::*;

use crate::vehicle::Chassis;

/// Distance (metres) beyond which a prop is hidden.
pub const LOD_DISTANCE: f32 = 250.0;

/// Run the LOD check every N frames (cost amortisation).
const LOD_CHECK_INTERVAL: u32 = 8;

/// Attach this component to any prop entity to opt into distance-based visibility culling.
/// Half-height is stored so the culling system can do a sphere-vs-point check accounting
/// for the prop's extent (prevents pop-in of tall objects).
#[derive(Component, Clone, Copy)]
pub struct PropLod {
    /// The vertical half-extent of the prop's bounding box (metres).
    /// Used to bias the distance check so a tall pole peeking over the LOD edge
    /// isn't prematurely hidden.  Set to 0.0 if you don't care.
    pub half_height: f32,
}

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct PropLodPlugin;

impl Plugin for PropLodPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LodFrameCounter>()
            .add_systems(Update, prop_lod_cull_system);
    }
}

// ---------------------------------------------------------------------------
// Frame counter resource
// ---------------------------------------------------------------------------

#[derive(Resource, Default)]
struct LodFrameCounter(u32);

// ---------------------------------------------------------------------------
// Culling system
// ---------------------------------------------------------------------------

fn prop_lod_cull_system(
    mut frame_counter: ResMut<LodFrameCounter>,
    chassis_q: Query<&Transform, With<Chassis>>,
    camera_q:  Query<&Transform, (With<Camera3d>, Without<Chassis>)>,
    mut props:  Query<(&Transform, &PropLod, &mut Visibility), Without<Chassis>>,
) {
    // Only run every LOD_CHECK_INTERVAL frames.
    frame_counter.0 = frame_counter.0.wrapping_add(1);
    if frame_counter.0 % LOD_CHECK_INTERVAL != 0 {
        return;
    }

    // Determine reference position: prefer chassis, fall back to camera.
    let ref_pos: Vec3 = if let Ok(chassis_xf) = chassis_q.single() {
        chassis_xf.translation
    } else if let Ok(cam_xf) = camera_q.single() {
        cam_xf.translation
    } else {
        // No chassis or camera yet (early startup) — keep everything visible.
        return;
    };

    let lod_sq = LOD_DISTANCE * LOD_DISTANCE;

    for (xf, lod, mut vis) in &mut props {
        // Use XZ distance only so elevated terrain doesn't cause premature hiding.
        let dx = xf.translation.x - ref_pos.x;
        let dz = xf.translation.z - ref_pos.z;
        // Include half_height as a bias on the distance to avoid pop-in for tall props.
        let dist_sq = dx * dx + dz * dz - lod.half_height * lod.half_height;
        *vis = if dist_sq > lod_sq {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
    }
}
