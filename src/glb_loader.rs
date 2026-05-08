// glTF/GLB loader: registers every vehicle declared in `assets/manifest.json`
// as a selectable scene asset via AssetServer.
//
// Sprint 46 reworked this to be **manifest-driven** instead of doing a live
// `std::fs::read_dir` scan of `assets/vehicles/`. Reasons:
//   1. WASM has no filesystem — the read_dir would always fail and the WASM
//      build couldn't ship a vehicle.
//   2. Manifest gives us metadata (mass, license, author) that the loose-file
//      scan was throwing away.
//   3. Users still drop their own GLBs in via the documented manifest entry
//      (see `docs/USER_VEHICLES.md`).
//
// Public API:
//   GlbLoaderPlugin
//   LoadedVehicleGlbs (resource, scene handle + metadata by stem name)

use bevy::prelude::*;
use std::collections::HashMap;

use crate::asset_manifest::{AssetManifest, VehicleClassEntry};

pub struct GlbLoaderPlugin;

impl Plugin for GlbLoaderPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LoadedVehicleGlbs>()
            .add_systems(PostStartup, load_vehicles_from_manifest);
    }
}

/// Snapshot of a single manifest vehicle that finished spawning a scene
/// handle. `meta` is the source manifest entry, `scene` the loaded GLB scene.
#[derive(Clone, Debug)]
pub struct VehicleAsset {
    pub meta: VehicleClassEntry,
    pub scene: Handle<Scene>,
}

#[derive(Resource, Default)]
pub struct LoadedVehicleGlbs {
    /// Keyed by basename of the GLB (e.g. `truck` for `vehicles/truck.glb`).
    pub by_name: HashMap<String, Handle<Scene>>,
    /// Full metadata-bearing entries in manifest order.
    pub entries: Vec<VehicleAsset>,
}

fn load_vehicles_from_manifest(
    asset_server: Res<AssetServer>,
    manifest: Res<AssetManifest>,
    mut loaded: ResMut<LoadedVehicleGlbs>,
) {
    if manifest.vehicles.is_empty() {
        info!("glb_loader: manifest declares no vehicles; user vehicles disabled");
        return;
    }

    for v in &manifest.vehicles {
        // Default scene of a glTF asset is named "Scene0" by Bevy's loader.
        let url = format!("{}#Scene0", v.glb_path);
        let scene: Handle<Scene> = asset_server.load(url);

        let stem = std::path::Path::new(&v.glb_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .map(|s| s.to_owned())
            .unwrap_or_else(|| v.name.clone());

        loaded.by_name.insert(stem.clone(), scene.clone());
        loaded.entries.push(VehicleAsset {
            meta: v.clone(),
            scene,
        });
    }

    info!(
        "glb_loader: loaded {} vehicles from manifest",
        loaded.entries.len()
    );
}
