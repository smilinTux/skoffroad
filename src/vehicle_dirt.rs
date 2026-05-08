// Vehicle dirt accumulation: per-frame, when the chassis is in dust/mud,
// nudge the chassis material color toward dirt brown. A wash key (Shift+W)
// resets to clean.
//
// Public API:
//   VehicleDirtPlugin
//   DirtState (resource)

use bevy::prelude::*;
use avian3d::prelude::LinearVelocity;

use crate::terrain::terrain_height_at;
use crate::vehicle::{Chassis, VehicleRoot};

// ---- Plugin -----------------------------------------------------------------

pub struct VehicleDirtPlugin;

impl Plugin for VehicleDirtPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DirtState>()
           .init_resource::<OriginalChassisColor>()
           .add_systems(Startup, cache_original_chassis_color)
           .add_systems(Update, (
               try_cache_chassis_color,
               accumulate_dirt,
               apply_dirt_to_material,
               wash_with_shift_w,
           ));
    }
}

// ---- Public resources -------------------------------------------------------

#[derive(Resource, Default, Clone, Copy)]
pub struct DirtState {
    pub level: f32, // 0..1
}

// ---- Internal resources -----------------------------------------------------

/// The original base_color of the chassis body material before any dirt tint.
/// Stored once at startup (or first time VehicleRoot is available).
#[derive(Resource, Default, Clone)]
struct OriginalChassisColor {
    color: Option<Color>,
    /// Handle to the chassis body material so we can mutate it cheaply.
    handle: Option<Handle<StandardMaterial>>,
}

// ---- Constants --------------------------------------------------------------

/// Dirt brown target color (sRGB).
const DIRT_BROWN: Color = Color::srgb(0.30, 0.22, 0.15);

/// Accumulation rate (level/s) while chassis is near terrain and moving.
const DIRT_RATE: f32 = 0.005;

/// Decay rate (level/s) while chassis is high in the air (wind cleans it).
const WIND_DECAY_RATE: f32 = 0.05;

/// Chassis height threshold below which dirt accumulates (metres above terrain).
const NEAR_GROUND_THRESHOLD: f32 = 1.5;

/// Chassis height above terrain above which wind decay kicks in.
const HIGH_IN_AIR_THRESHOLD: f32 = 3.0;

/// Minimum chassis speed (m/s) required for dirt to accumulate.
const MIN_SPEED_FOR_DIRT: f32 = 1.0;

// ---- Systems ----------------------------------------------------------------

/// Startup system: try to cache the chassis body material handle and original
/// base_color. If VehicleRoot isn't ready yet (it's inserted by spawn_vehicle
/// which also runs in Startup), the Update system `try_cache_chassis_color`
/// will retry once it becomes available.
fn cache_original_chassis_color(
    vehicle: Option<Res<VehicleRoot>>,
    mat_q: Query<&MeshMaterial3d<StandardMaterial>, With<Chassis>>,
    materials: Res<Assets<StandardMaterial>>,
    mut cache: ResMut<OriginalChassisColor>,
) {
    do_cache(&vehicle, &mat_q, &materials, &mut cache);
}

/// Update system with a Local<bool> guard: runs once until the color is cached.
/// This handles the case where VehicleRoot isn't available at Startup.
fn try_cache_chassis_color(
    mut done: Local<bool>,
    vehicle: Option<Res<VehicleRoot>>,
    mat_q: Query<&MeshMaterial3d<StandardMaterial>, With<Chassis>>,
    materials: Res<Assets<StandardMaterial>>,
    mut cache: ResMut<OriginalChassisColor>,
) {
    if *done || cache.color.is_some() {
        *done = true;
        return;
    }
    do_cache(&vehicle, &mat_q, &materials, &mut cache);
    if cache.color.is_some() {
        *done = true;
    }
}

/// Shared logic: query the chassis entity's material and store color + handle.
fn do_cache(
    vehicle: &Option<Res<VehicleRoot>>,
    mat_q: &Query<&MeshMaterial3d<StandardMaterial>, With<Chassis>>,
    materials: &Res<Assets<StandardMaterial>>,
    cache: &mut ResMut<OriginalChassisColor>,
) {
    let Some(vehicle) = vehicle else { return };
    let Ok(mat_handle) = mat_q.get(vehicle.chassis) else { return };
    let Some(mat) = materials.get(mat_handle.id()) else { return };
    cache.color = Some(mat.base_color);
    cache.handle = Some(mat_handle.0.clone());
}

/// Accumulate or decay dirt level each frame based on chassis height / speed.
fn accumulate_dirt(
    vehicle: Option<Res<VehicleRoot>>,
    chassis_q: Query<(&Transform, &LinearVelocity), With<Chassis>>,
    time: Res<Time>,
    mut dirt: ResMut<DirtState>,
) {
    let Some(vehicle) = vehicle else { return };
    let Ok((transform, lin_vel)) = chassis_q.get(vehicle.chassis) else { return };

    let dt = time.delta_secs();
    let pos = transform.translation;
    let terrain_y = terrain_height_at(pos.x, pos.z);
    let height_above = pos.y - terrain_y;
    let speed = Vec3::new(lin_vel.x, lin_vel.y, lin_vel.z).length();

    if height_above > HIGH_IN_AIR_THRESHOLD {
        // Wind cleans the chassis when it's high in the air.
        dirt.level = (dirt.level - WIND_DECAY_RATE * dt).max(0.0);
    } else if height_above < NEAR_GROUND_THRESHOLD && speed > MIN_SPEED_FOR_DIRT {
        // Near the ground and moving — accumulate dirt.
        dirt.level = (dirt.level + DIRT_RATE * dt).min(1.0);
    }
}

/// Apply the current dirt level to the chassis material color.
/// Only runs (and mutates the material) when DirtState has changed.
fn apply_dirt_to_material(
    dirt: Res<DirtState>,
    cache: Res<OriginalChassisColor>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if !dirt.is_changed() {
        return;
    }

    let (Some(ref original), Some(ref handle)) = (cache.color.clone(), cache.handle.clone()) else {
        return;
    };

    let Some(mat) = materials.get_mut(handle) else {
        return;
    };

    let orig = original.to_srgba();
    let dirt_brown = DIRT_BROWN.to_srgba();
    let t = dirt.level;

    mat.base_color = Color::srgba(
        orig.red   + (dirt_brown.red   - orig.red)   * t,
        orig.green + (dirt_brown.green - orig.green) * t,
        orig.blue  + (dirt_brown.blue  - orig.blue)  * t,
        orig.alpha,
    );
}

/// Shift+W resets dirt level to zero.
fn wash_with_shift_w(
    keys: Res<ButtonInput<KeyCode>>,
    mut dirt: ResMut<DirtState>,
) {
    if keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight) {
        if keys.just_pressed(KeyCode::KeyW) {
            dirt.level = 0.0;
            info!("vehicle washed clean");
        }
    }
}
