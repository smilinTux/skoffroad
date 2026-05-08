// Demolition: scatter ~30 smashable wooden crates around the map. Each
// hit by the chassis at >5 m/s shatters into smaller pieces (visual debris)
// and awards 100 score. X key toggles demolition mode (spawn/despawn crates).
//
// Public API:
//   DemolitionPlugin
//   DemolitionState (resource)

use bevy::prelude::*;
use avian3d::prelude::*;

use crate::terrain::terrain_height_at;
use crate::vehicle::{Chassis, VehicleRoot};

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

pub struct DemolitionPlugin;

impl Plugin for DemolitionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DemolitionState>()
           .add_systems(Startup, spawn_demolition_hud)
           .add_systems(Update, (
               toggle_with_x,
               detect_crate_smash.run_if(resource_exists::<VehicleRoot>),
               tick_debris,
               update_hud,
           ));
    }
}

#[derive(Resource, Default)]
pub struct DemolitionState {
    pub active: bool,
    pub crates_remaining: u32,
    pub score: u32,
}

// ---------------------------------------------------------------------------
// Private components
// ---------------------------------------------------------------------------

#[derive(Component)]
struct Crate;

#[derive(Component)]
struct Debris {
    age_s: f32,
}

#[derive(Component)]
struct DemolitionHudRoot;

#[derive(Component)]
struct DemolitionHudTitle;

#[derive(Component)]
struct DemolitionHudStats;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const CRATE_COUNT: u32     = 30;
const CRATE_HALF: f32      = 0.75;   // half-extent of each 1.5 m cube
const CRATE_MASS: f32      = 5.0;
const DEBRIS_HALF: f32     = 0.20;   // half-extent of 0.4 m debris cube
const DEBRIS_MASS: f32     = 0.5;
const DEBRIS_LIFETIME_S: f32 = 6.0;
const SMASH_SPEED_MPS: f32 = 5.0;
const SMASH_RADIUS_M: f32  = 2.0;
const SCATTER_RANGE: f32   = 50.0;   // crates in [-50, 50] ≡ 100 m square
const ORIGIN_EXCL: f32     = 15.0;   // skip 15 m radius around origin

// Simple LCG seeded independently for demolition crate placement.
struct Lcg(u64);

impl Lcg {
    fn new(seed: u32) -> Self { Self(seed as u64) }

    fn next_f32(&mut self) -> f32 {
        self.0 = self.0
            .wrapping_mul(1_664_525)
            .wrapping_add(1_013_904_223)
            & 0xFFFF_FFFF;
        self.0 as f32 / u32::MAX as f32
    }

    fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + self.next_f32() * (hi - lo)
    }
}

// ---------------------------------------------------------------------------
// Startup: hidden HUD panel
// ---------------------------------------------------------------------------

fn spawn_demolition_hud(mut commands: Commands) {
    // Top-centre panel: 280 × 44, hidden until mode is active.
    let panel = commands.spawn((
        DemolitionHudRoot,
        Node {
            position_type:   PositionType::Absolute,
            top:             Val::Px(8.0),
            // Centre horizontally: left offset = 50% - half width.
            // We use left + right auto via percent centering trick: place left
            // at 50% and then pull back half the panel width via negative margin.
            left:            Val::Percent(50.0),
            width:           Val::Px(280.0),
            height:          Val::Px(44.0),
            margin:          UiRect::left(Val::Px(-140.0)),
            flex_direction:  FlexDirection::Column,
            align_items:     AlignItems::Center,
            justify_content: JustifyContent::Center,
            row_gap:         Val::Px(2.0),
            padding:         UiRect::axes(Val::Px(12.0), Val::Px(4.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.10, 0.04, 0.04, 0.85)),
        Visibility::Hidden,
    )).id();

    let title = commands.spawn((
        DemolitionHudTitle,
        Text::new("DEMOLITION"),
        TextFont { font_size: 15.0, ..default() },
        TextColor(Color::srgb(1.0, 0.55, 0.15)),
    )).id();

    let stats = commands.spawn((
        DemolitionHudStats,
        Text::new("Score: 0  Crates: 0/30"),
        TextFont { font_size: 12.0, ..default() },
        TextColor(Color::srgb(0.90, 0.90, 0.90)),
    )).id();

    commands.entity(panel).add_children(&[title, stats]);
}

// ---------------------------------------------------------------------------
// Update: X key toggles demolition mode
// ---------------------------------------------------------------------------

fn toggle_with_x(
    keys:      Res<ButtonInput<KeyCode>>,
    mut state: ResMut<DemolitionState>,
    mut commands: Commands,
    mut meshes:   ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    crate_q:  Query<Entity, With<Crate>>,
    debris_q: Query<Entity, With<Debris>>,
) {
    if !keys.just_pressed(KeyCode::KeyX) {
        return;
    }

    state.active = !state.active;

    if state.active {
        // Reset counters and spawn 30 crates.
        state.score = 0;
        state.crates_remaining = 0;

        let crate_mesh = meshes.add(Cuboid::new(
            CRATE_HALF * 2.0,
            CRATE_HALF * 2.0,
            CRATE_HALF * 2.0,
        ));
        let crate_mat = materials.add(StandardMaterial {
            base_color: Color::srgb(0.55, 0.35, 0.18),
            perceptual_roughness: 0.85,
            ..default()
        });

        let mut lcg = Lcg::new(0xDE10_5EED);
        let mut spawned: u32 = 0;
        let mut attempts = 0u32;

        while spawned < CRATE_COUNT && attempts < 4000 {
            attempts += 1;
            let x = lcg.range(-SCATTER_RANGE, SCATTER_RANGE);
            let z = lcg.range(-SCATTER_RANGE, SCATTER_RANGE);

            if x * x + z * z < ORIGIN_EXCL * ORIGIN_EXCL {
                continue;
            }

            let y = terrain_height_at(x, z) + CRATE_HALF;

            commands.spawn((
                Crate,
                Mesh3d(crate_mesh.clone()),
                MeshMaterial3d(crate_mat.clone()),
                Transform::from_translation(Vec3::new(x, y, z)),
                RigidBody::Dynamic,
                Collider::cuboid(CRATE_HALF, CRATE_HALF, CRATE_HALF),
                Mass(CRATE_MASS),
            ));

            spawned += 1;
        }

        state.crates_remaining = spawned;
        info!("demolition: activated — {} crates spawned", spawned);
    } else {
        // Despawn all crates and debris.
        for e in crate_q.iter()  { commands.entity(e).despawn(); }
        for e in debris_q.iter() { commands.entity(e).despawn(); }
        state.crates_remaining = 0;
        state.score = 0;
        info!("demolition: deactivated — crates and debris cleared");
    }
}

// ---------------------------------------------------------------------------
// Update: detect chassis-vs-crate collision by proximity + speed
// ---------------------------------------------------------------------------

fn detect_crate_smash(
    mut commands: Commands,
    vehicle:      Res<VehicleRoot>,
    chassis_q:    Query<(&Transform, &LinearVelocity), With<Chassis>>,
    crate_q:      Query<(Entity, &Transform), With<Crate>>,
    mut state:    ResMut<DemolitionState>,
    mut meshes:   ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if !state.active { return; }

    let Ok((chassis_tf, lin_vel)) = chassis_q.get(vehicle.chassis) else { return };
    let chassis_pos = chassis_tf.translation;
    let speed = lin_vel.0.length();

    if speed <= SMASH_SPEED_MPS { return; }

    // Debris material — same wooden brown.
    let debris_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.55, 0.35, 0.18),
        perceptual_roughness: 0.90,
        ..default()
    });
    let debris_mesh = meshes.add(Cuboid::new(
        DEBRIS_HALF * 2.0,
        DEBRIS_HALF * 2.0,
        DEBRIS_HALF * 2.0,
    ));

    let mut to_smash: Vec<(Entity, Vec3)> = Vec::new();

    for (entity, crate_tf) in crate_q.iter() {
        let dist = chassis_pos.distance(crate_tf.translation);
        if dist < SMASH_RADIUS_M {
            to_smash.push((entity, crate_tf.translation));
        }
    }

    for (entity, crate_pos) in to_smash {
        // Despawn the crate.
        commands.entity(entity).despawn();

        // Spawn 6 debris cubes with outward + upward impulses.
        for i in 0..6usize {
            // Spread debris in six directions evenly around Y.
            let angle = (i as f32) * std::f32::consts::TAU / 6.0;
            let dir = Vec3::new(angle.cos(), 0.0, angle.sin());

            // Impulse strength in [5, 15] + 10 upward.
            // Use the direction index to get deterministic but varied magnitudes.
            let horiz_mag = 5.0 + (i as f32 * 1.67).min(10.0); // 5.0 … ~15.0
            let impulse_vec = dir * horiz_mag + Vec3::Y * 10.0;

            commands.spawn((
                Debris { age_s: 0.0 },
                Mesh3d(debris_mesh.clone()),
                MeshMaterial3d(debris_mat.clone()),
                Transform::from_translation(
                    crate_pos + Vec3::new(dir.x * 0.5, 0.3, dir.z * 0.5)
                ),
                RigidBody::Dynamic,
                Collider::cuboid(DEBRIS_HALF, DEBRIS_HALF, DEBRIS_HALF),
                Mass(DEBRIS_MASS),
                LinearVelocity(impulse_vec),
            ));
        }

        state.score += 100;
        if state.crates_remaining > 0 {
            state.crates_remaining -= 1;
        }

        info!("CRACK! +100 (score: {})", state.score);
    }
}

// ---------------------------------------------------------------------------
// Update: age debris and despawn after lifetime
// ---------------------------------------------------------------------------

fn tick_debris(
    mut commands: Commands,
    time: Res<Time>,
    mut debris_q: Query<(Entity, &mut Debris)>,
) {
    let dt = time.delta_secs();
    for (entity, mut debris) in debris_q.iter_mut() {
        debris.age_s += dt;
        if debris.age_s > DEBRIS_LIFETIME_S {
            commands.entity(entity).despawn();
        }
    }
}

// ---------------------------------------------------------------------------
// Update: refresh HUD visibility and text
// ---------------------------------------------------------------------------

fn update_hud(
    state:      Res<DemolitionState>,
    mut root_q: Query<&mut Visibility, With<DemolitionHudRoot>>,
    mut stats_q: Query<&mut Text, With<DemolitionHudStats>>,
) {
    let Ok(mut vis) = root_q.single_mut() else { return };
    *vis = if state.active {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };

    if !state.active { return; }

    let Ok(mut text) = stats_q.single_mut() else { return };
    text.0 = format!(
        "Score: {}  Crates: {}/{}",
        state.score,
        state.crates_remaining,
        CRATE_COUNT,
    );
}
