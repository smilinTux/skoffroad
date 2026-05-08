// In-game changelog viewer: ";"-toggleable list of version notes, scrollable
// with arrow keys. Static content embedded at compile time.
//
// Public API:
//   ChangelogPlugin

use bevy::prelude::*;

// ---- Plugin -----------------------------------------------------------------

pub struct ChangelogPlugin;

impl Plugin for ChangelogPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ChangelogState>()
            .add_systems(Startup, spawn_changelog_panel)
            .add_systems(
                Update,
                (toggle_with_semicolon, scroll_with_arrows, update_scroll_position).chain(),
            );
    }
}

// ---- Resources --------------------------------------------------------------

#[derive(Resource)]
pub struct ChangelogState {
    pub open:     bool,
    pub scroll_y: f32,
}

impl Default for ChangelogState {
    fn default() -> Self {
        Self { open: false, scroll_y: 0.0 }
    }
}

// ---- Component markers ------------------------------------------------------

/// Marks the full-screen transparent backdrop root.
#[derive(Component)]
struct ChangelogRoot;

/// Marks the inner scrolling column whose `top` offset is driven by scroll_y.
#[derive(Component)]
struct ChangelogColumn;

// ---- Colour constants -------------------------------------------------------

const PANEL_BG:     Color = Color::srgba(0.04, 0.04, 0.08, 0.95);
const COLOR_TITLE:  Color = Color::srgb(1.0, 0.9, 0.3);
const COLOR_HEADER: Color = Color::srgb(0.4, 0.95, 1.0);
const COLOR_BODY:   Color = Color::srgb(1.0, 1.0, 1.0);
const COLOR_FOOTER: Color = Color::srgb(0.45, 0.45, 0.5);

// ---- Static changelog content -----------------------------------------------

/// Each entry is either a section header (starts with "---") or a body line.
/// Empty strings become small vertical spacers.
const CHANGELOG: &[&str] = &[
    "--- v0.4.19 — Sprint 18 ---",
    "* Persistent config (~/.skoffroad/config.json)",
    "* Loading splash screen",
    "* Credits roll (K)",
    "* Centralized theme palette",
    "* Font assets seam",
    "",
    "--- v0.4.18 — Sprint 17 ---",
    "* Multiple maps: VALLEY / DUNES / CANYON",
    "* Desert biome: cacti + amber fog",
    "* Canyon biome: red rock pillars + dusty haze",
    "* Tab map-select modal",
    "* 1s black fade transition",
    "",
    "--- v0.4.17 — Sprint 16 ---",
    "* Progression XP curve (1..50)",
    "* Unlocks (8 tiers)",
    "* Career mode (8 objectives)",
    "* Daily challenge",
    "* Bronze/silver/gold medals",
    "",
    "--- v0.4.16 — Sprint 15 ---",
    "* AI rivals (RED/GRN/BLU)",
    "* Race state machine + countdown",
    "* Top-right leaderboard",
    "",
    "--- v0.4.15 — Sprint 14 ---",
    "* Procedural music with state machine",
    "* 4-cylinder engine synth layer",
    "* Surface-aware tire audio",
    "* World ambient (wind/birds/crickets)",
    "* Master mix with ducking",
    "* Photo-referenced vehicle silhouettes",
    "",
    "--- v0.3.0 ---",
    "* Major refactor: particles, weather, post-FX",
    "",
    "--- v0.2.0 ---",
    "* Audio system + weather effects",
    "",
    "--- v0.1.0 ---",
    "* Initial Bevy 0.18 port from 0.12",
];

/// Height per content row in the scrolling column (px).
const ROW_HEIGHT_PX: f32 = 24.0;
/// Amount to scroll per arrow-key press (px).
const SCROLL_STEP: f32 = 30.0;

// ---- Startup: build modal panel (hidden) ------------------------------------

fn spawn_changelog_panel(mut commands: Commands) {
    // Full-screen transparent backdrop — hidden by default.
    let root = commands
        .spawn((
            ChangelogRoot,
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

    // Centered 600x420 dark modal panel.
    let panel = commands
        .spawn((
            Node {
                width:          Val::Px(600.0),
                height:         Val::Px(420.0),
                flex_direction: FlexDirection::Column,
                align_items:    AlignItems::Center,
                padding:        UiRect::all(Val::Px(20.0)),
                row_gap:        Val::Px(0.0),
                overflow:       Overflow::clip(),
                ..default()
            },
            BackgroundColor(PANEL_BG),
        ))
        .id();

    // Fixed title at top.
    let title = commands
        .spawn((
            Text::new("CHANGELOG"),
            TextFont { font_size: 22.0, ..default() },
            TextColor(COLOR_TITLE),
            Node {
                margin: UiRect::bottom(Val::Px(12.0)),
                ..default()
            },
        ))
        .id();

    // Scrollable content column — position driven by update_scroll_position.
    let column = commands
        .spawn((
            ChangelogColumn,
            Node {
                position_type:  PositionType::Absolute,
                flex_direction: FlexDirection::Column,
                align_items:    AlignItems::FlexStart,
                width:          Val::Px(560.0),
                top:            Val::Px(60.0), // below the title area
                left:           Val::Px(20.0),
                row_gap:        Val::Px(4.0),
                ..default()
            },
        ))
        .id();

    // Build one Text entity per changelog line.
    let mut line_entities: Vec<Entity> = Vec::with_capacity(CHANGELOG.len());
    for &line in CHANGELOG {
        let is_header = line.starts_with("---");

        if line.is_empty() {
            // Blank spacer.
            let spacer = commands
                .spawn(Node {
                    height: Val::Px(ROW_HEIGHT_PX * 0.4),
                    ..default()
                })
                .id();
            line_entities.push(spacer);
        } else {
            let color     = if is_header { COLOR_HEADER } else { COLOR_BODY };
            let font_size = if is_header { 16.0 } else { 14.0 };
            let entity = commands
                .spawn((
                    Text::new(line),
                    TextFont { font_size, ..default() },
                    TextColor(color),
                ))
                .id();
            line_entities.push(entity);
        }
    }

    commands.entity(column).add_children(&line_entities);

    // Fixed footer at bottom of panel.
    let footer = commands
        .spawn((
            Text::new("\u{2191}\u{2193} scroll   ; close"),
            TextFont { font_size: 12.0, ..default() },
            TextColor(COLOR_FOOTER),
            Node {
                position_type: PositionType::Absolute,
                bottom:        Val::Px(14.0),
                ..default()
            },
        ))
        .id();

    // Wire hierarchy.
    commands.entity(panel).add_children(&[title, column, footer]);
    commands.entity(root).add_child(panel);
}

// ---- Toggle system ----------------------------------------------------------

fn toggle_with_semicolon(
    keys:      Res<ButtonInput<KeyCode>>,
    mut state: ResMut<ChangelogState>,
    mut roots: Query<&mut Node, With<ChangelogRoot>>,
) {
    let toggle = keys.just_pressed(KeyCode::Semicolon);
    let close  = keys.just_pressed(KeyCode::Escape);

    if !toggle && !close {
        return;
    }

    // Esc only closes if the panel is currently open.
    if close && !state.open {
        return;
    }

    if toggle {
        state.open = !state.open;
    } else {
        // Esc always closes and resets scroll.
        state.open     = false;
        state.scroll_y = 0.0;
    }

    let display = if state.open { Display::Flex } else { Display::None };
    for mut node in &mut roots {
        node.display = display;
    }
}

// ---- Scroll system ----------------------------------------------------------

fn scroll_with_arrows(
    keys:      Res<ButtonInput<KeyCode>>,
    mut state: ResMut<ChangelogState>,
) {
    if !state.open {
        return;
    }

    // Estimate max scrollable distance: total rows * row height minus visible area height.
    let content_height = CHANGELOG.len() as f32 * ROW_HEIGHT_PX;
    let visible_height = 420.0 - 60.0 - 40.0; // panel height minus title + footer
    let max_scroll = (content_height - visible_height).max(0.0);

    if keys.just_pressed(KeyCode::ArrowDown) {
        state.scroll_y = (state.scroll_y + SCROLL_STEP).min(max_scroll);
    }
    if keys.just_pressed(KeyCode::ArrowUp) {
        state.scroll_y = (state.scroll_y - SCROLL_STEP).max(0.0);
    }
}

// ---- Apply scroll to UI node ------------------------------------------------

fn update_scroll_position(
    state:    Res<ChangelogState>,
    mut cols: Query<&mut Node, With<ChangelogColumn>>,
) {
    if !state.is_changed() {
        return;
    }

    for mut node in &mut cols {
        // The column starts at top=60 (below title). Scrolling moves it upward.
        node.top = Val::Px(60.0 - state.scroll_y);
    }
}
