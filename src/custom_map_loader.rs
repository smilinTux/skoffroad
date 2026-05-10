// custom_map_loader.rs — Sprint 58
//
// Lets players load a photogrammetry scan (Polycam / Luma AI) exported as
// GLB/glTF as a custom drivable terrain.
//
// Workflow:
//   • Native : pass `--load-glb path.glb` (and optionally `--glb-scale 2.5`)
//   • WASM   : drag-drop a `.glb` or `.gltf` file onto the canvas.
//              The shared drag-drop dispatcher in `heightmap_loader::wasm_dragdrop`
//              detects the extension and writes to `PENDING_GLB_DATA_URL` instead
//              of the PNG cell.  `poll_glb_dragdrop` (Update, WASM only) reads it
//              here and sets `CustomGlbRequest`.
//
// When `CustomGlbRequest` is set:
//   1. Wait until `Assets<Scene>` reports the handle is loaded.
//   2. Despawn `ProceduralTerrainMarker` and any existing `CustomTerrainMarker`
//      or `CustomGlbTerrainMarker`.
//   3. Spawn the GLB scene at the requested scale.
//   4. Walk every `Mesh3d` child in the scene (via `SceneRoot` children query)
//      and add `ColliderConstructor::TrimeshFromMesh` so Avian builds collision
//      geometry from the imported mesh(es).
//
// NOTE: Bevy's SceneSpawner / SceneRoot doesn't give us mesh handles
// synchronously at spawn time; instead we mark the root with
// `CustomGlbTerrainMarker` and a one-shot `attach_glb_colliders` system
// walks the entity tree the frame *after* the scene assets are resolved,
// applying `RigidBody::Static + ColliderConstructor::TrimeshFromMesh` to
// every entity that has a `Mesh3d` component.
//
// Public API:
//   CustomMapLoaderPlugin
//   CustomGlbRequest   (Resource)
//   CustomGlbTerrainMarker (Component)

use bevy::prelude::*;
use avian3d::prelude::*;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Default uniform scale applied to the loaded GLB scene (1.0 = no rescale).
pub const DEFAULT_GLB_SCALE: f32 = 1.0;

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct CustomMapLoaderPlugin;

impl Plugin for CustomMapLoaderPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CustomGlbRequest>()
            .add_systems(Startup, parse_cli_glb)
            .add_systems(Update, (apply_custom_glb, attach_glb_colliders));

        // WASM-only: poll the shared data-URL cell written by the shared
        // drag-drop dispatcher in heightmap_loader::wasm_dragdrop.
        #[cfg(target_arch = "wasm32")]
        app.add_systems(Update, poll_glb_dragdrop);
    }
}

// ---------------------------------------------------------------------------
// Resources
// ---------------------------------------------------------------------------

/// When set (non-None), signals that the `apply_custom_glb` system should
/// load the GLB from `path_or_url` and swap the active terrain.
#[derive(Resource, Default)]
pub struct CustomGlbRequest {
    /// Asset path (native) or `data:` URL (WASM) for the GLB/glTF file.
    pub path_or_url: Option<String>,
    /// Uniform scale to apply to the spawned scene.
    pub scale: f32,
}

// ---------------------------------------------------------------------------
// Marker components
// ---------------------------------------------------------------------------

/// Marks the root entity of a player-loaded GLB terrain scene.
/// Despawned (recursively) when a new map is loaded.
#[derive(Component)]
pub struct CustomGlbTerrainMarker;

/// Applied to every mesh entity inside the GLB scene tree once the scene has
/// finished spawning.  Presence of this component tells `attach_glb_colliders`
/// that the entity has already been processed.
#[derive(Component)]
struct GlbColliderAttached;

// ---------------------------------------------------------------------------
// Startup system — CLI `--load-glb path.glb` (native only)
// ---------------------------------------------------------------------------

fn parse_cli_glb(mut request: ResMut<CustomGlbRequest>) {
    // WASM never has a meaningful argv.
    #[cfg(target_arch = "wasm32")]
    let _ = request;

    #[cfg(not(target_arch = "wasm32"))]
    {
        let mut args = std::env::args().peekable();
        let mut path_arg: Option<String> = None;
        let mut scale = DEFAULT_GLB_SCALE;

        while let Some(a) = args.next() {
            if let Some(rest) = a.strip_prefix("--load-glb=") {
                path_arg = Some(rest.to_string());
            } else if a == "--load-glb" {
                path_arg = args.next();
            } else if let Some(rest) = a.strip_prefix("--glb-scale=") {
                if let Ok(v) = rest.parse::<f32>() {
                    scale = v;
                }
            } else if a == "--glb-scale" {
                if let Some(v) = args.next().and_then(|s| s.parse::<f32>().ok()) {
                    scale = v;
                }
            }
        }

        if let Some(path) = path_arg {
            info!("custom_map_loader: --load-glb CLI: {path} (scale {scale})");
            request.path_or_url = Some(path);
            request.scale = scale;
        }
    }
}

// ---------------------------------------------------------------------------
// Update system — spawn the GLB scene when a request is present
// ---------------------------------------------------------------------------

fn apply_custom_glb(
    mut commands: Commands,
    mut request: ResMut<CustomGlbRequest>,
    asset_server: Res<AssetServer>,
    procedural_query: Query<Entity, With<crate::terrain::ProceduralTerrainMarker>>,
    custom_hm_query: Query<Entity, With<crate::heightmap_loader::CustomTerrainMarker>>,
    custom_glb_query: Query<Entity, With<CustomGlbTerrainMarker>>,
) {
    // Only act when a request has been placed.
    let Some(path_or_url) = request.path_or_url.clone() else {
        return;
    };

    // Clear immediately so we don't re-trigger every frame.
    request.path_or_url = None;
    let scale = if request.scale > 0.0 { request.scale } else { DEFAULT_GLB_SCALE };

    // Despawn procedural terrain.
    for entity in procedural_query.iter() {
        commands.entity(entity).despawn();
    }
    // Despawn any custom heightmap terrain.
    for entity in custom_hm_query.iter() {
        commands.entity(entity).despawn();
    }
    // Despawn any previous GLB terrain.
    for entity in custom_glb_query.iter() {
        commands.entity(entity).despawn();
    }

    // Ask AssetServer for the default GLB scene.  Bevy's glTF loader exposes
    // the first scene as "#Scene0".  If the user loaded a single-scene GLB
    // from Polycam/Luma this is always correct.
    let scene_url = if path_or_url.contains('#') {
        path_or_url.clone()
    } else {
        format!("{path_or_url}#Scene0")
    };

    info!("custom_map_loader: loading GLB scene from '{scene_url}' at scale {scale}");

    let scene_handle: Handle<Scene> = asset_server.load(scene_url);

    // Spawn the SceneRoot.  Bevy will asynchronously instantiate all entities
    // from the glTF into children of this root entity.
    commands.spawn((
        SceneRoot(scene_handle),
        Transform::from_scale(Vec3::splat(scale)),
        GlobalTransform::default(),
        Visibility::default(),
        CustomGlbTerrainMarker,
    ));

    info!("custom_map_loader: GLB scene root spawned — waiting for asset load");
}

// ---------------------------------------------------------------------------
// Update system — attach colliders to GLB mesh children once loaded
// ---------------------------------------------------------------------------

/// After `apply_custom_glb` spawns the `SceneRoot`, Bevy's scene spawner
/// populates child entities (including mesh entities) asynchronously over
/// several frames.  This system runs every frame and, for every mesh child
/// inside a `CustomGlbTerrainMarker` hierarchy that does not yet have a
/// collider attached, adds `RigidBody::Static + ColliderConstructor::TrimeshFromMesh`.
fn attach_glb_colliders(
    mut commands: Commands,
    root_query: Query<Entity, With<CustomGlbTerrainMarker>>,
    children_query: Query<&Children>,
    mesh_query: Query<(Entity, Has<GlbColliderAttached>), With<Mesh3d>>,
) {
    for root in root_query.iter() {
        // Walk the entire subtree under the GLB root.
        walk_tree(
            root,
            &children_query,
            &mesh_query,
            &mut commands,
        );
    }
}

/// Recursive DFS over the entity tree rooted at `entity`.
fn walk_tree(
    entity: Entity,
    children_query: &Query<&Children>,
    mesh_query: &Query<(Entity, Has<GlbColliderAttached>), With<Mesh3d>>,
    commands: &mut Commands,
) {
    // If this entity has a Mesh3d and hasn't had colliders attached yet, do it now.
    if let Ok((mesh_entity, already_done)) = mesh_query.get(entity) {
        if !already_done {
            commands.entity(mesh_entity).insert((
                RigidBody::Static,
                ColliderConstructor::TrimeshFromMesh,
                GlbColliderAttached,
            ));
        }
    }

    // Recurse into children.
    if let Ok(children) = children_query.get(entity) {
        for child in children.iter() {
            walk_tree(child, children_query, mesh_query, commands);
        }
    }
}

// ---------------------------------------------------------------------------
// WASM-only: poll the shared GLB data-URL cell
// ---------------------------------------------------------------------------

/// Thread-local cell written by `heightmap_loader::wasm_dragdrop` when the
/// dropped file has a `.glb` or `.gltf` extension.  We declare it here so
/// that the heightmap dispatcher can write to it via `crate::custom_map_loader::PENDING_GLB_DATA_URL`.
#[cfg(target_arch = "wasm32")]
pub use wasm_glb_cell::PENDING_GLB_DATA_URL;

#[cfg(target_arch = "wasm32")]
mod wasm_glb_cell {
    use std::cell::RefCell;

    thread_local! {
        pub static PENDING_GLB_DATA_URL: RefCell<Option<String>> = RefCell::new(None);
    }
}

#[cfg(target_arch = "wasm32")]
fn poll_glb_dragdrop(mut request: ResMut<CustomGlbRequest>) {
    let url = PENDING_GLB_DATA_URL.with(|cell| cell.borrow_mut().take());
    let Some(data_url) = url else { return };

    info!("custom_map_loader: triggering GLB terrain load from drag-drop data-URL");
    request.path_or_url = Some(data_url);
    request.scale = DEFAULT_GLB_SCALE;
}
