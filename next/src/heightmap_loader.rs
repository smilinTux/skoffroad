// Heightmap loader: scan assets/maps/*.png at startup, load each as a Bevy
// Image asset, and register it in LoadedHeightmaps by filename-without-extension.
//
// Industry-standard 16-bit grayscale PNG (World Machine / Gaea / Houdini).
// Bevy's image loader supports both 8-bit and 16-bit PNGs transparently;
// a 16-bit single-channel PNG is reported as TextureFormat::R16Unorm.
// Mesh generation from heightmap data is OUT OF SCOPE for this module — see
// terrain.rs and future sprint integration work.
//
// Public API:
//   HeightmapLoaderPlugin
//   LoadedHeightmaps  (Resource)
//   heightmap_to_mesh (stub — future sprints)

use bevy::{
    asset::RenderAssetUsages,
    mesh::{Indices, PrimitiveTopology},
    prelude::*,
};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct HeightmapLoaderPlugin;

impl Plugin for HeightmapLoaderPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LoadedHeightmaps>()
            .add_systems(Startup, scan_and_load_heightmaps);
    }
}

// ---------------------------------------------------------------------------
// Resource
// ---------------------------------------------------------------------------

/// Holds one AssetServer handle per discovered heightmap, keyed by the PNG
/// filename without its extension (e.g. "canyon_01" for "canyon_01.png").
#[derive(Resource, Default)]
pub struct LoadedHeightmaps {
    pub by_name: HashMap<String, Handle<Image>>,
}

// ---------------------------------------------------------------------------
// Startup system
// ---------------------------------------------------------------------------

fn scan_and_load_heightmaps(
    asset_server: Res<AssetServer>,
    mut loaded: ResMut<LoadedHeightmaps>,
) {
    let dir = std::fs::read_dir("assets/maps");

    let read_dir = match dir {
        Ok(rd) => rd,
        Err(_) => {
            info!("heightmap_loader: no assets/maps/ folder; user maps disabled");
            return;
        }
    };

    let mut n: usize = 0;

    for entry in read_dir.flatten() {
        let path = entry.path();

        // Filter to .png files only.
        let is_png = path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("png"))
            .unwrap_or(false);

        if !is_png {
            continue;
        }

        let Some(filename) = path.file_name().and_then(|f| f.to_str()) else {
            continue;
        };

        // Strip the extension to form the resource key.
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(filename)
            .to_string();

        // Load via AssetServer using the relative path under "assets/".
        // Bevy 0.18 resolves paths relative to the configured asset root.
        let asset_path = format!("maps/{filename}");
        let handle: Handle<Image> = asset_server.load(asset_path);

        loaded.by_name.insert(stem, handle);
        n += 1;
    }

    info!("heightmap_loader: loaded {n} heightmaps from assets/maps/");
}

// ---------------------------------------------------------------------------
// Future-use stub
// ---------------------------------------------------------------------------

/// Stub: convert a loaded heightmap Image into a Bevy Mesh.
///
/// Currently returns an empty mesh. Future sprints will sample pixel data from
/// `image.data`, build a vertex grid of `world_size_m` extent and heights
/// scaled to `max_height_m`, and emit a triangle-list mesh with normals.
///
/// Note on texture format: Bevy's PNG loader decodes 16-bit single-channel
/// PNGs (standard output from World Machine, Gaea, Houdini) as
/// `TextureFormat::R16Unorm`. Pixel values are stored as little-endian u16
/// pairs in `image.data`, giving 65 536 discrete elevation steps per texel.
pub fn heightmap_to_mesh(image: &Image, _world_size_m: Vec2, _max_height_m: f32) -> Mesh {
    let _ = image; // suppress unused-variable warning until implementation lands

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );

    // Populate with empty attribute arrays so the mesh is structurally valid.
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, Vec::<[f32; 3]>::new());
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, Vec::<[f32; 3]>::new());
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, Vec::<[f32; 2]>::new());
    mesh.insert_indices(Indices::U32(Vec::new()));

    mesh
}
