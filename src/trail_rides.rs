// trail_rides.rs — Sprint 63
//
// TrailRidesPlugin: loads assets/trails/manifest.json at startup, exposes the
// parsed trail list as a resource, and handles trail activation from the
// Mission Select menu.
//
// When a trail is selected (TrailRideRequest is set):
//   1. Teleport the chassis to spawn_x, spawn_z (via platform_storage-safe
//      approach: we set a pending request and act on it next Update tick).
//   2. If the trail has a non-null `glb`, fire a CustomGlbRequest to swap the
//      terrain.  If `glb` is null, keep the procedural terrain (GPX overlays
//      the procedural map — useful for the demo trail).
//   3. If the trail has a non-null `gpx`, read the GPX file from disk (native)
//      or from the bundled asset path (WASM via fetch) and set GpxOverlayRequest.
//
// Manifest format (assets/trails/manifest.json):
//   {
//     "trails": [
//       {
//         "id": "demo_loop",
//         "title": "Demo Loop",
//         "description": "...",
//         "spawn_x": 0.0, "spawn_z": 0.0,
//         "glb": null,                    // null = keep procedural terrain
//         "gpx": "trails/demo_loop.gpx", // relative to assets/, null = no GPX
//         "length_km": 0.5,
//         "difficulty": "easy"
//       }
//     ]
//   }
//
// Public API:
//   TrailRidesPlugin
//   TrailManifest      (Resource)
//   TrailEntry         (Data)
//   TrailRideRequest   (Resource)

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::custom_map_loader::CustomGlbRequest;
use crate::gpx_overlay::GpxOverlayRequest;
use crate::terrain::terrain_height_at;
use crate::vehicle::{Chassis, VehicleRoot};

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

/// One trail entry parsed from the manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrailEntry {
    /// Machine-readable identifier used as a persistence key.
    pub id: String,
    /// Human-readable display name.
    pub title: String,
    /// Short description shown in the Mission Select card.
    pub description: String,
    /// World-space spawn X for the chassis.
    pub spawn_x: f32,
    /// World-space spawn Z for the chassis.
    pub spawn_z: f32,
    /// Optional asset path for the terrain GLB (relative to assets/).
    /// `null` means "use the procedural terrain already in the scene".
    pub glb: Option<String>,
    /// Optional asset path for the GPX track file (relative to assets/).
    /// `null` means "no route overlay".
    pub gpx: Option<String>,
    /// Approximate trail length in kilometres (informational).
    pub length_km: f32,
    /// Difficulty label ("easy", "moderate", "hard", etc.).
    pub difficulty: String,
}

/// The full parsed manifest.  Inserted as a resource at startup (possibly empty
/// if the manifest could not be loaded — error is logged; game still runs).
#[derive(Resource, Default, Debug)]
pub struct TrailManifest {
    pub trails: Vec<TrailEntry>,
}

/// Set this resource to trigger a trail activation on the next Update tick.
#[derive(Resource, Default)]
pub struct TrailRideRequest {
    /// Index into `TrailManifest::trails`.  None = no pending request.
    pub trail_idx: Option<usize>,
}

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct TrailRidesPlugin;

impl Plugin for TrailRidesPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(TrailManifest::default())
            .insert_resource(TrailRideRequest::default())
            .add_systems(Startup, load_trail_manifest)
            .add_systems(Update, apply_trail_ride_request.run_if(resource_exists::<VehicleRoot>));
    }
}

// ---------------------------------------------------------------------------
// Startup: load the manifest
// ---------------------------------------------------------------------------

fn load_trail_manifest(mut manifest: ResMut<TrailManifest>) {
    // Native: read directly from the assets directory on disk.
    // WASM: Trunk copies the assets/ dir next to the .wasm; the file is
    //       available relative to the working directory.  For WASM we skip the
    //       disk read here and instead expect the Mission Select UI to provide
    //       the embedded default manifest (or a fetch-based loader in a future
    //       sprint).  For now we embed the manifest at compile time on WASM.

    #[cfg(not(target_arch = "wasm32"))]
    {
        // Look for the manifest relative to the process cwd (dev mode) or next
        // to the executable (release). Try both locations.
        let candidates = [
            std::path::PathBuf::from("assets/trails/manifest.json"),
            {
                let mut p = std::env::current_exe()
                    .unwrap_or_default()
                    .parent()
                    .unwrap_or_else(|| std::path::Path::new("."))
                    .to_path_buf();
                p.push("assets/trails/manifest.json");
                p
            },
        ];

        for path in &candidates {
            if let Ok(text) = std::fs::read_to_string(path) {
                match parse_manifest(&text) {
                    Some(m) => {
                        info!(
                            "trail_rides: loaded {} trail(s) from {}",
                            m.trails.len(),
                            path.display()
                        );
                        *manifest = m;
                        return;
                    }
                    None => {
                        warn!("trail_rides: failed to parse manifest at {}", path.display());
                    }
                }
            }
        }

        warn!("trail_rides: manifest not found — using empty trail list");
    }

    // WASM: embed the shipped manifest at compile time so the menu always has
    // at least one entry without a network fetch.
    #[cfg(target_arch = "wasm32")]
    {
        const EMBEDDED: &str = include_str!("../assets/trails/manifest.json");
        match parse_manifest(EMBEDDED) {
            Some(m) => {
                info!("trail_rides: loaded {} trail(s) from embedded manifest", m.trails.len());
                *manifest = m;
            }
            None => {
                warn!("trail_rides: could not parse embedded manifest");
            }
        }
    }
}

fn parse_manifest(text: &str) -> Option<TrailManifest> {
    #[derive(Deserialize)]
    struct Raw {
        trails: Vec<TrailEntry>,
    }
    serde_json::from_str::<Raw>(text)
        .map(|r| TrailManifest { trails: r.trails })
        .ok()
}

// ---------------------------------------------------------------------------
// Update: apply pending trail ride request
// ---------------------------------------------------------------------------

fn apply_trail_ride_request(
    mut request:      ResMut<TrailRideRequest>,
    manifest:         Res<TrailManifest>,
    vehicle:          Res<VehicleRoot>,
    mut chassis_q:    Query<(&mut Transform, &mut avian3d::prelude::LinearVelocity,
                             &mut avian3d::prelude::AngularVelocity), With<Chassis>>,
    mut glb_request:  ResMut<CustomGlbRequest>,
    mut gpx_request:  ResMut<GpxOverlayRequest>,
) {
    let Some(idx) = request.trail_idx.take() else { return };

    let Some(trail) = manifest.trails.get(idx) else {
        warn!("trail_rides: requested trail index {} out of range", idx);
        return;
    };

    info!("trail_rides: activating trail '{}' (id={})", trail.title, trail.id);

    // --- Teleport chassis ---------------------------------------------------
    if let Ok((mut tf, mut linvel, mut angvel)) = chassis_q.get_mut(vehicle.chassis) {
        let spawn_y = terrain_height_at(trail.spawn_x, trail.spawn_z) + 1.5;
        tf.translation = Vec3::new(trail.spawn_x, spawn_y, trail.spawn_z);
        tf.rotation = Quat::IDENTITY;
        linvel.0 = Vec3::ZERO;
        angvel.0 = Vec3::ZERO;
        info!(
            "trail_rides: teleported chassis to ({:.1}, {:.1}, {:.1})",
            trail.spawn_x, spawn_y, trail.spawn_z
        );
    }

    // --- Optional terrain GLB swap -----------------------------------------
    if let Some(ref glb_path) = trail.glb {
        info!("trail_rides: requesting GLB terrain swap: {glb_path}");
        glb_request.path_or_url = Some(glb_path.clone());
        glb_request.scale = 1.0;
    }
    // If glb is null we leave the procedural terrain in place.

    // --- Optional GPX overlay ----------------------------------------------
    if let Some(ref gpx_path) = trail.gpx {
        load_gpx_for_trail(gpx_path, &mut gpx_request);
    }
}

/// Load a GPX file and push it into GpxOverlayRequest.
/// Native: reads from disk; WASM: uses the `include_str!`-embedded GPX for
/// the known built-in trail, or logs a warning for custom trails.
fn load_gpx_for_trail(gpx_path: &str, request: &mut GpxOverlayRequest) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        // Try cwd-relative path first, then exe-sibling.
        let candidates = [
            std::path::PathBuf::from("assets").join(gpx_path),
            {
                let mut p = std::env::current_exe()
                    .unwrap_or_default()
                    .parent()
                    .unwrap_or_else(|| std::path::Path::new("."))
                    .to_path_buf();
                p.push("assets");
                p.push(gpx_path);
                p
            },
        ];

        for path in &candidates {
            if let Ok(xml) = std::fs::read_to_string(path) {
                info!("trail_rides: loaded GPX from {}", path.display());
                request.raw_xml = Some(xml);
                return;
            }
        }
        warn!("trail_rides: GPX not found at '{}'", gpx_path);
    }

    #[cfg(target_arch = "wasm32")]
    {
        // For the demo trail we embed at compile time.
        if gpx_path == "trails/demo_loop.gpx" {
            const DEMO_GPX: &str = include_str!("../assets/trails/demo_loop.gpx");
            info!("trail_rides: loaded demo GPX from embedded source");
            request.raw_xml = Some(DEMO_GPX.to_string());
        } else {
            warn!(
                "trail_rides: WASM cannot dynamically load GPX '{}' — \
                 embed it at compile time or use the drag-drop workflow",
                gpx_path
            );
        }
    }
}
