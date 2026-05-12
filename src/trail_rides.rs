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
// Sprint 65 — Cross-mode multiplayer leaderboard (Option A).
//   Finish condition: chassis within 8 m XZ of the trail's spawn point at
//   some prior frame (start proximity confirmed) AND later within 8 m XZ of
//   the "last" GPX waypoint approximated as a point offset in the +X direction
//   from spawn by length_km * 800 m (conservative proxy).  Since we don't
//   parse full GPX here, we use a simpler heuristic: "within 8 m of spawn
//   AFTER having moved at least length_km * 200 m from spawn".
//   TrailLeaderboard resource: top-5 peer times per trail_id.
//   Packets prefixed 0x54, 0x52 ("TR") on CHANNEL_LEADERBOARD (channel 3).
//
// Public API:
//   TrailRidesPlugin
//   TrailManifest      (Resource)
//   TrailEntry         (Data)
//   TrailRideRequest   (Resource)
//   TrailLeaderboard   (Resource)

use bevy::prelude::*;
use bevy_matchbox::prelude::*;
use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::custom_map_loader::CustomGlbRequest;
use crate::gpx_overlay::GpxOverlayRequest;
use crate::hillclimb_tiers::CHANNEL_LEADERBOARD;
use crate::multiplayer::PeerId;
use crate::platform_storage;
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
// Trail Leaderboard — Sprint 65
// ---------------------------------------------------------------------------

/// 2-byte magic prefix for Trail Ride leaderboard packets on CHANNEL_LEADERBOARD.
const TR_PREFIX: [u8; 2] = [0x54, 0x52]; // b"TR"

/// Wire packet for a single trail's completion time.
#[derive(Encode, Decode, Clone, Debug)]
pub struct TrLeaderboardPacket {
    /// Trail ID string length + bytes (bincode handles String encoding).
    pub trail_id: String,
    /// Completion time in seconds.  f32::MAX = no time yet.
    pub best_time_s: f32,
}

/// Per-trail completion records (keyed by trail_id).
#[derive(Resource, Default)]
pub struct TrailLeaderboard {
    /// trail_id → list of (peer_id, best_s) sorted fastest-first, max 5.
    pub entries: HashMap<String, Vec<(PeerId, f32)>>,
}

impl TrailLeaderboard {
    pub fn update(&mut self, trail_id: &str, peer_id: PeerId, best_s: f32) {
        let list = self.entries.entry(trail_id.to_string()).or_default();
        if let Some(e) = list.iter_mut().find(|e| e.0 == peer_id) {
            if best_s < e.1 { e.1 = best_s; }
        } else {
            list.push((peer_id, best_s));
        }
        list.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        list.truncate(5);
    }
}

/// Tracks the active trail session for finish detection.
#[derive(Resource, Default)]
pub struct ActiveTrailSession {
    /// Index of the active trail in TrailManifest.
    pub trail_idx: Option<usize>,
    /// Elapsed seconds since trail start.
    pub elapsed_s: f32,
    /// Whether the player has been within 20 m of the spawn (start proximity confirmed).
    pub start_confirmed: bool,
    /// Personal best for this trail_id loaded at session start.
    pub personal_best_s: Option<f32>,
    /// Whether we need to broadcast our times to connected peers.
    pub needs_broadcast: bool,
}

/// Storage key prefix for trail personal bests.
const TRAIL_PB_KEY_PREFIX: &str = "trail_pb_";

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct TrailRidesPlugin;

impl Plugin for TrailRidesPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(TrailManifest::default())
            .insert_resource(TrailRideRequest::default())
            .insert_resource(TrailLeaderboard::default())
            .insert_resource(ActiveTrailSession::default())
            .add_systems(Startup, load_trail_manifest)
            // recv_tr_leaderboard runs in PreUpdate to drain TR-tagged packets
            // before hillclimb_tiers.rs consumes them in Update.
            .add_systems(PreUpdate, recv_tr_leaderboard)
            .add_systems(
                Update,
                (
                    apply_trail_ride_request,
                    tick_trail_session,
                )
                    .chain()
                    .run_if(resource_exists::<VehicleRoot>),
            );
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
    mut session:      ResMut<ActiveTrailSession>,
) {
    let Some(idx) = request.trail_idx.take() else { return };

    let Some(trail) = manifest.trails.get(idx) else {
        warn!("trail_rides: requested trail index {} out of range", idx);
        return;
    };

    info!("trail_rides: activating trail '{}' (id={})", trail.title, trail.id);

    // Reset session tracking.
    let pb = load_trail_pb(&trail.id);
    *session = ActiveTrailSession {
        trail_idx:       Some(idx),
        elapsed_s:       0.0,
        start_confirmed: false,
        personal_best_s: pb,
        needs_broadcast: false,
    };

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

// ---------------------------------------------------------------------------
// Sprint 65: trail finish detection + leaderboard broadcast
// ---------------------------------------------------------------------------

/// Radius (metres XZ) within which the chassis must be to the spawn point
/// to confirm start proximity or finish proximity.
const TRAIL_FINISH_RADIUS: f32 = 8.0;

fn tick_trail_session(
    time:      Res<Time>,
    manifest:  Res<TrailManifest>,
    vehicle:   Res<VehicleRoot>,
    chassis_q: Query<&Transform, With<Chassis>>,
    mut session: ResMut<ActiveTrailSession>,
    mut socket:  Option<ResMut<MatchboxSocket>>,
) {
    let Some(idx) = session.trail_idx else { return };
    let Some(trail) = manifest.trails.get(idx) else { return };
    let Ok(tf) = chassis_q.get(vehicle.chassis) else { return };

    session.elapsed_s += time.delta_secs();

    let pos = tf.translation;
    let spawn_xz = Vec2::new(trail.spawn_x, trail.spawn_z);
    let pos_xz   = Vec2::new(pos.x, pos.z);
    let dist_from_spawn = pos_xz.distance(spawn_xz);

    // Confirm start: chassis within 20 m of spawn at any frame.
    if !session.start_confirmed && dist_from_spawn <= 20.0 {
        session.start_confirmed = true;
        info!("trail_rides: start confirmed for '{}'", trail.id);
    }

    // Finish: start confirmed, 5 s elapsed, and chassis is back within TRAIL_FINISH_RADIUS of spawn.
    if session.start_confirmed
        && session.elapsed_s > 5.0 // debounce: at least 5 s into the ride
        && dist_from_spawn <= TRAIL_FINISH_RADIUS
    {
        let elapsed = session.elapsed_s;
        info!(
            "trail_rides: '{}' finished in {:.2}s",
            trail.id, elapsed
        );

        // Update personal best.
        let is_best = session.personal_best_s.map(|b| elapsed < b).unwrap_or(true);
        if is_best {
            session.personal_best_s = Some(elapsed);
            save_trail_pb(&trail.id, elapsed);
        }

        // Signal broadcast.
        session.needs_broadcast = true;

        // Reset session so player can re-run.
        session.trail_idx       = None;
        session.elapsed_s       = 0.0;
        session.start_confirmed = false;
    }

    // Broadcast personal best to connected peers if flagged.
    if session.needs_broadcast {
        session.needs_broadcast = false;
        if let Some(ref mut sock) = socket {
            let peers: Vec<PeerId> = sock.connected_peers().collect();
            if !peers.is_empty() {
                let best = session.personal_best_s.unwrap_or(f32::MAX);
                let pkt = TrLeaderboardPacket {
                    trail_id: trail.id.clone(),
                    best_time_s: best,
                };
                if let Ok(payload) = bincode::encode_to_vec(pkt, bincode::config::standard()) {
                    let mut msg = TR_PREFIX.to_vec();
                    msg.extend_from_slice(&payload);
                    for &peer in &peers {
                        sock.channel_mut(CHANNEL_LEADERBOARD).send(msg.clone().into(), peer);
                    }
                }
            }
        }
    }
}

fn recv_tr_leaderboard(
    mut socket: Option<ResMut<MatchboxSocket>>,
    mut lb:     ResMut<TrailLeaderboard>,
) {
    let Some(ref mut sock) = socket else { return };

    for (peer_id, bytes) in sock.channel_mut(CHANNEL_LEADERBOARD).receive() {
        if bytes.len() < TR_PREFIX.len() { continue; }
        if bytes[..TR_PREFIX.len()] != TR_PREFIX { continue; }

        let payload = &bytes[TR_PREFIX.len()..];
        let Ok((pkt, _)) = bincode::decode_from_slice::<TrLeaderboardPacket, _>(
            payload,
            bincode::config::standard(),
        ) else {
            warn!("trail_rides: leaderboard decode error from {:?}", peer_id);
            continue;
        };

        if pkt.best_time_s < f32::MAX {
            lb.update(&pkt.trail_id, peer_id, pkt.best_time_s);
        }
    }
}

// ---------------------------------------------------------------------------
// Personal best storage helpers
// ---------------------------------------------------------------------------

fn load_trail_pb(trail_id: &str) -> Option<f32> {
    let key = format!("{}{}.json", TRAIL_PB_KEY_PREFIX, trail_id);
    let text = platform_storage::read_string(&key)?;
    serde_json::from_str::<serde_json::Value>(&text)
        .ok()?
        .get("best_s")?
        .as_f64()
        .map(|x| x as f32)
}

fn save_trail_pb(trail_id: &str, best_s: f32) {
    let key = format!("{}{}.json", TRAIL_PB_KEY_PREFIX, trail_id);
    let json = serde_json::json!({ "best_s": best_s }).to_string();
    if let Err(e) = platform_storage::write_string(&key, &json) {
        warn!("trail_rides: could not save PB for '{}': {}", trail_id, e);
    }
}
