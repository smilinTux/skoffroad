// AI driver controller: turns PathFollower lookahead into throttle/steer
// forces applied to a rival chassis. Mirrors vehicle.rs suspension logic
// but reads its own AiDriver component for input rather than DriveInput.
//
// Public API:
//   AiDriverPlugin
//   AiDriver { skill: 0..1, max_speed_mps, throttle_gain, steer_gain }

use bevy::prelude::*;
use avian3d::prelude::*;

use crate::ai_path::{PathFollower, RacePath};
use crate::rival::{RivalChassis, RivalWheel};
use crate::vehicle::Chassis;

// Drive force per wheel mirrors vehicle.rs DRIVE_FORCE_PER_WHEEL (1800 N).
const AI_DRIVE_FORCE_PER_WHEEL: f32 = 1800.0;
// Lateral grip coefficient mirrors vehicle.rs LATERAL_GRIP (8000 N·s/m).
const AI_LATERAL_GRIP: f32 = 8_000.0;
// Maximum steering angle in radians, mirrors vehicle.rs MAX_STEER_ANGLE (30°).
const AI_MAX_STEER_ANGLE: f32 = 30_f32 * std::f32::consts::PI / 180.0;

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct AiDriverPlugin;

impl Plugin for AiDriverPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PhysicsSchedule,
            ai_driver_system
                .after(PhysicsStepSystems::NarrowPhase)
                .before(PhysicsStepSystems::Solver),
        );
    }
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

#[derive(Component, Clone)]
pub struct AiDriver {
    pub skill: f32,
    pub max_speed_mps: f32,
    pub throttle_gain: f32,
    pub steer_gain: f32,
}

impl Default for AiDriver {
    fn default() -> Self {
        Self {
            skill: 0.7,
            max_speed_mps: 14.0,
            throttle_gain: 1.0,
            steer_gain: 1.5,
        }
    }
}

// ---------------------------------------------------------------------------
// System
// ---------------------------------------------------------------------------

fn ai_driver_system(
    race_path: Option<Res<RacePath>>,
    time: Res<Time>,
    mut chassis_q: Query<
        (
            Entity,
            Forces,
            &Transform,
            &AiDriver,
            &PathFollower,
        ),
        (With<RivalChassis>, Without<Chassis>),
    >,
    wheel_q: Query<(&RivalWheel, &ChildOf)>,
) {
    // If the race hasn't started yet and the path hasn't been inserted, bail.
    let Some(race_path) = race_path else { return };
    if race_path.waypoints.is_empty() {
        return;
    }

    let t = time.elapsed_secs();

    for (entity, mut forces, transform, driver, follower) in chassis_q.iter_mut() {
        let chassis_pos = transform.translation;
        let chassis_rot = transform.rotation;
        let chassis_fwd = (chassis_rot * Vec3::NEG_Z).normalize();
        let chassis_up  = (chassis_rot * Vec3::Y).normalize();

        // Current speed along chassis forward axis. Read velocity through Forces
        // (avoids B0001: Forces already has mutable LinearVelocity access).
        let lin_vel = forces.linear_velocity();
        let vel_v   = Vec3::new(lin_vel.x, lin_vel.y, lin_vel.z);
        let speed   = vel_v.dot(chassis_fwd);
        let speed_abs = vel_v.length();

        // --- 1. Lookahead point ------------------------------------------------
        // Walk the waypoint ring ahead of current_idx by (8 m + speed * 0.5 m).
        let lookahead_dist = 8.0 + speed_abs * 0.5;
        let target = path_lookahead(&race_path.waypoints, follower.current_idx, lookahead_dist, chassis_pos);

        // --- 2. Steering -------------------------------------------------------
        let to_target = target - chassis_pos;
        // Cross product of chassis forward and direction-to-target gives the
        // signed left/right error (positive = target is to the right).
        let cross = chassis_fwd.cross(to_target.normalize_or_zero());
        // Use the y-component of the cross product (the up-axis component) as
        // the signed angular error, then scale by steer_gain.
        let raw_angle_err = cross.y.clamp(-1.0, 1.0).asin();

        // Skill jitter: weaker AI weaves slightly. Phase offset by entity bits
        // so each rival has a distinct wobble.
        let entity_phase = (entity.to_bits() & 0xFF) as f32 * 0.1;
        let jitter = (1.0 - driver.skill) * ((t * 1.3 + entity_phase).sin());

        let steer_angle = (raw_angle_err * driver.steer_gain + jitter)
            .clamp(-AI_MAX_STEER_ANGLE, AI_MAX_STEER_ANGLE);

        // --- 3. Throttle -------------------------------------------------------
        // Slow into corners: target speed scales down proportional to angle error.
        let angle_err_norm = raw_angle_err.abs() / std::f32::consts::PI;
        let target_speed = driver.max_speed_mps * (1.0 - 0.5 * angle_err_norm);
        let throttle = ((target_speed - speed) * driver.throttle_gain).clamp(-1.0, 1.0);

        // --- 4. Collect wheels for this chassis --------------------------------
        let mut wheel_positions: Vec<Vec3> = Vec::new();
        for (rival_wheel, child_of) in &wheel_q {
            if child_of.parent() == entity {
                // Transform the local wheel position to world space.
                let world_pos = chassis_pos + chassis_rot * rival_wheel.local_pos;
                wheel_positions.push(world_pos);
            }
        }

        // Fallback: if no wheels are registered yet, use four canonical offsets
        // matching vehicle.rs so the AI doesn't stall on the first frame.
        if wheel_positions.is_empty() {
            for &local in &[
                Vec3::new(-1.1, -0.35, -1.4),
                Vec3::new( 1.1, -0.35, -1.4),
                Vec3::new(-1.1, -0.35,  1.4),
                Vec3::new( 1.1, -0.35,  1.4),
            ] {
                wheel_positions.push(chassis_pos + chassis_rot * local);
            }
        }

        let n_wheels = wheel_positions.len();

        // --- 5. Apply forces at each wheel position ----------------------------
        for (wi, &world_wheel) in wheel_positions.iter().enumerate() {
            // Ground normal assumed to be world Y (no terrain sampling here;
            // rival.rs handles suspension so we only add drive + steer forces).
            let normal = Vec3::Y;

            // Steering: front half of wheels (first n/2) get the steer rotation.
            let is_front = wi < (n_wheels / 2).max(1);
            let steer_fwd = if is_front {
                (Quat::from_axis_angle(chassis_up, steer_angle) * chassis_fwd).normalize()
            } else {
                chassis_fwd
            };

            let fwd_ground   = (steer_fwd - steer_fwd.dot(normal) * normal).normalize_or_zero();
            let right_ground = fwd_ground.cross(normal).normalize_or_zero();

            // Longitudinal drive / brake force.
            if throttle.abs() > 0.0 {
                let f_drive = throttle * AI_DRIVE_FORCE_PER_WHEEL;
                forces.apply_force_at_point(fwd_ground * f_drive, world_wheel);
            }

            // Lateral grip: cancel sideways slip at the wheel.
            let r        = world_wheel - chassis_pos;
            let ang_vel_v = forces.angular_velocity();
            let v_wheel   = vel_v + ang_vel_v.cross(r);
            let v_lat     = v_wheel.dot(right_ground);
            let f_lat     = (-AI_LATERAL_GRIP * v_lat).clamp(-AI_LATERAL_GRIP, AI_LATERAL_GRIP);
            forces.apply_force_at_point(right_ground * f_lat, world_wheel);
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Walk the waypoint ring forward from `start_idx` by `distance_m` metres,
/// returning the interpolated world position at that distance.
/// Falls back to the current waypoint if the path is empty.
fn path_lookahead(waypoints: &[Vec3], start_idx: usize, distance_m: f32, fallback: Vec3) -> Vec3 {
    let n = waypoints.len();
    if n == 0 {
        return fallback;
    }
    if n == 1 {
        return waypoints[0];
    }

    let mut remaining = distance_m.max(0.0);
    let mut idx = start_idx % n;

    loop {
        let next_idx = (idx + 1) % n;
        let seg_len  = waypoints[idx].distance(waypoints[next_idx]);
        if seg_len < 1e-6 {
            idx = next_idx;
            continue;
        }
        if remaining <= seg_len {
            let t = remaining / seg_len;
            return waypoints[idx].lerp(waypoints[next_idx], t);
        }
        remaining -= seg_len;
        idx = next_idx;
        // Safety: don't loop more than n times (full lap consumed means
        // we've walked the whole ring — just return the start waypoint).
        if idx == start_idx % n {
            break;
        }
    }

    waypoints[start_idx % n]
}
