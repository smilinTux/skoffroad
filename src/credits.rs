// Credits roll: K-toggleable scrolling text panel listing contributors,
// crates used, and a thank-you to playtesters.
//
// Public API:
//   CreditsPlugin

use bevy::prelude::*;

// ---- Plugin -----------------------------------------------------------------

pub struct CreditsPlugin;

impl Plugin for CreditsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CreditsState>()
            .add_systems(Startup, spawn_credits_overlay)
            .add_systems(Update, (toggle_with_k, auto_scroll, update_scroll_position).chain());
    }
}

// ---- Resources --------------------------------------------------------------

#[derive(Resource)]
pub struct CreditsState {
    pub open:     bool,
    pub scroll_y: f32,
}

impl Default for CreditsState {
    fn default() -> Self {
        Self { open: false, scroll_y: 0.0 }
    }
}

// ---- Component markers ------------------------------------------------------

/// Marks the full-screen overlay root.
#[derive(Component)]
struct CreditsRoot;

/// Marks the inner scrolling column whose `top` offset is driven by scroll_y.
#[derive(Component)]
struct CreditsColumn;

// ---- Colour constants -------------------------------------------------------

const OVERLAY_BG:   Color = Color::srgba(0.0, 0.0, 0.0, 0.85);
const COLOR_NORMAL: Color = Color::srgb(1.0, 1.0, 1.0);
const COLOR_HEADER: Color = Color::srgb(0.4, 0.95, 1.0);

// ---- Credits content --------------------------------------------------------

const CREDITS: &[&str] = &[
    "--- skoffroad — S&K OFFROAD ---",
    "",
    "Built with Claude Code + Sonnet subagents",
    "",
    "--- ENGINE ---",
    "bevy 0.18",
    "avian3d 0.6",
    "bevy_kira_audio 0.25",
    "bevy_hanabi 0.18",
    "noise 0.9",
    "",
    "--- VEHICLES ---",
    "Jeep TJ",
    "Ford Bronco",
    "Pickup Truck",
    "Hummer H1",
    "Sand Buggy",
    "",
    "--- BIOMES ---",
    "Valley",
    "Dunes",
    "Canyon",
    "",
    "--- GAMEPLAY SYSTEMS ---",
    "Off-road physics & suspension",
    "Terrain deformation & skid marks",
    "Particle effects & dust plumes",
    "Dynamic weather & sky",
    "Career progression & unlocks",
    "Daily challenges",
    "Rival AI racing",
    "Photo mode & replay",
    "",
    "--- THANKS ---",
    "Playtesters: chefboyrdave2.1",
    "",
    "Press K to close",
];

/// Estimated pixel height of the credits column (line count × row height).
/// Used to detect when the scroll has passed the end and should loop.
const ROW_HEIGHT_PX: f32 = 28.0;
const SCROLL_SPEED:  f32 = 30.0; // px / sec

// ---- Startup: build the overlay tree ----------------------------------------

fn spawn_credits_overlay(mut commands: Commands) {
    // Full-screen dim backdrop — hidden by default.
    let root = commands
        .spawn((
            CreditsRoot,
            Node {
                width:           Val::Percent(100.0),
                height:          Val::Percent(100.0),
                position_type:   PositionType::Absolute,
                align_items:     AlignItems::Center,
                justify_content: JustifyContent::FlexStart,
                overflow:        Overflow::clip(),
                display:         Display::None,
                ..default()
            },
            BackgroundColor(OVERLAY_BG),
            ZIndex(200),
        ))
        .id();

    // Scrolling inner column — positioned absolutely so we can drive its `top`.
    // Starts below the visible area (top = 100%) so text scrolls up into view.
    let column = commands
        .spawn((
            CreditsColumn,
            Node {
                position_type:  PositionType::Absolute,
                flex_direction: FlexDirection::Column,
                align_items:    AlignItems::Center,
                width:          Val::Percent(100.0),
                top:            Val::Percent(100.0),
                row_gap:        Val::Px(10.0),
                padding:        UiRect::all(Val::Px(16.0)),
                ..default()
            },
        ))
        .id();

    // Build one Text entity per credits line.
    let mut line_entities: Vec<Entity> = Vec::with_capacity(CREDITS.len());
    for &line in CREDITS {
        let is_header = line.starts_with("---");
        let is_empty  = line.is_empty();

        if is_empty {
            // Blank spacer row.
            let spacer = commands
                .spawn(Node {
                    height: Val::Px(ROW_HEIGHT_PX * 0.5),
                    ..default()
                })
                .id();
            line_entities.push(spacer);
        } else {
            let color = if is_header { COLOR_HEADER } else { COLOR_NORMAL };
            let size  = if is_header { 20.0 } else { 18.0 };
            let entity = commands
                .spawn((
                    Text::new(line),
                    TextFont { font_size: size, ..default() },
                    TextColor(color),
                ))
                .id();
            line_entities.push(entity);
        }
    }

    commands.entity(column).add_children(&line_entities);
    commands.entity(root).add_child(column);
}

// ---- Toggle system ----------------------------------------------------------

fn toggle_with_k(
    keys:      Res<ButtonInput<KeyCode>>,
    mut state: ResMut<CreditsState>,
    mut roots: Query<&mut Node, With<CreditsRoot>>,
) {
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);

    // K (without Shift, to avoid clashing with Shift+K skidmark-clear).
    let toggle = !shift && keys.just_pressed(KeyCode::KeyK);
    // Esc closes only.
    let close  = keys.just_pressed(KeyCode::Escape);

    if !toggle && !close {
        return;
    }

    if close && !state.open {
        return;
    }

    if toggle {
        state.open = !state.open;
    } else {
        // Esc always closes.
        state.open   = false;
        state.scroll_y = 0.0;
    }

    let display = if state.open { Display::Flex } else { Display::None };
    for mut node in &mut roots {
        node.display = display;
    }
}

// ---- Auto-scroll system -----------------------------------------------------

fn auto_scroll(
    time:  Res<Time>,
    mut state: ResMut<CreditsState>,
) {
    if !state.open {
        return;
    }

    state.scroll_y += SCROLL_SPEED * time.delta_secs();

    // Total column height estimate: one slot per line, half-slot per blank.
    // We add a full screen's worth (assume 800 px) so the text has time to
    // fully scroll off the top before looping.
    let col_height = CREDITS.len() as f32 * ROW_HEIGHT_PX + 800.0;

    if state.scroll_y > col_height {
        state.scroll_y = 0.0;
    }
}

// ---- Apply scroll to UI node ------------------------------------------------

fn update_scroll_position(
    state:   Res<CreditsState>,
    mut cols: Query<&mut Node, With<CreditsColumn>>,
) {
    if !state.is_changed() {
        return;
    }

    for mut node in &mut cols {
        // Column starts at the bottom of the screen (top = 100%) and scrolls up.
        // Using Percent(100) - Px(scroll_y) keeps the math simple and avoids
        // needing to know the actual window height.  Once scroll_y exceeds the
        // column length the auto_scroll system resets it to 0.
        let start_px = 800.0_f32; // approximate screen height as starting offset
        node.top = Val::Px(start_px - state.scroll_y);
    }
}
