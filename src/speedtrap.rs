// Speed-trap zones for skoffroad.
//
// Detection approach: gate-axis projection.
//   Each gate defines a measurement axis (the local Z of the spawned gate
//   transform).  Every frame the chassis XZ position is projected onto that
//   axis.  The chassis is considered "in zone" when:
//     |along_axis| <= HALF_DEPTH (5 m)  AND  |lateral_offset| <= HALF_WIDTH (5 m).
//   While in zone, peak XZ speed is tracked.  On exit (was_in && !now_in), the
//   run result is locked and a HUD popup is shown for 4 seconds.
//
// Visual: two cyan-emissive Cylinder pillars (r=0.4, h=4) placed 5 m either
//   side of the gate center, plus a thin banner cuboid bridging their tops.

use bevy::prelude::*;
use avian3d::prelude::LinearVelocity;

use crate::terrain::TERRAIN_SEED;
use crate::vehicle::{Chassis, VehicleRoot};

// ---- Constants ---------------------------------------------------------------

const NUM_TRAPS: u32    = 5;
const RANGE: f32        = 80.0;
const ORIGIN_CLEAR: f32 = 30.0;
const HALF_DEPTH: f32   = 5.0;  // along gate axis (direction of travel)
const HALF_WIDTH: f32   = 5.0;  // lateral extent of the gate
const PILLAR_GAP: f32   = 5.0;  // half-span: pillars sit +/-5 m from center
const PILLAR_RADIUS: f32 = 0.4;
const PILLAR_HEIGHT: f32 = 4.0;
const HUD_FADE_SECS: f32 = 4.0;
const MPS_TO_MPH: f32   = 2.237_f32;

// ---- Components / resources --------------------------------------------------

/// Per-gate state component.
#[derive(Component)]
pub struct SpeedTrap {
    pub id: u32,
    pub current_peak_mph: f32,
    pub best_peak_mph: f32,
    pub chassis_in_zone: bool,
    /// Gate center in world XZ.
    center: Vec2,
    /// Unit vector pointing along the gate's measurement axis (perpendicular to
    /// the banner, i.e. the direction a vehicle passes through).
    axis: Vec2,
}

/// Marker for the HUD popup root node.
#[derive(Component)]
struct SpeedTrapHud;

/// Text node inside the HUD popup.
#[derive(Component)]
struct SpeedTrapHudText;

/// Shared resource: most recent result message + countdown until hidden.
#[derive(Resource, Default)]
struct SpeedTrapHudState {
    message: String,
    ttl: f32,
}

// ---- Plugin ------------------------------------------------------------------

pub struct SpeedTrapPlugin;

impl Plugin for SpeedTrapPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SpeedTrapHudState>()
            .add_systems(Startup, (spawn_speed_traps, spawn_speedtrap_hud))
            .add_systems(
                Update,
                (update_speed_traps, update_speedtrap_hud)
                    .run_if(resource_exists::<VehicleRoot>),
            );
    }
}

// ---- LCG helper --------------------------------------------------------------

/// Simple 32-bit LCG — deterministic, no external deps.
struct Lcg(u64);

impl Lcg {
    fn new(seed: u64) -> Self { Self(seed) }

    fn next_u32(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.0 >> 33) as u32
    }

    /// Uniform float in [lo, hi].
    fn next_f32_range(&mut self, lo: f32, hi: f32) -> f32 {
        let t = (self.next_u32() as f64 / u32::MAX as f64) as f32;
        lo + t * (hi - lo)
    }
}

// ---- Startup: spawn traps ----------------------------------------------------

fn spawn_speed_traps(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let cyan_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.0, 1.0, 1.0),
        emissive: LinearRgba::rgb(0.0, 2.0, 2.0),
        perceptual_roughness: 0.4,
        ..default()
    });

    let pillar_mesh = meshes.add(Cylinder::new(PILLAR_RADIUS, PILLAR_HEIGHT));
    let banner_mesh = meshes.add(Cuboid::new(PILLAR_GAP * 2.0, 0.15, 0.15));

    let mut rng = Lcg::new((TERRAIN_SEED as u64).wrapping_add(17));

    for id in 0..NUM_TRAPS {
        // Pick XZ avoiding origin within ORIGIN_CLEAR metres.
        let (cx, cz) = loop {
            let x = rng.next_f32_range(-RANGE, RANGE);
            let z = rng.next_f32_range(-RANGE, RANGE);
            if x * x + z * z >= ORIGIN_CLEAR * ORIGIN_CLEAR {
                break (x, z);
            }
        };

        let angle = rng.next_f32_range(0.0, std::f32::consts::TAU);
        let rotation = Quat::from_rotation_y(angle);

        // Measurement axis is the gate's local +X (left-right through gate is X,
        // driving direction is Z).  The vehicle passes through along local Z.
        let axis = Vec2::new(angle.sin(), angle.cos()); // gate's local +Z in world

        let cy = crate::terrain::terrain_height_at(cx, cz) + PILLAR_HEIGHT * 0.5 + 0.1;

        // Gate parent (invisible transform anchor).
        let gate_entity = commands.spawn((
            SpeedTrap {
                id,
                current_peak_mph: 0.0,
                best_peak_mph: 0.0,
                chassis_in_zone: false,
                center: Vec2::new(cx, cz),
                axis,
            },
            Transform::from_xyz(cx, cy, cz).with_rotation(rotation),
            Visibility::default(),
        )).id();

        // Left pillar (local -X from center).
        let left = commands.spawn((
            Mesh3d(pillar_mesh.clone()),
            MeshMaterial3d(cyan_mat.clone()),
            Transform::from_xyz(-PILLAR_GAP, 0.0, 0.0),
        )).id();

        // Right pillar.
        let right = commands.spawn((
            Mesh3d(pillar_mesh.clone()),
            MeshMaterial3d(cyan_mat.clone()),
            Transform::from_xyz(PILLAR_GAP, 0.0, 0.0),
        )).id();

        // Banner between pillar tops.
        let banner = commands.spawn((
            Mesh3d(banner_mesh.clone()),
            MeshMaterial3d(cyan_mat.clone()),
            Transform::from_xyz(0.0, PILLAR_HEIGHT * 0.5, 0.0),
        )).id();

        commands.entity(gate_entity).add_children(&[left, right, banner]);
    }
}

// ---- Startup: spawn HUD ------------------------------------------------------

fn spawn_speedtrap_hud(mut commands: Commands) {
    let root = commands.spawn((
        SpeedTrapHud,
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(76.0),
            width: Val::Px(320.0),
            margin: UiRect {
                left: Val::Auto,
                right: Val::Auto,
                ..default()
            },
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            padding: UiRect::all(Val::Px(8.0)),
            display: Display::None,
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.10, 0.15, 0.85)),
        ZIndex(25),
    )).id();

    let text = commands.spawn((
        SpeedTrapHudText,
        Text::new(""),
        TextFont { font_size: 14.0, ..default() },
        TextColor(Color::srgb(0.0, 1.0, 1.0)),
    )).id();

    commands.entity(root).add_children(&[text]);
}

// ---- Update: detection -------------------------------------------------------

fn update_speed_traps(
    vehicle: Res<VehicleRoot>,
    chassis_q: Query<(&Transform, &LinearVelocity), With<Chassis>>,
    mut traps: Query<&mut SpeedTrap>,
    mut hud_state: ResMut<SpeedTrapHudState>,
    time: Res<Time>,
) {
    let Ok((transform, lin_vel)) = chassis_q.get(vehicle.chassis) else { return };

    let pos_xz = Vec2::new(transform.translation.x, transform.translation.z);
    // XZ speed (ignore vertical for trap speed measurement).
    let speed_xz_mps = Vec2::new(lin_vel.x, lin_vel.z).length();
    let speed_xz_mph = speed_xz_mps * MPS_TO_MPH;

    let dt = time.delta_secs();

    for mut trap in traps.iter_mut() {
        let to_chassis = pos_xz - trap.center;

        // Lateral axis = perpendicular to measurement axis (gate's local X).
        let lateral = Vec2::new(-trap.axis.y, trap.axis.x);

        let along   = to_chassis.dot(trap.axis);   // depth into gate
        let lat_off = to_chassis.dot(lateral);      // side offset

        let in_zone = along.abs() <= HALF_DEPTH && lat_off.abs() <= HALF_WIDTH;

        if in_zone {
            if speed_xz_mph > trap.current_peak_mph {
                trap.current_peak_mph = speed_xz_mph;
            }
        }

        let was_in = trap.chassis_in_zone;

        if was_in && !in_zone {
            // Chassis just exited — lock in the result.
            let peak = trap.current_peak_mph;
            if peak > trap.best_peak_mph {
                trap.best_peak_mph = peak;
            }
            hud_state.message = format!(
                "TRAP #{}: {:.1} mph  (best: {:.1} mph)",
                trap.id + 1,
                peak,
                trap.best_peak_mph,
            );
            hud_state.ttl = HUD_FADE_SECS;

            // Reset current-run peak for the next pass.
            trap.current_peak_mph = 0.0;
        }

        trap.chassis_in_zone = in_zone;

        let _ = dt; // dt available if future smoothing needed
    }
}

// ---- Update: HUD display -----------------------------------------------------

fn update_speedtrap_hud(
    mut hud_state: ResMut<SpeedTrapHudState>,
    time: Res<Time>,
    mut root_q: Query<&mut Node, With<SpeedTrapHud>>,
    mut text_q: Query<&mut Text, With<SpeedTrapHudText>>,
) {
    if hud_state.ttl > 0.0 {
        hud_state.ttl -= time.delta_secs();
    }

    let visible = hud_state.ttl > 0.0;

    for mut node in root_q.iter_mut() {
        node.display = if visible { Display::Flex } else { Display::None };
    }

    if visible {
        for mut text in text_q.iter_mut() {
            text.0 = hud_state.message.clone();
        }
    }
}
