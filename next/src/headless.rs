// Headless simulation harness.
// Runs the physics-only app (no window, no renderer) and collects telemetry.
//
// Plugin stack based on Avian's own test helpers:
//   MinimalPlugins + AssetPlugin + MeshPlugin + TransformPlugin + PhysicsPlugins
// MeshPlugin is required because ColliderConstructor::TrimeshFromMesh resolves
// mesh assets via the asset server; without it the collider never builds.

use std::time::Duration;
use bevy::{
    asset::AssetPlugin,
    // MeshPlugin lives in bevy_mesh (re-exported as bevy::mesh in Bevy 0.18).
    // It registers the Mesh asset type, which is required for
    // ColliderConstructor::TrimeshFromMesh to resolve handles at runtime.
    mesh::MeshPlugin,
    scene::ScenePlugin,
    time::TimeUpdateStrategy,
    prelude::*,
};
use avian3d::prelude::*;
use serde::{Deserialize, Serialize};

use crate::terrain::{TerrainPlugin, terrain_height_at};
use crate::vehicle::{DriveInput, VehiclePluginHeadless, VehicleRoot};

// ---------------------------------------------------------------------------
// Public API types
// ---------------------------------------------------------------------------

pub enum Scenario {
    Idle,
    Forward,
    Reverse,
    Left,
    Right,
    BrakeTest { accel_s: f32 },
    Script(Vec<ScriptStep>),
}

pub struct ScriptStep {
    pub at_seconds: f32,
    pub drive: f32,
    pub steer: f32,
    pub brake: bool,
}

#[derive(Serialize, Deserialize)]
pub struct TelemetrySummary {
    pub scenario_name: String,
    pub duration_s: f32,
    pub ticks: u32,
    pub start_pos: [f32; 3],
    pub end_pos: [f32; 3],
    pub displacement: [f32; 3],
    pub distance_traveled_m: f32,
    pub max_speed_mps: f32,
    pub mean_speed_mps: f32,
    pub max_tilt_deg: f32,
    pub did_flip: bool,
    pub final_chassis_height: f32,
    pub final_chassis_above_terrain: f32,
    pub samples: Vec<TelemetrySample>,
}

#[derive(Serialize, Deserialize)]
pub struct TelemetrySample {
    pub t_seconds: f32,
    pub pos: [f32; 3],
    pub speed_mps: f32,
    pub tilt_deg: f32,
}

// ---------------------------------------------------------------------------
// run_scenario
// ---------------------------------------------------------------------------

pub fn run_scenario(scenario: Scenario, duration_s: f32) -> TelemetrySummary {
    let scenario_name = scenario_label(&scenario);
    let total_ticks = (duration_s * 60.0) as u32;

    let mut app = App::new();
    // Plugin stack mirrors Avian's own src/tests/mod.rs:create_app — anything
    // less and PhysicsPlugins panics on a missing resource at first tick.
    app.add_plugins((
        MinimalPlugins,
        TransformPlugin,
        PhysicsPlugins::default(),
        AssetPlugin::default(),
        MeshPlugin,
        ScenePlugin,
    ))
    .init_asset::<StandardMaterial>()
    .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(1.0 / 60.0)))
    .add_plugins((TerrainPlugin, VehiclePluginHeadless));

    // Required for plugins that defer setup work to their finish() phase.
    // app.run() does this automatically; manual app.update() callers must do it.
    app.finish();

    // Warm-up: run enough ticks for Startup systems to fire and asset commands
    // (mesh insertions, collider construction) to settle.
    for _ in 0..3 {
        app.update();
    }

    // --- telemetry state ---
    let mut start_pos = [0.0_f32; 3];
    let mut prev_pos = Vec3::ZERO;
    let mut distance_traveled = 0.0_f32;
    let mut max_speed = 0.0_f32;
    let mut speed_sum = 0.0_f32;
    let mut max_tilt = 0.0_f32;
    let mut samples: Vec<TelemetrySample> = Vec::new();
    let mut initialized = false;

    for tick in 0..total_ticks {
        let t_s = tick as f32 / 60.0;

        // --- write DriveInput for this tick ---
        {
            let input = compute_input(&scenario, t_s);
            if let Some(mut res) = app.world_mut().get_resource_mut::<DriveInput>() {
                res.drive = input.0;
                res.steer = input.1;
                res.brake = input.2;
            }
        }

        app.update();

        // --- read chassis state ---
        let (pos, vel, tilt_deg) = read_chassis(&mut app);

        if !initialized {
            start_pos = [pos.x, pos.y, pos.z];
            prev_pos = pos;
            initialized = true;
        }

        let step = pos.distance(prev_pos);
        distance_traveled += step;
        prev_pos = pos;

        let speed = vel.length();
        if speed > max_speed { max_speed = speed; }
        speed_sum += speed;

        if tilt_deg > max_tilt { max_tilt = tilt_deg; }

        // sample once per simulated second
        if tick % 60 == 0 {
            samples.push(TelemetrySample {
                t_seconds: t_s,
                pos: [pos.x, pos.y, pos.z],
                speed_mps: speed,
                tilt_deg,
            });
        }
    }

    let end_pos = if initialized {
        let (p, _, _) = read_chassis(&mut app);
        [p.x, p.y, p.z]
    } else {
        start_pos
    };

    let displacement = [
        end_pos[0] - start_pos[0],
        end_pos[1] - start_pos[1],
        end_pos[2] - start_pos[2],
    ];

    let mean_speed = if total_ticks > 0 {
        speed_sum / total_ticks as f32
    } else {
        0.0
    };

    let terrain_h = terrain_height_at(end_pos[0], end_pos[2]);
    let final_chassis_above_terrain = end_pos[1] - terrain_h;

    TelemetrySummary {
        scenario_name,
        duration_s,
        ticks: total_ticks,
        start_pos,
        end_pos,
        displacement,
        distance_traveled_m: distance_traveled,
        max_speed_mps: max_speed,
        mean_speed_mps: mean_speed,
        max_tilt_deg: max_tilt,
        did_flip: max_tilt > 90.0,
        final_chassis_height: end_pos[1],
        final_chassis_above_terrain,
        samples,
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn scenario_label(s: &Scenario) -> String {
    match s {
        Scenario::Idle          => "idle".into(),
        Scenario::Forward       => "forward".into(),
        Scenario::Reverse       => "reverse".into(),
        Scenario::Left          => "left".into(),
        Scenario::Right         => "right".into(),
        Scenario::BrakeTest { accel_s } => format!("brake-test(accel={}s)", accel_s),
        Scenario::Script(_)     => "script".into(),
    }
}

/// Returns (drive, steer, brake) for the given tick time.
fn compute_input(scenario: &Scenario, t_s: f32) -> (f32, f32, bool) {
    match scenario {
        Scenario::Idle          => (0.0,  0.0, false),
        Scenario::Forward       => (1.0,  0.0, false),
        Scenario::Reverse       => (-1.0, 0.0, false),
        Scenario::Left          => (1.0,  1.0, false),
        Scenario::Right         => (1.0, -1.0, false),
        Scenario::BrakeTest { accel_s } => {
            if t_s < *accel_s {
                (1.0, 0.0, false)
            } else {
                (0.0, 0.0, true)
            }
        }
        Scenario::Script(steps) => {
            // Find the last step whose at_seconds <= t_s.
            let mut drive = 0.0_f32;
            let mut steer = 0.0_f32;
            let mut brake = false;
            for step in steps {
                if step.at_seconds <= t_s {
                    drive = step.drive;
                    steer = step.steer;
                    brake = step.brake;
                } else {
                    break;
                }
            }
            (drive, steer, brake)
        }
    }
}

/// Reads Transform and LinearVelocity from the chassis entity.
/// Returns (position, velocity, tilt_deg).
fn read_chassis(app: &mut App) -> (Vec3, Vec3, f32) {
    let world = app.world_mut();

    // Find VehicleRoot resource to get the chassis entity id.
    let chassis_entity = world
        .get_resource::<VehicleRoot>()
        .map(|r| r.chassis);

    let Some(entity) = chassis_entity else {
        return (Vec3::ZERO, Vec3::ZERO, 0.0);
    };

    let pos = world
        .get::<Transform>(entity)
        .map(|t| t.translation)
        .unwrap_or(Vec3::ZERO);

    let vel = world
        .get::<LinearVelocity>(entity)
        .map(|v| Vec3::new(v.x, v.y, v.z))
        .unwrap_or(Vec3::ZERO);

    let tilt_deg = world
        .get::<Transform>(entity)
        .map(|t| {
            let up = t.rotation * Vec3::Y;
            up.angle_between(Vec3::Y).to_degrees()
        })
        .unwrap_or(0.0);

    (pos, vel, tilt_deg)
}
