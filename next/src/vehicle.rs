// Raycast suspension vehicle model.
//
// TUNING CONSTANTS (physical rationale):
//   CHASSIS_MASS   = 1500 kg   — Jeep Wrangler TJ curb weight
//   SPRING_K       = 50_000    — ~0.07 m static sag per wheel at 1500 kg / 4
//   DAMPING_C      = 4_000     — near-critical per wheel; kills bounce within 1-2 cycles
//   SUSPENSION_LEN = 0.60 m    — axle-to-ground distance at natural rest
//   DRIVE_FORCE    = 700 N/whl — 2800 N total; adequate for off-road traction
//   LATERAL_GRIP   = 8_000     — N/(m/s) per wheel; prevents sideways slide
//   BRAKE_FORCE    = 3000 N/whl — overcomes ~11° slope gravity + stops from 4 m/s in ~0.5 s
//   MAX_STEER_DEG  = 30°       — typical off-road steering angle
//   ANG_DAMP       = 12.0      — prevents somersaults on rough terrain

use bevy::prelude::*;
use avian3d::prelude::*;
use crate::terrain::terrain_height_at;

pub struct VehiclePlugin;

impl Plugin for VehiclePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DriveInput>()
           .add_systems(Startup, spawn_vehicle)
           .add_systems(Update, drive_input_keyboard)
           .add_systems(
               PhysicsSchedule,
               suspension_system
                   .after(PhysicsStepSystems::NarrowPhase)
                   .before(PhysicsStepSystems::Solver),
           )
           .add_systems(Update, update_wheel_visuals);
    }
}

pub struct VehiclePluginHeadless;

impl Plugin for VehiclePluginHeadless {
    fn build(&self, app: &mut App) {
        app.init_resource::<DriveInput>()
           .add_systems(Startup, spawn_vehicle)
           .add_systems(
               PhysicsSchedule,
               suspension_system
                   .after(PhysicsStepSystems::NarrowPhase)
                   .before(PhysicsStepSystems::Solver),
           )
           .add_systems(Update, update_wheel_visuals);
    }
}

// ---- Components / Resources ----

#[derive(Component)]
pub struct Chassis;

#[derive(Component)]
pub struct Wheel {
    pub index: usize,
    pub current_compression: f32,
    pub spin: f32,
}

#[derive(Resource)]
pub struct VehicleRoot {
    pub chassis: Entity,
}

#[derive(Resource, Default)]
pub struct DriveInput {
    pub drive: f32,
    pub steer: f32,
    pub brake: bool,
}

// ---- Constants ----

const CHASSIS_HALF: Vec3         = Vec3::new(1.0, 0.4, 2.0);
const WHEEL_RADIUS: f32          = 0.35;
const WHEEL_HALF_WIDTH: f32      = 0.18;

const CHASSIS_MASS: f32          = 1500.0;
const SPRING_K: f32              = 50_000.0;
const DAMPING_C: f32             = 4_000.0; // ≈ critically-damped per wheel → fast settle
const SUSPENSION_LEN: f32        = 0.60;
const DRIVE_FORCE_PER_WHEEL: f32 = 700.0;
const LATERAL_GRIP: f32          = 8_000.0; // N per (m/s) lateral velocity per wheel
const BRAKE_FORCE_PER_WHEEL: f32 = 3_000.0;
const MAX_STEER_ANGLE: f32       = 30_f32 * std::f32::consts::PI / 180.0;
const ANG_DAMP: f32              = 12.0; // very high to prevent flip during initial landing

// FL, FR, RL, RR anchor offsets in chassis local space.
const WHEEL_OFFSETS: [Vec3; 4] = [
    Vec3::new(-1.1, -0.35, -1.4),
    Vec3::new( 1.1, -0.35, -1.4),
    Vec3::new(-1.1, -0.35,  1.4),
    Vec3::new( 1.1, -0.35,  1.4),
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
    let wheel_mesh = meshes.add(Cylinder::new(WHEEL_RADIUS, WHEEL_HALF_WIDTH * 2.0));

    // Spawn height: AVERAGE equilibrium chassis_y across all 4 wheels.
    // chassis_y for wheel i = terrain_h_i + SUSPENSION_LEN - offset_y_i
    // Using the average: no wheel starts far above terrain (no free-fall),
    // and no wheel starts so over-compressed that its anchor goes underground.
    let spawn_y = {
        let sum: f32 = WHEEL_OFFSETS.iter().map(|o| {
            let h = terrain_height_at(o.x, o.z);
            h + SUSPENSION_LEN - o.y
        }).sum::<f32>();
        sum / WHEEL_OFFSETS.len() as f32
    };
    let spawn_pos = Vec3::new(0.0, spawn_y, 0.0);

    // Chassis: full-size box collider (for inertia and fallback collision).
    // With SPAWN_HEIGHT near rest height, the collider bottom (spawn_y - 0.4) is well above
    // terrain (y≈0), so no chassis-terrain contact under normal operation.
    let chassis_id = commands.spawn((
        Chassis,
        Mesh3d(chassis_mesh),
        MeshMaterial3d(chassis_mat),
        Transform::from_translation(spawn_pos),
        RigidBody::Dynamic,
        Collider::cuboid(CHASSIS_HALF.x, CHASSIS_HALF.y, CHASSIS_HALF.z),
        Mass(CHASSIS_MASS),
        LinearDamping(0.5),
        AngularDamping(ANG_DAMP),
        SleepingDisabled,
    )).id();

    // Wheels are visual-only children parented to the chassis.
    for (i, &offset) in WHEEL_OFFSETS.iter().enumerate() {
        let wheel_id = commands.spawn((
            Wheel { index: i, current_compression: 0.0, spin: 0.0 },
            Mesh3d(wheel_mesh.clone()),
            MeshMaterial3d(wheel_mat.clone()),
            Transform::from_translation(offset)
                .with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)),
        )).id();
        commands.entity(chassis_id).add_child(wheel_id);
    }

    commands.insert_resource(VehicleRoot { chassis: chassis_id });
}

// ---- Input ----

pub fn drive_input_keyboard(
    keys: Res<ButtonInput<KeyCode>>,
    mut input: ResMut<DriveInput>,
) {
    input.drive = 0.0;
    input.steer = 0.0;
    input.brake = keys.pressed(KeyCode::Space);

    if keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::ArrowUp)    { input.drive += 1.0; }
    if keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown)  { input.drive -= 1.0; }
    if keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft)  { input.steer += 1.0; }
    if keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight) { input.steer -= 1.0; }
}

// Stub kept so any outside references compile cleanly.
pub fn apply_drive_input() {}

// ---- Suspension + drive (PhysicsSchedule, after narrow phase, before solver) ----

fn suspension_system(
    input: Res<DriveInput>,
    vehicle: Option<Res<VehicleRoot>>,
    mut chassis_q: Query<(Forces, &Transform), With<Chassis>>,
    spatial: SpatialQuery,
) {
    let Some(vehicle) = vehicle else { return };
    let Ok((mut forces, transform)) = chassis_q.get_mut(vehicle.chassis) else { return };

    let chassis_pos = transform.translation;
    let chassis_rot = transform.rotation;
    let chassis_fwd = (chassis_rot * Vec3::NEG_Z).normalize();
    let chassis_up  = (chassis_rot * Vec3::Y).normalize();
    // Velocities from Forces item avoids duplicate query access on LinearVelocity/AngularVelocity.
    let lin_vel_v   = forces.linear_velocity();
    let ang_vel_v   = forces.angular_velocity();

    let filter  = SpatialQueryFilter::from_excluded_entities([vehicle.chassis]);
    // Extend beyond rest length to catch ground when wheel is in-air (e.g. over terrain dips).
    let ray_len = SUSPENSION_LEN + 0.5;

    for (i, &local_anchor) in WHEEL_OFFSETS.iter().enumerate() {
        let world_anchor = chassis_pos + chassis_rot * local_anchor;

        let ray_hit = spatial.cast_ray(world_anchor, Dir3::NEG_Y, ray_len, true, &filter);
        let Some(hit) = ray_hit else {
            continue;
        };

        // hit.distance = distance from anchor (axle) to terrain surface.
        // At natural rest the axle is SUSPENSION_LEN above the terrain.
        // compression > 0 means terrain is closer than rest; = 0 means in air or at rest.
        let compression = (SUSPENSION_LEN - hit.distance).max(0.0);

        // Velocity of the anchor in world space.
        let r        = world_anchor - chassis_pos;
        let v_anchor = lin_vel_v + ang_vel_v.cross(r);

        let normal = Vec3::new(hit.normal.x, hit.normal.y, hit.normal.z);
        // compression_vel > 0 = anchor moving toward terrain (compressing).
        // v_anchor.dot(normal) < 0 when falling (normal points up, velocity points down).
        let compression_vel = -v_anchor.dot(normal);

        // Only apply force when wheel is actually in contact (compression > 0).
        if compression <= 0.0 {
            continue;
        }

        // Suspension force: spring + damper. Both resist the SAME direction:
        //   - compressing (compression_vel > 0): both push UP (large force stops fast fall)
        //   - extending (compression_vel < 0): spring pushes up, damper resists going up
        //     (net = spring - |damper| = smaller → releases energy more slowly → kills bounce)
        // Symmetric damping — DAMPING_C chosen at ~critical to kill bounce within 1-2 cycles.
        let f_damp = if compression_vel >= 0.0 {
            DAMPING_C * compression_vel.min(10.0)
        } else {
            DAMPING_C * compression_vel.max(-10.0) // negative: reduces spring force on rebound
        };
        // f_susp = spring + f_damp (damper adds when compressing, subtracts when extending)
        let f_susp = (SPRING_K * compression + f_damp).max(0.0);
        forces.apply_force_at_point(normal * f_susp, world_anchor);

        // Steer front wheels; rear wheels follow chassis forward.
        let steer_fwd = if i < 2 {
            let q = Quat::from_axis_angle(chassis_up, input.steer * MAX_STEER_ANGLE);
            (q * chassis_fwd).normalize()
        } else {
            chassis_fwd
        };

        // Project drive direction onto contact plane.
        let fwd_ground   = (steer_fwd - steer_fwd.dot(normal) * normal).normalize_or_zero();
        let right_ground = fwd_ground.cross(normal).normalize_or_zero();

        // Longitudinal: drive, brake, or rolling resistance (idle).
        if input.brake {
            let v_long  = v_anchor.dot(fwd_ground);
            let brake_f = (-BRAKE_FORCE_PER_WHEEL * v_long.signum())
                .clamp(-BRAKE_FORCE_PER_WHEEL, BRAKE_FORCE_PER_WHEEL);
            forces.apply_force_at_point(fwd_ground * brake_f, world_anchor);
        } else if input.drive.abs() > 0.0 {
            forces.apply_force_at_point(
                fwd_ground * input.drive * DRIVE_FORCE_PER_WHEEL,
                world_anchor,
            );
        } else {
            // Rolling resistance: oppose longitudinal motion (acts like tire friction at rest).
            let v_long = v_anchor.dot(fwd_ground);
            let resist_f = (-LATERAL_GRIP * v_long).clamp(-f_susp * 1.2, f_susp * 1.2);
            forces.apply_force_at_point(fwd_ground * resist_f, world_anchor);
        }

        // Lateral grip: oppose sideways slip (force in N, not force × mass × velocity).
        let v_lat = v_anchor.dot(right_ground);
        // Cap at f_susp * friction_mu to stay physically bounded by normal load.
        let f_lat_raw = -LATERAL_GRIP * v_lat;
        let f_lat = f_lat_raw.clamp(-f_susp * 1.2, f_susp * 1.2);
        forces.apply_force_at_point(right_ground * f_lat, world_anchor);
    }
}

// ---- Visual wheel update ----

fn update_wheel_visuals(
    vehicle: Option<Res<VehicleRoot>>,
    chassis_q: Query<(&Transform, &LinearVelocity), With<Chassis>>,
    mut wheel_q: Query<(&mut Transform, &mut Wheel), Without<Chassis>>,
    time: Res<Time>,
) {
    let Some(vehicle) = vehicle else { return };
    let Ok((c_transform, lin_vel)) = chassis_q.get(vehicle.chassis) else { return };

    let fwd   = *c_transform.forward();
    let speed = Vec3::new(lin_vel.x, lin_vel.y, lin_vel.z).dot(fwd);
    let dt    = time.delta_secs();

    for (mut transform, mut wheel) in wheel_q.iter_mut() {
        wheel.spin += speed * dt / WHEEL_RADIUS;
        let base_offset    = WHEEL_OFFSETS[wheel.index];
        let compress_delta = Vec3::new(0.0, -wheel.current_compression, 0.0);
        let base_rot       = Quat::from_rotation_z(std::f32::consts::FRAC_PI_2);
        let spin_rot       = Quat::from_rotation_y(wheel.spin);
        transform.translation = base_offset + compress_delta;
        transform.rotation    = base_rot * spin_rot;
    }
}
