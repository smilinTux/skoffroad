// Save / load system for SandK Offroad.
//
// Save file location (computed without external crates):
//   Linux  : $XDG_DATA_HOME/sandk-offroad/save.json
//          — fallback: $HOME/.local/share/sandk-offroad/save.json
//   Windows: $APPDATA/sandk-offroad/save.json
//   macOS  : $HOME/Library/Application Support/sandk-offroad/save.json
//   Else   : ./save.json  (logged as warning)
//
// Failure to read, parse, or write is always non-fatal — log and continue.

use std::{fs, path::PathBuf};

use bevy::prelude::*;
use avian3d::prelude::{AngularVelocity, LinearVelocity};
use serde::{Deserialize, Serialize};

use crate::hud::SessionStats;
use crate::sky::TimeOfDay;
use crate::vehicle::{Chassis, VehicleRoot};

// ---- Plugin -----------------------------------------------------------------

pub struct SavePlugin;

impl Plugin for SavePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostStartup, try_load_save)
           .add_systems(Update, (autosave_on_app_exit, manual_save));
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

// ---- Path resolution --------------------------------------------------------

fn resolve_save_path() -> PathBuf {
    // Try platform-specific data dirs in order; fall back to cwd.
    let base: Option<PathBuf> = {
        // 1. $XDG_DATA_HOME  (Linux, explicit)
        std::env::var("XDG_DATA_HOME").ok().map(PathBuf::from)
        // 2. $HOME/.local/share  (Linux, implicit)
        .or_else(|| {
            if cfg!(target_os = "linux") {
                std::env::var("HOME").ok()
                    .map(|h| PathBuf::from(h).join(".local").join("share"))
            } else {
                None
            }
        })
        // 3. $APPDATA  (Windows)
        .or_else(|| {
            if cfg!(target_os = "windows") {
                std::env::var("APPDATA").ok().map(PathBuf::from)
            } else {
                None
            }
        })
        // 4. $HOME/Library/Application Support  (macOS)
        .or_else(|| {
            if cfg!(target_os = "macos") {
                std::env::var("HOME").ok()
                    .map(|h| PathBuf::from(h)
                        .join("Library")
                        .join("Application Support"))
            } else {
                None
            }
        })
    };

    match base {
        Some(dir) => dir.join("sandk-offroad").join("save.json"),
        None => {
            warn!("save: could not determine data directory; using ./save.json");
            PathBuf::from("save.json")
        }
    }
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

fn write_save(save: &SaveFile) {
    let path = resolve_save_path();

    // Ensure parent directory exists.
    if let Some(parent) = path.parent() {
        if let Err(e) = fs::create_dir_all(parent) {
            error!("save: failed to create directory {}: {}", parent.display(), e);
            return;
        }
    }

    match serde_json::to_string_pretty(save) {
        Ok(json) => {
            match fs::write(&path, json) {
                Ok(()) => info!("save: wrote to {}", path.display()),
                Err(e) => error!("save: write failed {}: {}", path.display(), e),
            }
        }
        Err(e) => error!("save: serialisation failed: {}", e),
    }
}

// ---- PostStartup: load -------------------------------------------------------

// Runs once after Startup so the chassis entity already exists.
fn try_load_save(
    vehicle: Option<Res<VehicleRoot>>,
    mut chassis_q: Query<(&mut Transform, &mut LinearVelocity, &mut AngularVelocity), With<Chassis>>,
    mut tod: ResMut<TimeOfDay>,
    mut stats: Option<ResMut<SessionStats>>,
) {
    let path = resolve_save_path();

    let json = match fs::read_to_string(&path) {
        Ok(s) => s,
        Err(_) => {
            // File simply doesn't exist on first launch — not an error.
            info!("save: no save file found at {}; starting fresh", path.display());
            return;
        }
    };

    let save: SaveFile = match serde_json::from_str(&json) {
        Ok(s) => s,
        Err(e) => {
            warn!("save: failed to parse {}: {}", path.display(), e);
            return;
        }
    };

    // Restore time-of-day unconditionally.
    tod.t = save.time_of_day.t;
    tod.day_length_s = save.time_of_day.day_length_s;
    tod.paused = save.time_of_day.paused;

    // Restore chassis — requires the VehicleRoot resource.
    let Some(vehicle) = vehicle else {
        warn!("save: VehicleRoot not ready; skipping chassis restore");
        return;
    };

    let Ok((mut transform, mut lin_vel, mut ang_vel)) = chassis_q.get_mut(vehicle.chassis) else {
        warn!("save: chassis entity not found; skipping chassis restore");
        return;
    };

    let c = &save.chassis;
    transform.translation = Vec3::from(c.translation);
    transform.rotation = Quat::from_xyzw(c.rotation[0], c.rotation[1], c.rotation[2], c.rotation[3]);
    lin_vel.0 = Vec3::from(c.linear_vel);
    ang_vel.0 = Vec3::from(c.angular_vel);

    // Restore session stats if the resource exists (feature-detected at runtime).
    if let (Some(saved_stats), Some(ref mut res_stats)) = (&save.session_stats, &mut stats) {
        res_stats.distance_m    = saved_stats.distance_m;
        res_stats.max_speed_mps = saved_stats.max_speed_mps;
        res_stats.max_tilt_deg  = saved_stats.max_tilt_deg;
        res_stats.elapsed_s     = saved_stats.elapsed_s;
        // last_pos left as None so the distance accumulator doesn't get a false delta.
    }

    info!("save: restored from {}", path.display());
}

// ---- Update: save on AppExit ------------------------------------------------

fn autosave_on_app_exit(
    mut exit_events: MessageReader<AppExit>,
    vehicle: Option<Res<VehicleRoot>>,
    chassis_q: Query<(&Transform, &LinearVelocity, &AngularVelocity), With<Chassis>>,
    tod: Res<TimeOfDay>,
    stats: Option<Res<SessionStats>>,
) {
    // Only act when an exit event arrives.
    if exit_events.read().next().is_none() {
        return;
    }

    let Some(vehicle) = vehicle else { return };
    let Ok((transform, lin_vel, ang_vel)) = chassis_q.get(vehicle.chassis) else { return };

    let save = build_save(transform, lin_vel, ang_vel, &tod, stats.as_deref());
    write_save(&save);
}

// ---- Update: F5 manual save -------------------------------------------------

fn manual_save(
    keys: Res<ButtonInput<KeyCode>>,
    vehicle: Option<Res<VehicleRoot>>,
    chassis_q: Query<(&Transform, &LinearVelocity, &AngularVelocity), With<Chassis>>,
    tod: Res<TimeOfDay>,
    stats: Option<Res<SessionStats>>,
) {
    if !keys.just_pressed(KeyCode::F5) {
        return;
    }

    let Some(vehicle) = vehicle else { return };
    let Ok((transform, lin_vel, ang_vel)) = chassis_q.get(vehicle.chassis) else { return };

    let save = build_save(transform, lin_vel, ang_vel, &tod, stats.as_deref());
    write_save(&save);
}
