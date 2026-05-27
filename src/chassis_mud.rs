// Persistent chassis mud streaks — Effect 5 of Sprint 67.
//
// Tracks per-wheel-quadrant mud accumulation when the chassis wheels pass over
// a MudPuddle. Renders 1 dark-brown decal-like quad per quadrant (FL/FR/RL/RR)
// attached to the lower fender, opacity proportional to mud_amount.
//
// The R key (existing reset spawn) clears mud_amount back to 0.
//
// vehicle_dirt.rs tracks a global level (colour tint). This plugin adds the
// per-quadrant decal layer as a sibling — no overlap.
//
// Public API:
//   ChassisMudPlugin
//   ChassisMudState (resource)

use bevy::prelude::*;

use crate::mud_puddles::MudPuddle;
use crate::vehicle::{Chassis, VehicleRoot};
use avian3d::prelude::LinearVelocity;

// ---- Plugin -----------------------------------------------------------------

pub struct ChassisMudPlugin;

impl Plugin for ChassisMudPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ChassisMudState>()
           .add_systems(Update, (
               spawn_mud_decals,
               accumulate_quadrant_mud,
               apply_mud_opacity,
               clear_mud_on_respawn,
           ));
    }
}

// ---- Resource ---------------------------------------------------------------

/// Per-quadrant (FL=0, FR=1, RL=2, RR=3) mud accumulation in [0, 1].
#[derive(Resource)]
pub struct ChassisMudState {
    pub mud_amount: [f32; 4],
    /// Material handles for the four fender decal quads.
    pub mat_handles: [Option<Handle<StandardMaterial>>; 4],
    spawned: bool,
}

impl Default for ChassisMudState {
    fn default() -> Self {
        Self {
            mud_amount:  [0.0; 4],
            mat_handles: [None, None, None, None],
            spawned:     false,
        }
    }
}

/// Marker on each mud-streak decal quad.
#[derive(Component)]
struct MudDecal;

// ---- Wheel offsets in chassis local space (matches vehicle.rs) ---------------

const WHEEL_OFFSETS: [Vec3; 4] = [
    Vec3::new(-1.1, -0.35, -1.4), // FL
    Vec3::new( 1.1, -0.35, -1.4), // FR
    Vec3::new(-1.1, -0.35,  1.4), // RL
    Vec3::new( 1.1, -0.35,  1.4), // RR
];

// Fender decal positions (lower fender area, chassis local space).
// Slightly inboard and at wheel height on the fender flare.
const FENDER_POSITIONS: [Vec3; 4] = [
    Vec3::new(-1.08, -0.30, -1.4),  // FL fender
    Vec3::new( 1.08, -0.30, -1.4),  // FR fender
    Vec3::new(-1.08, -0.30,  1.4),  // RL fender
    Vec3::new( 1.08, -0.30,  1.4),  // RR fender
];

const MUD_RATE:            f32 = 0.25; // mud_amount/s while inside puddle
const PUDDLE_CHECK_RADIUS: f32 = 0.5;  // extra slop for wheel-in-puddle check

// ---- Spawn decals (runs once, guarded by state flag) -------------------------

fn spawn_mud_decals(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut mud_state: ResMut<ChassisMudState>,
    vehicle: Option<Res<VehicleRoot>>,
) {
    if mud_state.spawned {
        return;
    }
    let Some(vehicle) = vehicle else { return };

    // Fender streak quad: thin plane on the lower body side.
    let streak_mesh = meshes.add(Plane3d::new(Vec3::X, Vec2::new(0.25, 0.55)));

    for (i, &fender_pos) in FENDER_POSITIONS.iter().enumerate() {
        // Dark mud material; alpha driven by mud_amount. Start transparent.
        let mat = materials.add(StandardMaterial {
            base_color: Color::srgba(0.08, 0.06, 0.03, 0.0),
            alpha_mode: AlphaMode::Blend,
            perceptual_roughness: 0.9,
            double_sided: true,
            cull_mode: None,
            unlit: true,
            ..default()
        });
        mud_state.mat_handles[i] = Some(mat.clone());

        let streak_id = commands.spawn((
            MudDecal,
            Mesh3d(streak_mesh.clone()),
            MeshMaterial3d(mat),
            Transform::from_translation(fender_pos),
        )).id();

        commands.entity(vehicle.chassis).add_child(streak_id);
    }

    mud_state.spawned = true;
}

// ---- Accumulate mud per quadrant --------------------------------------------

fn accumulate_quadrant_mud(
    vehicle: Option<Res<VehicleRoot>>,
    chassis_q: Query<(&Transform, &LinearVelocity), With<Chassis>>,
    puddles: Query<&MudPuddle>,
    time: Res<Time>,
    mut mud_state: ResMut<ChassisMudState>,
) {
    let Some(vehicle) = vehicle else { return };
    let Ok((c_tf, _lin_vel)) = chassis_q.get(vehicle.chassis) else { return };

    let dt = time.delta_secs();
    let chassis_pos = c_tf.translation;
    let chassis_rot = c_tf.rotation;

    for (qi, &wheel_local) in WHEEL_OFFSETS.iter().enumerate() {
        // World position of this wheel.
        let wheel_world = chassis_pos + chassis_rot * wheel_local;

        // Check if this wheel is inside any mud puddle.
        let in_puddle = puddles.iter().any(|puddle| {
            let dx = wheel_world.x - puddle.center.x;
            let dz = wheel_world.z - puddle.center.z;
            let r = puddle.radius + PUDDLE_CHECK_RADIUS;
            dx * dx + dz * dz < r * r
        });

        if in_puddle {
            mud_state.mud_amount[qi] = (mud_state.mud_amount[qi] + MUD_RATE * dt).min(1.0);
        }
    }
}

// ---- Apply opacity to decal materials ---------------------------------------

fn apply_mud_opacity(
    mud_state: Res<ChassisMudState>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if !mud_state.is_changed() {
        return;
    }
    for (i, handle_opt) in mud_state.mat_handles.iter().enumerate() {
        let Some(handle) = handle_opt else { continue };
        let Some(mat) = materials.get_mut(handle) else { continue };
        let alpha = mud_state.mud_amount[i];
        mat.base_color = Color::srgba(0.08, 0.06, 0.03, alpha);
    }
}

// ---- Clear mud on R-key respawn ---------------------------------------------

fn clear_mud_on_respawn(
    keys: Res<ButtonInput<KeyCode>>,
    mut mud_state: ResMut<ChassisMudState>,
) {
    if keys.just_pressed(KeyCode::KeyR) {
        mud_state.mud_amount = [0.0; 4];
    }
}
