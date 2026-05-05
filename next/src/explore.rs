// Explore mode: 12 hidden glowing markers scattered around the map at
// deterministic terrain spots. Player drives close (<4m) to "discover" each.
// Discovery rewards 200 XP. HUD top-right shows "EXPLORE  N/12".
//
// Public API:
//   ExplorePlugin
//   ExploreState (resource)

use bevy::prelude::*;
use crate::terrain::terrain_height_at;
use crate::vehicle::{Chassis, VehicleRoot};
use crate::xp::XpState;

// ---- Constants --------------------------------------------------------------

const MARKER_COUNT: usize   = 12;
const DISCOVER_DIST: f32    = 4.0;
const AVOID_ORIGIN:  f32    = 30.0;
const MAP_HALF:      f32    = 100.0; // 200 m square → ±100 m
const PILLAR_RADIUS: f32    = 0.6;
const PILLAR_HEIGHT: f32    = 6.0;
const PILLAR_Y_OFFSET: f32  = 3.0;  // centre above terrain (half-height)
const XP_DISCOVER: i32      = 200;

const PANEL_TOP:    f32 = 280.0;
const PANEL_RIGHT:  f32 = 14.0;
const PANEL_W:      f32 = 200.0;
const PANEL_H:      f32 = 40.0;

// ---- LCG helpers ------------------------------------------------------------

#[inline]
fn lcg_next(state: &mut u64) -> u64 {
    *state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    *state
}

#[inline]
fn lcg_f32(state: &mut u64, lo: f32, hi: f32) -> f32 {
    let bits = lcg_next(state);
    let t = (bits >> 11) as f32 / (1u64 << 53) as f32;
    lo + t * (hi - lo)
}

// ---- Public resource --------------------------------------------------------

#[derive(Resource, Default)]
pub struct ExploreState {
    pub markers: Vec<Vec3>,
    pub found:   Vec<bool>,
}

impl ExploreState {
    pub fn found_count(&self) -> usize {
        self.found.iter().filter(|&&f| f).count()
    }
}

// ---- Private components -----------------------------------------------------

#[derive(Component)]
struct ExploreMarker {
    idx: usize,
}

#[derive(Component)] struct ExploreHudRoot;
#[derive(Component)] struct ExploreHudText;

// ---- Plugin -----------------------------------------------------------------

pub struct ExplorePlugin;

impl Plugin for ExplorePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ExploreState>()
            .add_systems(Startup, (spawn_markers, spawn_hud))
            .add_systems(Update, (
                check_proximity.run_if(resource_exists::<VehicleRoot>),
                pulse_markers,
                update_hud,
            ));
    }
}

// ---- Startup: generate and spawn 12 glowing pillars -------------------------

fn spawn_markers(
    mut commands:  Commands,
    mut meshes:    ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut state:     ResMut<ExploreState>,
) {
    let mesh = meshes.add(Cylinder::new(PILLAR_RADIUS, PILLAR_HEIGHT));
    let mat  = materials.add(StandardMaterial {
        base_color:        Color::srgba(0.4, 1.0, 0.5, 0.9),
        emissive:          LinearRgba::rgb(0.4, 1.5, 0.6),
        unlit:             true,
        alpha_mode:        AlphaMode::Blend,
        ..default()
    });

    // LCG seed = 42
    let mut lcg: u64 = 42;
    let mut positions: Vec<Vec3> = Vec::with_capacity(MARKER_COUNT);
    let mut attempts = 0usize;

    while positions.len() < MARKER_COUNT && attempts < 4000 {
        attempts += 1;
        let x = lcg_f32(&mut lcg, -MAP_HALF, MAP_HALF);
        let z = lcg_f32(&mut lcg, -MAP_HALF, MAP_HALF);

        // Skip positions too close to origin
        if x * x + z * z < AVOID_ORIGIN * AVOID_ORIGIN {
            continue;
        }

        let y = terrain_height_at(x, z) + PILLAR_Y_OFFSET;
        positions.push(Vec3::new(x, y, z));
    }

    // Populate the resource
    state.markers = positions.clone();
    state.found   = vec![false; positions.len()];

    // Spawn a pillar for each marker
    for (idx, &pos) in positions.iter().enumerate() {
        commands.spawn((
            ExploreMarker { idx },
            Mesh3d(mesh.clone()),
            MeshMaterial3d(mat.clone()),
            Transform::from_translation(pos),
        ));
    }
}

// ---- Startup: spawn HUD panel -----------------------------------------------

fn spawn_hud(mut commands: Commands) {
    let panel = commands.spawn((
        ExploreHudRoot,
        Node {
            position_type: PositionType::Absolute,
            top:           Val::Px(PANEL_TOP),
            right:         Val::Px(PANEL_RIGHT),
            width:         Val::Px(PANEL_W),
            height:        Val::Px(PANEL_H),
            display:       Display::Flex,
            flex_direction: FlexDirection::Column,
            align_items:   AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
        BackgroundColor(Color::srgba(0.04, 0.04, 0.06, 0.80)),
    )).id();

    let text = commands.spawn((
        ExploreHudText,
        Text::new(format!("EXPLORE  0/{}", MARKER_COUNT)),
        TextFont  { font_size: 14.0, ..default() },
        TextColor(Color::srgb(0.4, 1.0, 0.5)),
    )).id();

    commands.entity(panel).add_child(text);
}

// ---- Update: proximity check ------------------------------------------------

fn check_proximity(
    mut commands:   Commands,
    vehicle:        Res<VehicleRoot>,
    chassis_q:      Query<&Transform, With<Chassis>>,
    marker_q:       Query<(Entity, &Transform, &ExploreMarker)>,
    mut state:      ResMut<ExploreState>,
    mut xp:         ResMut<XpState>,
    time:           Res<Time>,
) {
    let Ok(chassis_tf) = chassis_q.get(vehicle.chassis) else { return };
    let cpos = chassis_tf.translation;

    for (entity, marker_tf, marker) in marker_q.iter() {
        let idx = marker.idx;
        if idx >= state.found.len() || state.found[idx] {
            continue;
        }
        let dist = cpos.distance(marker_tf.translation);
        if dist < DISCOVER_DIST {
            state.found[idx] = true;
            commands.entity(entity).despawn();

            let now = time.elapsed_secs();
            xp.total_xp   = xp.total_xp.saturating_add(XP_DISCOVER as u64);
            xp.session_xp = xp.session_xp.saturating_add(XP_DISCOVER as u64);
            xp.last_gain   = XP_DISCOVER;
            xp.last_gain_t = now;

            let found_count = state.found_count();
            info!("discovered marker {}/{}!", found_count, MARKER_COUNT);
        }
    }
}

// ---- Update: pulse unfound markers ------------------------------------------

fn pulse_markers(
    time: Res<Time>,
    mut q: Query<(&ExploreMarker, &mut Transform)>,
) {
    let t = time.elapsed_secs();
    for (marker, mut transform) in q.iter_mut() {
        let pulse = 1.0 + (t * 3.0 + marker.idx as f32 * 0.5).sin() * 0.05;
        transform.scale.y = pulse;
    }
}

// ---- Update: HUD refresh ----------------------------------------------------

fn update_hud(
    state:   Res<ExploreState>,
    mut text_q: Query<&mut Text, With<ExploreHudText>>,
) {
    let Ok(mut text) = text_q.single_mut() else { return };
    let found = state.found_count();
    let total = state.markers.len().max(MARKER_COUNT);
    **text = format!("EXPLORE  {}/{}", found, total);
}
