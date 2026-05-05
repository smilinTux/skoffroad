// Desert biome overlay: cactus props, amber sky/fog tint, warm ambient light.
// Activates when ActiveMap.0 == MapKind::Desert.
//
// Public API:
//   BiomeDesertPlugin
//   DesertProp    (component — marks spawned cactus entities)
//   DesertSpawnState (resource — tracks whether cacti are currently live)

use bevy::pbr::DistanceFog;
use bevy::prelude::*;
use avian3d::prelude::*;

use crate::maps::{ActiveMap, MapKind};
use crate::terrain::terrain_height_at;

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct BiomeDesertPlugin;

impl Plugin for BiomeDesertPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(DesertSpawnState { spawned: false })
           .add_systems(Update, (
               manage_desert_biome,
               update_desert_atmosphere,
           ));
    }
}

// ---------------------------------------------------------------------------
// Components / Resources
// ---------------------------------------------------------------------------

/// Marks every entity spawned as part of the desert cactus layer so they can
/// be bulk-despawned when the biome becomes inactive.
#[derive(Component)]
pub struct DesertProp;

/// Tracks whether the cactus set is currently live in the world.
#[derive(Resource)]
pub struct DesertSpawnState {
    pub spawned: bool,
}

// ---------------------------------------------------------------------------
// Placement constants
// ---------------------------------------------------------------------------

/// Total cactus props to attempt.
const CACTUS_COUNT: u32 = 40;
/// Square half-extent within which to scatter props (±60 m = 120 m side).
const SCATTER_HALF: f32 = 60.0;
/// Radius around origin kept clear (player spawn zone).
const SPAWN_CLEAR: f32 = 12.0;

// Atmosphere targets
const FOG_AMBER:   [f32; 4] = [0.90, 0.70, 0.40, 1.0];
const FOG_NEUTRAL: [f32; 4] = [0.70, 0.78, 0.85, 1.0];
const AMBIENT_WARM: [f32; 3] = [1.05, 0.95, 0.80];
// Neutral ambient (matches sky.rs day palette midpoint)
const AMBIENT_NEUTRAL: [f32; 3] = [0.50, 0.55, 0.65];
const FOG_LERP_SPEED: f32 = 0.5;

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

/// Spawns or despawns the desert cactus layer whenever `ActiveMap` changes.
pub fn manage_desert_biome(
    active_map:  Res<ActiveMap>,
    mut state:   ResMut<DesertSpawnState>,
    mut commands: Commands,
    mut meshes:   ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    props:       Query<Entity, With<DesertProp>>,
) {
    if !active_map.is_changed() {
        return;
    }

    let is_desert = active_map.0 == MapKind::Desert;

    if is_desert && !state.spawned {
        spawn_cacti(&mut commands, &mut meshes, &mut materials);
        state.spawned = true;
    } else if !is_desert && state.spawned {
        for entity in &props {
            commands.entity(entity).despawn();
        }
        state.spawned = false;
    }
}

/// Every frame: lerp fog color and ambient light toward Desert or neutral
/// targets depending on whether we are currently in the Desert biome.
pub fn update_desert_atmosphere(
    active_map: Res<ActiveMap>,
    time:       Res<Time>,
    mut fogs:   Query<&mut DistanceFog>,
    mut ambient: ResMut<GlobalAmbientLight>,
) {
    let is_desert = active_map.0 == MapKind::Desert;
    let dt = time.delta_secs();
    let t  = (dt * FOG_LERP_SPEED).clamp(0.0, 1.0);

    let [fr, fg, fb, fa] = if is_desert { FOG_AMBER } else { FOG_NEUTRAL };

    for mut fog in &mut fogs {
        let current = fog.color.to_srgba();
        fog.color = Color::srgba(
            lerp(current.red,   fr, t),
            lerp(current.green, fg, t),
            lerp(current.blue,  fb, t),
            lerp(current.alpha, fa, t),
        );
    }

    // Lerp ambient color toward warm desert tones or back to neutral.
    let [ar, ag, ab] = if is_desert { AMBIENT_WARM } else { AMBIENT_NEUTRAL };
    let ac = ambient.color.to_srgba();
    ambient.color = Color::srgb(
        lerp(ac.red,   ar, t),
        lerp(ac.green, ag, t),
        lerp(ac.blue,  ab, t),
    );
}

// ---------------------------------------------------------------------------
// Cactus spawner
// ---------------------------------------------------------------------------

fn spawn_cacti(
    commands:  &mut Commands,
    meshes:    &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    let cactus_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.20, 0.45, 0.20),
        perceptual_roughness: 0.85,
        ..default()
    });

    // Trunk: 0.4 × 2.5 × 0.4 cuboid
    let trunk_mesh = meshes.add(Cuboid::new(0.4, 2.5, 0.4));
    // Arm: 0.4 × 0.8 × 0.4 cuboid
    let arm_mesh   = meshes.add(Cuboid::new(0.4, 0.8, 0.4));

    for i in 0..CACTUS_COUNT {
        // LCG noise — deterministic from index, gives positions in [0, 1)
        let hx = lcg(i * 2)        * 2.0 * SCATTER_HALF - SCATTER_HALF;
        let hz = lcg(i * 2 + 1)    * 2.0 * SCATTER_HALF - SCATTER_HALF;

        // Skip spawn-clear zone
        if hx * hx + hz * hz < SPAWN_CLEAR * SPAWN_CLEAR {
            continue;
        }

        let y = terrain_height_at(hx, hz);

        // Trunk is centred; Bevy Cuboid is centred at origin, so lift by half height.
        let trunk_offset = Vec3::new(0.0, 2.5 * 0.5, 0.0);

        let parent = commands.spawn((
            Transform::from_translation(Vec3::new(hx, y, hz)),
            Visibility::default(),
            RigidBody::Static,
            Collider::cuboid(0.4, 2.5, 0.4),
            DesertProp,
        )).id();

        let trunk = commands.spawn((
            Mesh3d(trunk_mesh.clone()),
            MeshMaterial3d(cactus_mat.clone()),
            Transform::from_translation(trunk_offset),
        )).id();

        commands.entity(parent).add_children(&[trunk]);

        // 0-2 arms per cactus driven by deterministic hash
        let arm_count = (lcg(i * 3 + 100) * 3.0) as u32; // 0, 1, or 2
        for a in 0..arm_count.min(2) {
            // Side offset: alternate ±X
            let sign   = if a == 0 { 1.0_f32 } else { -1.0_f32 };
            let arm_ox = sign * 0.5;    // 0.5 m from trunk centre
            let arm_oy = 1.5;           // attach at height 1.5 m
            let arm_oz = 0.0_f32;

            let arm = commands.spawn((
                Mesh3d(arm_mesh.clone()),
                MeshMaterial3d(cactus_mat.clone()),
                // Arm cuboid centred at attachment point; arm extends upward from there
                Transform::from_translation(Vec3::new(
                    arm_ox,
                    arm_oy + 0.8 * 0.5, // lift by half arm height
                    arm_oz,
                )),
            )).id();

            commands.entity(parent).add_children(&[arm]);
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Minimal LCG hash: maps a u32 seed to a float in [0, 1).
/// Deterministic for any given i — no external noise crate needed.
#[inline]
fn lcg(seed: u32) -> f32 {
    // Knuth multiplicative hash
    let mut v = seed.wrapping_mul(2654435761);
    v ^= v >> 16;
    v = v.wrapping_mul(2246822519);
    v ^= v >> 13;
    v as f32 / u32::MAX as f32
}

#[inline]
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}
