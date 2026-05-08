// Winch system — Sprint 33.
//
// E key when within 10 m of any RockGardenRock attaches the winch line to the
// nearest boulder.  Y toggles spooling on/off (pulls vehicle toward anchor at
// ~1.5 m/s).  Esc detaches.  A white cylinder cable is shown between the
// chassis-front attachment point and the anchor while attached.
//
// HUD indicator: small panel bottom-left showing "WINCH: armed" / "WINCH: spooling".
//
// Physics: force is applied in PhysicsSchedule (after NarrowPhase, before
// Solver), same pattern as boost.rs and assists.rs, so no ambiguity ordering
// is needed beyond the schedule-level Warn downgrade already in main.rs.
//
// Public API:
//   WinchPlugin
//   WinchState (resource)

use bevy::prelude::*;
use avian3d::prelude::*;

use crate::rock_garden::RockGardenRock;
use crate::vehicle::{Chassis, VehicleRoot};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Maximum attach range in metres.
const ATTACH_RANGE: f32 = 10.0;
/// Pull force in Newtons.
const WINCH_FORCE: f32 = 8_000.0;
/// Target spool speed m/s — force is capped so the chassis cannot exceed this.
const MAX_SPOOL_SPEED: f32 = 1.5;
/// Auto-detach if anchor is this far away (cable snaps).
const SNAP_RANGE: f32 = 30.0;
/// Auto-detach when this close to anchor (arrived).
const ARRIVE_DIST: f32 = 1.5;
/// Chassis-local forward offset for the cable attachment point.
const CABLE_ATTACH_FWD: f32 = 2.0;
/// Cylinder cable visual radius.
const CABLE_RADIUS: f32 = 0.04;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

pub struct WinchPlugin;

impl Plugin for WinchPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WinchState>()
            .add_systems(Startup, spawn_winch_hud)
            .add_systems(
                Update,
                (
                    attach_with_e,
                    toggle_spooling_with_y,
                    detach_with_esc,
                    update_cable_visual,
                    update_winch_hud,
                )
                    .run_if(resource_exists::<VehicleRoot>),
            )
            .add_systems(
                PhysicsSchedule,
                apply_winch_force
                    .after(PhysicsStepSystems::NarrowPhase)
                    .before(PhysicsStepSystems::Solver),
            );
    }
}

#[derive(Resource, Default, Clone)]
pub struct WinchState {
    pub anchor_pos: Option<Vec3>,
    pub spooling: bool,
}

// ---------------------------------------------------------------------------
// Marker component for the cable visual entity.
// ---------------------------------------------------------------------------

#[derive(Component)]
pub struct WinchCable;

// ---------------------------------------------------------------------------
// HUD components
// ---------------------------------------------------------------------------

#[derive(Component)]
struct WinchHudPanel;

#[derive(Component)]
struct WinchHudText;

// ---------------------------------------------------------------------------
// Startup: HUD panel
// ---------------------------------------------------------------------------

fn spawn_winch_hud(mut commands: Commands) {
    // Small panel, bottom-left, just above the terrain (above FPS counter area).
    let panel = commands
        .spawn((
            WinchHudPanel,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(12.0),
                bottom: Val::Px(12.0),
                padding: UiRect::all(Val::Px(8.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.05, 0.05, 0.07, 0.75)),
            Visibility::Hidden,
        ))
        .id();

    let text = commands
        .spawn((
            WinchHudText,
            Text::new("WINCH: armed"),
            TextFont {
                font_size: 14.0,
                ..default()
            },
            TextColor(Color::srgb(0.9, 0.9, 0.55)),
        ))
        .id();

    commands.entity(panel).add_child(text);
}

// ---------------------------------------------------------------------------
// Update: attach_with_e
// ---------------------------------------------------------------------------

fn attach_with_e(
    keys: Res<ButtonInput<KeyCode>>,
    vehicle: Res<VehicleRoot>,
    chassis_q: Query<&Transform, With<Chassis>>,
    rocks_q: Query<&Transform, With<RockGardenRock>>,
    mut state: ResMut<WinchState>,
) {
    if !keys.just_pressed(KeyCode::KeyE) {
        return;
    }
    // Already attached — pressing E again does nothing (Esc detaches).
    if state.anchor_pos.is_some() {
        return;
    }

    let Ok(chassis_transform) = chassis_q.get(vehicle.chassis) else { return };
    let chassis_pos = chassis_transform.translation;

    let mut best_dist = f32::MAX;
    let mut best_pos: Option<Vec3> = None;

    for rock_transform in &rocks_q {
        let d = rock_transform.translation.distance(chassis_pos);
        if d < ATTACH_RANGE && d < best_dist {
            best_dist = d;
            best_pos = Some(rock_transform.translation);
        }
    }

    if let Some(anchor) = best_pos {
        state.anchor_pos = Some(anchor);
        info!("winch: attached to anchor at {:?}", anchor);
    } else {
        info!("winch: no anchor in range");
    }
}

// ---------------------------------------------------------------------------
// Update: toggle_spooling_with_y
// ---------------------------------------------------------------------------

fn toggle_spooling_with_y(
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<WinchState>,
) {
    if keys.just_pressed(KeyCode::KeyY) && state.anchor_pos.is_some() {
        state.spooling = !state.spooling;
        if state.spooling {
            info!("winch: SPOOLING");
        } else {
            info!("winch: PAUSED");
        }
    }
}

// ---------------------------------------------------------------------------
// Update: detach_with_esc (+ auto-detach conditions)
// ---------------------------------------------------------------------------

fn detach_with_esc(
    keys: Res<ButtonInput<KeyCode>>,
    vehicle: Res<VehicleRoot>,
    chassis_q: Query<&Transform, With<Chassis>>,
    mut state: ResMut<WinchState>,
) {
    let Some(anchor) = state.anchor_pos else { return };

    let manual = keys.just_pressed(KeyCode::Escape);

    let auto_detach = if let Ok(chassis_transform) = chassis_q.get(vehicle.chassis) {
        let d = chassis_transform.translation.distance(anchor);
        d > SNAP_RANGE || d < ARRIVE_DIST
    } else {
        false
    };

    if manual || auto_detach {
        state.anchor_pos = None;
        state.spooling = false;
        info!("winch: detached");
    }
}

// ---------------------------------------------------------------------------
// PhysicsSchedule: apply_winch_force
// ---------------------------------------------------------------------------

fn apply_winch_force(
    state: Res<WinchState>,
    vehicle: Option<Res<VehicleRoot>>,
    mut chassis_q: Query<(Forces, &Transform), With<Chassis>>,
) {
    if !state.spooling {
        return;
    }
    let Some(anchor) = state.anchor_pos else { return };
    let Some(vehicle) = vehicle else { return };

    let Ok((mut forces, transform)) = chassis_q.get_mut(vehicle.chassis) else { return };

    let chassis_pos = transform.translation;
    let delta = anchor - chassis_pos;
    let dist = delta.length();
    if dist < 0.01 {
        return;
    }
    let dir = delta / dist;

    // Current velocity component toward the anchor.
    let vel = forces.linear_velocity();
    let speed_toward = vel.dot(dir);

    // Scale force down so we don't overshoot the target speed.
    let force_scale = if speed_toward >= MAX_SPOOL_SPEED {
        0.0
    } else {
        ((MAX_SPOOL_SPEED - speed_toward) / MAX_SPOOL_SPEED).clamp(0.0, 1.0)
    };

    forces.apply_force(dir * WINCH_FORCE * force_scale);
}

// ---------------------------------------------------------------------------
// Update: update_cable_visual
// ---------------------------------------------------------------------------

fn update_cable_visual(
    state: Res<WinchState>,
    vehicle: Res<VehicleRoot>,
    chassis_q: Query<&Transform, With<Chassis>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut cable_entity: Local<Option<Entity>>,
    existing_q: Query<Entity, With<WinchCable>>,
) {
    let Ok(chassis_transform) = chassis_q.get(vehicle.chassis) else { return };

    match state.anchor_pos {
        None => {
            // Detached — despawn the cable if it exists.
            if let Some(ent) = cable_entity.take() {
                // Guard: only despawn if entity still exists.
                if existing_q.get(ent).is_ok() {
                    commands.entity(ent).despawn();
                }
            }
        }
        Some(anchor) => {
            let fwd = (chassis_transform.rotation * Vec3::NEG_Z).normalize();
            let attach_point = chassis_transform.translation + fwd * CABLE_ATTACH_FWD;
            let midpoint = (attach_point + anchor) * 0.5;
            let cable_vec = anchor - attach_point;
            let length = cable_vec.length();
            if length < 0.01 {
                return;
            }
            let cable_dir = cable_vec / length;

            // Rotation to align cylinder Y-axis with cable direction.
            let rotation = Quat::from_rotation_arc(Vec3::Y, cable_dir);

            if let Some(ent) = *cable_entity {
                if existing_q.get(ent).is_ok() {
                    // Update transform of existing cable entity.
                    commands.entity(ent).insert(Transform {
                        translation: midpoint,
                        rotation,
                        scale: Vec3::ONE,
                    });
                    return;
                }
            }

            // Spawn a new cable cylinder.
            let mesh = meshes.add(Cylinder::new(CABLE_RADIUS, length));
            let mat = materials.add(StandardMaterial {
                base_color: Color::srgb(0.95, 0.95, 0.92),
                perceptual_roughness: 0.5,
                ..default()
            });

            let ent = commands
                .spawn((
                    WinchCable,
                    Mesh3d(mesh),
                    MeshMaterial3d(mat),
                    Transform {
                        translation: midpoint,
                        rotation,
                        scale: Vec3::ONE,
                    },
                    Visibility::default(),
                ))
                .id();

            *cable_entity = Some(ent);
        }
    }
}

// ---------------------------------------------------------------------------
// Update: update_winch_hud
// ---------------------------------------------------------------------------

fn update_winch_hud(
    state: Res<WinchState>,
    mut panel_q: Query<&mut Visibility, With<WinchHudPanel>>,
    mut text_q: Query<(&mut Text, &mut TextColor), With<WinchHudText>>,
) {
    for mut vis in &mut panel_q {
        *vis = if state.anchor_pos.is_some() {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }

    if state.anchor_pos.is_some() {
        for (mut text, mut color) in &mut text_q {
            if state.spooling {
                text.0 = "WINCH: spooling".to_string();
                color.0 = Color::srgb(0.35, 0.95, 0.35);
            } else {
                text.0 = "WINCH: armed".to_string();
                color.0 = Color::srgb(0.9, 0.9, 0.55);
            }
        }
    }
}
