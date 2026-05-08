// Fast-travel spawn points: F1..F4 sets the slot to current chassis
// position. Pressing Shift+F1..F4 teleports back to that saved location.
// Persists to ~/.skoffroad/spawnpoints.json.
//
// Public API:
//   SpawnPointsPlugin
//   SpawnPointsState (resource)

use bevy::prelude::*;
use avian3d::prelude::{AngularVelocity, LinearVelocity};

use crate::platform_storage;
use crate::vehicle::{Chassis, VehicleRoot};

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

pub struct SpawnPointsPlugin;

impl Plugin for SpawnPointsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SpawnPointsState>()
            .insert_resource(SpawnDebounce::default())
            .add_systems(Startup, (load_slots, spawn_hud).chain())
            .add_systems(Update, (handle_keys, update_hud, save_on_change));
    }
}

/// Up to 4 saved chassis positions; None = empty slot.
#[derive(Resource, Default, Clone)]
pub struct SpawnPointsState {
    pub slots: [Option<Vec3>; 4],
}

// ---------------------------------------------------------------------------
// HUD marker components
// ---------------------------------------------------------------------------

/// Marker on each slot value text node so update_hud can target it.
#[derive(Component)]
struct SlotText(usize);

// ---------------------------------------------------------------------------
// Internal resources
// ---------------------------------------------------------------------------

#[derive(Resource, Default)]
struct SpawnDebounce {
    pending:   bool,
    elapsed_s: f32,
}

// ---------------------------------------------------------------------------
// Storage key
// ---------------------------------------------------------------------------

const STORAGE_KEY: &str = "spawnpoints.json";

fn spawnpoints_label() -> String {
    platform_storage::debug_path(STORAGE_KEY)
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| format!("localStorage[{}]", STORAGE_KEY))
}

// ---------------------------------------------------------------------------
// JSON serialisation (hand-rolled; serde_json is already in Cargo.toml)
// ---------------------------------------------------------------------------

fn to_json(slots: &[Option<Vec3>; 4]) -> String {
    let parts: Vec<String> = slots
        .iter()
        .map(|slot| match slot {
            Some(v) => format!("{{\"x\":{},\"y\":{},\"z\":{}}}", v.x, v.y, v.z),
            None => "null".to_string(),
        })
        .collect();
    format!("{{\"slots\":[{}]}}", parts.join(","))
}

fn from_json(src: &str) -> Option<[Option<Vec3>; 4]> {
    let v: serde_json::Value = serde_json::from_str(src).ok()?;
    let arr = v.as_object()?.get("slots")?.as_array()?;
    let mut slots: [Option<Vec3>; 4] = [None; 4];
    for (i, item) in arr.iter().enumerate().take(4) {
        if item.is_null() {
            slots[i] = None;
        } else if let Some(obj) = item.as_object() {
            let x = obj.get("x").and_then(|n| n.as_f64())? as f32;
            let y = obj.get("y").and_then(|n| n.as_f64())? as f32;
            let z = obj.get("z").and_then(|n| n.as_f64())? as f32;
            slots[i] = Some(Vec3::new(x, y, z));
        }
    }
    Some(slots)
}

// ---------------------------------------------------------------------------
// Startup: load slots from disk
// ---------------------------------------------------------------------------

fn load_slots(mut state: ResMut<SpawnPointsState>) {
    let label = spawnpoints_label();
    match platform_storage::read_string(STORAGE_KEY) {
        None => {
            info!(
                "spawn_points: no saved file at {}; starting empty",
                label,
            );
        }
        Some(text) => match from_json(&text) {
            None => {
                info!("spawn_points: could not parse {}; starting empty", label);
            }
            Some(slots) => {
                state.slots = slots;
                let filled = slots.iter().filter(|s| s.is_some()).count();
                info!(
                    "spawn_points: loaded {} slot(s) from {}",
                    filled,
                    label,
                );
            }
        },
    }
}

// ---------------------------------------------------------------------------
// Startup: spawn HUD panels (4 small 60x40 panels stacked top-left)
// ---------------------------------------------------------------------------

const PANEL_BG: Color = Color::srgba(0.05, 0.05, 0.07, 0.75);
const PANEL_W: f32 = 60.0;
const PANEL_H: f32 = 40.0;
const PANEL_TOP: f32 = 14.0;
const PANEL_LEFT: f32 = 14.0;
const PANEL_GAP: f32 = 4.0;

fn spawn_hud(mut commands: Commands, state: Res<SpawnPointsState>) {
    for i in 0..4usize {
        let top_offset = PANEL_TOP + (i as f32) * (PANEL_H + PANEL_GAP);
        let label = format!("F{}", i + 1);
        let val = slot_display(state.slots[i]);

        let panel = commands
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    top:  Val::Px(top_offset),
                    left: Val::Px(PANEL_LEFT),
                    width:  Val::Px(PANEL_W),
                    height: Val::Px(PANEL_H),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    padding: UiRect::all(Val::Px(2.0)),
                    ..default()
                },
                BackgroundColor(PANEL_BG),
            ))
            .id();

        let key_node = commands
            .spawn((
                Text::new(label),
                TextFont { font_size: 10.0, ..default() },
                TextColor(Color::srgb(0.6, 0.6, 0.6)),
            ))
            .id();

        let val_node = commands
            .spawn((
                SlotText(i),
                Text::new(val),
                TextFont { font_size: 11.0, ..default() },
                TextColor(Color::WHITE),
            ))
            .id();

        commands.entity(panel).add_children(&[key_node, val_node]);
    }
}

// ---------------------------------------------------------------------------
// Update: handle F1..F4 (save) and Shift+F1..F4 (teleport)
// ---------------------------------------------------------------------------

fn handle_keys(
    keys: Res<ButtonInput<KeyCode>>,
    vehicle: Option<Res<VehicleRoot>>,
    mut chassis_q: Query<
        (&mut Transform, &mut LinearVelocity, &mut AngularVelocity),
        With<Chassis>,
    >,
    mut state: ResMut<SpawnPointsState>,
) {
    let Some(vehicle) = vehicle else { return };
    let Ok((mut transform, mut lin_vel, mut ang_vel)) =
        chassis_q.get_mut(vehicle.chassis) else { return };

    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);

    let fkeys = [KeyCode::F1, KeyCode::F2, KeyCode::F3, KeyCode::F4];
    for (n, &fkey) in fkeys.iter().enumerate() {
        if !keys.just_pressed(fkey) {
            continue;
        }

        if shift {
            // Teleport chassis to saved slot.
            if let Some(pos) = state.slots[n] {
                transform.translation = pos;
                lin_vel.0 = Vec3::ZERO;
                ang_vel.0 = Vec3::ZERO;
                info!(
                    "F{}: teleported to {:.0},{:.0},{:.0}",
                    n + 1,
                    pos.x,
                    pos.y,
                    pos.z
                );
            }
        } else {
            // Save current chassis position.
            let pos = transform.translation;
            state.slots[n] = Some(pos);
            info!(
                "F{}: saved {:.0},{:.0},{:.0}",
                n + 1,
                pos.x,
                pos.y,
                pos.z
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Update: refresh slot text labels each frame
// ---------------------------------------------------------------------------

fn update_hud(
    state: Res<SpawnPointsState>,
    mut text_q: Query<(&SlotText, &mut Text)>,
) {
    for (slot_text, mut text) in text_q.iter_mut() {
        **text = slot_display(state.slots[slot_text.0]);
    }
}

fn slot_display(slot: Option<Vec3>) -> String {
    match slot {
        None => "\u{2014}".to_string(), // em-dash "—"
        Some(v) => format!("{},{}", v.x.round() as i32, v.z.round() as i32),
    }
}

// ---------------------------------------------------------------------------
// Update: persist to disk whenever state changes (debounced 0.5 s)
// ---------------------------------------------------------------------------

fn save_on_change(
    state: Res<SpawnPointsState>,
    mut deb: ResMut<SpawnDebounce>,
    time: Res<Time>,
) {
    if state.is_changed() {
        deb.pending   = true;
        deb.elapsed_s = 0.0;
        return;
    }

    if !deb.pending {
        return;
    }

    deb.elapsed_s += time.delta_secs();
    if deb.elapsed_s < 0.5 {
        return;
    }

    // Timer expired — write.
    deb.pending   = false;
    deb.elapsed_s = 0.0;

    let json = to_json(&state.slots);
    let label = spawnpoints_label();

    match platform_storage::write_string(STORAGE_KEY, &json) {
        Err(e) => warn!("spawn_points: {}", e),
        Ok(()) => info!("spawn_points: saved to {}", label),
    }
}
