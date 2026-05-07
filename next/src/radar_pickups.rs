// radar_pickups.rs — Sprint 29
//
// Colored dot overlay on the minimap for nearby pickups:
//   • Cyan  — gas stations (3 fixed world positions)
//   • Green — unfound explore markers (from ExploreState resource)
//
// Collectible positions are not publicly accessible from collectibles.rs,
// so yellow collectible dots are intentionally omitted.
//
// The overlay is a transparent Node tree positioned to match the inner
// 200×200 px map area of the minimap (left:16, bottom:16 after the 4 px
// padding of the minimap's outer panel). Dots are 6×6 px nodes with
// PositionType::Absolute.  The child list is rebuilt every frame so that
// explore markers disappear as they are found.
//
// Public API:
//   RadarPickupsPlugin

use bevy::prelude::*;
use crate::explore::ExploreState;

// ── Constants ──────────────────────────────────────────────────────────────────

/// Inner map area in screen pixels (matches minimap.rs MAP_PX = 200).
const MAP_PX: f32 = 200.0;

/// Half-extent of the world in metres (minimap covers ±100 m on X and Z).
const WORLD_HALF: f32 = 100.0;

/// Diameter of each pickup dot in pixels.
const DOT_PX: f32 = 6.0;

/// Gas-station XZ world positions — identical to gas_stations::PUMP_XZ.
const GAS_STATION_XZ: [(f32, f32); 3] = [
    ( 20.0,  35.0),
    (-30.0, -25.0),
    ( 60.0, -65.0),
];

// ── Components ─────────────────────────────────────────────────────────────────

/// Marker for the transparent overlay Node that sits on top of the minimap.
#[derive(Component)]
struct RadarOverlayRoot;

/// Marker for individual pickup dots so they can be bulk-despawned each frame.
#[derive(Component)]
struct RadarDot;

// ── Plugin ─────────────────────────────────────────────────────────────────────

pub struct RadarPickupsPlugin;

impl Plugin for RadarPickupsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_overlay)
           .add_systems(Update, update_dots);
    }
}

// ── Startup: spawn the transparent overlay root ───────────────────────────────

fn spawn_overlay(mut commands: Commands) {
    // The minimap outer panel sits at left:12, bottom:12, size:208×208 with
    // 4 px of padding on all sides.  The inner 200×200 map area therefore
    // starts at left:16, bottom:16.  We match that rect exactly so our
    // absolute-positioned dot coordinates (0..200) align with the minimap
    // pixel coordinates used for the chassis dot in minimap.rs.
    commands.spawn((
        RadarOverlayRoot,
        Node {
            position_type: PositionType::Absolute,
            left:   Val::Px(16.0),
            bottom: Val::Px(16.0),
            width:  Val::Px(MAP_PX),
            height: Val::Px(MAP_PX),
            overflow: Overflow::clip(),
            ..default()
        },
        BackgroundColor(Color::NONE),
        ZIndex(15),
    ));
}

// ── Per-frame: rebuild pickup dots ────────────────────────────────────────────

fn update_dots(
    mut commands:   Commands,
    overlay_q:      Query<Entity, With<RadarOverlayRoot>>,
    dot_q:          Query<Entity, With<RadarDot>>,
    explore_state:  Res<ExploreState>,
) {
    // Despawn all existing dots.
    for dot_entity in dot_q.iter() {
        commands.entity(dot_entity).despawn();
    }

    let Ok(overlay) = overlay_q.single() else { return };

    // Helper: convert world XZ to minimap pixel coordinates (left, top).
    // Matches the formula in minimap.rs: px = (wx / WORLD_EXTENT + 0.5) * MAP_PX
    // where WORLD_EXTENT = 200.  top=0 corresponds to world_z = -100.
    let to_pixel = |wx: f32, wz: f32| -> (f32, f32) {
        let left = (wx / (WORLD_HALF * 2.0) + 0.5) * MAP_PX;
        let top  = (wz / (WORLD_HALF * 2.0) + 0.5) * MAP_PX;
        (
            (left - DOT_PX / 2.0).clamp(0.0, MAP_PX - DOT_PX),
            (top  - DOT_PX / 2.0).clamp(0.0, MAP_PX - DOT_PX),
        )
    };

    // Cyan dots — gas stations.
    for &(wx, wz) in &GAS_STATION_XZ {
        let (left, top) = to_pixel(wx, wz);
        let dot = commands.spawn((
            RadarDot,
            Node {
                position_type: PositionType::Absolute,
                left:   Val::Px(left),
                top:    Val::Px(top),
                width:  Val::Px(DOT_PX),
                height: Val::Px(DOT_PX),
                ..default()
            },
            BackgroundColor(Color::srgb(0.0, 1.0, 1.0)),
            ZIndex(16),
        )).id();
        commands.entity(overlay).add_child(dot);
    }

    // Green dots — unfound explore markers.
    for (i, &pos) in explore_state.markers.iter().enumerate() {
        let found = explore_state.found.get(i).copied().unwrap_or(false);
        if found {
            continue;
        }
        let (left, top) = to_pixel(pos.x, pos.z);
        let dot = commands.spawn((
            RadarDot,
            Node {
                position_type: PositionType::Absolute,
                left:   Val::Px(left),
                top:    Val::Px(top),
                width:  Val::Px(DOT_PX),
                height: Val::Px(DOT_PX),
                ..default()
            },
            BackgroundColor(Color::srgb(0.0, 1.0, 0.2)),
            ZIndex(16),
        )).id();
        commands.entity(overlay).add_child(dot);
    }
}
