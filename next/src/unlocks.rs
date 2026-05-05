// Unlocks: gates content (vehicle variants, decals, race tracks, features)
// behind ProgressionState.level. Exposes UnlockState resource and a
// UnlocksHud panel showing the next unlock.
//
// Public API:
//   UnlocksPlugin
//   UnlockState { unlocked: HashSet<Unlockable> }
//   Unlockable enum

use bevy::prelude::*;
use std::collections::HashSet;

use crate::progression::ProgressionState;

// ---- Plugin -----------------------------------------------------------------

pub struct UnlocksPlugin;

impl Plugin for UnlocksPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<UnlockState>()
            .init_resource::<ToastQueue>()
            .add_systems(Startup, spawn_unlocks_panel)
            .add_systems(
                Update,
                (
                    check_unlocks,
                    show_toast,
                    tick_toast_timer,
                    toggle_panel_with_l,
                    update_panel_rows,
                ),
            );
    }
}

// ---- Public types -----------------------------------------------------------

#[derive(Resource, Default, Clone)]
pub struct UnlockState {
    pub unlocked: HashSet<Unlockable>,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Unlockable {
    VariantBronco,
    VariantPickup,
    VariantHummer,
    VariantBuggy,
    NightDriving,
    PhotoMode,
    DroneCam,
    BoostBoost,
}

// ---- Threshold table --------------------------------------------------------

struct UnlockEntry {
    item:  Unlockable,
    level: u32,
    name:  &'static str,
}

const UNLOCK_TABLE: &[UnlockEntry] = &[
    UnlockEntry { item: Unlockable::VariantBronco, level:  2, name: "Bronco"        },
    UnlockEntry { item: Unlockable::VariantPickup, level:  4, name: "Pickup"        },
    UnlockEntry { item: Unlockable::PhotoMode,     level:  6, name: "Photo Mode"    },
    UnlockEntry { item: Unlockable::NightDriving,  level:  8, name: "Night Driving" },
    UnlockEntry { item: Unlockable::VariantHummer, level: 10, name: "Hummer"        },
    UnlockEntry { item: Unlockable::DroneCam,      level: 14, name: "Drone Cam"     },
    UnlockEntry { item: Unlockable::VariantBuggy,  level: 18, name: "Buggy"         },
    UnlockEntry { item: Unlockable::BoostBoost,    level: 25, name: "Boost Boost"   },
];

/// Human-readable display name for an unlockable item.
pub fn display_name(item: &Unlockable) -> &'static str {
    UNLOCK_TABLE
        .iter()
        .find(|e| &e.item == item)
        .map(|e| e.name)
        .unwrap_or("Unknown")
}

// ---- Toast queue resource ---------------------------------------------------

/// Internal resource that bridges `check_unlocks` → `show_toast` /
/// `tick_toast_timer`.  Not part of the public API.
#[derive(Resource, Default)]
struct ToastQueue {
    /// Messages waiting to be displayed.
    pending: Vec<String>,
    /// Currently visible toast: (message, seconds_remaining).
    active: Option<(String, f32)>,
}

// ---- Toast constants --------------------------------------------------------

const TOAST_DURATION_S: f32 = 3.0;
const TOAST_FADE_IN_S:  f32 = 0.3;
const TOAST_FADE_OUT_S: f32 = 0.5;
const TOAST_COLOR:      Color = Color::srgb(0.4, 1.0, 0.5);
const TOAST_FONT_SIZE:  f32  = 32.0;

// ---- Panel constants --------------------------------------------------------

const PANEL_W:          f32   = 360.0;
const PANEL_H:          f32   = 320.0;
const PANEL_RIGHT:      f32   = 14.0;
const PANEL_TOP:        f32   = 80.0;
const PANEL_BG:         Color = Color::srgba(0.04, 0.04, 0.06, 0.92);
const ROW_LOCKED_CLR:   Color = Color::srgb(0.50, 0.50, 0.55);
const ROW_UNLOCKED_CLR: Color = Color::srgb(0.40, 1.00, 0.50);
const PANEL_TITLE_CLR:  Color = Color::srgb(1.00, 0.90, 0.30);

// ---- Component markers ------------------------------------------------------

/// Root node for the unlocks side-panel (toggled with L key).
#[derive(Component)]
struct UnlocksPanelRoot;

/// Text node inside the panel; carries the `Unlockable` variant it represents.
#[derive(Component, Clone, Copy)]
struct UnlockRowText(Unlockable);

/// The single mid-screen toast text node.
#[derive(Component)]
struct UnlockToastNode;

// ---- Startup: spawn HUD elements -------------------------------------------

fn spawn_unlocks_panel(mut commands: Commands) {
    // ---- Side panel (top-right, hidden) ------------------------------------
    let root = commands
        .spawn((
            UnlocksPanelRoot,
            Node {
                position_type:   PositionType::Absolute,
                right:           Val::Px(PANEL_RIGHT),
                top:             Val::Px(PANEL_TOP),
                width:           Val::Px(PANEL_W),
                height:          Val::Px(PANEL_H),
                flex_direction:  FlexDirection::Column,
                padding:         UiRect::all(Val::Px(14.0)),
                row_gap:         Val::Px(6.0),
                display:         Display::None,
                ..default()
            },
            BackgroundColor(PANEL_BG),
            ZIndex(200),
        ))
        .id();

    // Title.
    let title = commands
        .spawn((
            Text::new("UNLOCKS  (L to close)"),
            TextFont { font_size: 14.0, ..default() },
            TextColor(PANEL_TITLE_CLR),
            Node { margin: UiRect::bottom(Val::Px(6.0)), ..default() },
        ))
        .id();
    commands.entity(root).add_child(title);

    // One row per entry in the threshold table.
    for entry in UNLOCK_TABLE {
        let label = format!("[ ] {} — Lv{}", entry.name, entry.level);
        let row = commands
            .spawn((
                UnlockRowText(entry.item),
                Text::new(label),
                TextFont { font_size: 13.0, ..default() },
                TextColor(ROW_LOCKED_CLR),
            ))
            .id();
        commands.entity(root).add_child(row);
    }

    // ---- Mid-screen toast node (invisible at rest) -------------------------
    commands.spawn((
        UnlockToastNode,
        Text::new(String::new()),
        TextFont { font_size: TOAST_FONT_SIZE, ..default() },
        TextColor(Color::srgba(0.4, 1.0, 0.5, 0.0)),
        Node {
            position_type: PositionType::Absolute,
            left:          Val::Percent(50.0),
            top:           Val::Percent(40.0),
            ..default()
        },
        ZIndex(400),
    ));
}

// ---- check_unlocks ----------------------------------------------------------

/// Each frame compare every threshold against `ProgressionState.level`.
/// Newly eligible items are inserted into `UnlockState` and a toast message
/// is pushed to `ToastQueue.pending`.
fn check_unlocks(
    progression: Option<Res<ProgressionState>>,
    mut state:   ResMut<UnlockState>,
    mut tq:      ResMut<ToastQueue>,
) {
    let level = progression.as_ref().map(|p| p.level).unwrap_or(0);

    for entry in UNLOCK_TABLE {
        if level >= entry.level && !state.unlocked.contains(&entry.item) {
            state.unlocked.insert(entry.item);
            tq.pending.push(format!("UNLOCKED: {}", entry.name));
        }
    }
}

// ---- show_toast -------------------------------------------------------------

/// Pops the next pending message when no toast is currently active and sets
/// the toast text node to full opacity.
fn show_toast(
    mut tq:      ResMut<ToastQueue>,
    mut toast_q: Query<(&mut Text, &mut TextColor), With<UnlockToastNode>>,
) {
    if tq.active.is_some() {
        return;
    }

    if let Some(msg) = tq.pending.first().cloned() {
        tq.pending.remove(0);
        tq.active = Some((msg.clone(), TOAST_DURATION_S));
        for (mut text, mut color) in &mut toast_q {
            *text  = Text::new(msg.clone());
            color.0 = TOAST_COLOR;
        }
    } else {
        // Keep invisible when the queue is empty.
        for (_, mut color) in &mut toast_q {
            color.0 = Color::srgba(0.4, 1.0, 0.5, 0.0);
        }
    }
}

// ---- tick_toast_timer -------------------------------------------------------

/// Advances the active toast timer and drives the fade-in / hold / fade-out
/// alpha curve.  Clears the active slot when the timer expires.
fn tick_toast_timer(
    time:        Res<Time>,
    mut tq:      ResMut<ToastQueue>,
    mut toast_q: Query<(&mut Text, &mut TextColor), With<UnlockToastNode>>,
) {
    let dt = time.delta_secs();
    let Some((_, ref mut remaining)) = tq.active else { return };

    *remaining -= dt;
    let elapsed = TOAST_DURATION_S - *remaining;

    let alpha = if elapsed < TOAST_FADE_IN_S {
        elapsed / TOAST_FADE_IN_S
    } else if *remaining > TOAST_FADE_OUT_S {
        1.0_f32
    } else {
        (*remaining / TOAST_FADE_OUT_S).max(0.0)
    };

    for (_, mut color) in &mut toast_q {
        color.0 = Color::srgba(0.4, 1.0, 0.5, alpha);
    }

    if *remaining <= 0.0 {
        tq.active = None;
        for (mut text, mut color) in &mut toast_q {
            *text  = Text::new(String::new());
            color.0 = Color::srgba(0.4, 1.0, 0.5, 0.0);
        }
    }
}

// ---- toggle_panel_with_l ----------------------------------------------------

fn toggle_panel_with_l(
    keys:       Res<ButtonInput<KeyCode>>,
    mut root_q: Query<&mut Node, With<UnlocksPanelRoot>>,
) {
    if keys.just_pressed(KeyCode::KeyL) {
        for mut node in &mut root_q {
            node.display = match node.display {
                Display::None => Display::Flex,
                _             => Display::None,
            };
        }
    }
}

// ---- update_panel_rows ------------------------------------------------------

/// Refreshes row labels and colours whenever `UnlockState` changes.
fn update_panel_rows(
    state:    Res<UnlockState>,
    mut rows: Query<(&UnlockRowText, &mut Text, &mut TextColor)>,
) {
    if !state.is_changed() {
        return;
    }

    for (row_tag, mut text, mut color) in &mut rows {
        let item = row_tag.0;
        let Some(entry) = UNLOCK_TABLE.iter().find(|e| e.item == item) else {
            continue;
        };

        if state.unlocked.contains(&item) {
            *text  = Text::new(format!("[X] {} — Lv{}", entry.name, entry.level));
            color.0 = ROW_UNLOCKED_CLR;
        } else {
            *text  = Text::new(format!("[ ] {} — Lv{}", entry.name, entry.level));
            color.0 = ROW_LOCKED_CLR;
        }
    }
}
