// Simplified vehicle: chassis rigid body + 4 wheel spheres attached via
// RevoluteJoint (allows spinning, acts as welded suspension).
// WASD / arrow keys drive. Space brakes.

use bevy::prelude::*;
use avian3d::prelude::*;

pub struct VehiclePlugin;

impl Plugin for VehiclePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_vehicle)
           .add_systems(Update, drive_input);
    }
}

// ---- Components ----

#[derive(Component)]
pub struct Chassis;

#[derive(Component)]
pub struct Wheel {
    // 0=FL, 1=FR, 2=RL, 3=RR (unused at runtime, kept for future use)
    #[allow(dead_code)]
    pub index: usize,
}

#[derive(Resource)]
pub struct VehicleRoot {
    pub chassis: Entity,
}

// ---- Constants ----

const CHASSIS_HALF: Vec3 = Vec3::new(1.0, 0.4, 2.0);
const WHEEL_RADIUS: f32 = 0.35;
const WHEEL_HALF_WIDTH: f32 = 0.18;
const SPAWN_HEIGHT: f32 = 8.0;

// Wheel offsets from chassis centre in chassis local space.
const WHEEL_OFFSETS: [Vec3; 4] = [
    Vec3::new(-1.1, -0.35, -1.4), // front-left
    Vec3::new( 1.1, -0.35, -1.4), // front-right
    Vec3::new(-1.1, -0.35,  1.4), // rear-left
    Vec3::new( 1.1, -0.35,  1.4), // rear-right
];

// ---- Spawn ----

fn spawn_vehicle(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let chassis_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.8, 0.2, 0.1),
        perceptual_roughness: 0.6,
        ..default()
    });
    let wheel_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.12, 0.12, 0.12),
        perceptual_roughness: 0.9,
        ..default()
    });

    let chassis_mesh = meshes.add(Cuboid::new(
        CHASSIS_HALF.x * 2.0,
        CHASSIS_HALF.y * 2.0,
        CHASSIS_HALF.z * 2.0,
    ));

    let spawn_pos = Vec3::new(0.0, SPAWN_HEIGHT, 0.0);

    let chassis_id = commands.spawn((
        Chassis,
        Mesh3d(chassis_mesh),
        MeshMaterial3d(chassis_mat),
        Transform::from_translation(spawn_pos),
        RigidBody::Dynamic,
        Collider::cuboid(CHASSIS_HALF.x, CHASSIS_HALF.y, CHASSIS_HALF.z),
        LinearDamping(0.3),
        AngularDamping(2.0),
        // SleepingDisabled keeps the chassis always awake so input is never dropped.
        SleepingDisabled,
        // Initial zero forces; drive_input will overwrite them each frame.
        ConstantForce::new(0.0, 0.0, 0.0),
        ConstantTorque::new(0.0, 0.0, 0.0),
    )).id();

    // Cylinder mesh — Bevy cylinders stand along Y; the joint axis is X.
    let wheel_mesh = meshes.add(Cylinder::new(WHEEL_RADIUS, WHEEL_HALF_WIDTH * 2.0));

    for (i, &offset) in WHEEL_OFFSETS.iter().enumerate() {
        let wheel_pos = spawn_pos + offset;
        let wheel_id = commands.spawn((
            Wheel { index: i },
            Mesh3d(wheel_mesh.clone()),
            MeshMaterial3d(wheel_mat.clone()),
            Transform::from_translation(wheel_pos)
                .with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)),
            RigidBody::Dynamic,
            Collider::sphere(WHEEL_RADIUS),
            LinearDamping(0.3),
            AngularDamping(0.5),
        )).id();

        // Revolute joint allows the wheel to spin around the shared X axis.
        commands.spawn(
            RevoluteJoint::new(chassis_id, wheel_id)
                .with_local_anchor1(offset)
                .with_local_anchor2(Vec3::ZERO)
                .with_hinge_axis(Vec3::X),
        );
    }

    commands.insert_resource(VehicleRoot { chassis: chassis_id });
}

// ---- Input ----

fn drive_input(
    keys: Res<ButtonInput<KeyCode>>,
    vehicle: Option<Res<VehicleRoot>>,
    mut chassis_q: Query<
        (&mut ConstantForce, &mut ConstantTorque, &Transform),
        With<Chassis>,
    >,
) {
    let Some(vehicle) = vehicle else { return };
    let Ok((mut force, mut torque, transform)) = chassis_q.get_mut(vehicle.chassis) else { return };

    let forward = transform.forward();

    let mut drive = 0.0_f32;
    let mut steer = 0.0_f32;
    let braking = keys.pressed(KeyCode::Space);

    if keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::ArrowUp)    { drive += 1.0; }
    if keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown)  { drive -= 1.0; }
    if keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft)  { steer += 1.0; }
    if keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight) { steer -= 1.0; }

    const DRIVE_FORCE:  f32 = 3500.0;
    const STEER_TORQUE: f32 = 1200.0;
    const BRAKE_FORCE:  f32 = 2000.0;

    let drive_vec = forward.as_vec3() * drive * DRIVE_FORCE
        - if braking { forward.as_vec3() * BRAKE_FORCE * drive.signum().max(0.0) } else { Vec3::ZERO };

    *force  = ConstantForce::new(drive_vec.x, drive_vec.y, drive_vec.z);
    *torque = ConstantTorque::new(0.0, steer * STEER_TORQUE, 0.0);
}
