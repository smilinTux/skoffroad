// Heat haze: shimmery transparent quad above the engine bay that pulses
// alpha when engine RPM is high. Subtle but adds life.
//
// Sprint 36 — PRD v3 S4
//
// Systems:
//   attach_haze_once  — Update (Local<bool> guard) — spawns the quad once
//                       as a chassis child when VehicleRoot is ready.
//   pulse_haze        — Update — reads EngineState.rpm each frame and
//                       updates the material alpha with shimmer wobble.
//
// Public API:
//   HeatHazePlugin

use bevy::prelude::*;

use crate::engine_torque::EngineState;
use crate::vehicle::VehicleRoot;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Chassis-local position of the heat haze quad — just above the V8.
const HAZE_LOCAL_POS: Vec3 = Vec3::new(0.0, 0.65, -1.4);

/// Quad half-extents: 0.6 m wide × 0.4 m deep.
const HAZE_HALF: Vec2 = Vec2::new(0.3, 0.2);

/// Idle RPM — below this the haze is fully transparent.
const IDLE_RPM: f32 = 700.0;

/// RPM at which maximum base alpha is reached.
const PEAK_RPM: f32 = 4500.0;

/// Maximum base alpha (excluding shimmer).
const MAX_ALPHA: f32 = 0.30;

/// Shimmer amplitude added on top of base alpha.
const SHIMMER_AMP: f32 = 0.03;

/// Shimmer frequency in radians per second.
const SHIMMER_FREQ: f32 = 4.5;

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

/// Marker on the heat-haze quad.  Stores the material handle so `pulse_haze`
/// can look it up without a separate query join.
#[derive(Component)]
pub struct HeatHaze {
    pub mat: Handle<StandardMaterial>,
}

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct HeatHazePlugin;

impl Plugin for HeatHazePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (attach_haze_once, pulse_haze));
    }
}

// ---------------------------------------------------------------------------
// attach_haze_once
// ---------------------------------------------------------------------------

/// Runs every frame but executes its body exactly once (guarded by a
/// `Local<bool>`).  Waits until `VehicleRoot` is inserted by vehicle.rs,
/// then spawns the haze quad as a child of the chassis.
fn attach_haze_once(
    mut done: Local<bool>,
    vehicle: Option<Res<VehicleRoot>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if *done {
        return;
    }
    let Some(vehicle) = vehicle else { return };
    *done = true;

    let mat_handle = materials.add(StandardMaterial {
        base_color: Color::srgba(0.95, 0.85, 0.7, 0.0),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        double_sided: true,
        cull_mode: None,
        ..default()
    });

    let mesh_handle = meshes.add(Plane3d::new(Vec3::Y, HAZE_HALF));

    let haze_entity = commands
        .spawn((
            HeatHaze {
                mat: mat_handle.clone(),
            },
            Mesh3d(mesh_handle),
            MeshMaterial3d(mat_handle),
            Transform::from_translation(HAZE_LOCAL_POS),
            Visibility::default(),
        ))
        .id();

    commands.entity(vehicle.chassis).add_child(haze_entity);
}

// ---------------------------------------------------------------------------
// pulse_haze
// ---------------------------------------------------------------------------

/// Every frame: maps RPM to a base alpha, adds a sinusoidal shimmer, and
/// writes the result back into the haze material.
fn pulse_haze(
    engine: Res<EngineState>,
    time: Res<Time>,
    haze_q: Query<&HeatHaze>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let Ok(haze) = haze_q.single() else { return };
    let Some(mat) = materials.get_mut(&haze.mat) else { return };

    // Linear ramp: 0.0 at idle, MAX_ALPHA at PEAK_RPM.
    let t = ((engine.rpm - IDLE_RPM) / (PEAK_RPM - IDLE_RPM)).clamp(0.0, 1.0);
    let base_alpha = t * MAX_ALPHA;

    // Sinusoidal shimmer layered on top.
    let shimmer = (time.elapsed_secs() * SHIMMER_FREQ).sin() * SHIMMER_AMP;
    let alpha = (base_alpha + shimmer * t).clamp(0.0, 1.0);

    mat.base_color = Color::srgba(0.95, 0.85, 0.7, alpha);
}
