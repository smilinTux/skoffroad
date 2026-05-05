// Map selection UI: Tab key opens a centered list of maps; arrow keys cycle,
// Enter applies (triggers a transition + ActiveMap swap), Tab/Esc closes.
//
// Public API:
//   MapSelectPlugin

use bevy::prelude::*;

use crate::maps::{map_catalog, ActiveMap};
use crate::transition::TransitionRequest;

// ---- Plugin ------------------------------------------------------------------

pub struct MapSelectPlugin;

impl Plugin for MapSelectPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MapSelectState>()
            .init_resource::<MapSelectOpen>()
            .add_systems(Startup, spawn_map_select_panel)
            .add_systems(
                Update,
                (
                    toggle_with_tab,
                    cycle_selection,
                    apply_selection,
                    update_panel_view,
                )
                    .chain(),
            );
    }
}

// ---- Resources ---------------------------------------------------------------

/// Internal state: whether the panel is open and which catalog row is focused.
#[derive(Resource, Default)]
pub struct MapSelectState {
    pub open: bool,
    pub cursor_idx: usize,
}

/// Observable resource: other systems (course, race, etc.) can read this to
/// detect when the map-select overlay is active.
#[derive(Resource, Default)]
pub struct MapSelectOpen(pub bool);

// ---- Component markers -------------------------------------------------------

#[derive(Component)]
struct MapSelectRoot;

#[derive(Component)]
struct MapSelectRow(usize);

#[derive(Component)]
struct MapSelectRowPrefix(#[allow(dead_code)] usize);

// ---- Colour constants --------------------------------------------------------

const PANEL_BG:     Color = Color::srgba(0.04, 0.04, 0.08, 0.95);
const COLOR_TITLE:  Color = Color::srgb(1.0, 0.9, 0.3);
const COLOR_SEL:    Color = Color::srgb(1.0, 0.9, 0.3);
const COLOR_NORMAL: Color = Color::srgb(0.7, 0.7, 0.7);
const COLOR_FOOTER: Color = Color::srgb(0.5, 0.5, 0.55);

// ---- Startup: build modal panel (hidden) -------------------------------------

fn spawn_map_select_panel(mut commands: Commands) {
    // Full-screen transparent backdrop — hidden by default.
    let root = commands
        .spawn((
            MapSelectRoot,
            Node {
                width:           Val::Percent(100.0),
                height:          Val::Percent(100.0),
                position_type:   PositionType::Absolute,
                align_items:     AlignItems::Center,
                justify_content: JustifyContent::Center,
                display:         Display::None,
                ..default()
            },
            ZIndex(200),
        ))
        .id();

    // Centred 480x320 dark panel.
    let panel = commands
        .spawn((
            Node {
                width:          Val::Px(480.0),
                height:         Val::Px(320.0),
                flex_direction: FlexDirection::Column,
                align_items:    AlignItems::Center,
                padding:        UiRect::all(Val::Px(24.0)),
                row_gap:        Val::Px(10.0),
                ..default()
            },
            BackgroundColor(PANEL_BG),
        ))
        .id();

    // Title.
    let title = commands
        .spawn((
            Text::new("SELECT MAP"),
            TextFont { font_size: 22.0, ..default() },
            TextColor(COLOR_TITLE),
            Node {
                margin: UiRect::bottom(Val::Px(8.0)),
                ..default()
            },
        ))
        .id();

    // Catalog rows — one per entry in map_catalog().
    let catalog = map_catalog();
    let mut row_entities: Vec<Entity> = Vec::with_capacity(catalog.len());

    for (idx, entry) in catalog.iter().enumerate() {
        // Row container (horizontal).
        let row = commands
            .spawn((
                MapSelectRow(idx),
                Node {
                    width:          Val::Px(420.0),
                    flex_direction: FlexDirection::Row,
                    column_gap:     Val::Px(6.0),
                    ..default()
                },
            ))
            .id();

        // Prefix: "[ ]" or "[*]" — updated each frame via update_panel_view.
        let prefix = commands
            .spawn((
                MapSelectRowPrefix(idx),
                Text::new("[ ]"),
                TextFont { font_size: 15.0, ..default() },
                TextColor(COLOR_NORMAL),
                Node { width: Val::Px(34.0), ..default() },
            ))
            .id();

        // Row label: "> NAME — description" or "  NAME — description".
        let label_text = format!("  {} \u{2014} {}", entry.name, entry.description);
        let label = commands
            .spawn((
                Text::new(label_text),
                TextFont { font_size: 15.0, ..default() },
                TextColor(COLOR_NORMAL),
            ))
            .id();

        commands.entity(row).add_children(&[prefix, label]);
        row_entities.push(row);
    }

    // Footer hint.
    let footer = commands
        .spawn((
            Text::new("\u{2191}\u{2193} select   ENTER apply   TAB close"),
            TextFont { font_size: 12.0, ..default() },
            TextColor(COLOR_FOOTER),
            Node {
                margin: UiRect::top(Val::Px(12.0)),
                ..default()
            },
        ))
        .id();

    // Wire hierarchy.
    let mut panel_children: Vec<Entity> = Vec::new();
    panel_children.push(title);
    panel_children.extend_from_slice(&row_entities);
    panel_children.push(footer);

    commands.entity(panel).add_children(&panel_children);
    commands.entity(root).add_child(panel);
}

// ---- Toggle system -----------------------------------------------------------

fn toggle_with_tab(
    keys:      Res<ButtonInput<KeyCode>>,
    mut state: ResMut<MapSelectState>,
    mut open:  ResMut<MapSelectOpen>,
    active:    Res<ActiveMap>,
    mut roots: Query<&mut Node, With<MapSelectRoot>>,
) {
    if keys.just_pressed(KeyCode::Tab) {
        state.open = !state.open;

        if state.open {
            // Initialise cursor to the currently-active map's catalog position.
            let catalog = map_catalog();
            state.cursor_idx = catalog
                .iter()
                .position(|m| m.kind == active.0)
                .unwrap_or(0);
        }

        open.0 = state.open;

        for mut node in &mut roots {
            node.display = if state.open { Display::Flex } else { Display::None };
        }
    }
}

// ---- Cycle selection ---------------------------------------------------------

fn cycle_selection(
    keys:      Res<ButtonInput<KeyCode>>,
    mut state: ResMut<MapSelectState>,
) {
    if !state.open {
        return;
    }

    let len = map_catalog().len();

    if keys.just_pressed(KeyCode::ArrowUp) {
        state.cursor_idx = if state.cursor_idx == 0 { len - 1 } else { state.cursor_idx - 1 };
    }
    if keys.just_pressed(KeyCode::ArrowDown) {
        state.cursor_idx = (state.cursor_idx + 1) % len;
    }
}

// ---- Apply selection ---------------------------------------------------------

fn apply_selection(
    keys:         Res<ButtonInput<KeyCode>>,
    mut state:    ResMut<MapSelectState>,
    mut open:     ResMut<MapSelectOpen>,
    active:       Res<ActiveMap>,
    mut commands: Commands,
    mut roots:    Query<&mut Node, With<MapSelectRoot>>,
) {
    if !state.open {
        return;
    }

    if keys.just_pressed(KeyCode::Enter) {
        let catalog = map_catalog();
        let chosen = catalog[state.cursor_idx].kind;
        if chosen != active.0 {
            commands.trigger(TransitionRequest { target: chosen });
        }
        state.open = false;
        open.0 = false;
        for mut node in &mut roots {
            node.display = Display::None;
        }
        return;
    }

    // Tab is handled by toggle_with_tab; Esc closes without applying.
    if keys.just_pressed(KeyCode::Escape) {
        state.open = false;
        open.0 = false;
        for mut node in &mut roots {
            node.display = Display::None;
        }
    }
}

// ---- Update panel view -------------------------------------------------------

/// Refreshes every row's text colour and active/cursor markers each frame.
fn update_panel_view(
    state:        Res<MapSelectState>,
    active:       Res<ActiveMap>,
    rows:         Query<(&MapSelectRow, &Children)>,
    prefix_query: Query<&MapSelectRowPrefix>,
    mut texts:    Query<&mut Text>,
    mut colors:   Query<&mut TextColor>,
) {
    let catalog = map_catalog();

    for (row_marker, children) in &rows {
        let idx = row_marker.0;
        let entry = &catalog[idx];

        let is_cursor = idx == state.cursor_idx;
        let is_active = entry.kind == active.0;

        // Children layout: [prefix_entity, label_entity]
        if children.len() < 2 {
            continue;
        }
        let prefix_entity = children[0];
        let label_entity  = children[1];

        // Update prefix text ("[*]" or "[ ]").
        if prefix_query.get(prefix_entity).is_ok() {
            if let Ok(mut text) = texts.get_mut(prefix_entity) {
                **text = if is_active { "[*]".to_string() } else { "[ ]".to_string() };
            }
        }

        // Update cursor indicator and colour in label.
        if let Ok(mut text) = texts.get_mut(label_entity) {
            let cursor_char = if is_cursor { ">" } else { " " };
            **text = format!("{} {} \u{2014} {}", cursor_char, entry.name, entry.description);
        }

        // Row highlight colour.
        let row_color = if is_cursor { COLOR_SEL } else { COLOR_NORMAL };

        if let Ok(mut color) = colors.get_mut(prefix_entity) {
            *color = TextColor(row_color);
        }
        if let Ok(mut color) = colors.get_mut(label_entity) {
            *color = TextColor(row_color);
        }
    }
}
