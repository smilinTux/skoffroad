// Asset manifest: JSON-based config in assets/manifest.json declaring
// vehicle classes (mass, glb path, license), map registry (heightmap, license),
// and mods (name, version, author, license, root path).
//
// Parses with serde_json (already in Cargo.toml). TOML not required.
// F10 hot-reloads the manifest at runtime.
//
// Public API:
//   AssetManifestPlugin
//   AssetManifest (resource)
//   VehicleClassEntry, MapEntry, ModEntry

use bevy::prelude::*;

// ── Acceptable open-source / attribution licenses ───────────────────────────
const ALLOWED_LICENSES: &[&str] = &[
    "CC0",
    "CC-BY-4.0",
    "CC-BY-SA-4.0",
    "MIT",
    "Apache-2.0",
    "Unlicense",
    "Public Domain",
];

// ── Entry types ──────────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub struct VehicleClassEntry {
    pub name: String,
    pub glb_path: String,
    pub mass_kg: f32,
    pub license: String,
    pub author: String,
}

#[derive(Clone, Debug)]
pub struct MapEntry {
    pub name: String,
    pub heightmap_path: String,
    pub license: String,
}

#[derive(Clone, Debug)]
pub struct ModEntry {
    pub name: String,
    pub version: String,
    pub author: String,
    pub license: String,
    pub root: String,
}

// ── Resource ─────────────────────────────────────────────────────────────────

#[derive(Resource, Default, Clone)]
pub struct AssetManifest {
    pub vehicles: Vec<VehicleClassEntry>,
    pub maps: Vec<MapEntry>,
    pub mods: Vec<ModEntry>,
}

// ── Plugin ───────────────────────────────────────────────────────────────────

pub struct AssetManifestPlugin;

impl Plugin for AssetManifestPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AssetManifest>()
            .add_systems(Startup, load_manifest)
            .add_systems(Update, reload_on_f10);
    }
}

// ── Parsing helpers ──────────────────────────────────────────────────────────

/// Returns `true` when `license` is in the allow-list (case-sensitive match).
fn license_ok(license: &str) -> bool {
    ALLOWED_LICENSES.contains(&license)
}

/// Read `assets/manifest.json` from disk and populate an `AssetManifest`.
/// Returns `None` only when the file is missing; parse/validation errors are
/// logged as warnings and the affected entry is skipped.
fn parse_manifest(path: &str) -> Option<AssetManifest> {
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(_) => return None,
    };

    let root: serde_json::Value = match serde_json::from_str(&text) {
        Ok(v) => v,
        Err(e) => {
            warn!("asset_manifest: failed to parse {}: {}", path, e);
            return Some(AssetManifest::default());
        }
    };

    let mut manifest = AssetManifest::default();

    // ── vehicles ─────────────────────────────────────────────────────────────
    if let Some(arr) = root.get("vehicles").and_then(|v| v.as_array()) {
        for entry in arr {
            let license = entry
                .get("license")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if license.is_empty() {
                warn!("rejected manifest entry: missing license");
                continue;
            }
            if !license_ok(license) {
                warn!(
                    "rejected manifest entry: unrecognised license \"{}\"",
                    license
                );
                continue;
            }
            let name = entry
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let glb_path = entry
                .get("glb_path")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let mass_kg = entry
                .get("mass_kg")
                .and_then(|v| v.as_f64())
                .unwrap_or(1000.0) as f32;
            let author = entry
                .get("author")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            manifest.vehicles.push(VehicleClassEntry {
                name,
                glb_path,
                mass_kg,
                license: license.to_string(),
                author,
            });
        }
    }

    // ── maps ──────────────────────────────────────────────────────────────────
    if let Some(arr) = root.get("maps").and_then(|v| v.as_array()) {
        for entry in arr {
            let license = entry
                .get("license")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if license.is_empty() {
                warn!("rejected manifest entry: missing license");
                continue;
            }
            if !license_ok(license) {
                warn!(
                    "rejected manifest entry: unrecognised license \"{}\"",
                    license
                );
                continue;
            }
            let name = entry
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let heightmap_path = entry
                .get("heightmap_path")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            manifest.maps.push(MapEntry {
                name,
                heightmap_path,
                license: license.to_string(),
            });
        }
    }

    // ── mods ──────────────────────────────────────────────────────────────────
    if let Some(arr) = root.get("mods").and_then(|v| v.as_array()) {
        for entry in arr {
            let license = entry
                .get("license")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if license.is_empty() {
                warn!("rejected manifest entry: missing license");
                continue;
            }
            if !license_ok(license) {
                warn!(
                    "rejected manifest entry: unrecognised license \"{}\"",
                    license
                );
                continue;
            }
            let name = entry
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let version = entry
                .get("version")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let author = entry
                .get("author")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let root_path = entry
                .get("root")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            manifest.mods.push(ModEntry {
                name,
                version,
                author,
                license: license.to_string(),
                root: root_path,
            });
        }
    }

    Some(manifest)
}

// ── Systems ──────────────────────────────────────────────────────────────────

const MANIFEST_PATH: &str = "assets/manifest.json";

fn load_manifest(mut manifest: ResMut<AssetManifest>) {
    match parse_manifest(MANIFEST_PATH) {
        Some(m) => {
            info!(
                "asset_manifest: loaded {} vehicles, {} maps, {} mods",
                m.vehicles.len(),
                m.maps.len(),
                m.mods.len()
            );
            *manifest = m;
        }
        None => {
            info!(
                "asset_manifest: {} not found; using empty manifest",
                MANIFEST_PATH
            );
        }
    }
}

fn reload_on_f10(keys: Res<ButtonInput<KeyCode>>, mut manifest: ResMut<AssetManifest>) {
    if !keys.just_pressed(KeyCode::F10) {
        return;
    }
    match parse_manifest(MANIFEST_PATH) {
        Some(m) => {
            let nv = m.vehicles.len();
            let nm = m.maps.len();
            let nd = m.mods.len();
            *manifest = m;
            info!(
                "manifest reloaded: {} vehicles, {} maps, {} mods",
                nv, nm, nd
            );
        }
        None => {
            info!(
                "asset_manifest: F10 reload — {} not found; manifest unchanged",
                MANIFEST_PATH
            );
        }
    }
}
