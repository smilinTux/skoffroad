// Collectible glowing spheres for SandK Offroad.
//
// 30 spheres are placed procedurally via LCG seeded on TERRAIN_SEED + 31.
// Each sphere floats 1.2 m above the terrain, bobs and spins. Drive through
// one to collect it: the sphere despawns and the counter increments.
// A top-right HUD panel shows CYAN: collected / total.

use bevy::prelude::*;
use crate::terrain::{terrain_height_at, TERRAIN_SEED};
use crate::vehicle::{Chassis, VehicleRoot};

// ---- Public types -----------------------------------------------------------

pub struct CollectiblesPlugin;

impl Plugin for CollectiblesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CollectibleCount>()
           .add_systems(Startup, (spawn_collectibles, spawn_collectible_hud))
           .add_systems(Update, (
               animate_collectibles,
               detect_pickup.run_if(resource_exists::<VehicleRoot>),
               update_collectible_hud,
           ));
    }
}

#[derive(Resource, Default)]
pub struct CollectibleCount {
    pub collected: u32,
    pub total: u32,
}

// ---- Private components -----------------------------------------------------

#[derive(Component)]
struct Collectible {
    base_y: f32,
    phase:  f32,
}

#[derive(Component)] struct CollectibleHudRoot;
#[derive(Component)] struct CollectibleHudText;

// ---- Constants --------------------------------------------------------------

const SPHERE_RADIUS: f32   = 0.6;
const FLOAT_HEIGHT: f32    = 1.2;
const BOB_AMP: f32         = 0.3;
const BOB_FREQ: f32        = 1.5; // rad/s
const SPIN_RATE: f32       = 1.0; // rad/s
const PICKUP_DIST_XZ: f32  = 1.8; // m (chassis half-width + sphere radius)
const SPAWN_COUNT: usize   = 30;
const AVOID_ORIGIN: f32    = 10.0; // m radius around origin to skip

// ---- LCG helpers ------------------------------------------------------------

fn lcg_next(state: &mut u64) -> u64 {
    // Classic Knuth MMIX LCG
    *state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    *state
}

/// Map a u64 into [lo, hi].
fn lcg_f32(state: &mut u64, lo: f32, hi: f32) -> f32 {
    let bits = lcg_next(state);
    let t = (bits >> 11) as f32 / (1u64 << 53) as f32;
    lo + t * (hi - lo)
}

// ---- Startup: spawn spheres -------------------------------------------------

fn spawn_collectibles(
    mut commands: Commands,
    mut meshes:   ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut count: ResMut<CollectibleCount>,
) {
    let mesh = meshes.add(Sphere::new(SPHERE_RADIUS).mesh().ico(2).unwrap());
    let mat  = materials.add(StandardMaterial {
        base_color: Color::srgb(0.2, 1.0, 1.0),
        emissive:   LinearRgba::rgb(0.0, 3.0, 3.0),
        ..default()
    });

    let mut state: u64 = (TERRAIN_SEED as u64).wrapping_add(31);
    let mut spawned: u32 = 0;
    let mut attempts = 0usize;

    while spawned < SPAWN_COUNT as u32 && attempts < 2000 {
        attempts += 1;
        let x = lcg_f32(&mut state, -90.0, 90.0);
        let z = lcg_f32(&mut state, -90.0, 90.0);

        if x * x + z * z < AVOID_ORIGIN * AVOID_ORIGIN {
            continue;
        }

        let phase  = lcg_f32(&mut state, 0.0, std::f32::consts::TAU);
        let ground = terrain_height_at(x, z);
        let base_y = ground + FLOAT_HEIGHT;

        commands.spawn((
            Collectible { base_y, phase },
            Mesh3d(mesh.clone()),
            MeshMaterial3d(mat.clone()),
            Transform::from_translation(Vec3::new(x, base_y, z)),
        ));

        spawned += 1;
    }

    count.total = spawned;
}

// ---- Startup: spawn HUD panel -----------------------------------------------

fn spawn_collectible_hud(mut commands: Commands) {
    let panel = commands.spawn((
        CollectibleHudRoot,
        Node {
            position_type: PositionType::Absolute,
            // Moved from right-side to left-side to clear the boost/fuel/gauge
            // stack that grew along the right edge. Top-left stack:
            // HUD (top:12) → free → collectibles (top:336).
            top:  Val::Px(336.0),
            left: Val::Px(12.0),
            width: Val::Px(160.0),
            padding: UiRect::all(Val::Px(8.0)),
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            ..default()
        },
        BackgroundColor(Color::srgba(0.05, 0.05, 0.07, 0.80)),
        Visibility::Hidden,
    )).id();

    let label = commands.spawn((
        CollectibleHudText,
        Text::new("CYAN: 0 / 0"),
        TextFont { font_size: 13.0, ..default() },
        TextColor(Color::srgb(0.2, 1.0, 1.0)),
    )).id();

    commands.entity(panel).add_child(label);
}

// ---- Update: animate --------------------------------------------------------

fn animate_collectibles(
    time: Res<Time>,
    mut q: Query<(&Collectible, &mut Transform)>,
) {
    let t = time.elapsed_secs();
    for (col, mut transform) in q.iter_mut() {
        let bob = (t * BOB_FREQ + col.phase).sin() * BOB_AMP;
        transform.translation.y = col.base_y + bob;
        transform.rotation = Quat::from_rotation_y(t * SPIN_RATE + col.phase);
    }
}

// ---- Update: pickup detection -----------------------------------------------

fn detect_pickup(
    mut commands:   Commands,
    vehicle:        Res<VehicleRoot>,
    chassis_q:      Query<&Transform, With<Chassis>>,
    collectible_q:  Query<(Entity, &Transform), With<Collectible>>,
    mut count:      ResMut<CollectibleCount>,
) {
    let Ok(chassis_tf) = chassis_q.get(vehicle.chassis) else { return };
    let cpos = chassis_tf.translation;

    for (entity, col_tf) in collectible_q.iter() {
        let coll_pos = col_tf.translation;
        let dx = cpos.x - coll_pos.x;
        let dz = cpos.z - coll_pos.z;
        let dist_xz = (dx * dx + dz * dz).sqrt();
        if dist_xz < PICKUP_DIST_XZ {
            commands.entity(entity).despawn();
            count.collected += 1;
            info!("collectible: {} / {}", count.collected, count.total);
        }
    }
}

// ---- Update: HUD update -----------------------------------------------------

fn update_collectible_hud(
    count: Res<CollectibleCount>,
    mut root_q: Query<&mut Visibility, With<CollectibleHudRoot>>,
    mut text_q: Query<(&mut Text, &mut TextColor), With<CollectibleHudText>>,
) {
    let Ok(mut vis) = root_q.single_mut() else { return };

    if count.total == 0 {
        *vis = Visibility::Hidden;
        return;
    }
    *vis = Visibility::Inherited;

    let Ok((mut text, mut color)) = text_q.single_mut() else { return };
    **text = format!("CYAN: {} / {}", count.collected, count.total);

    if count.collected >= count.total {
        color.0 = Color::srgb(0.2, 1.0, 0.2); // green when complete
    } else {
        color.0 = Color::srgb(0.2, 1.0, 1.0); // cyan otherwise
    }
}
