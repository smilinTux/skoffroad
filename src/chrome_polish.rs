// Chrome polish: pumps up the metallic + reflectivity values on chrome
// accents (mirrors, headlight reflectors, hub caps, lug nuts, valve covers,
// door handles, light bars) for a true mirror-chrome look.
//
// Strategy: scan ALL StandardMaterials each Update (debounced to ~1 Hz) for
// materials whose base_color falls in the chrome sRGB band:
//   R, G, B all > 0.78
//   abs(R-G) < 0.10, abs(R-B) < 0.12, abs(G-B) < 0.12
// Matching materials that have no emissive contribution are upgraded to
//   metallic = 0.95, perceptual_roughness = 0.05, reflectance = 0.95.
// (Sprint 81: reflectance added so bumpers / trim match the new chrome rims.
//  Detection band slightly relaxed to catch the darker chrome-barrel ring.)
// Already-polished asset IDs are stored in a Local<HashSet<...>> and skipped on
// subsequent runs so no frame-by-frame churn occurs.
//
// Public API:
//   ChromePolishPlugin

use bevy::prelude::*;
use std::collections::HashSet;

// ── Plugin ────────────────────────────────────────────────────────────────────

pub struct ChromePolishPlugin;

impl Plugin for ChromePolishPlugin {
    fn build(&self, app: &mut App) {
        // Startup: intentionally empty — chrome materials are spawned lazily by
        // various Update systems (vehicle_detail, wheel_detail, engine_bay,
        // roof_rack, etc.) so they don't exist at Startup yet.
        app.add_systems(Update, polish_chrome_materials);
    }
}

// ── System ────────────────────────────────────────────────────────────────────

/// Iterates all `StandardMaterial` assets once per second and upgrades any
/// chrome-colored, non-emissive material to metallic=0.95 / roughness=0.05 /
/// reflectance=0.95 (Sprint 81).
/// Already-polished IDs are tracked in a `Local<HashSet>` so each material is
/// touched at most once over the lifetime of the app.
fn polish_chrome_materials(
    time: Res<Time>,
    mut assets: ResMut<Assets<StandardMaterial>>,
    mut cooldown: Local<f32>,
    mut polished: Local<HashSet<AssetId<StandardMaterial>>>,
) {
    // ── Debounce: run at most once per second ─────────────────────────────────
    *cooldown -= time.delta_secs();
    if *cooldown > 0.0 {
        return;
    }
    *cooldown = 1.0;

    // ── Scan all materials ────────────────────────────────────────────────────
    let mut n: u32 = 0;

    for (id, mat) in assets.iter_mut() {
        // Skip materials we have already polished.
        if polished.contains(&id) {
            continue;
        }

        // Skip emissive materials (lights, glow accents) — they are not chrome.
        let e = mat.emissive;
        if e.red > 0.01 || e.green > 0.01 || e.blue > 0.01 {
            continue;
        }

        // Extract sRGB components from the base_color.
        let c = mat.base_color.to_srgba();
        let r = c.red;
        let g = c.green;
        let b = c.blue;

        // Chrome color band: all channels bright, tightly balanced, no strong hue.
        // Sprint 81: lower bound 0.78 (was 0.80) and relaxed channel deltas
        // to catch the new rim-barrel accent ring without false positives on
        // mid-gray steel parts (which have R,G,B around 0.28–0.50).
        let is_chrome = r > 0.78
            && g > 0.78
            && b > 0.78
            && (r - g).abs() < 0.10
            && (r - b).abs() < 0.12
            && (g - b).abs() < 0.12;

        if !is_chrome {
            continue;
        }

        // Skip materials already at chrome quality (set directly by vehicle_detail
        // or wheel_rims on Medium+) — mark polished, avoid redundant mutation.
        if mat.metallic >= 0.94 && mat.perceptual_roughness <= 0.06 {
            polished.insert(id);
            continue;
        }

        // Upgrade to true chrome.
        mat.metallic             = 0.95;
        mat.perceptual_roughness = 0.05;
        mat.reflectance          = 0.95;

        polished.insert(id);
        n += 1;
    }

    if n > 0 {
        info!("chrome polish: {n} materials upgraded");
    }
}
