use bevy::prelude::*;
use avian3d::prelude::*;
use noise::{NoiseFn, Perlin};

use crate::terrain::{terrain_height_at, TERRAIN_SEED};
use crate::vehicle::{Chassis, VehicleRoot};

pub struct MudPlugin;

impl Plugin for MudPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MudActive>()
           .add_systems(Startup, spawn_mud_patches)
           .add_systems(PhysicsSchedule,
               apply_mud_drag
                   .after(PhysicsStepSystems::NarrowPhase)
                   .before(PhysicsStepSystems::Solver),
           );
    }
}

// ---------------------------------------------------------------------------
// Resources / Components
// ---------------------------------------------------------------------------

#[derive(Resource, Default)]
pub struct MudActive {
    /// True if the chassis is currently in any mud patch.
    pub in_mud: bool,
    /// 0..=1, max submersion across all overlapping patches this frame.
    pub max_submersion: f32,
}

/// Marker placed on each mud patch entity.
#[derive(Component)]
pub struct MudZone {
    pub radius: f32,
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const WORLD_HALF: f32 = 90.0;
// Minimum XZ distance from origin so we don't drown the spawn point.
const SPAWN_CLEAR_RADIUS: f32 = 15.0;
// Target number of mud patches. The LCG loop tries this many candidates and
// skips any that land inside SPAWN_CLEAR_RADIUS.
const PATCH_COUNT: usize = 12;
// Mud patches seeded distinctly from trees (+1) and rocks (+2).
const MUD_SEED: u32 = TERRAIN_SEED + 7;
// Drag coefficient (N per unit submersion); lighter than water (800).
const MUD_DRAG_COEFF: f32 = 400.0;
// Chassis mass mirrored from vehicle.rs — used for the slight sinking force.
const CHASSIS_MASS: f32 = 1500.0;

// ---------------------------------------------------------------------------
// Startup: spawn mud patches
// ---------------------------------------------------------------------------

fn spawn_mud_patches(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Muddy-brown material: highly rough, faintly emissive so it reads against
    // the varied terrain colours without being garish.
    let mud_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.30, 0.20, 0.12),
        perceptual_roughness: 0.95,
        emissive: LinearRgba::new(0.04, 0.025, 0.008, 1.0),
        ..default()
    });

    // LCG state seeded from MUD_SEED. Two independent streams for x/z so
    // the patch positions aren't correlated along the diagonal.
    let mut lcg_x = lcg_init(MUD_SEED);
    let mut lcg_z = lcg_init(MUD_SEED ^ 0xDEAD_BEEF);
    let mut lcg_r = lcg_init(MUD_SEED.wrapping_add(13));

    // Perlin noise used to bias placement toward low-lying wet areas
    // (low noise value → depression → more likely to hold water/mud).
    let perlin = Perlin::new(MUD_SEED);

    let mut spawned = 0usize;
    let mut attempts = 0usize;

    while spawned < PATCH_COUNT && attempts < PATCH_COUNT * 20 {
        attempts += 1;

        let (x, lcg_x2) = lcg_next(lcg_x);
        let (z, lcg_z2) = lcg_next(lcg_z);
        let (r_raw, lcg_r2) = lcg_next(lcg_r);
        lcg_x = lcg_x2;
        lcg_z = lcg_z2;
        lcg_r = lcg_r2;

        let wx = (x - 0.5) * 2.0 * WORLD_HALF;
        let wz = (z - 0.5) * 2.0 * WORLD_HALF;

        // Skip if too close to the spawn origin.
        if wx * wx + wz * wz < SPAWN_CLEAR_RADIUS * SPAWN_CLEAR_RADIUS {
            continue;
        }

        // Use Perlin noise to prefer low-lying depressions.  Patches with noise
        // < 0.35 (normalised 0..1) are discarded — those tend to be on ridges.
        let nx = (wx / 200.0 + 0.5) as f64;
        let nz = (wz / 200.0 + 0.5) as f64;
        let n_val = perlin.get([nx * 4.0, nz * 4.0]) as f32 * 0.5 + 0.5;
        if n_val < 0.35 {
            continue;
        }

        // Radius in [2.0, 6.0] m.
        let radius = 2.0 + r_raw * 4.0;

        let y = terrain_height_at(wx, wz) + 0.05;

        // Thin cylinder approximates a flat disk (height = 0.05 m).
        let mesh = meshes.add(Cylinder::new(radius, 0.05));

        commands.spawn((
            MudZone { radius },
            Mesh3d(mesh),
            MeshMaterial3d(mud_mat.clone()),
            Transform::from_translation(Vec3::new(wx, y, wz)),
        ));

        spawned += 1;
    }
}

// ---------------------------------------------------------------------------
// Physics: drag + sinking force
// ---------------------------------------------------------------------------

fn apply_mud_drag(
    vehicle: Option<Res<VehicleRoot>>,
    mut chassis_q: Query<(Forces, &Transform), With<Chassis>>,
    mud_zones: Query<(&Transform, &MudZone)>,
    mut mud_active: ResMut<MudActive>,
) {
    // Reset frame state.
    mud_active.in_mud = false;
    mud_active.max_submersion = 0.0;

    let Some(vehicle) = vehicle else { return };
    let Ok((mut forces, chassis_tf)) = chassis_q.get_mut(vehicle.chassis) else { return };

    let chassis_xz = Vec2::new(chassis_tf.translation.x, chassis_tf.translation.z);

    for (zone_tf, zone) in mud_zones.iter() {
        let zone_xz = Vec2::new(zone_tf.translation.x, zone_tf.translation.z);
        let dist = (chassis_xz - zone_xz).length();

        if dist >= zone.radius {
            continue;
        }

        // 1.0 at the centre, 0.0 at the edge.
        let submersion = 1.0 - dist / zone.radius;

        mud_active.in_mud = true;
        if submersion > mud_active.max_submersion {
            mud_active.max_submersion = submersion;
        }

        // Horizontal drag opposes the chassis's current XZ velocity.
        let vel = forces.linear_velocity();
        let drag_coeff = MUD_DRAG_COEFF * submersion;
        forces.apply_force(Vec3::new(
            -vel.x * drag_coeff,
            0.0,
            -vel.z * drag_coeff,
        ));

        // Slight downward press — chassis feels like it's sinking into the mire.
        forces.apply_force(Vec3::new(0.0, -CHASSIS_MASS * 1.0 * submersion, 0.0));
    }
}

// ---------------------------------------------------------------------------
// LCG helpers — deterministic float in [0, 1)
// ---------------------------------------------------------------------------

/// Initialise LCG state from a u32 seed (Wang hash to avoid low-entropy seeds).
#[inline]
fn lcg_init(seed: u32) -> u64 {
    let mut s = seed as u64;
    s ^= s << 17;
    s ^= s >> 31;
    s ^= s << 8;
    (s | 1) as u64 // must be odd for the multiplier to form a full-period LCG
}

/// Advance the LCG and return a float in [0, 1) plus the new state.
#[inline]
fn lcg_next(state: u64) -> (f32, u64) {
    // Knuth's MMIX coefficients.
    let next = state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    let f = (next >> 33) as f32 / (u32::MAX as f32);
    (f, next)
}
