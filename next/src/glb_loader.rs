// glTF/GLB loader: scan assets/vehicles/*.glb at startup, register each
// loaded scene as a selectable vehicle skin via AssetServer. Bevy 0.18
// supports glTF 2.0 natively (.glb is the binary container).
//
// Public API:
//   GlbLoaderPlugin
//   LoadedVehicleGlbs (resource, list of asset handles by name)

use bevy::prelude::*;
use std::collections::HashMap;

pub struct GlbLoaderPlugin;

impl Plugin for GlbLoaderPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LoadedVehicleGlbs>()
            .add_systems(Startup, scan_and_load_vehicles);
    }
}

#[derive(Resource, Default)]
pub struct LoadedVehicleGlbs {
    pub by_name: HashMap<String, Handle<Scene>>,
}

fn scan_and_load_vehicles(
    asset_server: Res<AssetServer>,
    mut loaded: ResMut<LoadedVehicleGlbs>,
) {
    let dir = match std::fs::read_dir("assets/vehicles") {
        Ok(d) => d,
        Err(_) => {
            info!("glb_loader: no assets/vehicles/ folder; user vehicles disabled");
            return;
        }
    };

    for entry in dir.flatten() {
        let path = entry.path();
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        if ext.eq_ignore_ascii_case("glb") {
            let filename = match path.file_name().and_then(|n| n.to_str()) {
                Some(n) => n.to_owned(),
                None => continue,
            };
            let stem = match path.file_stem().and_then(|s| s.to_str()) {
                Some(s) => s.to_owned(),
                None => continue,
            };
            let asset_url = format!("vehicles/{}#Scene0", filename);
            let handle: Handle<Scene> = asset_server.load(asset_url);
            loaded.by_name.insert(stem, handle);
        }
    }

    let n = loaded.by_name.len();
    if n == 0 {
        info!("glb_loader: no .glb files found in assets/vehicles/; user vehicles disabled");
    } else {
        info!("glb_loader: loaded {} vehicles from assets/vehicles/", n);
    }
}
