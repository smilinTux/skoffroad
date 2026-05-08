// Pursuit mode: a single AI "cop" vehicle chases the player. Survive 60 s to
// escape. P key toggles pursuit mode (spawns/despawns the cop).
//
// Public API:
//   PursuitPlugin
//   PursuitState (resource)

use bevy::prelude::*;
use avian3d::prelude::*;

use crate::terrain::terrain_height_at;
use crate::vehicle::{Chassis, VehicleRoot};
use crate::ai_driver::AiDriver;
use crate::ai_path::PathFollower;

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct PursuitPlugin;

impl Plugin for PursuitPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PursuitState>()
           .add_systems(Startup, spawn_pursuit_hud)
           .add_systems(Update, (toggle_with_p, tick_pursuit, cop_steering, update_hud))
           .add_systems(
               PhysicsSchedule,
               cop_suspension
                   .after(PhysicsStepSystems::NarrowPhase)
                   .before(PhysicsStepSystems::Solver),
           );
    }
}

// ---------------------------------------------------------------------------
// Public resource
// ---------------------------------------------------------------------------

#[derive(Resource, Default)]
pub struct PursuitState {
    pub active: bool,
    pub elapsed_s: f32,
    pub distance_to_player_m: f32,
    pub caught: bool,
}

// ---------------------------------------------------------------------------
// Components
// ---------------------------------------------------------------------------

/// Marker on the cop chassis entity.
#[derive(Component)]
pub struct CopChassis;

/// Per-wheel component for the cop vehicle (mirrors RivalWheel).
#[derive(Component)]
pub struct CopWheel {
    /// Local-space mount position relative to the cop chassis origin.
    pub local_pos: Vec3,
    pub current_compression: f32,
    pub is_grounded: bool,
    pub ground_normal: Vec3,
}

impl CopWheel {
    fn new(local_pos: Vec3) -> Self {
        Self {
            local_pos,
            current_compression: 0.0,
            is_grounded: false,
            ground_normal: Vec3::Y,
        }
    }
}

/// Resource holding the spawned cop entity so we can despawn it later.
#[derive(Resource)]
struct CopRoot {
    chassis: Entity,
}

/// Marker on the pursuit HUD root node.
#[derive(Component)]
struct PursuitHudRoot;

/// Marker on the pursuit HUD text node.
#[derive(Component)]
struct PursuitHudText;

// ---------------------------------------------------------------------------
// Physics constants (match rival / vehicle tuning)
// ---------------------------------------------------------------------------

const CHASSIS_HALF: Vec3   = Vec3::new(1.0, 0.4, 2.0);
const CHASSIS_MASS: f32    = 1500.0;
const SUSPENSION_LEN: f32  = 0.60;
const RAY_LEN: f32         = SUSPENSION_LEN + 0.5;
const SPRING_K: f32        = 50_000.0;
const DAMPING_C: f32       = 5_000.0;
const LATERAL_GRIP: f32    = 8_000.0;
const ANTI_ROLL_K: f32     = 30_000.0;
const ANTI_PITCH_K: f32    = 30_000.0;
const LINEAR_DAMPING: f32  = 0.5;
const ANGULAR_DAMPING: f32 = 25.0;
const WHEEL_RADIUS: f32    = 0.35;
const WHEEL_HALF_WIDTH: f32 = 0.18;

/// Drive force per wheel applied by the cop steering controller.
const COP_DRIVE_FORCE_PER_WHEEL: f32 = 1800.0;
/// Fixed throttle fraction the cop uses when chasing the player.
const COP_THROTTLE: f32 = 0.9;

/// FL, FR, RL, RR — same geometry as player vehicle.
const WHEEL_OFFSETS: [Vec3; 4] = [
    Vec3::new(-1.1, -0.35, -1.4),
    Vec3::new( 1.1, -0.35, -1.4),
    Vec3::new(-1.1, -0.35,  1.4),
    Vec3::new( 1.1, -0.35,  1.4),
];

// ---------------------------------------------------------------------------
// HUD colors
// ---------------------------------------------------------------------------

const HUD_BG: Color   = Color::srgba(0.08, 0.08, 0.14, 0.85);
const HUD_TEXT: Color = Color::srgb(0.95, 0.75, 0.20);

// ---------------------------------------------------------------------------
// Startup: spawn the pursuit HUD (hidden by default)
// ---------------------------------------------------------------------------

fn spawn_pursuit_hud(mut commands: Commands) {
    let root = commands.spawn((
        PursuitHudRoot,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Percent(35.0),
            right: Val::Percent(35.0),
            top: Val::Px(10.0),
            width: Val::Auto,
            height: Val::Auto,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            padding: UiRect {
                left:   Val::Px(16.0),
                right:  Val::Px(16.0),
                top:    Val::Px(8.0),
                bottom: Val::Px(8.0),
            },
            ..default()
        },
        BackgroundColor(HUD_BG),
        Visibility::Hidden,
    )).id();

    let text = commands.spawn((
        PursuitHudText,
        Text::new("PURSUIT 00:60  GAP: --m"),
        TextFont { font_size: 22.0, ..default() },
        TextColor(HUD_TEXT),
    )).id();

    commands.entity(root).add_child(text);
}

// ---------------------------------------------------------------------------
// toggle_with_p: P just_pressed flips active; spawns / despawns cop.
// ---------------------------------------------------------------------------

fn toggle_with_p(
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<PursuitState>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    cop_root: Option<Res<CopRoot>>,
    player_root: Option<Res<VehicleRoot>>,
    player_q: Query<&Transform, With<Chassis>>,
) {
    if !keys.just_pressed(KeyCode::KeyP) {
        return;
    }

    if state.active {
        // --- Deactivate -------------------------------------------------------
        state.active = false;
        state.elapsed_s = 0.0;
        state.distance_to_player_m = 0.0;
        state.caught = false;

        if let Some(cop) = cop_root {
            commands.entity(cop.chassis).despawn();
            commands.remove_resource::<CopRoot>();
        }
    } else {
        // --- Activate ---------------------------------------------------------
        state.active = true;
        state.elapsed_s = 0.0;
        state.caught = false;

        // Determine spawn position: 30 m behind player along chassis -Z.
        let spawn_pos = if let Some(player) = player_root {
            if let Ok(player_tf) = player_q.get(player.chassis) {
                let fwd = (player_tf.rotation * Vec3::NEG_Z).normalize();
                // 30 m behind = player pos - 30 * fwd
                let behind = player_tf.translation - fwd * 30.0;
                let y = terrain_height_at(behind.x, behind.z) + 2.0;
                Vec3::new(behind.x, y, behind.z)
            } else {
                Vec3::new(0.0, terrain_height_at(0.0, 30.0) + 2.0, 30.0)
            }
        } else {
            Vec3::new(0.0, terrain_height_at(0.0, 30.0) + 2.0, 30.0)
        };

        // Cop chassis mesh + material.
        let chassis_mesh = meshes.add(Cuboid::new(
            CHASSIS_HALF.x * 2.0,
            CHASSIS_HALF.y * 2.0,
            CHASSIS_HALF.z * 2.0,
        ));
        let cop_mat = materials.add(StandardMaterial {
            base_color: Color::srgb(0.1, 0.1, 0.15),
            perceptual_roughness: 0.7,
            ..default()
        });
        let wheel_mesh = meshes.add(Cylinder::new(WHEEL_RADIUS, WHEEL_HALF_WIDTH * 2.0));
        let wheel_mat = materials.add(StandardMaterial {
            base_color: Color::srgb(0.10, 0.10, 0.10),
            perceptual_roughness: 0.9,
            ..default()
        });

        let chassis_id = commands.spawn((
            CopChassis,
            AiDriver {
                skill: 0.8,
                max_speed_mps: 15.0,
                throttle_gain: 1.5,
                steer_gain: 2.0,
            },
            PathFollower::default(),
            Transform::from_translation(spawn_pos),
            Visibility::default(),
            RigidBody::Dynamic,
            Collider::cuboid(CHASSIS_HALF.x, CHASSIS_HALF.y, CHASSIS_HALF.z),
            Mass(CHASSIS_MASS),
            LinearDamping(LINEAR_DAMPING),
            AngularDamping(ANGULAR_DAMPING),
            SleepingDisabled,
        )).id();

        // Body mesh child.
        let body_id = commands.spawn((
            Mesh3d(chassis_mesh),
            MeshMaterial3d(cop_mat),
            Transform::IDENTITY,
        )).id();
        commands.entity(chassis_id).add_child(body_id);

        // 4 wheel children.
        let tire_rot = Quat::from_rotation_z(std::f32::consts::FRAC_PI_2);
        for &offset in &WHEEL_OFFSETS {
            let wheel_id = commands.spawn((
                CopWheel::new(offset),
                Mesh3d(wheel_mesh.clone()),
                MeshMaterial3d(wheel_mat.clone()),
                Transform::from_translation(offset).with_rotation(tire_rot),
            )).id();
            commands.entity(chassis_id).add_child(wheel_id);
        }

        commands.insert_resource(CopRoot { chassis: chassis_id });
    }
}

// ---------------------------------------------------------------------------
// tick_pursuit: update elapsed, distance, win/lose checks.
// ---------------------------------------------------------------------------

fn tick_pursuit(
    mut state: ResMut<PursuitState>,
    time: Res<Time>,
    cop_root: Option<Res<CopRoot>>,
    player_root: Option<Res<VehicleRoot>>,
    cop_q: Query<&Transform, With<CopChassis>>,
    player_q: Query<&Transform, (With<Chassis>, Without<CopChassis>)>,
    mut close_time: Local<f32>,
) {
    if !state.active || state.caught {
        *close_time = 0.0;
        return;
    }

    let dt = time.delta_secs();
    state.elapsed_s += dt;

    // --- Compute gap ----------------------------------------------------------
    let distance = match (cop_root.as_ref(), player_root.as_ref()) {
        (Some(cop), Some(player)) => {
            match (cop_q.get(cop.chassis), player_q.get(player.chassis)) {
                (Ok(cop_tf), Ok(player_tf)) => {
                    cop_tf.translation.distance(player_tf.translation)
                }
                _ => f32::MAX,
            }
        }
        _ => f32::MAX,
    };

    state.distance_to_player_m = distance;

    // --- Close-proximity timer ------------------------------------------------
    if distance < 5.0 {
        *close_time += dt;
    } else {
        *close_time = (*close_time - dt * 0.5).max(0.0);
    }

    if *close_time > 2.0 {
        state.caught = true;
        info!("PURSUIT: CAUGHT after {:.1}s", state.elapsed_s);
        state.active = false;
        *close_time = 0.0;
        return;
    }

    // --- Escape condition -----------------------------------------------------
    if state.elapsed_s >= 60.0 {
        info!("PURSUIT: escaped!");
        state.active = false;
        *close_time = 0.0;
    }
}

// ---------------------------------------------------------------------------
// cop_steering: cross-product steer toward player, throttle 0.9.
// ---------------------------------------------------------------------------

fn cop_steering(
    state: Res<PursuitState>,
    cop_root: Option<Res<CopRoot>>,
    player_root: Option<Res<VehicleRoot>>,
    player_q: Query<&Transform, (With<Chassis>, Without<CopChassis>)>,
    mut cop_q: Query<(Forces, &Transform, &Children), With<CopChassis>>,
    wheel_q: Query<(&CopWheel, &ChildOf)>,
) {
    if !state.active {
        return;
    }

    let (Some(cop), Some(player)) = (cop_root.as_ref(), player_root.as_ref()) else {
        return;
    };

    let Ok(player_tf) = player_q.get(player.chassis) else { return };
    let Ok((mut forces, cop_tf, children)) = cop_q.get_mut(cop.chassis) else { return };

    let cop_pos    = cop_tf.translation;
    let cop_rot    = cop_tf.rotation;
    let cop_fwd    = (cop_rot * Vec3::NEG_Z).normalize();
    let cop_up     = (cop_rot * Vec3::Y).normalize();
    let player_pos = player_tf.translation;

    // Direction vector to player (projected flat for steering computation).
    let to_player   = (player_pos - cop_pos).normalize_or_zero();
    // Cross product gives signed steer: positive y → player is to the right.
    let cross       = cop_fwd.cross(to_player);
    let signed_steer = cross.dot(cop_up).clamp(-1.0, 1.0);

    // Maximum steer angle (30°).
    let max_steer = 30_f32.to_radians();
    let steer_angle = signed_steer * max_steer;

    // Collect cop wheel world positions.
    let chassis_entity = cop.chassis;
    let mut wheel_positions: Vec<(Vec3, bool)> = Vec::new();
    for (cop_wheel, child_of) in &wheel_q {
        if child_of.parent() == chassis_entity {
            let world_pos = cop_pos + cop_rot * cop_wheel.local_pos;
            let is_front = cop_wheel.local_pos.z < 0.0;
            wheel_positions.push((world_pos, is_front));
        }
    }

    // Fallback to canonical offsets if wheels not yet registered.
    if wheel_positions.is_empty() {
        for &local in &WHEEL_OFFSETS {
            let world_pos = cop_pos + cop_rot * local;
            let is_front = local.z < 0.0;
            wheel_positions.push((world_pos, is_front));
        }
    }

    let vel_v    = forces.linear_velocity();
    let ang_vel  = forces.angular_velocity();

    for (world_wheel, is_front) in &wheel_positions {
        let normal = Vec3::Y;

        // Steering: front wheels get steer rotation.
        let steer_fwd = if *is_front {
            (Quat::from_axis_angle(cop_up, steer_angle) * cop_fwd).normalize()
        } else {
            cop_fwd
        };

        let fwd_ground   = (steer_fwd - steer_fwd.dot(normal) * normal).normalize_or_zero();
        let right_ground = fwd_ground.cross(normal).normalize_or_zero();

        // Drive force toward player.
        let f_drive = COP_THROTTLE * COP_DRIVE_FORCE_PER_WHEEL;
        forces.apply_force_at_point(fwd_ground * f_drive, *world_wheel);

        // Lateral grip.
        let r       = *world_wheel - cop_pos;
        let v_wheel = vel_v + ang_vel.cross(r);
        let v_lat   = v_wheel.dot(right_ground);
        let f_lat   = (-LATERAL_GRIP * v_lat).clamp(-LATERAL_GRIP, LATERAL_GRIP);
        forces.apply_force_at_point(right_ground * f_lat, *world_wheel);
    }

    // Drop children borrow — prevent unused-variable warning.
    let _ = children;
}

// ---------------------------------------------------------------------------
// cop_suspension: spring + damper + lateral grip per wheel.
// ---------------------------------------------------------------------------

fn cop_suspension(
    cop_root: Option<Res<CopRoot>>,
    mut cop_q: Query<(Forces, &Transform, &Children), With<CopChassis>>,
    mut wheel_q: Query<(&mut CopWheel, &mut Transform), Without<CopChassis>>,
    spatial: SpatialQuery,
) {
    let Some(cop) = cop_root else { return };

    let Ok((mut forces, cop_tf, children)) = cop_q.get_mut(cop.chassis) else { return };

    let chassis_pos = cop_tf.translation;
    let chassis_rot = cop_tf.rotation;
    let lin_vel = forces.linear_velocity();
    let ang_vel = forces.angular_velocity();

    let filter = SpatialQueryFilter::from_excluded_entities([cop.chassis]);

    let mut compressions  = [0.0_f32; 4];
    let mut world_anchors = [Vec3::ZERO; 4];
    let mut normals       = [Vec3::Y; 4];
    let mut contacts      = [false; 4];

    let wheel_children: Vec<Entity> = children.iter().collect();

    // --- Raycast pass ---------------------------------------------------------
    let mut wheel_idx = 0usize;
    for &child in &wheel_children {
        let Ok((wheel, _)) = wheel_q.get(child) else { continue };
        if wheel_idx >= 4 { break; }

        let local_anchor = wheel.local_pos;
        let world_anchor = chassis_pos + chassis_rot * local_anchor;
        world_anchors[wheel_idx] = world_anchor;

        if let Some(hit) = spatial.cast_ray(world_anchor, Dir3::NEG_Y, RAY_LEN, true, &filter) {
            let c = (SUSPENSION_LEN - hit.distance).max(0.0);
            compressions[wheel_idx] = c;
            normals[wheel_idx] = Vec3::new(hit.normal.x, hit.normal.y, hit.normal.z);
            contacts[wheel_idx] = c > 0.0;
        }

        wheel_idx += 1;
    }

    // --- Write state back to CopWheel components + visual update -------------
    let mut w_idx = 0usize;
    for &child in &wheel_children {
        let Ok((mut wheel, mut w_tf)) = wheel_q.get_mut(child) else { continue };
        if w_idx >= 4 { break; }
        wheel.current_compression = compressions[w_idx];
        wheel.is_grounded = contacts[w_idx];
        wheel.ground_normal = normals[w_idx];

        let base  = wheel.local_pos;
        let delta = Vec3::new(0.0, -compressions[w_idx], 0.0);
        w_tf.translation = base + delta;

        w_idx += 1;
    }

    // --- Force application pass -----------------------------------------------
    for i in 0..4 {
        if !contacts[i] { continue; }

        let world_anchor = world_anchors[i];
        let normal = normals[i];
        let r = world_anchor - chassis_pos;
        let v_anchor = lin_vel + ang_vel.cross(r);
        let comp_vel = -v_anchor.dot(normal);

        let f_damp = DAMPING_C * comp_vel.clamp(-10.0, 10.0);
        let f_susp = (SPRING_K * compressions[i] + f_damp).max(0.0);

        forces.apply_force_at_point(normal * f_susp, world_anchor);

        let chassis_fwd  = (chassis_rot * Vec3::NEG_Z).normalize();
        let fwd_ground   = (chassis_fwd - chassis_fwd.dot(normal) * normal).normalize_or_zero();
        let right_ground = fwd_ground.cross(normal).normalize_or_zero();

        let v_lat = v_anchor.dot(right_ground);
        let f_lat = (-LATERAL_GRIP * v_lat).clamp(-f_susp * 1.2, f_susp * 1.2);
        forces.apply_force_at_point(right_ground * f_lat, world_anchor);

        // Light rolling resistance.
        let v_long = v_anchor.dot(fwd_ground);
        let f_roll = (-LATERAL_GRIP * 0.15 * v_long).clamp(-f_susp * 0.3, f_susp * 0.3);
        forces.apply_force_at_point(fwd_ground * f_roll, world_anchor);
    }

    // Anti-roll per axle.
    for &(l, r) in &[(0usize, 1usize), (2usize, 3usize)] {
        if !contacts[l] && !contacts[r] { continue; }
        let arb = ANTI_ROLL_K * (compressions[l] - compressions[r]);
        forces.apply_force_at_point(Vec3::Y *   arb,  world_anchors[l]);
        forces.apply_force_at_point(Vec3::Y * (-arb), world_anchors[r]);
    }

    // Anti-pitch.
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

// ---------------------------------------------------------------------------
// update_hud: show/hide panel, refresh text.
// ---------------------------------------------------------------------------

fn update_hud(
    state: Res<PursuitState>,
    mut hud_root_q: Query<&mut Visibility, With<PursuitHudRoot>>,
    mut hud_text_q: Query<&mut Text, With<PursuitHudText>>,
) {
    let Ok(mut vis) = hud_root_q.single_mut() else { return };
    let Ok(mut text) = hud_text_q.single_mut() else { return };

    if state.active {
        *vis = Visibility::Visible;

        let remaining = (60.0 - state.elapsed_s).max(0.0) as u32;
        let secs = remaining % 60;
        let mins = remaining / 60;
        let gap_m = state.distance_to_player_m as u32;

        text.0 = format!("PURSUIT {:02}:{:02}  GAP: {}m", mins, secs, gap_m);
    } else {
        *vis = Visibility::Hidden;
    }
}
