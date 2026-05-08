// Fish: 3 schools of small fish drifting under water surfaces.
// Each school = 8 small dark blue cuboids that swim in formation around a
// center point using a parent/child entity hierarchy.
//
// Public API:
//   FishPlugin

use std::f32::consts::PI;

use bevy::prelude::*;

// ---- Plugin -----------------------------------------------------------------

pub struct FishPlugin;

impl Plugin for FishPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_schools)
           .add_systems(Update, move_schools);
    }
}

// ---- Components -------------------------------------------------------------

/// Attached to the invisible parent entity that represents one school.
/// `base` is the spawn-time anchor; `center` is the current world position.
#[derive(Component)]
struct FishSchool {
    /// Fixed anchor point around which the circle drifts.
    base:        Vec3,
    /// Current world-space center of the school.
    center:      Vec3,
    /// Heading angle in radians; drives the circling orbit.
    heading_yaw: f32,
}

/// Attached to each individual fish child entity.
#[derive(Component)]
struct Fish {
    /// Resting offset from the school center, in school-local space.
    offset_local: Vec3,
}

// ---- Constants --------------------------------------------------------------

const FISH_PER_SCHOOL: usize = 8;

/// School spawn anchors (x, y, z).  Y is ignored for the anchor — the
/// dynamic Y formula keeps them underwater at all times.
const SCHOOL_ANCHORS: [(f32, f32, f32); 3] = [
    (-50.0, 0.0,  80.0),
    ( 70.0, 0.0, -55.0),
    ( 10.0, 0.0, 100.0),
];

// ---- Deterministic LCG ------------------------------------------------------

/// Returns a value in [−1, +1) from an integer seed (no rand crate needed).
#[inline]
fn lcg_signed(seed: usize) -> f32 {
    let mut v = (seed as u32).wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    v ^= v >> 16;
    v  = v.wrapping_mul(0x45d9f3b);
    v ^= v >> 16;
    (v as f32) / (u32::MAX as f32) * 2.0 - 1.0
}

// ---- Startup system ---------------------------------------------------------

fn spawn_schools(
    mut commands:  Commands,
    mut meshes:    ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Shared mesh: small cuboid  width × height × depth  (0.25 × 0.10 × 0.60)
    let mesh_handle = meshes.add(Cuboid::new(0.25, 0.10, 0.60));

    // Dark blue, unlit so the fish are visible even in shadowed water.
    let mat_handle = materials.add(StandardMaterial {
        base_color: Color::srgb(0.15, 0.30, 0.55),
        unlit:      true,
        ..default()
    });

    for (school_idx, &(bx, by, bz)) in SCHOOL_ANCHORS.iter().enumerate() {
        let base   = Vec3::new(bx, by, bz);
        let center = Vec3::new(bx, -0.8, bz); // start underwater

        // Spawn the invisible parent (no mesh, just a Transform).
        let parent = commands.spawn((
            Transform::from_translation(center),
            Visibility::default(),
            FishSchool {
                base,
                center,
                heading_yaw: 0.0,
            },
        )).id();

        // Spawn 8 fish children.
        for fish_idx in 0..FISH_PER_SCHOOL {
            let seed_base = school_idx * FISH_PER_SCHOOL * 3 + fish_idx * 3;

            // XZ offset in [−1.5, +1.5]; Y offset is negative to stay under water.
            let ox = lcg_signed(seed_base)     * 1.5;
            let oy = lcg_signed(seed_base + 1) * 1.5 - 0.75; // bias downward
            let oz = lcg_signed(seed_base + 2) * 1.5;
            let offset_local = Vec3::new(ox, oy, oz);

            let child = commands.spawn((
                Mesh3d(mesh_handle.clone()),
                MeshMaterial3d(mat_handle.clone()),
                Transform::from_translation(center + offset_local),
                Fish { offset_local },
            )).id();

            commands.entity(parent).add_child(child);
        }
    }
}

// ---- Update system ----------------------------------------------------------

fn move_schools(
    time:       Res<Time>,
    mut schools: Query<(&mut FishSchool, &mut Transform, &Children)>,
    mut fish_q: Query<(&Fish, &mut Transform), Without<FishSchool>>,
) {
    let dt = time.delta_secs();
    let t  = time.elapsed_secs();

    for (mut school, mut parent_tf, children) in &mut schools {
        // Advance heading — one full circle every ~63 s (2π / 0.1 ≈ 62.8 s).
        school.heading_yaw += dt * 0.1;
        let yaw = school.heading_yaw;

        // Update center: slow circle of radius 6 around the anchor.
        school.center.x = school.base.x + yaw.cos() * 6.0;
        school.center.z = school.base.z + yaw.sin() * 6.0;
        // Gentle vertical bob — always negative so fish remain underwater.
        school.center.y = -0.8 + (t * 0.5).sin() * 0.3;

        // Move parent transform to the new center.
        parent_tf.translation = school.center;

        // Direction the school is swimming (perpendicular to the radius).
        let face_rotation = Quat::from_rotation_y(yaw + PI / 2.0);

        // Build a 2-D rotation matrix (XZ plane) for the heading.
        let (sin_yaw, cos_yaw) = yaw.sin_cos();

        for child_entity in children.iter() {
            let Ok((fish, mut fish_tf)) = fish_q.get_mut(child_entity) else {
                continue;
            };

            // Rotate the local offset by the school heading so the formation
            // stays coherent as the school turns.
            let lo = fish.offset_local;
            let rotated = Vec3::new(
                lo.x * cos_yaw - lo.z * sin_yaw,
                lo.y,
                lo.x * sin_yaw + lo.z * cos_yaw,
            );

            // Small per-fish flutter on the vertical axis.
            let flutter = (t * 1.3 + fish.offset_local.x).sin() * 0.15;
            let world_pos = school.center
                + rotated
                + Vec3::new(0.0, flutter, 0.0);

            fish_tf.translation = world_pos;
            fish_tf.rotation    = face_rotation;
        }
    }
}
