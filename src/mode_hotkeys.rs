// mode_hotkeys.rs — Sprint 68
//
// ModeHotkeysPlugin: Alt+letter fast-travel to any gameplay area without
// opening the Mission Select menu.
//
// Hotkeys (Alt must be held; single-letter bindings unaffected):
//   Alt+H  → Hillclimb Tiers — Beginner tier start (TierLayout.start_pos[0])
//   Alt+R  → Rock Crawl Trail — Boulder Stairs section start
//   Alt+O  → Obstacle Course — Beginner level (ObstacleCourseLayout.start_pos[0])
//   Alt+T  → Trail Rides — first manifest entry's spawn_x/spawn_z
//   Alt+S  → Reset to default spawn (origin, y = terrain + 1.5)
//
// Each teleport:
//   • Sets Transform::translation on the Chassis entity
//   • Zeros LinearVelocity + AngularVelocity (Avian components)
//   • Resets rotation to identity
//   • Pushes an EventLog entry for HUD feedback
//
// The existing single-letter bindings (H = hillclimb legacy, R = reset spawn,
// O = drone toggle, T = pause TOD, S = throttle reverse) are completely
// unaffected because we gate on Alt being pressed.
//
// Public API:
//   ModeHotkeysPlugin

use bevy::prelude::*;
use avian3d::prelude::{AngularVelocity, LinearVelocity};

use crate::events::EventLog;
use crate::hillclimb_tiers::TierLayout;
use crate::obstacle_course::ObstacleCourseLayout;
use crate::terrain::terrain_height_at;
use crate::trail_rides::TrailManifest;
use crate::vehicle::{Chassis, VehicleRoot};

// ---------------------------------------------------------------------------
// Rock Crawl section 0 (Boulder Stairs) start position.
// Mirrors the constants in rock_crawl_trail.rs (SECTION_CX[0] - CORRIDOR_HALF[0]).
// ---------------------------------------------------------------------------

const RC_BOULDER_STAIRS_X: f32 = 102.0; // 120.0 - 18.0
const RC_BOULDER_STAIRS_Z: f32 = 0.0;

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct ModeHotkeysPlugin;

impl Plugin for ModeHotkeysPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            handle_mode_hotkeys.run_if(resource_exists::<VehicleRoot>),
        );
    }
}

// ---------------------------------------------------------------------------
// System
// ---------------------------------------------------------------------------

fn handle_mode_hotkeys(
    keys:          Res<ButtonInput<KeyCode>>,
    hc_layout:     Res<TierLayout>,
    oc_layout:     Res<ObstacleCourseLayout>,
    manifest:      Res<TrailManifest>,
    vehicle:       Res<VehicleRoot>,
    mut chassis_q: Query<
        (
            &mut Transform,
            &mut LinearVelocity,
            &mut AngularVelocity,
        ),
        With<Chassis>,
    >,
    mut event_log: ResMut<EventLog>,
    time:          Res<Time>,
) {
    let alt = keys.pressed(KeyCode::AltLeft) || keys.pressed(KeyCode::AltRight);
    if !alt {
        return;
    }

    // Which alt-hotkey just fired (at most one per frame is typical).
    enum Target {
        Hillclimb,
        RockCrawl,
        Obstacle,
        Trail,
        Spawn,
    }

    let target = if keys.just_pressed(KeyCode::KeyH) {
        Some(Target::Hillclimb)
    } else if keys.just_pressed(KeyCode::KeyR) {
        Some(Target::RockCrawl)
    } else if keys.just_pressed(KeyCode::KeyO) {
        Some(Target::Obstacle)
    } else if keys.just_pressed(KeyCode::KeyT) {
        Some(Target::Trail)
    } else if keys.just_pressed(KeyCode::KeyS) {
        Some(Target::Spawn)
    } else {
        None
    };

    let Some(target) = target else { return };

    // Resolve destination.
    let (destination, label): (Vec3, &str) = match &target {
        Target::Hillclimb => {
            let pos = hc_layout.start_pos[0]; // Beginner tier
            (pos, "Hillclimb Tiers (Beginner)")
        }
        Target::RockCrawl => {
            let y = terrain_height_at(RC_BOULDER_STAIRS_X, RC_BOULDER_STAIRS_Z) + 1.5;
            (
                Vec3::new(RC_BOULDER_STAIRS_X, y, RC_BOULDER_STAIRS_Z),
                "Rock Crawl — Boulder Stairs",
            )
        }
        Target::Obstacle => {
            let pos = oc_layout.start_pos[0]; // Beginner level
            (pos, "Obstacle Course (Beginner)")
        }
        Target::Trail => {
            if let Some(trail) = manifest.trails.first() {
                let y = terrain_height_at(trail.spawn_x, trail.spawn_z) + 1.5;
                (Vec3::new(trail.spawn_x, y, trail.spawn_z), "Trail Rides")
            } else {
                info!("mode_hotkeys: Alt+T fired but TrailManifest is empty — no teleport");
                return;
            }
        }
        Target::Spawn => {
            let y = terrain_height_at(0.0, 0.0) + 1.5;
            (Vec3::new(0.0, y, 0.0), "Default Spawn")
        }
    };

    // Perform the teleport.
    match chassis_q.get_mut(vehicle.chassis) {
        Ok((mut tf, mut linvel, mut angvel)) => {
            tf.translation = destination;
            tf.rotation = Quat::IDENTITY;
            linvel.0 = Vec3::ZERO;
            angvel.0 = Vec3::ZERO;
            info!("mode_hotkeys: teleported to {} at {:.1?}", label, destination);

            // Push EventLog toast.
            let now = time.elapsed_secs();
            event_log.push_fast_travel(now, label);
        }
        Err(e) => {
            warn!("mode_hotkeys: could not get chassis entity: {}", e);
        }
    }
}
