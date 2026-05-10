// gpx_overlay.rs — Sprint 59
//
// Loads a GPS track from a GPX file and renders it as a glowing line strip
// floating ~0.5 m above the terrain.
//
// Workflow:
//   • Native : pass `--load-gpx path.gpx` on the command line.
//   • WASM   : drag-drop a `.gpx` file onto the canvas.
//              The shared drag-drop dispatcher in `heightmap_loader::wasm_dragdrop`
//              detects the `.gpx` extension and writes to `PENDING_GPX_DATA_URL`
//              declared in this module.  `poll_gpx_dragdrop` (Update, WASM only)
//              reads it and sets `GpxOverlayRequest`.
//
// Projection: equirectangular centred on the first track point.
//   x =  (lon − lon₀) × R × cos(lat₀)
//   z = −(lat − lat₀) × R          (negative: Bevy +Z is south)
//   y =  ele − ele₀ + 0.5          (float 0.5 m above terrain reference)
//
// where R = 6 371 000 m.
//
// The GPX XML is parsed by hand (no external crate) — only `<trkpt lat="…"
// lon="…" ele="…">` attributes are extracted.
//
// Public API:
//   GpxOverlayPlugin
//   GpxOverlayRequest  (Resource)
//   GpxOverlayMarker   (Component)

use bevy::{
    asset::RenderAssetUsages,
    mesh::PrimitiveTopology,
    prelude::*,
};

// ---------------------------------------------------------------------------
// Earth radius constant
// ---------------------------------------------------------------------------

const EARTH_R: f64 = 6_371_000.0;

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct GpxOverlayPlugin;

impl Plugin for GpxOverlayPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GpxOverlayRequest>()
            .add_systems(Startup, parse_cli_gpx)
            .add_systems(Update, apply_gpx_overlay);

        // WASM-only: poll the data-URL cell written by the shared drag-drop
        // dispatcher in heightmap_loader::wasm_dragdrop.
        #[cfg(target_arch = "wasm32")]
        app.add_systems(Update, poll_gpx_dragdrop);
    }
}

// ---------------------------------------------------------------------------
// Resource
// ---------------------------------------------------------------------------

/// When `raw_xml` is `Some`, `apply_gpx_overlay` will parse it and spawn (or
/// replace) the GPX line overlay.
#[derive(Resource, Default)]
pub struct GpxOverlayRequest {
    /// Raw GPX XML string (native path: read from disk; WASM path: decoded from data-URL).
    pub raw_xml: Option<String>,
}

// ---------------------------------------------------------------------------
// Marker component
// ---------------------------------------------------------------------------

/// Marks the line-strip entity that renders the GPX overlay.
/// Despawned before a new overlay is spawned so re-loading replaces it.
#[derive(Component)]
pub struct GpxOverlayMarker;

// ---------------------------------------------------------------------------
// Startup system — CLI `--load-gpx path.gpx` (native only)
// ---------------------------------------------------------------------------

fn parse_cli_gpx(mut request: ResMut<GpxOverlayRequest>) {
    // WASM never has a meaningful argv.
    #[cfg(target_arch = "wasm32")]
    let _ = request;

    #[cfg(not(target_arch = "wasm32"))]
    {
        let mut args = std::env::args().peekable();
        let mut path_arg: Option<String> = None;

        while let Some(a) = args.next() {
            if let Some(rest) = a.strip_prefix("--load-gpx=") {
                path_arg = Some(rest.to_string());
            } else if a == "--load-gpx" {
                path_arg = args.next();
            }
        }

        if let Some(path) = path_arg {
            info!("gpx_overlay: loading GPX from CLI: {path}");
            match std::fs::read_to_string(&path) {
                Ok(xml) => {
                    request.raw_xml = Some(xml);
                }
                Err(e) => {
                    warn!("gpx_overlay: failed to read '{path}': {e}");
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Update system — parse XML + spawn/replace line overlay
// ---------------------------------------------------------------------------

fn apply_gpx_overlay(
    mut commands: Commands,
    mut request: ResMut<GpxOverlayRequest>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    overlay_query: Query<Entity, With<GpxOverlayMarker>>,
) {
    let Some(xml) = request.raw_xml.take() else {
        return;
    };

    // Parse track points from the GPX XML.
    let points = parse_gpx(&xml);
    if points.len() < 2 {
        warn!(
            "gpx_overlay: fewer than 2 track points found (got {}); overlay skipped",
            points.len()
        );
        return;
    }

    // Project into Bevy world space.
    let positions = project_points(&points);

    // Despawn any existing overlay before spawning a new one.
    for entity in overlay_query.iter() {
        commands.entity(entity).despawn();
    }

    // Build a LineStrip mesh.
    let mut mesh = Mesh::new(PrimitiveTopology::LineStrip, RenderAssetUsages::default());
    let pos_vec: Vec<[f32; 3]> = positions.iter().map(|v| [v.x, v.y, v.z]).collect();
    let normals: Vec<[f32; 3]> = vec![[0.0, 1.0, 0.0]; pos_vec.len()];
    let uvs: Vec<[f32; 2]> = (0..pos_vec.len())
        .map(|i| [i as f32 / (pos_vec.len() as f32 - 1.0).max(1.0), 0.0])
        .collect();

    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, pos_vec);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);

    // Emissive yellow-orange material — unlit so it glows regardless of sun angle.
    let material = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.65, 0.0),
        emissive: LinearRgba::new(1.0, 0.55, 0.0, 1.0),
        unlit: true,
        ..default()
    });

    commands.spawn((
        Mesh3d(meshes.add(mesh)),
        MeshMaterial3d(material),
        Transform::default(),
        GpxOverlayMarker,
    ));

    info!(
        "gpx_overlay: overlay spawned with {} track points",
        points.len()
    );
}

// ---------------------------------------------------------------------------
// GPX XML parser (hand-rolled, no dependencies)
// ---------------------------------------------------------------------------

/// A raw track point extracted from the GPX XML.
#[derive(Debug, Clone, Copy)]
struct TrackPt {
    lat_deg: f64,
    lon_deg: f64,
    /// Elevation in metres. Defaults to 0 if the `<ele>` child is absent.
    ele_m: f64,
}

/// Parse all `<trkpt lat="…" lon="…" …>` elements from `xml`.
/// Elevation is extracted from the inner `<ele>` child text if present.
fn parse_gpx(xml: &str) -> Vec<TrackPt> {
    let mut points = Vec::new();

    // Iterate over occurrences of "<trkpt" in the source.
    let mut search = xml;
    while let Some(start) = search.find("<trkpt") {
        // Advance past the "<trkpt" marker.
        let rest = &search[start + 6..];

        // Find the end of the opening tag (either ">" or "/>").
        let tag_end = match rest.find('>') {
            Some(i) => i,
            None => break,
        };
        let tag_attrs = &rest[..tag_end];

        let lat = parse_attr(tag_attrs, "lat");
        let lon = parse_attr(tag_attrs, "lon");

        let (lat_deg, lon_deg) = match (lat, lon) {
            (Some(la), Some(lo)) => (la, lo),
            _ => {
                // Advance past this tag so we don't loop forever.
                search = &search[start + 6..];
                continue;
            }
        };

        // Look for the closing </trkpt> tag to extract the <ele> child.
        let closing_tag = "</trkpt>";
        let ele_m = if let Some(close_offset) = rest.find(closing_tag) {
            let inner = &rest[..close_offset];
            parse_ele_child(inner)
        } else {
            0.0
        };

        points.push(TrackPt { lat_deg, lon_deg, ele_m });

        // Advance search past the start of this tag so we find the next one.
        search = &search[start + 6..];
    }

    points
}

/// Extract a named attribute value (f64) from a tag attribute string like
/// ` lat="47.123" lon="-122.456" ele="312"`.
fn parse_attr(tag_attrs: &str, name: &str) -> Option<f64> {
    // Build the pattern we're looking for: `name="`
    let needle = format!("{name}=\"");
    let pos = tag_attrs.find(needle.as_str())?;
    let after = &tag_attrs[pos + needle.len()..];
    let end = after.find('"')?;
    after[..end].trim().parse::<f64>().ok()
}

/// Extract the text content of the `<ele>` child element inside the
/// `<trkpt>…</trkpt>` inner XML.
fn parse_ele_child(inner: &str) -> f64 {
    let start_tag = "<ele>";
    let end_tag = "</ele>";
    let start = match inner.find(start_tag) {
        Some(i) => i + start_tag.len(),
        None => return 0.0,
    };
    let end = match inner[start..].find(end_tag) {
        Some(i) => i,
        None => return 0.0,
    };
    inner[start..start + end].trim().parse::<f64>().unwrap_or(0.0)
}

// ---------------------------------------------------------------------------
// Equirectangular projection
// ---------------------------------------------------------------------------

/// Project a slice of `TrackPt` into Bevy world-space `Vec3` positions.
///
/// Projection is centred on the first point (lat₀, lon₀, ele₀):
///   x =  (lon − lon₀) × R × cos(lat₀)
///   z = −(lat − lat₀) × R
///   y =   ele − ele₀ + 0.5
fn project_points(points: &[TrackPt]) -> Vec<Vec3> {
    let first = points[0];
    let lat0 = first.lat_deg.to_radians();
    let lon0 = first.lon_deg;
    let ele0 = first.ele_m;

    let cos_lat0 = lat0.cos();

    points
        .iter()
        .map(|p| {
            let dlat = (p.lat_deg - first.lat_deg).to_radians();
            let dlon = (p.lon_deg - lon0).to_radians();
            let x = (dlon * EARTH_R * cos_lat0) as f32;
            let z = -(dlat * EARTH_R) as f32;
            let y = (p.ele_m - ele0 + 0.5) as f32;
            Vec3::new(x, y, z)
        })
        .collect()
}

// ---------------------------------------------------------------------------
// WASM-only: thread-local data-URL cell + poll system
// ---------------------------------------------------------------------------

/// Thread-local cell written by `heightmap_loader::wasm_dragdrop` when the
/// dropped file has a `.gpx` extension.
///
/// Exposed as `pub` so the drag-drop dispatcher can write to it via
/// `crate::gpx_overlay::PENDING_GPX_DATA_URL`.
#[cfg(target_arch = "wasm32")]
pub use wasm_gpx_cell::PENDING_GPX_DATA_URL;

#[cfg(target_arch = "wasm32")]
mod wasm_gpx_cell {
    use std::cell::RefCell;

    thread_local! {
        pub static PENDING_GPX_DATA_URL: RefCell<Option<String>> = RefCell::new(None);
    }
}

/// WASM Update system: if a GPX data-URL has been dropped, decode it from
/// base64 and place the raw XML in `GpxOverlayRequest`.
#[cfg(target_arch = "wasm32")]
fn poll_gpx_dragdrop(mut request: ResMut<GpxOverlayRequest>) {
    let url = PENDING_GPX_DATA_URL.with(|cell| cell.borrow_mut().take());
    let Some(data_url) = url else { return };

    // A data-URL for a GPX/XML file looks like:
    //   data:application/gpx+xml;base64,<base64-data>
    // or
    //   data:text/xml;base64,<base64-data>
    // or in some browsers just:
    //   data:application/octet-stream;base64,<base64-data>
    //
    // We need the raw UTF-8 text.  Extract the base64 payload and decode it.
    if let Some(raw_xml) = decode_data_url_to_string(&data_url) {
        info!(
            "gpx_overlay: triggering GPX overlay from drag-drop ({} chars)",
            raw_xml.len()
        );
        request.raw_xml = Some(raw_xml);
    } else {
        warn!("gpx_overlay: could not decode GPX data-URL");
    }
}

/// Decode a `data:…;base64,<payload>` URL to a UTF-8 string.
/// Returns `None` if the URL is malformed or the payload is not valid UTF-8.
#[cfg(target_arch = "wasm32")]
fn decode_data_url_to_string(data_url: &str) -> Option<String> {
    // Find the comma separating header from payload.
    let comma = data_url.find(',')?;
    let payload = &data_url[comma + 1..];

    // Check for the base64 marker in the header.
    let header = &data_url[..comma];
    if !header.contains("base64") {
        // Plain-text data-URL: everything after the comma is the raw content.
        return Some(
            js_sys::decode_uri_component(payload)
                .ok()
                .and_then(|s| s.as_string())
                .unwrap_or_else(|| payload.to_string()),
        );
    }

    // Decode base64 using the browser's `atob` function via js_sys.
    let decoded = js_sys::eval(&format!("atob({:?})", payload))
        .ok()
        .and_then(|v| v.as_string())?;

    // `atob` returns a Latin-1 string (byte values 0-255 as char code points).
    // Re-encode as UTF-8.
    let bytes: Vec<u8> = decoded.chars().map(|c| c as u8).collect();
    String::from_utf8(bytes).ok()
}
