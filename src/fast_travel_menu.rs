// Fast-travel menu: F5 opens a centered modal listing all named
// destinations (race start, all 4 spawn point slots, all 3 landmarks).
// Up/Down to select, Enter to teleport, Esc/F5 to close.
//
// Input from other systems is not explicitly paused — physics keeps
// running — but while the modal is open the normal drive input still
// fires. That is acceptable per the sprint spec ("just don't pause physics").
//
// Public API:
//   FastTravelMenuPlugin

use bevy::prelude::*;
use avian3d::prelude::{AngularVelocity, LinearVelocity};

use crate::spawn_points::SpawnPointsState;
use crate::terrain::terrain_height_at;
use crate::vehicle::{Chassis, VehicleRoot};

// ---------------------------------------------------------------------------
// Public plugin
// ---------------------------------------------------------------------------

pub struct FastTravelMenuPlugin;

impl Plugin for FastTravelMenuPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FastTravelMenuState>()
            .add_systems(Startup, spawn_modal)
            .add_systems(
                Update,
                (
                    toggle_with_f5,
                    cycle_cursor,
                    apply_teleport,
                    update_menu_view,
                ),
            );
    }
}

// ---------------------------------------------------------------------------
// Public resource
// ---------------------------------------------------------------------------

/// Tracks whether the fast-travel modal is open and which row is selected.
#[derive(Resource, Default)]
pub struct FastTravelMenuState {
    pub open:       bool,
    pub cursor_idx: usize,
}

// ---------------------------------------------------------------------------
// Destination catalogue
// ---------------------------------------------------------------------------

/// A single entry in the destination list.
struct Dest {
    name: &'static str,
    /// Fixed x/z; y is computed from terrain at query time.
    x: f32,
    z: f32,
    /// Extra clearance added on top of terrain_height_at.
    clearance: f32,
}

/// The 5 hard-coded destinations (indices 0-4 before slot entries).
const FIXED_DESTS: [Dest; 4] = [
    Dest { name: "Race Start",  x:  5.0, z:  -5.0, clearance: 1.0 },
    Dest { name: "Water Tower", x: -80.0, z: -80.0, clearance: 2.0 },
    Dest { name: "Lighthouse",  x:  90.0, z:  95.0, clearance: 2.0 },
    Dest { name: "Radio Tower", x: -95.0, z:  75.0, clearance: 2.0 },
];

const SLOT_NAMES: [&str; 4] = ["Slot F1", "Slot F2", "Slot F3", "Slot F4"];

/// Total number of rows in the destination list (4 fixed + 4 slots).
const DEST_COUNT: usize = 8;

/// Resolve the world position for the given destination index.
/// Returns `None` for a slot that has not been set.
fn resolve_dest(idx: usize, slots: &[Option<Vec3>; 4]) -> Option<Vec3> {
    if idx < FIXED_DESTS.len() {
        let d = &FIXED_DESTS[idx];
        let y = terrain_height_at(d.x, d.z) + d.clearance;
        Some(Vec3::new(d.x, y, d.z))
    } else {
        let slot_idx = idx - FIXED_DESTS.len();
        slots[slot_idx]
    }
}

fn dest_name(idx: usize) -> &'static str {
    if idx < FIXED_DESTS.len() {
        FIXED_DESTS[idx].name
    } else {
        SLOT_NAMES[idx - FIXED_DESTS.len()]
    }
}

// ---------------------------------------------------------------------------
// Component markers
// ---------------------------------------------------------------------------

/// Marks the full-screen backdrop root of the fast-travel modal.
#[derive(Component)]
struct FastTravelRoot;

/// Marks the individual destination row text nodes.
/// The `usize` is the row index (0-7).
#[derive(Component)]
struct DestRowText(usize);

// ---------------------------------------------------------------------------
// Colours
// ---------------------------------------------------------------------------

const MODAL_BG:   Color = Color::srgba(0.04, 0.04, 0.08, 0.95);
const OVERLAY_BG: Color = Color::srgba(0.0,  0.0,  0.0,  0.55);
const COL_TITLE:  Color = Color::srgb(1.0,  0.9,  0.2);   // yellow
const COL_SELECT: Color = Color::srgb(1.0,  0.9,  0.2);   // selected row: yellow
const COL_NORMAL: Color = Color::srgb(0.65, 0.65, 0.65);  // unselected: grey
const COL_DIM:    Color = Color::srgb(0.35, 0.35, 0.35);  // disabled slot: dim
const COL_FOOTER: Color = Color::srgb(0.45, 0.45, 0.50);  // footer hint

// ---------------------------------------------------------------------------
// Startup: build modal hierarchy
// ---------------------------------------------------------------------------

fn spawn_modal(mut commands: Commands) {
    // Full-screen dim backdrop — hidden by default.
    let root = commands
        .spawn((
            FastTravelRoot,
            Node {
                width:           Val::Percent(100.0),
                height:          Val::Percent(100.0),
                position_type:   PositionType::Absolute,
                align_items:     AlignItems::Center,
                justify_content: JustifyContent::Center,
                display:         Display::None,
                ..default()
            },
            BackgroundColor(OVERLAY_BG),
            ZIndex(200),
        ))
        .id();

    // Centered panel: 380 x 320.
    let panel = commands
        .spawn((
            Node {
                width:          Val::Px(380.0),
                height:         Val::Px(320.0),
                flex_direction: FlexDirection::Column,
                align_items:    AlignItems::Center,
                padding:        UiRect::all(Val::Px(18.0)),
                row_gap:        Val::Px(6.0),
                ..default()
            },
            BackgroundColor(MODAL_BG),
        ))
        .id();

    // Title.
    let title = commands
        .spawn((
            Text::new("FAST TRAVEL"),
            TextFont { font_size: 22.0, ..default() },
            TextColor(COL_TITLE),
            Node {
                margin: UiRect::bottom(Val::Px(8.0)),
                ..default()
            },
        ))
        .id();

    commands.entity(panel).add_child(title);

    // One row per destination.
    for i in 0..DEST_COUNT {
        let label = if i < FIXED_DESTS.len() {
            FIXED_DESTS[i].name.to_string()
        } else {
            SLOT_NAMES[i - FIXED_DESTS.len()].to_string()
        };

        let row = commands
            .spawn((
                DestRowText(i),
                Text::new(label),
                TextFont { font_size: 18.0, ..default() },
                TextColor(COL_NORMAL),
                Node {
                    width: Val::Percent(100.0),
                    ..default()
                },
            ))
            .id();

        commands.entity(panel).add_child(row);
    }

    // Footer hint.
    let footer = commands
        .spawn((
            Text::new("\u{2191}\u{2193}  Enter  Esc"),
            TextFont { font_size: 11.0, ..default() },
            TextColor(COL_FOOTER),
            Node {
                margin: UiRect::top(Val::Px(10.0)),
                ..default()
            },
        ))
        .id();

    commands.entity(panel).add_child(footer);
    commands.entity(root).add_child(panel);
}

// ---------------------------------------------------------------------------
// Update: toggle modal with F5
// ---------------------------------------------------------------------------

fn toggle_with_f5(
    keys:      Res<ButtonInput<KeyCode>>,
    mut state: ResMut<FastTravelMenuState>,
    mut root:  Query<&mut Node, With<FastTravelRoot>>,
) {
    if !keys.just_pressed(KeyCode::F5) {
        return;
    }

    state.open = !state.open;

    if state.open {
        state.cursor_idx = 0;
    }

    for mut node in &mut root {
        node.display = if state.open { Display::Flex } else { Display::None };
    }
}

// ---------------------------------------------------------------------------
// Update: Up/Down to move cursor; Esc to close
// ---------------------------------------------------------------------------

fn cycle_cursor(
    keys:      Res<ButtonInput<KeyCode>>,
    mut state: ResMut<FastTravelMenuState>,
    mut root:  Query<&mut Node, With<FastTravelRoot>>,
) {
    if !state.open {
        return;
    }

    if keys.just_pressed(KeyCode::Escape) {
        state.open = false;
        for mut node in &mut root {
            node.display = Display::None;
        }
        return;
    }

    if keys.just_pressed(KeyCode::ArrowUp) {
        if state.cursor_idx == 0 {
            state.cursor_idx = DEST_COUNT - 1;
        } else {
            state.cursor_idx -= 1;
        }
    }

    if keys.just_pressed(KeyCode::ArrowDown) {
        state.cursor_idx = (state.cursor_idx + 1) % DEST_COUNT;
    }
}

// ---------------------------------------------------------------------------
// Update: Enter to teleport
// ---------------------------------------------------------------------------

fn apply_teleport(
    keys:        Res<ButtonInput<KeyCode>>,
    mut state:   ResMut<FastTravelMenuState>,
    spawn_state: Res<SpawnPointsState>,
    vehicle:     Option<Res<VehicleRoot>>,
    mut chassis_q: Query<
        (&mut Transform, &mut LinearVelocity, &mut AngularVelocity),
        With<Chassis>,
    >,
    mut root: Query<&mut Node, With<FastTravelRoot>>,
) {
    if !state.open {
        return;
    }

    if !keys.just_pressed(KeyCode::Enter) {
        return;
    }

    let idx = state.cursor_idx;
    let dest = resolve_dest(idx, &spawn_state.slots);

    let Some(pos) = dest else {
        // Slot not set — nothing to do (no audio yet; just silently return).
        return;
    };

    let Some(vehicle) = vehicle else { return };
    let Ok((mut transform, mut lin_vel, mut ang_vel)) =
        chassis_q.get_mut(vehicle.chassis) else { return };

    transform.translation = pos;
    lin_vel.0  = Vec3::ZERO;
    ang_vel.0  = Vec3::ZERO;

    let name = dest_name(idx);
    info!("fast travel \u{2192} {}", name);

    // Close the menu.
    state.open = false;
    for mut node in &mut root {
        node.display = Display::None;
    }
}

// ---------------------------------------------------------------------------
// Update: refresh row colours to reflect cursor + slot availability
// ---------------------------------------------------------------------------

fn update_menu_view(
    state:       Res<FastTravelMenuState>,
    spawn_state: Res<SpawnPointsState>,
    mut rows:    Query<(&DestRowText, &mut TextColor)>,
) {
    for (row, mut color) in &mut rows {
        let i = row.0;

        // Determine if this row is available.
        let available = if i < FIXED_DESTS.len() {
            true
        } else {
            spawn_state.slots[i - FIXED_DESTS.len()].is_some()
        };

        *color = if i == state.cursor_idx && state.open {
            TextColor(COL_SELECT)
        } else if !available {
            TextColor(COL_DIM)
        } else {
            TextColor(COL_NORMAL)
        };
    }
}
