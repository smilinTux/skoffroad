// Help overlay: press Shift+/ (?) to toggle a full-screen keybind reference.
//
// The entire text tree is static — built once at Startup, shown/hidden by flipping
// the root node's Display.  No per-frame update systems are needed.

use bevy::prelude::*;

// ---- Plugin -----------------------------------------------------------------

pub struct HelpPlugin;

impl Plugin for HelpPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_help_overlay)
           .add_systems(Update, toggle_help);
    }
}

// ---- Component markers ------------------------------------------------------

#[derive(Component)]
struct HelpRoot;

// ---- Colour constants (match settings.rs / hud.rs palette) -----------------

const OVERLAY_BG:  Color = Color::srgba(0.0,  0.0,  0.0,  0.72);
const PANEL_BG:    Color = Color::srgba(0.05, 0.05, 0.07, 0.92);
const COLOR_TITLE: Color = Color::srgb(1.0,  0.9,  0.3);   // amber
const COLOR_KEY:   Color = Color::srgb(0.95, 0.95, 0.95);  // near-white
const COLOR_DESC:  Color = Color::srgb(0.7,  0.7,  0.7);   // grey
const COLOR_HEAD:  Color = Color::srgb(0.55, 0.85, 0.95);  // section header tint

// ---- Startup: build the static overlay tree --------------------------------

fn spawn_help_overlay(mut commands: Commands) {
    // Full-screen dim backdrop — hidden by default.
    let root = commands
        .spawn((
            HelpRoot,
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
            ZIndex(100),
        ))
        .id();

    // Centred panel (~600 × auto height).
    let panel = commands
        .spawn((
            Node {
                width:          Val::Px(640.0),
                flex_direction: FlexDirection::Column,
                padding:        UiRect::all(Val::Px(24.0)),
                row_gap:        Val::Px(14.0),
                ..default()
            },
            BackgroundColor(PANEL_BG),
        ))
        .id();

    // Title row.
    let title = commands
        .spawn((
            Text::new("KEYBINDS  —  press ? to close"),
            TextFont { font_size: 22.0, ..default() },
            TextColor(COLOR_TITLE),
            Node { margin: UiRect::bottom(Val::Px(6.0)), ..default() },
        ))
        .id();

    // Two-column body.
    let columns = commands
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            column_gap:     Val::Px(24.0),
            ..default()
        })
        .id();

    let col_left  = build_column_left(&mut commands);
    let col_right = build_column_right(&mut commands);

    // Wire hierarchy.
    commands.entity(columns).add_children(&[col_left, col_right]);
    commands.entity(panel).add_children(&[title, columns]);
    commands.entity(root).add_child(panel);
}

// ---- Column builders --------------------------------------------------------

/// Column 1 — Driving + Camera
fn build_column_left(commands: &mut Commands) -> Entity {
    let col = commands
        .spawn(Node {
            width:          Val::Percent(50.0),
            flex_direction: FlexDirection::Column,
            row_gap:        Val::Px(4.0),
            ..default()
        })
        .id();

    let driving_header = section_header(commands, "DRIVING");
    let driving_rows   = [
        ("W / S",          "Throttle / reverse"),
        ("A / D",          "Steer left / right"),
        ("Arrow keys",     "Same as W/A/S/D"),
        ("Space",          "Brake"),
        ("Left Shift",     "Boost / nitrous"),
        ("R",              "Reset chassis to spawn"),
        ("J",              "Auto-flip recovery"),
        ("N",              "Horn"),
    ];

    let vehicle_header = section_header(commands, "VEHICLE");
    let vehicle_rows   = [
        ("1 - 5",          "Cycle paint livery"),
        ("\\",             "Cycle vehicle silhouette"),
        ("Y",              "Headlights (Shift+Y auto)"),
    ];

    let camera_header = section_header(commands, "CAMERA");
    let camera_rows   = [
        ("V",              "Toggle chase / cockpit"),
        ("Q / E",          "Orbit left / right (chase)"),
        ("RMB drag",       "Mouse orbit (chase)"),
        ("P",              "Photo mode"),
        (".",              "Replay last 10 s"),
        ("F12",            "Screenshot"),
    ];

    let mut children: Vec<Entity> = Vec::new();
    children.push(driving_header);
    for (k, d) in &driving_rows  { children.push(bind_row(commands, k, d)); }
    children.push(spacer(commands));
    children.push(vehicle_header);
    for (k, d) in &vehicle_rows  { children.push(bind_row(commands, k, d)); }
    children.push(spacer(commands));
    children.push(camera_header);
    for (k, d) in &camera_rows   { children.push(bind_row(commands, k, d)); }

    commands.entity(col).add_children(&children);
    col
}

/// Column 2 — UI / Save / System / Time / Settings
fn build_column_right(commands: &mut Commands) -> Entity {
    let col = commands
        .spawn(Node {
            width:          Val::Percent(50.0),
            flex_direction: FlexDirection::Column,
            row_gap:        Val::Px(4.0),
            ..default()
        })
        .id();

    let ui_header = section_header(commands, "UI / OVERLAYS");
    let ui_rows   = [
        ("H",              "Toggle HUD"),
        ("M",              "Toggle mini-map"),
        ("C",              "Toggle compass"),
        ("E",              "Toggle event log"),
        ("L",              "Toggle trial timer"),
        ("G",              "Toggle speedometer"),
        ("Z",              "Toggle wind indicator"),
        ("X",              "Toggle speed-line vignette"),
        ("F8 / F9",        "Perf / fuel toggle"),
        ("?",              "This help screen"),
        ("Tab (hold)",     "Stats screen"),
    ];

    let game_header = section_header(commands, "GAMEPLAY");
    let game_rows   = [
        ("B",              "Breadcrumbs (Shift+B clear)"),
        ("K",              "Skid marks (Shift+K clear)"),
        ("O",              "Toggle drone"),
    ];

    let sys_header = section_header(commands, "SYSTEM");
    let sys_rows   = [
        ("Esc",            "Pause / settings overlay"),
        ("F5 / F6 / F7",   "Save to slot 1 / 2 / 3"),
        ("F1 / F2 / F4",   "Load slot 1 / 2 / 3"),
        ("F3",             "Dev inspector (--features dev)"),
    ];

    let time_header = section_header(commands, "TIME");
    let time_rows   = [
        ("T",              "Pause day cycle"),
        ("[ / ]",          "Scrub time of day"),
    ];

    let cfg_header = section_header(commands, "SETTINGS (while paused)");
    let cfg_rows   = [
        ("- / =",          "Volume down / up"),
        (", / .",          "Sensitivity down / up"),
        ("; / '",          "Day length down / up"),
    ];

    let mut children: Vec<Entity> = Vec::new();
    children.push(ui_header);
    for (k, d) in &ui_rows  { children.push(bind_row(commands, k, d)); }
    children.push(spacer(commands));
    children.push(game_header);
    for (k, d) in &game_rows { children.push(bind_row(commands, k, d)); }
    children.push(spacer(commands));
    children.push(sys_header);
    for (k, d) in &sys_rows { children.push(bind_row(commands, k, d)); }
    children.push(spacer(commands));
    children.push(time_header);
    for (k, d) in &time_rows { children.push(bind_row(commands, k, d)); }
    children.push(spacer(commands));
    children.push(cfg_header);
    for (k, d) in &cfg_rows { children.push(bind_row(commands, k, d)); }

    commands.entity(col).add_children(&children);
    col
}

// ---- UI helpers -------------------------------------------------------------

/// A single key→description row.
fn bind_row(commands: &mut Commands, key: &str, desc: &str) -> Entity {
    let row = commands
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            column_gap:     Val::Px(8.0),
            ..default()
        })
        .id();

    // Fixed-width key label so descriptions line up.
    let key_node = commands
        .spawn((
            Text::new(key.to_string()),
            TextFont { font_size: 13.0, ..default() },
            TextColor(COLOR_KEY),
            Node { width: Val::Px(115.0), ..default() },
        ))
        .id();

    let desc_node = commands
        .spawn((
            Text::new(desc.to_string()),
            TextFont { font_size: 13.0, ..default() },
            TextColor(COLOR_DESC),
        ))
        .id();

    commands.entity(row).add_children(&[key_node, desc_node]);
    row
}

/// Bold section label (e.g. "DRIVING").
fn section_header(commands: &mut Commands, label: &str) -> Entity {
    commands
        .spawn((
            Text::new(label.to_string()),
            TextFont { font_size: 12.0, ..default() },
            TextColor(COLOR_HEAD),
            Node {
                margin: UiRect::top(Val::Px(4.0)),
                ..default()
            },
        ))
        .id()
}

/// A small vertical gap between section groups.
fn spacer(commands: &mut Commands) -> Entity {
    commands
        .spawn(Node {
            height: Val::Px(6.0),
            ..default()
        })
        .id()
}

// ---- Toggle system ----------------------------------------------------------

fn toggle_help(
    keys:     Res<ButtonInput<KeyCode>>,
    mut root: Query<&mut Node, With<HelpRoot>>,
) {
    // Shift + Slash = '?' on a standard US layout.
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if shift && keys.just_pressed(KeyCode::Slash) {
        for mut node in &mut root {
            node.display = match node.display {
                Display::None => Display::Flex,
                _             => Display::None,
            };
        }
    }
}
