// Visible suspension control arms: 4 cuboid arms attached to the chassis,
// pivoting in real-time to track each wheel's travel. Makes articulation
// visually obvious (especially when the wheel-cam is on).
//
// Public API:
//   SuspensionArmsPlugin

use bevy::prelude::*;

use crate::vehicle::{Chassis, VehicleRoot, Wheel};

// ---- Constants (mirror vehicle.rs) ----

// FL, FR, RL, RR wheel anchor offsets in chassis local space.
const WHEEL_OFFSETS: [Vec3; 4] = [
    Vec3::new(-1.1, -0.35, -1.4),
    Vec3::new( 1.1, -0.35, -1.4),
    Vec3::new(-1.1, -0.35,  1.4),
    Vec3::new( 1.1, -0.35,  1.4),
];

// Chassis-side mount for each arm — pulled halfway toward chassis centre in X.
// X is halved (±0.4 instead of ±1.1); Y and Z match the wheel offset.
const ATTACH_POINTS: [Vec3; 4] = [
    Vec3::new(-0.4, -0.35, -1.4),
    Vec3::new( 0.4, -0.35, -1.4),
    Vec3::new(-0.4, -0.35,  1.4),
    Vec3::new( 0.4, -0.35,  1.4),
];

// Arm cuboid dimensions: long in X (lateral), thin in Y and Z.
const ARM_LEN:  f32 = 0.8;
const ARM_DIM:  f32 = 0.08;

// ---- Components ----

/// Marks one of the four visible suspension control arms.
#[derive(Component)]
pub struct SuspensionArm {
    /// Index into WHEEL_OFFSETS (0=FL, 1=FR, 2=RL, 3=RR).
    pub wheel_index: usize,
    /// Chassis-local position of the chassis-side pivot.
    pub attach_point: Vec3,
}

// ---- Plugin ----

pub struct SuspensionArmsPlugin;

impl Plugin for SuspensionArmsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (attach_arms_once, tick_arms));
    }
}

// ---- System: spawn arms once VehicleRoot is ready ----

fn attach_arms_once(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    vehicle: Option<Res<VehicleRoot>>,
    mut done: Local<bool>,
) {
    if *done { return; }
    let Some(vehicle) = vehicle else { return };

    let arm_mesh = meshes.add(Cuboid::new(ARM_LEN, ARM_DIM, ARM_DIM));
    let arm_mat  = materials.add(StandardMaterial {
        base_color: Color::srgb(0.20, 0.20, 0.22),
        perceptual_roughness: 0.7,
        ..default()
    });

    let chassis = vehicle.chassis;

    for i in 0..4 {
        let attach = ATTACH_POINTS[i];
        let wheel  = WHEEL_OFFSETS[i];
        let mid    = (attach + wheel) * 0.5;

        let arm_id = commands.spawn((
            SuspensionArm { wheel_index: i, attach_point: attach },
            Mesh3d(arm_mesh.clone()),
            MeshMaterial3d(arm_mat.clone()),
            Transform::from_translation(mid),
        )).id();

        commands.entity(chassis).add_child(arm_id);
    }

    *done = true;
}

// ---- System: update arm transform every frame ----

fn tick_arms(
    mut arm_q: Query<(&SuspensionArm, &mut Transform), Without<Chassis>>,
    wheel_q: Query<&Wheel, Without<SuspensionArm>>,
) {
    for (arm, mut transform) in arm_q.iter_mut() {
        // Find the Wheel component matching this arm's index.
        let Some(wheel) = wheel_q.iter().find(|w| w.index == arm.wheel_index) else {
            continue;
        };

        let attach    = arm.attach_point;
        let wheel_pos = WHEEL_OFFSETS[arm.wheel_index]
            + Vec3::new(0.0, -wheel.current_compression, 0.0);

        let delta  = wheel_pos - attach;
        let length = delta.length();

        // Avoid degenerate rotation when the two points coincide.
        if length < 1e-4 { continue; }

        let mid       = (attach + wheel_pos) * 0.5;
        let direction = delta / length;

        transform.translation = mid;
        transform.rotation    = Quat::from_rotation_arc(Vec3::X, direction);
        // Scale X so the cuboid stretches to exactly span attach → wheel.
        transform.scale        = Vec3::new(length / ARM_LEN, 1.0, 1.0);
    }
}
