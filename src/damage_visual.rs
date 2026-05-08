// Damage visual: chassis material darkens proportional to accumulated HardImpact
// events. Caps at 50% darker (level 0..1). Resets on Shift+W (same key as
// vehicle_dirt wash).
//
// Interaction with vehicle_dirt: both modules mutate chassis base_color. Damage
// starts from the cached original color and lerps toward dark gray; dirt starts
// from the same original and lerps toward dirt brown. Because both apply from
// the original independently, the two effects do NOT fight: the last writer each
// frame wins, but neither "sees" the other's modification as its own baseline.
// The visual ordering (dirt then damage, or vice-versa) depends on system
// schedule order, which is left as-is (both run in Update, unordered). The
// combined result — slightly darker AND slightly browner — is visually
// acceptable for the current sprint.
//
// Public API:
//   DamageVisualPlugin
//   DamageVisualState (resource)

use bevy::prelude::*;

use crate::events::{EventLog, GameEvent};
use crate::vehicle::{Chassis, VehicleRoot};

// ---- Plugin -----------------------------------------------------------------

pub struct DamageVisualPlugin;

impl Plugin for DamageVisualPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DamageVisualState>()
           .init_resource::<DamageOriginalColor>()
           .add_systems(Startup, cache_original_chassis_color)
           .add_systems(Update, (
               try_cache_chassis_color,
               accumulate_damage,
               apply_damage_to_material,
               wash_with_shift_w,
           ));
    }
}

// ---- Public resources -------------------------------------------------------

#[derive(Resource, Default, Clone, Copy)]
pub struct DamageVisualState {
    pub level: f32, // 0..1
}

// ---- Internal resources -----------------------------------------------------

/// Caches the original (un-dirtied, un-damaged) chassis base_color and the
/// material handle so we can apply damage by lerping from the true original.
#[derive(Resource, Default, Clone)]
struct DamageOriginalColor {
    color: Option<Color>,
    handle: Option<Handle<StandardMaterial>>,
}

// ---- Constants --------------------------------------------------------------

/// How much each HardImpact increments the damage level.
const DAMAGE_PER_IMPACT: f32 = 0.15;

/// At level=1 the chassis is darkened to this fraction of its original
/// brightness (0.5 = 50% darker).
const MAX_DARKNESS_FACTOR: f32 = 0.5;

// ---- Startup / caching systems ----------------------------------------------

/// Startup system: try to cache chassis material handle + original color.
/// If VehicleRoot isn't available yet, `try_cache_chassis_color` retries.
fn cache_original_chassis_color(
    vehicle: Option<Res<VehicleRoot>>,
    mat_q: Query<&MeshMaterial3d<StandardMaterial>, With<Chassis>>,
    materials: Res<Assets<StandardMaterial>>,
    mut cache: ResMut<DamageOriginalColor>,
) {
    do_cache_color(&vehicle, &mat_q, &materials, &mut cache);
}

/// Update-phase fallback: runs every frame until the cache is populated.
fn try_cache_chassis_color(
    mut done: Local<bool>,
    vehicle: Option<Res<VehicleRoot>>,
    mat_q: Query<&MeshMaterial3d<StandardMaterial>, With<Chassis>>,
    materials: Res<Assets<StandardMaterial>>,
    mut cache: ResMut<DamageOriginalColor>,
) {
    if *done || cache.color.is_some() {
        *done = true;
        return;
    }
    do_cache_color(&vehicle, &mat_q, &materials, &mut cache);
    if cache.color.is_some() {
        *done = true;
    }
}

/// Shared implementation used by both caching systems.
fn do_cache_color(
    vehicle: &Option<Res<VehicleRoot>>,
    mat_q: &Query<&MeshMaterial3d<StandardMaterial>, With<Chassis>>,
    materials: &Res<Assets<StandardMaterial>>,
    cache: &mut ResMut<DamageOriginalColor>,
) {
    let Some(vehicle) = vehicle else { return };
    let Ok(mat_handle) = mat_q.get(vehicle.chassis) else { return };
    let Some(mat) = materials.get(mat_handle.id()) else { return };
    cache.color = Some(mat.base_color);
    cache.handle = Some(mat_handle.0.clone());
}

// ---- Update systems ---------------------------------------------------------

/// Watch EventLog for new HardImpact events (watermark pattern from mixer.rs).
/// Each new impact increments the damage level, capped at 1.0.
fn accumulate_damage(
    event_log: Option<Res<EventLog>>,
    mut damage: ResMut<DamageVisualState>,
    mut last_seen: Local<f32>,
) {
    let Some(event_log) = event_log else { return };

    let mut newest_ts = *last_seen;

    for (ts, ev) in &event_log.events {
        if *ts <= *last_seen {
            continue;
        }
        if *ts > newest_ts {
            newest_ts = *ts;
        }
        if let GameEvent::HardImpact { .. } = ev {
            damage.level = (damage.level + DAMAGE_PER_IMPACT).min(1.0);
            info!("damage: {:.2}", damage.level);
        }
    }

    *last_seen = newest_ts;
}

/// Lerp chassis base_color from original toward dark gray by damage.level.
/// dark_factor = 1 - level * MAX_DARKNESS_FACTOR
///   level=0 → factor=1.0 (no change)
///   level=1 → factor=0.5 (50% darker)
fn apply_damage_to_material(
    damage: Res<DamageVisualState>,
    cache: Res<DamageOriginalColor>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if !damage.is_changed() && !cache.is_changed() {
        return;
    }

    let (Some(ref original), Some(ref handle)) =
        (cache.color.clone(), cache.handle.clone())
    else {
        return;
    };

    let Some(mat) = materials.get_mut(handle) else {
        return;
    };

    let orig = original.to_srgba();
    let factor = 1.0 - damage.level * MAX_DARKNESS_FACTOR;

    mat.base_color = Color::srgba(
        orig.red   * factor,
        orig.green * factor,
        orig.blue  * factor,
        orig.alpha,
    );
}

/// Shift+W resets damage level to zero (mirrors vehicle_dirt wash key).
fn wash_with_shift_w(
    keys: Res<ButtonInput<KeyCode>>,
    mut damage: ResMut<DamageVisualState>,
) {
    if keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight) {
        if keys.just_pressed(KeyCode::KeyW) {
            damage.level = 0.0;
            info!("damage repaired");
        }
    }
}
