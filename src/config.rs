// Persistent config file: loads ~/.skoffroad/config.json on startup,
// re-saves whenever SettingsState changes (debounced to 0.5 s).
//
// Public API:
//   ConfigPlugin
//   PersistedConfig  (resource; `loaded` is true after a successful file read)

use bevy::prelude::*;
use std::fs;
use std::io::Write as IoWrite;
use std::path::PathBuf;

use crate::settings::SettingsState;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

pub struct ConfigPlugin;

impl Plugin for ConfigPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(PersistedConfig::default())
            .insert_resource(SaveDebounce::default())
            .add_systems(Startup, load_config)
            .add_systems(Update, save_on_change);
    }
}

/// Marker resource; `loaded` is set to `true` after the config file was
/// successfully read and applied to `SettingsState`.
#[derive(Resource, Default, Clone)]
pub struct PersistedConfig {
    pub loaded: bool,
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Accumulates elapsed time since the last detected settings change so we
/// can debounce disk writes to at most once per 0.5 s.
#[derive(Resource, Default)]
struct SaveDebounce {
    pending:     bool,
    elapsed_s:   f32,
}

/// Flat representation of the three persisted f32 knobs.  Only these fields
/// are written; `paused` is intentionally excluded (it is session-only state).
struct ConfigData {
    master_volume:     f32,
    mouse_sensitivity: f32,
    day_length_s:      f32,
}

impl Default for ConfigData {
    fn default() -> Self {
        let d = SettingsState::default();
        Self {
            master_volume:     d.master_volume,
            mouse_sensitivity: d.mouse_sensitivity,
            day_length_s:      d.day_length_s,
        }
    }
}

// ---------------------------------------------------------------------------
// Config file path
// ---------------------------------------------------------------------------

fn config_path() -> PathBuf {
    // Prefer the `dirs`-equivalent: $HOME / .skoffroad / config.json.
    // We do NOT depend on the `dirs` crate here — just HOME.
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    let mut p = PathBuf::from(home);
    p.push(".skoffroad");
    p.push("config.json");
    p
}

// ---------------------------------------------------------------------------
// JSON serialisation (hand-rolled — avoids adding a serde derive to
// SettingsState itself; serde_json is already in Cargo.toml so we can use
// serde_json::Value for parsing).
// ---------------------------------------------------------------------------

fn to_json(data: &ConfigData) -> String {
    format!(
        "{{\n  \"master_volume\": {},\n  \"mouse_sensitivity\": {},\n  \"day_length_s\": {}\n}}",
        data.master_volume,
        data.mouse_sensitivity,
        data.day_length_s,
    )
}

fn from_json(src: &str) -> Option<ConfigData> {
    // Use serde_json (already a dep) for robust parsing.
    let v: serde_json::Value = serde_json::from_str(src).ok()?;
    let obj = v.as_object()?;

    let mut out = ConfigData::default();
    if let Some(n) = obj.get("master_volume").and_then(|x| x.as_f64()) {
        out.master_volume = (n as f32).clamp(0.0, 1.0);
    }
    if let Some(n) = obj.get("mouse_sensitivity").and_then(|x| x.as_f64()) {
        out.mouse_sensitivity = (n as f32).clamp(0.1, 3.0);
    }
    if let Some(n) = obj.get("day_length_s").and_then(|x| x.as_f64()) {
        out.day_length_s = (n as f32).clamp(30.0, 600.0);
    }
    Some(out)
}

// ---------------------------------------------------------------------------
// Startup system: read → parse → apply
// ---------------------------------------------------------------------------

fn load_config(
    mut settings: ResMut<SettingsState>,
    mut persisted: ResMut<PersistedConfig>,
) {
    let path = config_path();

    match fs::read_to_string(&path) {
        Err(e) => {
            info!(
                "config: no saved config at {} ({}); using defaults",
                path.display(),
                e
            );
        }
        Ok(text) => match from_json(&text) {
            None => {
                info!(
                    "config: could not parse {}; using defaults",
                    path.display()
                );
            }
            Some(data) => {
                settings.master_volume     = data.master_volume;
                settings.mouse_sensitivity = data.mouse_sensitivity;
                settings.day_length_s      = data.day_length_s;
                persisted.loaded           = true;
                info!(
                    "config: loaded from {} (vol={:.2}, sens={:.2}, day={:.0}s)",
                    path.display(),
                    data.master_volume,
                    data.mouse_sensitivity,
                    data.day_length_s,
                );
            }
        },
    }
}

// ---------------------------------------------------------------------------
// Update system: detect change → debounce 0.5 s → write
// ---------------------------------------------------------------------------

fn save_on_change(
    settings:  Res<SettingsState>,
    mut deb:   ResMut<SaveDebounce>,
    time:      Res<Time>,
) {
    // Arm the debounce timer whenever the resource is mutated.
    if settings.is_changed() {
        deb.pending   = true;
        deb.elapsed_s = 0.0;
        return;
    }

    if !deb.pending {
        return;
    }

    deb.elapsed_s += time.delta_secs();
    if deb.elapsed_s < 0.5 {
        return;
    }

    // Timer expired — write to disk.
    deb.pending   = false;
    deb.elapsed_s = 0.0;

    let data = ConfigData {
        master_volume:     settings.master_volume,
        mouse_sensitivity: settings.mouse_sensitivity,
        day_length_s:      settings.day_length_s,
    };
    let json = to_json(&data);
    let path = config_path();

    // Ensure the parent directory exists.
    if let Some(parent) = path.parent() {
        if let Err(e) = fs::create_dir_all(parent) {
            warn!("config: could not create directory {}: {}", parent.display(), e);
            return;
        }
    }

    match fs::File::create(&path) {
        Err(e) => {
            warn!("config: could not open {} for writing: {}", path.display(), e);
        }
        Ok(mut f) => {
            if let Err(e) = f.write_all(json.as_bytes()) {
                warn!("config: write failed for {}: {}", path.display(), e);
            } else {
                info!(
                    "config: saved to {} (vol={:.2}, sens={:.2}, day={:.0}s)",
                    path.display(),
                    data.master_volume,
                    data.mouse_sensitivity,
                    data.day_length_s,
                );
            }
        }
    }
}
