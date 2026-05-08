// Reflective water: upgrade water materials with high metallic + low roughness,
// plus a subtle per-frame wave shimmer on the blue channel.
//
// Strategy: scan ALL StandardMaterials each Update (debounced to ~1 Hz) for
// materials whose base_color falls in the water sRGB band:
//   R ∈ 0.10..0.30, G ∈ 0.30..0.60, B ∈ 0.50..0.80, A ∈ 0.60..1.0
//   Blue dominant: B > 0.5, R < 0.5, G ∈ (R, B)
// Matching materials are upgraded to metallic=0.85, perceptual_roughness=0.15.
// Already-polished asset IDs are kept in a Local<HashSet<…>> so the upgrade
// write only happens once per material.
//
// animate_water_normal runs every frame and applies a subtle sin-based alpha
// modulation (±0.05) to all polished materials to simulate a gentle wave.
//
// Distinct from water.rs (buoyancy physics) and mud_puddles.rs (dark mud colors).
//
// Public API:
//   WaterReflectivePlugin

use bevy::prelude::*;
use std::collections::HashSet;

// ── Plugin ────────────────────────────────────────────────────────────────────

pub struct WaterReflectivePlugin;

impl Plugin for WaterReflectivePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (polish_water_materials, animate_water_normal),
        );
    }
}

// ── Color-band helpers ────────────────────────────────────────────────────────

/// Returns true when `c` falls within the water color band.
///
/// Band definition (sRGB, linear Bevy channels stored as sRGBA):
///   R ∈ [0.10, 0.30)
///   G ∈ [0.30, 0.60)
///   B ∈ [0.50, 0.80)
///   A ∈ [0.60, 1.00]
/// Additional hue constraint: B > 0.5, R < 0.5, G is between R and B
/// (i.e. R ≤ G ≤ B), ensuring blue dominance.
#[inline]
fn is_water_color(c: Srgba) -> bool {
    let r = c.red;
    let g = c.green;
    let b = c.blue;
    let a = c.alpha;

    r >= 0.10 && r < 0.30
        && g >= 0.30 && g < 0.60
        && b >= 0.50 && b < 0.80
        && a >= 0.60 && a <= 1.0
        && b > 0.5
        && r < 0.5
        && g >= r && g <= b
}

// ── Systems ───────────────────────────────────────────────────────────────────

/// Scans all `StandardMaterial` assets once per second and upgrades any
/// water-colored material to metallic=0.85 / roughness=0.15.
/// Each material is upgraded at most once (tracked via a `Local<HashSet>`).
fn polish_water_materials(
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
        if polished.contains(&id) {
            continue;
        }

        let c = mat.base_color.to_srgba();
        if !is_water_color(c) {
            continue;
        }

        // Upgrade to reflective water appearance.
        mat.metallic = 0.85;
        mat.perceptual_roughness = 0.15;

        polished.insert(id);
        n += 1;
    }

    if n > 0 {
        info!("water_reflective: {n} water material(s) polished");
    }
}

/// Every frame, applies a very subtle alpha modulation to all polished water
/// materials — sin(t * 0.5) * 0.05 — to simulate a gentle wave shimmer.
/// The base_color alpha is clamped to [0.60, 1.0] so it never goes opaque
/// or invisible regardless of frame timing.
fn animate_water_normal(
    time: Res<Time>,
    mut assets: ResMut<Assets<StandardMaterial>>,
    polished: Local<HashSet<AssetId<StandardMaterial>>>,
) {
    if polished.is_empty() {
        return;
    }

    let t = time.elapsed_secs();
    let delta_alpha = (t * 0.5).sin() * 0.05;

    for id in polished.iter() {
        let Some(mat) = assets.get_mut(*id) else {
            continue;
        };

        let c = mat.base_color.to_srgba();
        let new_alpha = (c.alpha + delta_alpha).clamp(0.60, 1.0);

        mat.base_color = Color::srgba(c.red, c.green, c.blue, new_alpha);
    }
}
