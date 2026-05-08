// Roof rack: a procedural offroad rack on top of the chassis with
// 4 corner pillars, 2 longitudinal side rails, 2 lateral crossbars,
// and 4 LED light bars (emissive, toggleable with F2).
//
// Public API:
//   RoofRackPlugin
//   RoofRackState  (resource — lights_on toggled by F2)

use bevy::prelude::*;
use crate::vehicle::VehicleRoot;

// ── Plugin ────────────────────────────────────────────────────────────────────

pub struct RoofRackPlugin;

impl Plugin for RoofRackPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RoofRackState>()
           .add_systems(Update, (
               attach_rack_once,
               toggle_lights_with_f2,
               apply_light_state,
           ));
    }
}

// ── Resource ──────────────────────────────────────────────────────────────────

#[derive(Resource, Default, Clone, Copy)]
pub struct RoofRackState {
    pub lights_on: bool,
}

// ── Component marker ──────────────────────────────────────────────────────────

/// Marks each of the four LED light bar cuboids so `apply_light_state`
/// can mutate their emissive channel independently.
#[derive(Component)]
pub struct LightBar;

// ── Constants ─────────────────────────────────────────────────────────────────

/// Dark structural gray shared by pillars, side rails, and crossbars.
const METAL_COLOR: Color = Color::srgb(0.20, 0.20, 0.22);

// Pillar dimensions and corner positions (chassis-local).
// Chassis top face is at local Y = +0.30 (half of 0.60 height).
// Pillars sit with their base at Y = 0.30 and their top at Y = 0.60;
// local_y = 0.45 centres a 0.30-tall pillar.
const PILLAR_SIZE: (f32, f32, f32) = (0.06, 0.30, 0.06);
const PILLAR_POSITIONS: [Vec3; 4] = [
    Vec3::new(-0.85, 0.45, -1.7),
    Vec3::new( 0.85, 0.45, -1.7),
    Vec3::new(-0.85, 0.45,  1.7),
    Vec3::new( 0.85, 0.45,  1.7),
];

// Side rail: longitudinal tube along the chassis top, one per side.
const RAIL_SIZE: (f32, f32, f32) = (0.05, 0.05, 3.4);
const RAIL_POSITIONS: [Vec3; 2] = [
    Vec3::new(-0.85, 0.60, 0.0),
    Vec3::new( 0.85, 0.60, 0.0),
];

// Crossbar: lateral tube spanning both rails, one front and one rear.
const CROSSBAR_SIZE: (f32, f32, f32) = (1.7, 0.05, 0.05);
const CROSSBAR_POSITIONS: [Vec3; 2] = [
    Vec3::new(0.0, 0.60, -1.5),
    Vec3::new(0.0, 0.60,  1.5),
];

// LED light bars: 4 across the front crossbar, facing forward.
const LIGHTBAR_SIZE: (f32, f32, f32) = (0.5, 0.06, 0.05);
const LIGHTBAR_POSITIONS: [Vec3; 4] = [
    Vec3::new(-0.6, 0.65, -1.55),
    Vec3::new(-0.2, 0.65, -1.55),
    Vec3::new( 0.2, 0.65, -1.55),
    Vec3::new( 0.6, 0.65, -1.55),
];

/// Cream-white base color for all four light bars.
const LIGHTBAR_BASE: Color = Color::srgb(0.95, 0.95, 0.85);
/// HDR warm-white emissive value used when lights are on.
const EMIT_ON:  LinearRgba = LinearRgba::rgb(2.0, 1.9, 1.3);
/// Zero emission used when lights are off.
const EMIT_OFF: LinearRgba = LinearRgba::rgb(0.0, 0.0, 0.0);

// ── Systems ───────────────────────────────────────────────────────────────────

/// Spawns all rack geometry as children of the chassis entity.
/// Runs every frame but executes its body only once (guarded by Local<bool>).
fn attach_rack_once(
    mut done:      Local<bool>,
    vehicle:       Option<Res<VehicleRoot>>,
    mut commands:  Commands,
    mut meshes:    ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if *done { return; }
    let Some(vehicle) = vehicle else { return };
    *done = true;

    let chassis = vehicle.chassis;

    // ── Shared structural material ────────────────────────────────────────────
    let metal_mat = materials.add(StandardMaterial {
        base_color: METAL_COLOR,
        perceptual_roughness: 0.75,
        metallic: 0.3,
        ..default()
    });

    // ── 4 corner pillars ──────────────────────────────────────────────────────
    let pillar_mesh = meshes.add(Cuboid::new(
        PILLAR_SIZE.0, PILLAR_SIZE.1, PILLAR_SIZE.2,
    ));
    for pos in PILLAR_POSITIONS {
        let e = commands.spawn((
            Mesh3d(pillar_mesh.clone()),
            MeshMaterial3d(metal_mat.clone()),
            Transform::from_translation(pos),
        )).id();
        commands.entity(chassis).add_child(e);
    }

    // ── 2 longitudinal side rails ─────────────────────────────────────────────
    let rail_mesh = meshes.add(Cuboid::new(
        RAIL_SIZE.0, RAIL_SIZE.1, RAIL_SIZE.2,
    ));
    for pos in RAIL_POSITIONS {
        let e = commands.spawn((
            Mesh3d(rail_mesh.clone()),
            MeshMaterial3d(metal_mat.clone()),
            Transform::from_translation(pos),
        )).id();
        commands.entity(chassis).add_child(e);
    }

    // ── 2 lateral crossbars ───────────────────────────────────────────────────
    let crossbar_mesh = meshes.add(Cuboid::new(
        CROSSBAR_SIZE.0, CROSSBAR_SIZE.1, CROSSBAR_SIZE.2,
    ));
    for pos in CROSSBAR_POSITIONS {
        let e = commands.spawn((
            Mesh3d(crossbar_mesh.clone()),
            MeshMaterial3d(metal_mat.clone()),
            Transform::from_translation(pos),
        )).id();
        commands.entity(chassis).add_child(e);
    }

    // ── 4 LED light bars ──────────────────────────────────────────────────────
    // Each bar gets its own material handle so emissive can be mutated per-entity.
    let lightbar_mesh = meshes.add(Cuboid::new(
        LIGHTBAR_SIZE.0, LIGHTBAR_SIZE.1, LIGHTBAR_SIZE.2,
    ));
    for pos in LIGHTBAR_POSITIONS {
        let lb_mat = materials.add(StandardMaterial {
            base_color: LIGHTBAR_BASE,
            emissive: EMIT_OFF,
            perceptual_roughness: 0.15,
            ..default()
        });
        let e = commands.spawn((
            LightBar,
            Mesh3d(lightbar_mesh.clone()),
            MeshMaterial3d(lb_mat),
            Transform::from_translation(pos),
        )).id();
        commands.entity(chassis).add_child(e);
    }
}

/// F2 just-pressed: flip RoofRackState::lights_on.
fn toggle_lights_with_f2(
    keys:      Res<ButtonInput<KeyCode>>,
    mut state: ResMut<RoofRackState>,
) {
    if keys.just_pressed(KeyCode::F2) {
        state.lights_on = !state.lights_on;
        info!("roof rack lights: {}", state.lights_on);
    }
}

/// Runs only when RoofRackState changes. Updates the emissive channel on every
/// LightBar entity's material: warm-white HDR when on, black when off.
fn apply_light_state(
    state:         Res<RoofRackState>,
    lightbar_q:    Query<&MeshMaterial3d<StandardMaterial>, With<LightBar>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if !state.is_changed() { return; }

    let target_emit = if state.lights_on { EMIT_ON } else { EMIT_OFF };

    for mat_handle in &lightbar_q {
        if let Some(mat) = materials.get_mut(mat_handle) {
            mat.emissive = target_emit;
        }
    }
}
