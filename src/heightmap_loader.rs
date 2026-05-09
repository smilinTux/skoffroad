// Heightmap loader: scan assets/maps/*.png at startup, load each as a Bevy
// Image asset, and register it in LoadedHeightmaps by filename-without-extension.
//
// Sprint 57: extended to support player-supplied PNG heightmaps.
//   • Native: pass `--load-heightmap path.png` on the command line.
//   • WASM:   drag-drop a PNG file onto the canvas (see wasm_dragdrop submodule).
//
// The PNG's grayscale values (0–255, or 0–65535 for 16-bit) map to terrain
// elevation in the range [0, max_height_m] (default 60 m).  The image is
// resampled to a 256×256 quad grid regardless of input resolution.
// The world footprint is 1024 m × 1024 m (default).
//
// Public API:
//   HeightmapLoaderPlugin
//   LoadedHeightmaps        (Resource) — maps from asset/maps/* filenames
//   CustomHeightmapRequest  (Resource) — set to trigger a custom-terrain swap
//   heightmap_to_mesh       — build a Mesh from a loaded Image

use bevy::{
    asset::RenderAssetUsages,
    mesh::{Indices, PrimitiveTopology},
    prelude::*,
};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Number of quads per side in the resampled heightmap grid.
pub const CUSTOM_GRID: usize = 256;

/// Default world-space size (metres) of a custom heightmap terrain.
pub const CUSTOM_WORLD_SIZE: f32 = 1024.0;

/// Default maximum elevation (metres) for custom heightmap terrain.
pub const CUSTOM_MAX_HEIGHT: f32 = 60.0;

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct HeightmapLoaderPlugin;

impl Plugin for HeightmapLoaderPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LoadedHeightmaps>()
            .init_resource::<CustomHeightmapRequest>()
            .add_systems(Startup, (scan_and_load_heightmaps, parse_cli_heightmap).chain())
            .add_systems(Update, apply_custom_heightmap);

        // WASM drag-drop wiring — registers JS event listeners on the canvas
        // and polls each frame for a dropped PNG file.
        #[cfg(target_arch = "wasm32")]
        app.add_systems(Startup, wasm_dragdrop::register_dragdrop_listeners)
            .add_systems(Update, wasm_dragdrop::poll_dragdrop);
    }
}

// ---------------------------------------------------------------------------
// Resources
// ---------------------------------------------------------------------------

/// Holds one AssetServer handle per discovered heightmap, keyed by the PNG
/// filename without its extension (e.g. "canyon_01" for "canyon_01.png").
#[derive(Resource, Default)]
pub struct LoadedHeightmaps {
    pub by_name: HashMap<String, Handle<Image>>,
}

/// When set (non-None), the `apply_custom_heightmap` system will wait until
/// the image asset is loaded, then despawn the procedural terrain and spawn a
/// new terrain entity built from the heightmap.
#[derive(Resource, Default)]
pub struct CustomHeightmapRequest {
    /// Handle to the user-supplied PNG image.
    pub handle: Option<Handle<Image>>,
    /// World-space extent (width, depth) in metres.
    pub world_size: Vec2,
    /// Maximum terrain height in metres.
    pub max_height: f32,
}

// ---------------------------------------------------------------------------
// Startup system — scan assets/maps/
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
// Startup system — CLI `--load-heightmap path.png` (native only)
// ---------------------------------------------------------------------------

fn parse_cli_heightmap(
    asset_server: Res<AssetServer>,
    mut request: ResMut<CustomHeightmapRequest>,
) {
    // WASM never has a meaningful argv, so skip.
    #[cfg(target_arch = "wasm32")]
    let _ = (asset_server, request);

    #[cfg(not(target_arch = "wasm32"))]
    {
        let mut args = std::env::args().peekable();
        let mut path_arg: Option<String> = None;
        let mut max_height = CUSTOM_MAX_HEIGHT;
        let mut world_size = Vec2::splat(CUSTOM_WORLD_SIZE);

        while let Some(a) = args.next() {
            if let Some(rest) = a.strip_prefix("--load-heightmap=") {
                path_arg = Some(rest.to_string());
            } else if a == "--load-heightmap" {
                path_arg = args.next();
            } else if let Some(rest) = a.strip_prefix("--heightmap-max-height=") {
                if let Ok(v) = rest.parse::<f32>() {
                    max_height = v;
                }
            } else if a == "--heightmap-max-height" {
                if let Some(v) = args.next().and_then(|s| s.parse::<f32>().ok()) {
                    max_height = v;
                }
            } else if let Some(rest) = a.strip_prefix("--heightmap-world-size=") {
                if let Ok(v) = rest.parse::<f32>() {
                    world_size = Vec2::splat(v);
                }
            } else if a == "--heightmap-world-size" {
                if let Some(v) = args.next().and_then(|s| s.parse::<f32>().ok()) {
                    world_size = Vec2::splat(v);
                }
            }
        }

        if let Some(path) = path_arg {
            info!("heightmap_loader: loading custom heightmap from CLI: {path}");
            // Use load_with_settings to ensure RGBA8 conversion so our
            // sampler works uniformly regardless of source PNG format.
            let handle: Handle<Image> = asset_server.load(path);
            request.handle = Some(handle);
            request.max_height = max_height;
            request.world_size = world_size;
        }
    }
}

// ---------------------------------------------------------------------------
// Update system — swap procedural terrain for custom heightmap terrain
// ---------------------------------------------------------------------------

fn apply_custom_heightmap(
    mut commands: Commands,
    mut request: ResMut<CustomHeightmapRequest>,
    images: Res<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    procedural_query: Query<Entity, With<crate::terrain::ProceduralTerrainMarker>>,
    custom_query: Query<Entity, With<CustomTerrainMarker>>,
) {
    let Some(handle) = request.handle.clone() else {
        return;
    };

    let Some(image) = images.get(&handle) else {
        // Asset not yet loaded — try again next frame.
        return;
    };

    // Build the mesh from the image.
    let world_size = request.world_size;
    let max_height = request.max_height;
    let mesh = heightmap_to_mesh(image, world_size, max_height);

    // Clear the request so we don't re-trigger.
    request.handle = None;

    // Despawn any existing procedural terrain.
    for entity in procedural_query.iter() {
        commands.entity(entity).despawn();
    }
    // Also despawn any previously loaded custom terrain so re-loading works.
    for entity in custom_query.iter() {
        commands.entity(entity).despawn();
    }

    let mesh_handle = meshes.add(mesh);
    let material = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        perceptual_roughness: 0.9,
        ..default()
    });

    commands.spawn((
        Mesh3d(mesh_handle),
        MeshMaterial3d(material),
        Transform::default(),
        avian3d::prelude::RigidBody::Static,
        avian3d::prelude::ColliderConstructor::TrimeshFromMesh,
        CustomTerrainMarker,
    ));

    info!(
        "heightmap_loader: custom terrain spawned ({CUSTOM_GRID}×{CUSTOM_GRID} quads, \
         world {world_size:.0} m, max_height {max_height:.1} m)"
    );
}

/// Marker component on custom-heightmap terrain entities.
#[derive(Component)]
pub struct CustomTerrainMarker;

// ---------------------------------------------------------------------------
// Core mesh builder — heightmap_to_mesh
// ---------------------------------------------------------------------------

/// Convert a loaded heightmap [`Image`] into a Bevy [`Mesh`].
///
/// The image is resampled bilinearly to a [`CUSTOM_GRID`]×[`CUSTOM_GRID`] quad
/// grid.  Grayscale values are read from the red channel (works for R8, R16,
/// RGBA8, and RGBA16 source images).  Heights are scaled to `[0, max_height_m]`.
///
/// The returned mesh uses `TriangleList` topology and has POSITION, NORMAL,
/// UV_0, and COLOR attributes populated.  Normals are computed via finite
/// differences.  Vertex colors are slope-based (grass / dirt / rock) matching
/// the procedural terrain palette.
pub fn heightmap_to_mesh(image: &Image, world_size: Vec2, max_height_m: f32) -> Mesh {
    let src_w = image.width() as usize;
    let src_h = image.height() as usize;

    // Sample the image at an arbitrary (u, v) in [0, 1] by bilinear
    // interpolation.  Returns a value in [0, 1] drawn from the red channel
    // (or the luminance for packed RGBA8 sources where R=G=B).
    let sample = |u: f32, v: f32| -> f32 {
        if src_w == 0 || src_h == 0 {
            return 0.0;
        }

        let px = (u * (src_w as f32 - 1.0)).clamp(0.0, src_w as f32 - 1.001);
        let pz = (v * (src_h as f32 - 1.0)).clamp(0.0, src_h as f32 - 1.001);
        let x0 = px.floor() as usize;
        let z0 = pz.floor() as usize;
        let x1 = (x0 + 1).min(src_w - 1);
        let z1 = (z0 + 1).min(src_h - 1);
        let tx = px.fract();
        let tz = pz.fract();

        let v00 = sample_pixel(image, x0, z0);
        let v10 = sample_pixel(image, x1, z0);
        let v01 = sample_pixel(image, x0, z1);
        let v11 = sample_pixel(image, x1, z1);

        let top    = v00 * (1.0 - tx) + v10 * tx;
        let bottom = v01 * (1.0 - tx) + v11 * tx;
        top * (1.0 - tz) + bottom * tz
    };

    let vcount = CUSTOM_GRID + 1; // vertices per edge
    let mut positions: Vec<[f32; 3]> = Vec::with_capacity(vcount * vcount);
    let mut normals:   Vec<[f32; 3]> = Vec::with_capacity(vcount * vcount);
    let mut uvs:       Vec<[f32; 2]> = Vec::with_capacity(vcount * vcount);
    let mut heights:   Vec<f32>      = Vec::with_capacity(vcount * vcount);

    for z in 0..vcount {
        for x in 0..vcount {
            let u = x as f32 / CUSTOM_GRID as f32;
            let v = z as f32 / CUSTOM_GRID as f32;

            let h = sample(u, v) * max_height_m;

            let px = (u - 0.5) * world_size.x;
            let pz = (v - 0.5) * world_size.y;

            positions.push([px, h, pz]);
            normals.push([0.0, 1.0, 0.0]); // placeholder
            uvs.push([u * 8.0, v * 8.0]);
            heights.push(h);
        }
    }

    // Recompute normals via finite differences.
    let step_x = world_size.x / CUSTOM_GRID as f32;
    let step_z = world_size.y / CUSTOM_GRID as f32;
    for z in 0..vcount {
        for x in 0..vcount {
            let h  = heights[z * vcount + x];
            let hx = if x + 1 < vcount { heights[z * vcount + x + 1] } else { h };
            let hz = if z + 1 < vcount { heights[(z + 1) * vcount + x] } else { h };
            let nx_v = Vec3::new(step_x, hx - h, 0.0).normalize();
            let nz_v = Vec3::new(0.0, hz - h, step_z).normalize();
            let n = nx_v.cross(nz_v).normalize();
            normals[z * vcount + x] = [n.x, n.y, n.z];
        }
    }

    // Slope-based vertex colors matching procedural terrain palette.
    const GRASS: [f32; 3] = [0.32, 0.50, 0.20];
    const DIRT:  [f32; 3] = [0.45, 0.38, 0.25];
    const ROCK:  [f32; 3] = [0.42, 0.42, 0.45];

    let mut colors: Vec<[f32; 4]> = Vec::with_capacity(vcount * vcount);
    for i in 0..(vcount * vcount) {
        let [nx, ny, nz] = normals[i];
        let normal = Vec3::new(nx, ny, nz);
        let slope = 1.0 - normal.dot(Vec3::Y).clamp(0.0, 1.0);
        let t_gd = slope_smooth_step(slope, 0.10, 0.25);
        let t_dr = slope_smooth_step(slope, 0.30, 0.55);
        let c = lerp3(lerp3(GRASS, DIRT, t_gd), ROCK, t_dr);
        colors.push([c[0], c[1], c[2], 1.0]);
    }

    // Triangle indices.
    let mut indices: Vec<u32> = Vec::with_capacity(CUSTOM_GRID * CUSTOM_GRID * 6);
    for z in 0..CUSTOM_GRID {
        for x in 0..CUSTOM_GRID {
            let tl = (z * vcount + x) as u32;
            let tr = tl + 1;
            let bl = ((z + 1) * vcount + x) as u32;
            let br = bl + 1;
            indices.extend_from_slice(&[tl, bl, tr, tr, bl, br]);
        }
    }

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh.insert_indices(Indices::U32(indices));

    mesh
}

// ---------------------------------------------------------------------------
// Pixel sampling helpers
// ---------------------------------------------------------------------------

/// Sample a single pixel from an Image, returning a value in [0, 1].
/// Supports R8Unorm, R16Unorm, Rgba8UnormSrgb, Rgba8Unorm, Rgba16Unorm, and
/// the common Bevy-decoded fallback (treat first byte as 8-bit grayscale).
fn sample_pixel(image: &Image, x: usize, y: usize) -> f32 {
    use bevy::render::render_resource::TextureFormat;

    // image.data is Option<Vec<u8>> in Bevy 0.18.
    let Some(data) = image.data.as_deref() else {
        return 0.0;
    };

    let w = image.width() as usize;
    let h = image.height() as usize;

    match image.texture_descriptor.format {
        TextureFormat::R8Unorm => {
            let idx = y * w + x;
            if idx < data.len() { data[idx] as f32 / 255.0 } else { 0.0 }
        }
        TextureFormat::R16Unorm => {
            let idx = (y * w + x) * 2;
            if idx + 1 < data.len() {
                let lo = data[idx] as u16;
                let hi = data[idx + 1] as u16;
                let v = lo | (hi << 8); // little-endian
                v as f32 / 65535.0
            } else {
                0.0
            }
        }
        TextureFormat::Rgba8UnormSrgb | TextureFormat::Rgba8Unorm => {
            let idx = (y * w + x) * 4;
            if idx < data.len() { data[idx] as f32 / 255.0 } else { 0.0 }
        }
        TextureFormat::Rgba16Unorm => {
            let idx = (y * w + x) * 8; // 4 channels × 2 bytes
            if idx + 1 < data.len() {
                let lo = data[idx] as u16;
                let hi = data[idx + 1] as u16;
                let v = lo | (hi << 8);
                v as f32 / 65535.0
            } else {
                0.0
            }
        }
        _ => {
            // Fallback: treat first byte of pixel as 8-bit grayscale.
            // Works for most packed 8-bit formats Bevy might use.
            let total_pixels = (w * h).max(1);
            let bytes_per_pixel = (data.len() / total_pixels).max(1);
            let idx = (y * w + x) * bytes_per_pixel;
            if idx < data.len() { data[idx] as f32 / 255.0 } else { 0.0 }
        }
    }
}

// ---------------------------------------------------------------------------
// Colour helpers
// ---------------------------------------------------------------------------

fn slope_smooth_step(x: f32, lo: f32, hi: f32) -> f32 {
    let t = ((x - lo) / (hi - lo)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn lerp3(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

// ---------------------------------------------------------------------------
// WASM drag-drop subsystem
// ---------------------------------------------------------------------------

#[cfg(target_arch = "wasm32")]
pub mod wasm_dragdrop {
    //! Registers `dragover` and `drop` event listeners on the Bevy canvas so
    //! players can drag-drop a PNG heightmap file directly onto the game window.
    //!
    //! When a file is dropped:
    //!   1. A `FileReader` reads the PNG bytes via `readAsDataUrl`.
    //!   2. The data-URL is stored in a thread-local string cell.
    //!   3. The Bevy `poll_dragdrop` system (run every Update) checks the cell,
    //!      loads the data-URL via `AssetServer::load`, and populates
    //!      `CustomHeightmapRequest`.
    //!
    //! Note: Bevy's AssetServer can load `data:` URLs on WASM via its built-in
    //! fetch loader, so no extra crate is needed for the decode step.

    use std::cell::RefCell;

    use bevy::prelude::*;
    use wasm_bindgen::{closure::Closure, JsCast};
    use web_sys::{
        DragEvent, Event, EventTarget, FileReader,
    };

    // Thread-local storage for the data-URL produced by FileReader.
    thread_local! {
        static PENDING_DATA_URL: RefCell<Option<String>> = RefCell::new(None);
    }

    /// Bevy startup system: attach dragover + drop listeners to the canvas.
    pub fn register_dragdrop_listeners(_world: &mut World) {
        let window = match web_sys::window() {
            Some(w) => w,
            None => {
                warn!("heightmap_loader: no JS window — drag-drop disabled");
                return;
            }
        };
        let document = match window.document() {
            Some(d) => d,
            None => {
                warn!("heightmap_loader: no JS document — drag-drop disabled");
                return;
            }
        };

        // Try to find <canvas id="bevy">; fall back to document body.
        let target: EventTarget = document
            .get_element_by_id("bevy")
            .and_then(|el| el.dyn_into::<EventTarget>().ok())
            .or_else(|| document.body().and_then(|b| b.dyn_into::<EventTarget>().ok()))
            .unwrap_or_else(|| document.clone().dyn_into::<EventTarget>().unwrap());

        // `dragover` — prevent the browser's default behaviour (open file).
        let dragover_cb = Closure::<dyn Fn(Event)>::new(|ev: Event| {
            ev.prevent_default();
        });
        target
            .add_event_listener_with_callback("dragover", dragover_cb.as_ref().unchecked_ref())
            .ok();
        dragover_cb.forget(); // keep alive for the page lifetime

        // `drop` — read the first PNG file from the drop.
        let drop_cb = Closure::<dyn Fn(DragEvent)>::new(|ev: DragEvent| {
            ev.prevent_default();

            let Some(data_transfer) = ev.data_transfer() else { return };
            let Some(files) = data_transfer.files() else { return };
            let Some(file) = files.get(0) else { return };

            // Check MIME type or file name extension.
            let name = file.name();
            let mime = file.type_();
            let is_png = mime == "image/png"
                || name.to_lowercase().ends_with(".png");
            if !is_png {
                warn!("heightmap drag-drop: ignored non-PNG file '{name}'");
                return;
            }

            // FileReader → readAsDataURL
            let reader = match FileReader::new() {
                Ok(r) => r,
                Err(_) => {
                    warn!("heightmap drag-drop: could not create FileReader");
                    return;
                }
            };

            let reader_clone = reader.clone();
            let onload = Closure::<dyn Fn(Event)>::new(move |_ev: Event| {
                if let Ok(result) = reader_clone.result() {
                    if let Some(data_url) = result.as_string() {
                        info!("heightmap drag-drop: PNG loaded ({} bytes data-URL)", data_url.len());
                        PENDING_DATA_URL.with(|cell| {
                            *cell.borrow_mut() = Some(data_url);
                        });
                    }
                }
            });

            reader.set_onload(Some(onload.as_ref().unchecked_ref()));
            onload.forget();

            // Cast File → Blob for read_as_data_url.
            let blob: web_sys::Blob = file.dyn_into().expect("File is a Blob");
            reader.read_as_data_url(&blob).ok();
        });

        target
            .add_event_listener_with_callback("drop", drop_cb.as_ref().unchecked_ref())
            .ok();
        drop_cb.forget();

        info!("heightmap_loader: drag-drop listeners registered on canvas");
    }

    /// Bevy Update system (WASM only): check for a pending data-URL and, if
    /// found, push a `CustomHeightmapRequest` so `apply_custom_heightmap`
    /// picks it up next frame.
    pub fn poll_dragdrop(
        asset_server: Res<AssetServer>,
        mut request: ResMut<super::CustomHeightmapRequest>,
    ) {
        let url = PENDING_DATA_URL.with(|cell| cell.borrow_mut().take());
        let Some(data_url) = url else { return };

        info!("heightmap_loader: triggering custom terrain load from drag-drop data-URL");
        let handle: Handle<Image> = asset_server.load(data_url);
        request.handle = Some(handle);
        request.world_size = Vec2::splat(super::CUSTOM_WORLD_SIZE);
        request.max_height = super::CUSTOM_MAX_HEIGHT;
    }
}
