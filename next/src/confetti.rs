// Confetti burst system.
//
// Fires a burst of ~30 tiny coloured sphere particles whenever:
//   - The player reaches a new waypoint (Waypoint.reached_count increases).
//   - The player earns a new achievement (EarnedAchievements.earned.len() increases).
//
// Each confetti piece is a tiny low-poly sphere with an initial random outward+upward
// velocity. Gravity is integrated manually each frame. Particles self-despawn after
// 2 seconds.

use bevy::prelude::*;
use crate::achievements::EarnedAchievements;
use crate::compass::Waypoint;
use crate::vehicle::{Chassis, VehicleRoot};

// ---- Plugin -----------------------------------------------------------------

pub struct ConfettiPlugin;

impl Plugin for ConfettiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, init_confetti_resources)
           .add_systems(Update, (trigger_burst, update_confetti));
    }
}

// ---- Resources & components -------------------------------------------------

#[derive(Resource)]
struct ConfettiAssets {
    mesh: Handle<Mesh>,
    materials: Vec<Handle<StandardMaterial>>,
}

#[derive(Component)]
struct Confetti {
    pub velocity: Vec3,
    pub lifetime_s: f32,
}

// ---- Constants --------------------------------------------------------------

const BURST_COUNT: usize   = 30;
const LIFETIME_S: f32      = 2.0;
const BASE_SPEED: f32      = 5.0;
const SPEED_RANGE: f32     = 4.0;
const GRAVITY: f32         = 9.81;
const SPAWN_Y_OFFSET: f32  = 1.0;

// 5-colour palette (R, G, B as f32 triples).
const PALETTE: [(f32, f32, f32); 5] = [
    (0.95, 0.15, 0.15), // red
    (0.95, 0.85, 0.10), // yellow
    (0.15, 0.85, 0.25), // green
    (0.20, 0.55, 0.95), // blue
    (0.85, 0.20, 0.85), // magenta
];

// ---- Startup: pre-build shared mesh + materials -----------------------------

fn init_confetti_resources(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mesh = meshes.add(Sphere::new(0.08).mesh().ico(0).unwrap());

    let mats: Vec<Handle<StandardMaterial>> = PALETTE
        .iter()
        .map(|&(r, g, b)| {
            materials.add(StandardMaterial {
                base_color: Color::srgb(r, g, b),
                perceptual_roughness: 0.7,
                ..default()
            })
        })
        .collect();

    commands.insert_resource(ConfettiAssets { mesh, materials: mats });
}

// ---- LCG helpers (no external deps) ----------------------------------------

/// Advance an LCG seed and return a value in [0, 1).
fn lcg_next(seed: &mut u32) -> f32 {
    *seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    *seed as f32 / u32::MAX as f32
}

/// Return a value in [-1, 1).
fn lcg_signed(seed: &mut u32) -> f32 {
    lcg_next(seed) * 2.0 - 1.0
}

// ---- Trigger system ---------------------------------------------------------

fn trigger_burst(
    mut commands: Commands,
    assets: Option<Res<ConfettiAssets>>,
    vehicle: Option<Res<VehicleRoot>>,
    chassis_q: Query<&Transform, With<Chassis>>,
    waypoint: Option<Res<Waypoint>>,
    achievements: Option<Res<EarnedAchievements>>,
    mut last_waypoint_count: Local<u32>,
    mut last_achievement_count: Local<usize>,
    time: Res<Time>,
) {
    let Some(assets) = assets else { return };
    let Some(vehicle) = vehicle else { return };
    let Ok(chassis_tf) = chassis_q.get(vehicle.chassis) else { return };

    let chassis_pos = chassis_tf.translation;

    // Check waypoint progress.
    let current_wpt = waypoint.as_ref().map(|w| w.reached_count).unwrap_or(0);
    let wpt_fired = current_wpt > *last_waypoint_count;
    *last_waypoint_count = current_wpt;

    // Check achievement progress.
    let current_ach = achievements.as_ref().map(|a| a.earned.len()).unwrap_or(0);
    let ach_fired = current_ach > *last_achievement_count;
    *last_achievement_count = current_ach;

    if !wpt_fired && !ach_fired {
        return;
    }

    // Seed from elapsed time so each burst looks different.
    let mut seed = (time.elapsed_secs() * 1_000_000.0) as u32;
    let spawn_pos = chassis_pos + Vec3::Y * SPAWN_Y_OFFSET;

    for _ in 0..BURST_COUNT {
        // Random outward+upward direction; ensure y >= 0.3 so pieces arc upward.
        let dx = lcg_signed(&mut seed);
        let dz = lcg_signed(&mut seed);
        let dy = 0.3 + lcg_next(&mut seed) * 0.7; // 0.3..1.0
        let dir = Vec3::new(dx, dy, dz).normalize_or(Vec3::Y);

        let speed = BASE_SPEED + lcg_next(&mut seed) * SPEED_RANGE;
        let velocity = dir * speed;

        // Pick a material from the palette.
        let mat_idx = (lcg_next(&mut seed) * PALETTE.len() as f32) as usize;
        let mat_idx = mat_idx.min(PALETTE.len() - 1);
        let material = assets.materials[mat_idx].clone();

        commands.spawn((
            Confetti { velocity, lifetime_s: LIFETIME_S },
            Mesh3d(assets.mesh.clone()),
            MeshMaterial3d(material),
            Transform::from_translation(spawn_pos),
        ));
    }
}

// ---- Per-frame update -------------------------------------------------------

fn update_confetti(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut Transform, &mut Confetti)>,
) {
    let dt = time.delta_secs();

    for (entity, mut transform, mut confetti) in &mut query {
        // Integrate position.
        transform.translation += confetti.velocity * dt;

        // Apply gravity.
        confetti.velocity.y -= GRAVITY * dt;

        // Count down lifetime and despawn when expired.
        confetti.lifetime_s -= dt;
        if confetti.lifetime_s <= 0.0 {
            commands.entity(entity).despawn();
        }
    }
}
