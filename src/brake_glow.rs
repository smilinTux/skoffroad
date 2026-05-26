// Brake rotor glow — Effect 3 of Sprint 67.
//
// When the chassis decelerates hard (speed drop > 5 m/s² OR brake held at
// speed > 10 m/s), heat each wheel's brake rotor. Each rotor is a small
// Cylinder mesh child of the chassis tagged with BrakeRotor + DefaultSkin.
// Emissive colour ramps from dark → orange → red-hot with heat.
//
// Public API:
//   BrakeGlowPlugin
//   BrakeRotorState (resource)

use bevy::prelude::*;
use avian3d::prelude::LinearVelocity;

use crate::vehicle::{Chassis, DefaultSkin, DriveInput, VehicleRoot};

// ---- Plugin -----------------------------------------------------------------

pub struct BrakeGlowPlugin;

impl Plugin for BrakeGlowPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BrakeRotorState>()
           .add_systems(Update, (
               spawn_rotors,
               update_rotor_heat,
               apply_rotor_emissive,
           ));
    }
}

// ---- Resources / Components -------------------------------------------------

#[derive(Resource)]
pub struct BrakeRotorState {
    pub per_wheel: [f32; 4],
    /// Rotor material handles, one per wheel (index matches WHEEL_OFFSETS).
    pub mat_handles: [Option<Handle<StandardMaterial>>; 4],
    prev_speed: f32,
    spawned: bool,
}

impl Default for BrakeRotorState {
    fn default() -> Self {
        Self {
            per_wheel:   [0.0; 4],
            mat_handles: [None, None, None, None],
            prev_speed:  0.0,
            spawned:     false,
        }
    }
}

/// Marker: this entity is a brake rotor mesh.
#[derive(Component)]
pub struct BrakeRotor {
    pub wheel_index: usize,
}

// ---- Wheel offsets (local space, matches vehicle.rs) ------------------------

const WHEEL_OFFSETS: [Vec3; 4] = [
    Vec3::new(-1.1, -0.35, -1.4),
    Vec3::new( 1.1, -0.35, -1.4),
    Vec3::new(-1.1, -0.35,  1.4),
    Vec3::new( 1.1, -0.35,  1.4),
];

const HEAT_RISE:  f32 = 0.8;  // per second under hard braking
const HEAT_DECAY: f32 = 0.4;  // per second at idle/low brake
const HARD_DECEL: f32 = 5.0;  // m/s² threshold
const BRAKE_SPEED_MIN: f32 = 10.0; // m/s — "brake held + speed" condition

// ---- Spawn rotors (runs once, guarded by state flag) -------------------------

fn spawn_rotors(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut rotor_state: ResMut<BrakeRotorState>,
    vehicle: Option<Res<VehicleRoot>>,
) {
    if rotor_state.spawned {
        return;
    }
    let Some(vehicle) = vehicle else { return };

    for (i, &offset) in WHEEL_OFFSETS.iter().enumerate() {
        // Dark steel base material; emissive mutated per frame in apply_rotor_emissive.
        let mat = materials.add(StandardMaterial {
            base_color: Color::srgb(0.12, 0.12, 0.14),
            perceptual_roughness: 0.65,
            metallic: 0.85,
            emissive: LinearRgba::BLACK,
            ..default()
        });
        rotor_state.mat_handles[i] = Some(mat.clone());

        // Position the rotor at the wheel offset, slightly inboard (reduce x).
        // The cylinder axis is Y in Bevy; wheels spin around local X (chassis
        // roll axis). Rotate 90° around Z so the disk faces the wheel laterally.
        let rotor_offset = Vec3::new(offset.x * 0.85, offset.y, offset.z);
        let rotor_rot = Quat::from_rotation_z(std::f32::consts::FRAC_PI_2);

        let rotor_id = commands.spawn((
            BrakeRotor { wheel_index: i },
            DefaultSkin,
            Mesh3d(meshes.add(Cylinder::new(0.18, 0.04))),
            MeshMaterial3d(mat),
            Transform::from_translation(rotor_offset).with_rotation(rotor_rot),
        )).id();

        commands.entity(vehicle.chassis).add_child(rotor_id);
    }

    rotor_state.spawned = true;
}

// ---- Heat update ------------------------------------------------------------

fn update_rotor_heat(
    vehicle: Option<Res<VehicleRoot>>,
    chassis_q: Query<&LinearVelocity, With<Chassis>>,
    input: Res<DriveInput>,
    time: Res<Time>,
    mut rotor_state: ResMut<BrakeRotorState>,
) {
    let Some(vehicle) = vehicle else { return };
    let Ok(lin_vel) = chassis_q.get(vehicle.chassis) else { return };

    let dt = time.delta_secs();
    let speed = Vec3::new(lin_vel.x, lin_vel.y, lin_vel.z).length();

    // Compute deceleration (positive = slowing down).
    let decel = (rotor_state.prev_speed - speed) / dt.max(1e-4);
    rotor_state.prev_speed = speed;

    let hard_brake = decel > HARD_DECEL || (input.brake && speed > BRAKE_SPEED_MIN);

    for heat in &mut rotor_state.per_wheel {
        if hard_brake {
            *heat = (*heat + HEAT_RISE * dt).min(1.0);
        } else {
            *heat = (*heat - HEAT_DECAY * dt).max(0.0);
        }
    }
}

// ---- Emissive application ---------------------------------------------------

fn apply_rotor_emissive(
    rotor_state: Res<BrakeRotorState>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (i, maybe_handle) in rotor_state.mat_handles.iter().enumerate() {
        let Some(handle) = maybe_handle else { continue };
        let Some(mat) = materials.get_mut(handle) else { continue };
        let heat = rotor_state.per_wheel[i];
        // heat=0 → black, heat=0.5 → orange, heat=1.0 → red-hot bright.
        mat.emissive = LinearRgba::rgb(heat * 2.0, heat * 0.4, 0.0) * heat;
    }
}
