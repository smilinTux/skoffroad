// Map registry: defines the catalog of selectable maps and tracks which is
// currently active. Biome plugins (biome_desert, biome_canyon) read
// ActiveMap to decide whether to install their decorative overlays.
//
// Public API:
//   MapsPlugin
//   ActiveMap (resource)
//   MapKind enum
//   MapInfo struct
//   pub fn map_catalog() -> &'static [MapInfo]
//   pub fn display_name(MapKind) -> &'static str
//   pub fn description(MapKind) -> &'static str

use bevy::prelude::*;

// ---- Catalog -----------------------------------------------------------------

/// Static catalog of every selectable map.
pub fn map_catalog() -> &'static [MapInfo] {
    &[
        MapInfo {
            kind: MapKind::Default,
            name: "VALLEY",
            description: "Rolling hills with trees and lakes.",
        },
        MapInfo {
            kind: MapKind::Desert,
            name: "DUNES",
            description: "Cactus-dotted desert under amber sun.",
        },
        MapInfo {
            kind: MapKind::Canyon,
            name: "CANYON",
            description: "Tall red rock pillars and dust haze.",
        },
    ]
}

// ---- Helpers -----------------------------------------------------------------

/// Returns the display name for the given map kind, looked up from the catalog.
pub fn display_name(kind: MapKind) -> &'static str {
    map_catalog()
        .iter()
        .find(|m| m.kind == kind)
        .map(|m| m.name)
        .unwrap_or("UNKNOWN")
}

/// Returns the description for the given map kind, looked up from the catalog.
pub fn description(kind: MapKind) -> &'static str {
    map_catalog()
        .iter()
        .find(|m| m.kind == kind)
        .map(|m| m.description)
        .unwrap_or("")
}

// ---- Plugin ------------------------------------------------------------------

pub struct MapsPlugin;

impl Plugin for MapsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ActiveMap>()
           .add_systems(Update, log_active_map_on_change);
    }
}

/// Logs the active map name once whenever `ActiveMap` changes.
/// Biome plugins can piggyback on this signal to confirm activation.
fn log_active_map_on_change(active: Res<ActiveMap>) {
    if active.is_changed() {
        info!("[maps] active map → {}", display_name(active.0));
    }
}

// ---- Resources & types -------------------------------------------------------

#[derive(Resource, Clone, Copy, PartialEq, Eq, Debug)]
pub struct ActiveMap(pub MapKind);

impl Default for ActiveMap {
    fn default() -> Self {
        Self(MapKind::Default)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum MapKind {
    Default,
    Desert,
    Canyon,
}

#[derive(Clone, Debug)]
pub struct MapInfo {
    pub kind: MapKind,
    pub name: &'static str,
    pub description: &'static str,
}
