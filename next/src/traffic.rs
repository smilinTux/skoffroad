// Ambient traffic: 5 NPC trucks wandering randomly between waypoints.
// Decorative only — they don't participate in races and ignore the player.
// Toggle with the "8" key. Default OFF.
//
// Public API:
//   TrafficPlugin
//   TrafficState (resource)
//
// Markers:
//   TrafficChassis  — on the chassis RigidBody::Dynamic entity
//   TrafficWheel    — per-wheel component (mirrors RivalWheel fields)
//
// System flow (all in Update unless noted):
//   toggle_with_8        → flip TrafficState::active, spawn/despawn trucks
//   pick_target_for_idle → assign new random waypoint when truck is near target
//   traffic_steering     → apply drive + lateral grip forces
//   traffic_suspension   → raycast spring + damper + anti-roll (PhysicsSchedule)

use bevy::prelude::*;
use avian3d::prelude::*;
use crate::terrain::terrain_height_at;

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct TrafficPlugin;

impl Plugin for TrafficPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TrafficState>()
           .add_systems(Update, (
               toggle_with_8,
               pick_target_for_idle,
               traffic_steering,
           ))
           .add_systems(
               PhysicsSchedule,
               traffic_suspension
                   .after(PhysicsStepSystems::NarrowPhase)
                   .before(PhysicsStepSystems::Solver),
           );
    }
}

// ---------------------------------------------------------------------------
// Public resource
// ---------------------------------------------------------------------------

#[derive(Resource, Default)]
pub struct TrafficState {
    pub active: bool,
}

// ---------------------------------------------------------------------------
// Components
// ---------------------------------------------------------------------------

/// Marker + state on each traffic chassis entity.
#[derive(Component)]
pub struct TrafficChassis {
    pub id: u32,
    /// Current XZ target position in world space.
    pub target_xz: Vec2,
    /// Absolute time (elapsed_secs) after which a new target should be chosen.
    pub idle_until: f32,
}

/// Per-wheel suspension data — mirrors RivalWheel field-for-field.
#[derive(Component)]
pub struct TrafficWheel {
    /// Local-space mount position relative to the chassis origin.
    pub local_pos: Vec3,
    /// How far (m) the wheel is compressed from rest length this tick.
    pub current_compression: f32,
    /// True when the wheel is touching the ground.
    pub is_grounded: bool,
    /// Ground contact normal from the last raycast.
    pub ground_normal: Vec3,
    /// Natural suspension length (m) at rest.
    pub rest_length: f32,
    /// Spring stiffness (N/m).
    pub suspension_stiffness: f32,
    /// Damping coefficient (N·s/m).
    pub suspension_damping: f32,
}

impl TrafficWheel {
    fn new(local_pos: Vec3) -> Self {
        Self {
            local_pos,
            current_compression: 0.0,
            is_grounded: false,
            ground_normal: Vec3::Y,
            rest_length: SUSPENSION_LEN,
            suspension_stiffness: SPRING_K,
            suspension_damping: DAMPING_C,
        }
    }
}

// ---------------------------------------------------------------------------
// Physics constants
// ---------------------------------------------------------------------------

/// Chassis half-extents: 2.0 × 0.6 × 4.0 full → 1.0 × 0.3 × 2.0 half
const CHASSIS_HALF: Vec3  = Vec3::new(1.0, 0.3, 2.0);
const CHASSIS_MASS: f32   = 1500.0;
const SUSPENSION_LEN: f32 = 0.60;
const SPRING_K: f32       = 50_000.0;
const DAMPING_C: f32      = 5_000.0;
const RAY_LEN: f32        = SUSPENSION_LEN + 0.5;
const LATERAL_GRIP: f32   = 8_000.0;
const ANTI_ROLL_K: f32    = 30_000.0;
const ANTI_PITCH_K: f32   = 30_000.0;
const LINEAR_DAMPING: f32 = 0.5;
const ANGULAR_DAMPING: f32 = 25.0;
const WHEEL_RADIUS: f32   = 0.35;
const WHEEL_HALF_WIDTH: f32 = 0.18;
/// Slow cruise throttle — ambient, not racing.
const THROTTLE: f32       = 0.4;
/// N per grounded wheel for drive force.
const DRIVE_FORCE_PER_WHEEL: f32 = 1000.0;

// FL, FR, RL, RR anchor offsets in chassis local space.
const WHEEL_OFFSETS: [Vec3; 4] = [
    Vec3::new(-1.1, -0.25, -1.4), // FL
    Vec3::new( 1.1, -0.25, -1.4), // FR
    Vec3::new(-1.1, -0.25,  1.4), // RL
    Vec3::new( 1.1, -0.25,  1.4), // RR
];

// ---------------------------------------------------------------------------
// Pastel colour palette (5 trucks, one colour each)
// ---------------------------------------------------------------------------

const PALETTE: [Color; 5] = [
    Color::srgb(0.7, 0.5, 0.3), // tan
    Color::srgb(0.5, 0.7, 0.4), // olive
    Color::srgb(0.4, 0.5, 0.7), // faded blue
    Color::srgb(0.7, 0.3, 0.3), // brick red
    Color::srgb(0.5, 0.5, 0.5), // gray
];

const TRUCK_COUNT: u32 = 5;

// ---------------------------------------------------------------------------
// Minimal LCG random number generator (no external rand crate needed)
// ---------------------------------------------------------------------------

/// LCG state — uses Knuth multiplicative constants.
struct Lcg(u64);

impl Lcg {
    fn new(seed: u64) -> Self { Self(seed.wrapping_add(1)) }

    /// Returns the next u64 pseudo-random value.
    fn next(&mut self) -> u64 {
        self.0 = self.0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0
    }

    /// Uniform float in [lo, hi).
    fn range_f32(&mut self, lo: f32, hi: f32) -> f32 {
        let bits = (self.next() >> 11) as f32;
        let t = bits / (1u64 << 53) as f32;
        lo + t * (hi - lo)
    }
}

/// Pick an XZ position in the 200 m square, excluding the 30 m origin keep-out.
fn random_xz(lcg: &mut Lcg) -> Vec2 {
    loop {
        let x = lcg.range_f32(-90.0, 90.0);
        let z = lcg.range_f32(-90.0, 90.0);
        if x * x + z * z > 30.0 * 30.0 {
            return Vec2::new(x, z);
        }
    }
}

// ---------------------------------------------------------------------------
// Toggle system: 8 key spawns / despawns all traffic trucks
// ---------------------------------------------------------------------------

fn toggle_with_8(
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<TrafficState>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    chassis_q: Query<Entity, With<TrafficChassis>>,
    wheel_q: Query<Entity, With<TrafficWheel>>,
    time: Res<Time>,
) {
    if !keys.just_pressed(KeyCode::Digit8) {
        return;
    }

    state.active = !state.active;

    if state.active {
        spawn_traffic_trucks(&mut commands, &mut meshes, &mut materials, time.elapsed_secs());
    } else {
        // Despawn all chassis (Bevy 0.18: despawn_recursive → despawn).
        // Collecting first to avoid borrow conflict with commands.
        let chassis_entities: Vec<Entity> = chassis_q.iter().collect();
        for entity in chassis_entities {
            commands.entity(entity).despawn();
        }
        // Despawn any orphaned wheel entities not caught above.
        let wheel_entities: Vec<Entity> = wheel_q.iter().collect();
        for entity in wheel_entities {
            commands.entity(entity).despawn();
        }
    }
}

fn spawn_traffic_trucks(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    elapsed: f32,
) {
    let chassis_mesh = meshes.add(Cuboid::new(
        CHASSIS_HALF.x * 2.0,
        CHASSIS_HALF.y * 2.0,
        CHASSIS_HALF.z * 2.0,
    ));
    let wheel_mesh = meshes.add(Cylinder::new(WHEEL_RADIUS, WHEEL_HALF_WIDTH * 2.0));
    let wheel_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.10, 0.10, 0.10),
        perceptual_roughness: 0.9,
        ..default()
    });
    let tire_rot = Quat::from_rotation_z(std::f32::consts::FRAC_PI_2);

    let mut lcg = Lcg::new((elapsed * 1000.0) as u64 ^ 0xDEAD_BEEF);

    for id in 0..TRUCK_COUNT {
        let spawn_xz = random_xz(&mut lcg);
        let terrain_y = terrain_height_at(spawn_xz.x, spawn_xz.y);
        let spawn_y = terrain_y + 2.0;

        let target_xz = random_xz(&mut lcg);

        let color = PALETTE[id as usize % PALETTE.len()];
        let chassis_mat = materials.add(StandardMaterial {
            base_color: color,
            perceptual_roughness: 0.6,
            ..default()
        });

        // Chassis
        let chassis_entity = commands.spawn((
            TrafficChassis {
                id,
                target_xz,
                idle_until: elapsed + 30.0,
            },
            Transform::from_translation(Vec3::new(spawn_xz.x, spawn_y, spawn_xz.y)),
            Visibility::default(),
            RigidBody::Dynamic,
            Collider::cuboid(CHASSIS_HALF.x, CHASSIS_HALF.y, CHASSIS_HALF.z),
            Mass(CHASSIS_MASS),
            LinearDamping(LINEAR_DAMPING),
            AngularDamping(ANGULAR_DAMPING),
            SleepingDisabled,
        )).id();

        // Body mesh child
        let body_mesh_entity = commands.spawn((
            Mesh3d(chassis_mesh.clone()),
            MeshMaterial3d(chassis_mat),
            Transform::IDENTITY,
        )).id();
        commands.entity(chassis_entity).add_child(body_mesh_entity);

        // 4 wheels
        for &offset in &WHEEL_OFFSETS {
            let wheel_entity = commands.spawn((
                TrafficWheel::new(offset),
                Mesh3d(wheel_mesh.clone()),
                MeshMaterial3d(wheel_mat.clone()),
                Transform::from_translation(offset).with_rotation(tire_rot),
            )).id();
            commands.entity(chassis_entity).add_child(wheel_entity);
        }
    }
}

// ---------------------------------------------------------------------------
// Pick a new waypoint for idle trucks
// ---------------------------------------------------------------------------

fn pick_target_for_idle(
    mut chassis_q: Query<
        (&Transform, &mut TrafficChassis),
        (Without<crate::vehicle::Chassis>, Without<crate::rival::RivalChassis>),
    >,
    time: Res<Time>,
) {
    let elapsed = time.elapsed_secs();
    let mut lcg = Lcg::new((elapsed * 10000.0) as u64 ^ 0xCAFE_BABE);

    for (transform, mut chassis) in chassis_q.iter_mut() {
        let pos_xz = Vec2::new(transform.translation.x, transform.translation.z);
        let dist = pos_xz.distance(chassis.target_xz);

        let needs_new_target = dist < 6.0 || elapsed > chassis.idle_until;
        if needs_new_target {
            chassis.target_xz = random_xz(&mut lcg);
            chassis.idle_until = elapsed + 30.0;
        }
    }
}

// ---------------------------------------------------------------------------
// Traffic steering: compute drive + yaw forces toward current target
// ---------------------------------------------------------------------------

fn traffic_steering(
    mut chassis_q: Query<
        (Forces, &Transform, &Children, &TrafficChassis),
        (Without<crate::vehicle::Chassis>, Without<crate::rival::RivalChassis>),
    >,
    wheel_q: Query<&TrafficWheel>,
) {
    for (mut forces, transform, children, chassis) in chassis_q.iter_mut() {
        let pos = transform.translation;
        let chassis_rot = transform.rotation;
        let chassis_fwd = (chassis_rot * Vec3::NEG_Z).normalize();

        let target_world = Vec3::new(chassis.target_xz.x, pos.y, chassis.target_xz.y);
        let to_target = (target_world - pos).normalize_or_zero();

        // Signed turn: cross product Y component gives left/right steer signal.
        let cross_y = chassis_fwd.cross(to_target).y;
        // Steer moment: apply opposing lateral yaw impulse at front axle anchors.
        // We just scale the drive force direction — actual turning comes from
        // differential application at front vs rear wheels in a simplified way:
        // drive force is applied along a steered forward direction.
        let steer_angle = cross_y.clamp(-1.0, 1.0) * 25_f32.to_radians();
        let chassis_up = (chassis_rot * Vec3::Y).normalize();
        let steer_fwd = (Quat::from_axis_angle(chassis_up, steer_angle) * chassis_fwd).normalize();

        // Count grounded wheels for force scaling.
        let wheel_children: Vec<Entity> = children.iter().collect();
        let mut grounded_count = 0u32;
        for &child in &wheel_children {
            if let Ok(wheel) = wheel_q.get(child) {
                if wheel.is_grounded {
                    grounded_count += 1;
                }
            }
        }
        if grounded_count == 0 {
            continue;
        }

        // Apply drive force at chassis centre — suspension handles traction
        // transfer per-wheel; this keeps the system simple.
        let drive_total = THROTTLE * DRIVE_FORCE_PER_WHEEL * grounded_count as f32;
        forces.apply_force(steer_fwd * drive_total);

        // Lateral grip at chassis level: resist sideways sliding.
        let lin_vel = forces.linear_velocity();
        let right = chassis_fwd.cross(Vec3::Y).normalize_or_zero();
        let v_lat = lin_vel.dot(right);
        let f_lat = -LATERAL_GRIP * v_lat * grounded_count as f32;
        forces.apply_force(right * f_lat);
    }
}

// ---------------------------------------------------------------------------
// Traffic suspension (PhysicsSchedule — parallel to rival.rs and vehicle.rs)
//
// For each traffic chassis:
//   1. Raycast down from each wheel mount.
//   2. Spring + damper + lateral grip force at contact point.
//   3. Anti-roll and anti-pitch couples.
//   4. Write is_grounded / current_compression / ground_normal back to TrafficWheel.
//
// Queries With<TrafficChassis> so this never touches player or rivals.
// ---------------------------------------------------------------------------

fn traffic_suspension(
    mut chassis_q: Query<
        (Entity, Forces, &Transform, &Children),
        (With<TrafficChassis>,
         Without<crate::vehicle::Chassis>,
         Without<crate::rival::RivalChassis>),
    >,
    mut wheel_q: Query<
        (&mut TrafficWheel, &mut Transform),
        (Without<TrafficChassis>,
         Without<crate::vehicle::Chassis>,
         Without<crate::rival::RivalChassis>),
    >,
    spatial: SpatialQuery,
) {
    for (chassis_entity, mut forces, chassis_transform, children) in chassis_q.iter_mut() {
        let chassis_pos = chassis_transform.translation;
        let chassis_rot = chassis_transform.rotation;
        let lin_vel = forces.linear_velocity();
        let ang_vel = forces.angular_velocity();

        let filter = SpatialQueryFilter::from_excluded_entities([chassis_entity]);

        let mut compressions  = [0.0_f32; 4];
        let mut world_anchors = [Vec3::ZERO; 4];
        let mut normals       = [Vec3::Y; 4];
        let mut contacts      = [false; 4];

        // Collect wheel children (up to 4, in spawn order = WHEEL_OFFSETS order).
        let wheel_children: Vec<Entity> = children.iter().collect();

        let mut wheel_idx = 0usize;
        for &child in &wheel_children {
            let Ok((wheel, _)) = wheel_q.get(child) else { continue };
            if wheel_idx >= 4 { break; }

            let local_anchor = wheel.local_pos;
            let world_anchor = chassis_pos + chassis_rot * local_anchor;
            world_anchors[wheel_idx] = world_anchor;

            if let Some(hit) = spatial.cast_ray(
                world_anchor, Dir3::NEG_Y, RAY_LEN, true, &filter,
            ) {
                let c = (SUSPENSION_LEN - hit.distance).max(0.0);
                compressions[wheel_idx] = c;
                normals[wheel_idx] = Vec3::new(hit.normal.x, hit.normal.y, hit.normal.z);
                contacts[wheel_idx] = c > 0.0;
            }

            wheel_idx += 1;
        }

        // Write state back to TrafficWheel components + update visual translation.
        let mut w_idx = 0usize;
        for &child in &wheel_children {
            let Ok((mut wheel, mut w_transform)) = wheel_q.get_mut(child) else { continue };
            if w_idx >= 4 { break; }

            wheel.current_compression = compressions[w_idx];
            wheel.is_grounded         = contacts[w_idx];
            wheel.ground_normal       = normals[w_idx];

            let base  = wheel.local_pos;
            let delta = Vec3::new(0.0, -compressions[w_idx], 0.0);
            w_transform.translation = base + delta;

            w_idx += 1;
        }

        // Force application pass.
        for i in 0..4 {
            if !contacts[i] { continue; }

            let world_anchor = world_anchors[i];
            let normal       = normals[i];
            let r            = world_anchor - chassis_pos;
            let v_anchor     = lin_vel + ang_vel.cross(r);
            let comp_vel     = -v_anchor.dot(normal);

            // Spring + damper.
            let f_damp = DAMPING_C * comp_vel.clamp(-10.0, 10.0);
            let f_susp = (SPRING_K * compressions[i] + f_damp).max(0.0);

            forces.apply_force_at_point(normal * f_susp, world_anchor);

            // Lateral grip: resist sideways sliding.
            let chassis_fwd  = (chassis_rot * Vec3::NEG_Z).normalize();
            let fwd_ground   = (chassis_fwd - chassis_fwd.dot(normal) * normal).normalize_or_zero();
            let right_ground = fwd_ground.cross(normal).normalize_or_zero();

            let v_lat = v_anchor.dot(right_ground);
            let f_lat = (-LATERAL_GRIP * v_lat).clamp(-f_susp * 1.2, f_susp * 1.2);
            forces.apply_force_at_point(right_ground * f_lat, world_anchor);

            // Rolling resistance along forward direction.
            let v_long = v_anchor.dot(fwd_ground);
            let f_roll = (-LATERAL_GRIP * 0.15 * v_long).clamp(-f_susp * 0.3, f_susp * 0.3);
            forces.apply_force_at_point(fwd_ground * f_roll, world_anchor);
        }

        // Anti-roll: resist left/right differential compression per axle.
        for &(l, r) in &[(0usize, 1usize), (2usize, 3usize)] {
            if !contacts[l] && !contacts[r] { continue; }
            let arb = ANTI_ROLL_K * (compressions[l] - compressions[r]);
            forces.apply_force_at_point(Vec3::Y *   arb,  world_anchors[l]);
            forces.apply_force_at_point(Vec3::Y * (-arb), world_anchors[r]);
        }

        // Anti-pitch: resist front/rear differential compression.
        let front_avg  = 0.5 * (compressions[0] + compressions[1]);
        let rear_avg   = 0.5 * (compressions[2] + compressions[3]);
        let any_front  = contacts[0] || contacts[1];
        let any_rear   = contacts[2] || contacts[3];
        if any_front && any_rear {
            let pitch = ANTI_PITCH_K * (front_avg - rear_avg);
            forces.apply_force_at_point(Vec3::Y *  pitch, world_anchors[0]);
            forces.apply_force_at_point(Vec3::Y *  pitch, world_anchors[1]);
            forces.apply_force_at_point(Vec3::Y * -pitch, world_anchors[2]);
            forces.apply_force_at_point(Vec3::Y * -pitch, world_anchors[3]);
        }
    }
}
