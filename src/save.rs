// Save / load system for skoffroad.
//
// Save file location (computed without external crates):
//   Linux  : $XDG_DATA_HOME/skoffroad/save_N.json
//          — fallback: $HOME/.local/share/skoffroad/save_N.json
//   Windows: $APPDATA/skoffroad/save_N.json
//   macOS  : $HOME/Library/Application Support/skoffroad/save_N.json
//   Else   : ./save_N.json  (logged as warning)
//
// Three slots:  save_1.json (autosave / slot 1)
//               save_2.json (slot 2)
//               save_3.json (slot 3)
//
// Failure to read, parse, or write is always non-fatal — log and continue.
//
// Key bindings:
//   F1  — load slot 1
//   F2  — load slot 2
//   F4  — load slot 3  (F3 is reserved for the dev inspector in main.rs)
//   F5  — save slot 1
//   F6  — save slot 2
//   F7  — save slot 3
//   R   — reset chassis to spawn (no file I/O)

use bevy::prelude::*;
use avian3d::prelude::{AngularVelocity, LinearVelocity};
use serde::{Deserialize, Serialize};

use crate::hud::SessionStats;
use crate::platform_storage;
use crate::sky::TimeOfDay;
use crate::vehicle::{Chassis, VehicleRoot};

// ---- Plugin -----------------------------------------------------------------

pub struct SavePlugin;

impl Plugin for SavePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostStartup, try_load_save)
           .add_systems(Update, (
               autosave_on_app_exit,
               manual_save,
               manual_load,
               reset_chassis,
           ));
    }
}

// ---- Save-file schema -------------------------------------------------------

#[derive(Serialize, Deserialize)]
struct SaveFile {
    version: u32,
    chassis: ChassisSave,
    time_of_day: TimeOfDaySave,
    #[serde(skip_serializing_if = "Option::is_none")]
    session_stats: Option<SessionStatsSave>,
}

#[derive(Serialize, Deserialize)]
struct ChassisSave {
    translation: [f32; 3],
    rotation: [f32; 4],       // [x, y, z, w]
    linear_vel: [f32; 3],
    angular_vel: [f32; 3],
}

#[derive(Serialize, Deserialize)]
struct TimeOfDaySave {
    t: f32,
    day_length_s: f32,
    paused: bool,
}

#[derive(Serialize, Deserialize)]
struct SessionStatsSave {
    distance_m: f32,
    max_speed_mps: f32,
    max_tilt_deg: f32,
    elapsed_s: f32,
}

// ---- Storage key ------------------------------------------------------------

/// Slot key (1..=3). Save files now live in $HOME/.skoffroad/save_<slot>.json
/// on native and localStorage["skoffroad/save_<slot>.json"] in WASM. Earlier
/// versions used XDG_DATA_HOME / APPDATA / Library — those legacy files are
/// orphaned but the unified path keeps the desktop and browser builds in sync.
fn save_key(slot: u8) -> String {
    format!("save_{}.json", slot)
}

fn save_label(slot: u8) -> String {
    let key = save_key(slot);
    platform_storage::debug_path(&key)
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| format!("localStorage[{}]", key))
}

// ---- Serialise / deserialise helpers ----------------------------------------

fn build_save(
    transform: &Transform,
    lin_vel: &LinearVelocity,
    ang_vel: &AngularVelocity,
    tod: &TimeOfDay,
    stats: Option<&SessionStats>,
) -> SaveFile {
    let q = transform.rotation;
    SaveFile {
        version: 1,
        chassis: ChassisSave {
            translation: transform.translation.into(),
            rotation: [q.x, q.y, q.z, q.w],
            linear_vel: lin_vel.0.into(),
            angular_vel: ang_vel.0.into(),
        },
        time_of_day: TimeOfDaySave {
            t: tod.t,
            day_length_s: tod.day_length_s,
            paused: tod.paused,
        },
        session_stats: stats.map(|s| SessionStatsSave {
            distance_m: s.distance_m,
            max_speed_mps: s.max_speed_mps,
            max_tilt_deg: s.max_tilt_deg,
            elapsed_s: s.elapsed_s,
        }),
    }
}

fn write_save(save: &SaveFile, slot: u8) {
    let key   = save_key(slot);
    let label = save_label(slot);

    match serde_json::to_string_pretty(save) {
        Ok(json) => match platform_storage::write_string(&key, &json) {
            Ok(()) => info!("save: wrote slot {} to {}", slot, label),
            Err(e) => error!("save: {}", e),
        },
        Err(e) => error!("save: serialisation failed: {}", e),
    }
}

fn apply_save(
    save: &SaveFile,
    path_label: &str,
    vehicle: &VehicleRoot,
    chassis_q: &mut Query<(&mut Transform, &mut LinearVelocity, &mut AngularVelocity), With<Chassis>>,
    tod: &mut ResMut<TimeOfDay>,
    stats: &mut Option<ResMut<SessionStats>>,
) {
    tod.t            = save.time_of_day.t;
    tod.day_length_s = save.time_of_day.day_length_s;
    tod.paused       = save.time_of_day.paused;

    let Ok((mut transform, mut lin_vel, mut ang_vel)) = chassis_q.get_mut(vehicle.chassis) else {
        warn!("save: chassis entity not found; skipping chassis restore");
        return;
    };

    let c = &save.chassis;
    transform.translation = Vec3::from(c.translation);
    transform.rotation    = Quat::from_xyzw(c.rotation[0], c.rotation[1], c.rotation[2], c.rotation[3]);
    lin_vel.0  = Vec3::from(c.linear_vel);
    ang_vel.0  = Vec3::from(c.angular_vel);

    if let (Some(saved_stats), Some(ref mut res_stats)) = (&save.session_stats, stats) {
        res_stats.distance_m    = saved_stats.distance_m;
        res_stats.max_speed_mps = saved_stats.max_speed_mps;
        res_stats.max_tilt_deg  = saved_stats.max_tilt_deg;
        res_stats.elapsed_s     = saved_stats.elapsed_s;
        // last_pos left as None so the distance accumulator doesn't get a false delta.
    }

    info!("save: restored from {}", path_label);
}

fn read_save(slot: u8) -> Option<(SaveFile, String)> {
    let key   = save_key(slot);
    let label = save_label(slot);

    let json = match platform_storage::read_string(&key) {
        Some(s) => s,
        None => {
            info!("save: no save file found at {}; starting fresh", label);
            return None;
        }
    };

    match serde_json::from_str::<SaveFile>(&json) {
        Ok(s)  => Some((s, label)),
        Err(e) => {
            warn!("save: failed to parse {}: {}", label, e);
            None
        }
    }
}

// ---- PostStartup: load slot 1 (autosave) ------------------------------------

// Runs once after Startup so the chassis entity already exists.
fn try_load_save(
    vehicle: Option<Res<VehicleRoot>>,
    mut chassis_q: Query<(&mut Transform, &mut LinearVelocity, &mut AngularVelocity), With<Chassis>>,
    mut tod: ResMut<TimeOfDay>,
    mut stats: Option<ResMut<SessionStats>>,
) {
    let Some((save, path)) = read_save(1) else { return };

    // Restore time-of-day unconditionally; chassis requires VehicleRoot.
    tod.t            = save.time_of_day.t;
    tod.day_length_s = save.time_of_day.day_length_s;
    tod.paused       = save.time_of_day.paused;

    let Some(vehicle) = vehicle else {
        warn!("save: VehicleRoot not ready; skipping chassis restore");
        return;
    };

    apply_save(&save, &path, &vehicle, &mut chassis_q, &mut tod, &mut stats);
}

// ---- Update: autosave on AppExit (slot 1) -----------------------------------

fn autosave_on_app_exit(
    mut exit_events: MessageReader<AppExit>,
    vehicle: Option<Res<VehicleRoot>>,
    chassis_q: Query<(&Transform, &LinearVelocity, &AngularVelocity), With<Chassis>>,
    tod: Res<TimeOfDay>,
    stats: Option<Res<SessionStats>>,
) {
    if exit_events.read().next().is_none() {
        return;
    }

    let Some(vehicle) = vehicle else { return };
    let Ok((transform, lin_vel, ang_vel)) = chassis_q.get(vehicle.chassis) else { return };

    let save = build_save(transform, lin_vel, ang_vel, &tod, stats.as_deref());
    write_save(&save, 1);
}

// ---- Update: manual save (F5 = slot 1, F6 = slot 2, F7 = slot 3) -----------

fn manual_save(
    keys: Res<ButtonInput<KeyCode>>,
    vehicle: Option<Res<VehicleRoot>>,
    chassis_q: Query<(&Transform, &LinearVelocity, &AngularVelocity), With<Chassis>>,
    tod: Res<TimeOfDay>,
    stats: Option<Res<SessionStats>>,
) {
    let slot = if keys.just_pressed(KeyCode::F5) {
        1
    } else if keys.just_pressed(KeyCode::F6) {
        2
    } else if keys.just_pressed(KeyCode::F7) {
        3
    } else {
        return;
    };

    let Some(vehicle) = vehicle else { return };
    let Ok((transform, lin_vel, ang_vel)) = chassis_q.get(vehicle.chassis) else { return };

    let save = build_save(transform, lin_vel, ang_vel, &tod, stats.as_deref());
    write_save(&save, slot);
}

// ---- Update: manual load (F1 = slot 1, F2 = slot 2, F4 = slot 3) -----------
//
// F3 is intentionally skipped: main.rs binds F3 to the dev inspector toggle
// when compiled with `--features dev`. Using F4 here avoids the conflict.

fn manual_load(
    keys: Res<ButtonInput<KeyCode>>,
    vehicle: Option<Res<VehicleRoot>>,
    mut chassis_q: Query<(&mut Transform, &mut LinearVelocity, &mut AngularVelocity), With<Chassis>>,
    mut tod: ResMut<TimeOfDay>,
    mut stats: Option<ResMut<SessionStats>>,
) {
    let slot = if keys.just_pressed(KeyCode::F1) {
        1
    } else if keys.just_pressed(KeyCode::F2) {
        2
    } else if keys.just_pressed(KeyCode::F4) {
        3
    } else {
        return;
    };

    let Some((save, path)) = read_save(slot) else { return };

    let Some(vehicle) = vehicle else {
        warn!("save: VehicleRoot not ready; skipping load");
        return;
    };

    apply_save(&save, &path, &vehicle, &mut chassis_q, &mut tod, &mut stats);
}

// ---- Update: R — reset chassis to spawn, no file I/O ------------------------

fn reset_chassis(
    keys: Res<ButtonInput<KeyCode>>,
    vehicle: Option<Res<VehicleRoot>>,
    mut chassis_q: Query<(&mut Transform, &mut LinearVelocity, &mut AngularVelocity), With<Chassis>>,
) {
    if !keys.just_pressed(KeyCode::KeyR) {
        return;
    }

    let Some(vehicle) = vehicle else { return };
    let Ok((mut transform, mut lin_vel, mut ang_vel)) = chassis_q.get_mut(vehicle.chassis) else {
        return;
    };

    transform.translation = Vec3::new(0.0, 8.0, 0.0);
    transform.rotation    = Quat::IDENTITY;
    lin_vel.0  = Vec3::ZERO;
    ang_vel.0  = Vec3::ZERO;

    info!("save: chassis reset");
}
