// Brand pack loader.
//
// Loads a JSON-described "brand pack" at startup and exposes it as the
// ActiveBrandPack resource. A brand pack defines: splash CTA, an optional
// custom livery entry, and the set of sponsor billboards to scatter.
//
// On native, packs are read from $CARGO_MANIFEST_DIR/assets/brand_packs/<id>.json.
// On WASM, packs ship inside the asset bundle (Trunk copy-dir = "assets").
//
// The default pack id is "_house" (ships in-tree as assets/brand_packs/_house.json).
// To switch packs at runtime, mutate ActiveBrandPack.pack_id and trigger a reload.
//
// Public API:
//   BrandPackPlugin
//   ActiveBrandPack          (resource — current pack id + parsed data)
//   BrandPack, Billboard, Livery, SplashScreen, ScatterCfg  (data types)

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct BrandPackPlugin;

impl Plugin for BrandPackPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ActiveBrandPack::default())
            .add_systems(Startup, load_default_pack);
    }
}

// ---------------------------------------------------------------------------
// Public data
// ---------------------------------------------------------------------------

#[derive(Resource, Default, Clone)]
pub struct ActiveBrandPack {
    pub pack_id: String,
    pub pack: Option<BrandPack>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BrandPack {
    pub id: String,
    pub display_name: String,
    pub version: u32,

    #[serde(default)]
    pub splash: Option<SplashScreen>,

    #[serde(default)]
    pub livery: Option<BrandLivery>,

    #[serde(default)]
    pub billboards: Vec<BrandBillboard>,

    #[serde(default)]
    pub scatter: ScatterCfg,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SplashScreen {
    #[serde(default)]
    pub logo_texture: Option<String>,
    #[serde(default)]
    pub tagline: Option<String>,
    pub cta_url: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BrandLivery {
    pub id: String,
    pub name: String,
    pub base_color: [f32; 3],
    #[serde(default)]
    pub decal_texture: Option<String>,
    #[serde(default)]
    pub unlock_requires_video: bool,
    #[serde(default)]
    pub unlock_requires_premium_pass: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BrandBillboard {
    #[serde(default)]
    pub texture: Option<String>,
    #[serde(default = "default_fallback_color")]
    pub fallback_color: [f32; 3],
    #[serde(default = "default_weight")]
    pub weight: f32,
    #[serde(default)]
    pub tagline: Option<String>,
    pub click_url: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ScatterCfg {
    #[serde(default = "default_density")]
    pub density: f32,
    #[serde(default = "default_min_distance")]
    pub min_distance_m: f32,
    #[serde(default = "default_edge_only")]
    pub edge_only: bool,
    #[serde(default = "default_slope_band")]
    pub slope_band: [f32; 2],
}

impl Default for ScatterCfg {
    fn default() -> Self {
        Self {
            density: default_density(),
            min_distance_m: default_min_distance(),
            edge_only: default_edge_only(),
            slope_band: default_slope_band(),
        }
    }
}

fn default_fallback_color() -> [f32; 3] { [0.85, 0.30, 0.15] }
fn default_weight() -> f32 { 1.0 }
fn default_density() -> f32 { 0.08 }
fn default_min_distance() -> f32 { 16.0 }
fn default_edge_only() -> bool { true }
fn default_slope_band() -> [f32; 2] { [0.05, 0.22] }

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

pub const DEFAULT_PACK_ID: &str = "_house";

// ---------------------------------------------------------------------------
// Loading
// ---------------------------------------------------------------------------

fn load_default_pack(mut active: ResMut<ActiveBrandPack>) {
    active.pack_id = DEFAULT_PACK_ID.to_string();
    match load_pack_json(DEFAULT_PACK_ID) {
        Some(json) => match serde_json::from_str::<BrandPack>(&json) {
            Ok(pack) => {
                info!("brand_pack: loaded \"{}\" (v{}, {} billboards)",
                    pack.display_name, pack.version, pack.billboards.len());
                active.pack = Some(pack);
            }
            Err(e) => warn!("brand_pack: failed to parse {}.json: {}", DEFAULT_PACK_ID, e),
        },
        None => warn!("brand_pack: {}.json not found — sponsor scatter will be empty",
            DEFAULT_PACK_ID),
    }
}

/// Native: read from filesystem. WASM: fetch from asset URL.
/// Returns None on any failure.
fn load_pack_json(pack_id: &str) -> Option<String> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("assets/brand_packs")
            .join(format!("{}.json", pack_id));
        std::fs::read_to_string(path).ok()
    }
    #[cfg(target_arch = "wasm32")]
    {
        // On WASM, we currently embed the default house pack at compile time
        // via include_str! to avoid an async fetch in a Startup system. Once
        // we need runtime swap, route this through AssetServer instead.
        if pack_id == DEFAULT_PACK_ID {
            Some(include_str!("../assets/brand_packs/_house.json").to_string())
        } else {
            None
        }
    }
}
