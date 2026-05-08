// Rival vehicle spawning: 3 AI-controlled trucks placed alongside the
// player at the start gate. Each rival gets a chassis + 4 wheels with
// raycast suspension (parallel system to vehicle.rs — operates on
// RivalWheel so the two suspension systems never conflict on queries),
// an AiDriver component and a PathFollower so ai_driver can steer it.
//
// Public API (required by Sprint-15 contract):
//   RivalPlugin
//   Rival { id, name, color }
//   RivalChassis  — marker on chassis entity
//   RivalWheel    — per-wheel component (local_pos, is_grounded, …)
//   RivalRoots    — resource; list of chassis entities for race.rs / rival_hud.rs

use bevy::prelude::*;
use avian3d::prelude::*;
use crate::terrain::terrain_height_at;
use crate::ai_driver::AiDriver;
use crate::ai_path::PathFollower;

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct RivalPlugin;

impl Plugin for RivalPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RivalRoots>()
           .add_systems(Startup, spawn_rivals)
           .add_systems(
               PhysicsSchedule,
               rival_suspension_system
                   .after(PhysicsStepSystems::NarrowPhase)
                   .before(PhysicsStepSystems::Solver),
           );
    }
}

// ---------------------------------------------------------------------------
// Components & Resources
// ---------------------------------------------------------------------------

/// One row in the rival roster. Carried on each chassis entity.
#[derive(Component, Clone)]
pub struct Rival {
    pub id: u32,
    pub name: String,
    pub color: Color,
}

/// Marker on the chassis entity (the Avian RigidBody::Dynamic).
#[derive(Component)]
pub struct RivalChassis;

/// Per-wheel component for each of a rival's 4 wheels.
/// Fields mirror Wheel in vehicle.rs where semantically equivalent.
#[derive(Component)]
pub struct RivalWheel {
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

impl RivalWheel {
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

/// Resource that exposes chassis entity IDs to race.rs and rival_hud.rs.
#[derive(Resource, Default)]
pub struct RivalRoots {
    pub chassis_ids: Vec<Entity>,
}

// ---------------------------------------------------------------------------
// Physics constants  (tuned to match the player vehicle feel)
// ---------------------------------------------------------------------------

const CHASSIS_HALF: Vec3    = Vec3::new(1.0, 0.4, 2.0);
const CHASSIS_MASS: f32     = 1500.0;
const SUSPENSION_LEN: f32   = 0.60;
const SPRING_K: f32         = 50_000.0;
const DAMPING_C: f32        = 5_000.0;
const RAY_LEN: f32          = SUSPENSION_LEN + 0.5;
const LATERAL_GRIP: f32     = 8_000.0;
const ANTI_ROLL_K: f32      = 30_000.0;
const ANTI_PITCH_K: f32     = 30_000.0;
const LINEAR_DAMPING: f32   = 0.5;
const ANGULAR_DAMPING: f32  = 25.0;
const WHEEL_RADIUS: f32     = 0.35;
const WHEEL_HALF_WIDTH: f32 = 0.18;

// FL, FR, RL, RR wheel anchor offsets in chassis local space — same
// geometry as the player vehicle so the suspension behaviour matches.
const WHEEL_OFFSETS: [Vec3; 4] = [
    Vec3::new(-1.1, -0.35, -1.4), // FL
    Vec3::new( 1.1, -0.35, -1.4), // FR
    Vec3::new(-1.1, -0.35,  1.4), // RL
    Vec3::new( 1.1, -0.35,  1.4), // RR
];

// ---------------------------------------------------------------------------
// Rival definitions  (id, name, color, world-x, world-z, AiDriver params)
// ---------------------------------------------------------------------------

struct RivalDef {
    id: u32,
    name: &'static str,
    color: Color,
    world_x: f32,
    world_z: f32,
    skill: f32,
    max_speed_mps: f32,
}

const RIVALS: [RivalDef; 3] = [
    RivalDef {
        id: 1, name: "RED",
        color: Color::srgb(0.9, 0.2, 0.2),
        world_x: 3.0, world_z: -7.0,
        skill: 0.6, max_speed_mps: 12.0,
    },
    RivalDef {
        id: 2, name: "GRN",
        color: Color::srgb(0.2, 0.9, 0.2),
        world_x: 7.0, world_z: -7.0,
        skill: 0.75, max_speed_mps: 13.0,
    },
    RivalDef {
        id: 3, name: "BLU",
        color: Color::srgb(0.2, 0.4, 0.95),
        world_x: 5.0, world_z: -10.0,
        skill: 0.85, max_speed_mps: 14.0,
    },
];

// ---------------------------------------------------------------------------
// Spawn system
// ---------------------------------------------------------------------------

fn spawn_rivals(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut roots: ResMut<RivalRoots>,
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

    for def in &RIVALS {
        let terrain_y = terrain_height_at(def.world_x, def.world_z);
        let spawn_y = terrain_y + 2.0;

        // Per-rival coloured chassis material.
        let chassis_mat = materials.add(StandardMaterial {
            base_color: def.color,
            perceptual_roughness: 0.6,
            ..default()
        });

        // --- Chassis rigid body ------------------------------------------
        let chassis_id = commands.spawn((
            RivalChassis,
            Rival {
                id: def.id,
                name: def.name.to_string(),
                color: def.color,
            },
            AiDriver {
                skill: def.skill,
                max_speed_mps: def.max_speed_mps,
                throttle_gain: 1.0,
                steer_gain: 1.5,
            },
            PathFollower::default(),
            Transform::from_translation(Vec3::new(def.world_x, spawn_y, def.world_z)),
            Visibility::default(),
            RigidBody::Dynamic,
            Collider::cuboid(CHASSIS_HALF.x, CHASSIS_HALF.y, CHASSIS_HALF.z),
            Mass(CHASSIS_MASS),
            LinearDamping(LINEAR_DAMPING),
            AngularDamping(ANGULAR_DAMPING),
            SleepingDisabled,
        )).id();

        // --- Chassis body mesh child -------------------------------------
        let body_mesh_id = commands.spawn((
            Mesh3d(chassis_mesh.clone()),
            MeshMaterial3d(chassis_mat),
            Transform::IDENTITY,
        )).id();
        commands.entity(chassis_id).add_child(body_mesh_id);

        // --- 4 wheels (visual + RivalWheel suspension data) --------------
        for &offset in &WHEEL_OFFSETS {
            let wheel_id = commands.spawn((
                RivalWheel::new(offset),
                Mesh3d(wheel_mesh.clone()),
                MeshMaterial3d(wheel_mat.clone()),
                Transform::from_translation(offset).with_rotation(tire_rot),
            )).id();
            commands.entity(chassis_id).add_child(wheel_id);
        }

        roots.chassis_ids.push(chassis_id);
    }
}

// ---------------------------------------------------------------------------
// Rival suspension system  (PhysicsSchedule, parallel to vehicle.rs)
// ---------------------------------------------------------------------------
//
// For each rival chassis we:
//   1. Raycast downward from each wheel mount point.
//   2. Compute spring + damper force and apply it at the mount world pos.
//   3. Apply lateral grip force to resist sideways sliding.
//   4. Apply anti-roll and anti-pitch couples.
//   5. Update RivalWheel.is_grounded / current_compression / ground_normal.
//
// We query With<RivalChassis> so this never touches the player chassis, and
// vehicle.rs queries With<Chassis> so it never touches rival chassis —
// zero system ambiguity on the entity sets.

fn rival_suspension_system(
    roots: Option<Res<RivalRoots>>,
    mut chassis_q: Query<(Forces, &Transform, &Children), With<RivalChassis>>,
    mut wheel_q: Query<(&mut RivalWheel, &mut Transform), Without<RivalChassis>>,
    spatial: SpatialQuery,
) {
    let Some(roots) = roots else { return };

    for &chassis_entity in &roots.chassis_ids {
        let Ok((mut forces, chassis_transform, children)) =
            chassis_q.get_mut(chassis_entity) else { continue };

        let chassis_pos = chassis_transform.translation;
        let chassis_rot = chassis_transform.rotation;
        let lin_vel = forces.linear_velocity();
        let ang_vel = forces.angular_velocity();

        let filter = SpatialQueryFilter::from_excluded_entities([chassis_entity]);

        // Per-wheel raycast pass.
        let mut compressions  = [0.0_f32; 4];
        let mut world_anchors = [Vec3::ZERO; 4];
        let mut normals       = [Vec3::Y; 4];
        let mut contacts      = [false; 4];

        // Collect wheel children in local_pos order so index maps to WHEEL_OFFSETS.
        let wheel_children: Vec<Entity> = children.iter().collect();

        let mut wheel_idx = 0usize;
        for &child in &wheel_children {
            let Ok((wheel, _wheel_transform)) = wheel_q.get(child) else { continue };
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

        // Write compression + grounded state back to RivalWheel components.
        let mut w_idx = 0usize;
        for &child in &wheel_children {
            let Ok((mut wheel, mut w_transform)) = wheel_q.get_mut(child) else { continue };
            if w_idx >= 4 { break; }
            wheel.current_compression = compressions[w_idx];
            wheel.is_grounded = contacts[w_idx];
            wheel.ground_normal = normals[w_idx];

            // Visual: move wheel visual up/down with compression.
            let base = wheel.local_pos;
            let delta = Vec3::new(0.0, -compressions[w_idx], 0.0);
            w_transform.translation = base + delta;

            w_idx += 1;
        }

        // Force application pass.
        for i in 0..4 {
            if !contacts[i] { continue; }

            let world_anchor = world_anchors[i];
            let normal = normals[i];
            let r = world_anchor - chassis_pos;
            let v_anchor = lin_vel + ang_vel.cross(r);
            let comp_vel = -v_anchor.dot(normal);

            // Spring + damper (clamp damper velocity to avoid impulse spikes).
            let f_damp = DAMPING_C * comp_vel.clamp(-10.0, 10.0);
            let f_susp = (SPRING_K * compressions[i] + f_damp).max(0.0);

            forces.apply_force_at_point(normal * f_susp, world_anchor);

            // Lateral grip: resist sideways sliding of the chassis.
            let chassis_fwd   = (chassis_rot * Vec3::NEG_Z).normalize();
            let fwd_ground    = (chassis_fwd - chassis_fwd.dot(normal) * normal).normalize_or_zero();
            let right_ground  = fwd_ground.cross(normal).normalize_or_zero();

            let v_lat = v_anchor.dot(right_ground);
            let f_lat = (-LATERAL_GRIP * v_lat).clamp(-f_susp * 1.2, f_susp * 1.2);
            forces.apply_force_at_point(right_ground * f_lat, world_anchor);

            // Rolling resistance (light drag along forward direction when no AI
            // throttle is applied). AiDriver will add drive forces later.
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
        let front_avg = 0.5 * (compressions[0] + compressions[1]);
        let rear_avg  = 0.5 * (compressions[2] + compressions[3]);
        let any_front = contacts[0] || contacts[1];
        let any_rear  = contacts[2] || contacts[3];
        if any_front && any_rear {
            let pitch = ANTI_PITCH_K * (front_avg - rear_avg);
            forces.apply_force_at_point(Vec3::Y *  pitch, world_anchors[0]);
            forces.apply_force_at_point(Vec3::Y *  pitch, world_anchors[1]);
            forces.apply_force_at_point(Vec3::Y * -pitch, world_anchors[2]);
            forces.apply_force_at_point(Vec3::Y * -pitch, world_anchors[3]);
        }
    }
}
