// Title-screen overlay for skoffroad.
//
// Shown at startup as a full-screen opaque panel. Dismissed by the player via
// Space, W, Enter, or any arrow key. Physics runs beneath it; the player just
// can't see (or accidentally control) the world yet.
//
// A real Bevy States machine is intentionally deferred to a future version.
// This approach avoids touching every other plugin with state-gating.

use bevy::prelude::*;

// ---------------------------------------------------------------------------
// Public plugin
// ---------------------------------------------------------------------------

pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MenuState>()
            .add_systems(Startup, spawn_title_screen)
            .add_systems(Update, dismiss_title_screen);
    }
}

// ---------------------------------------------------------------------------
// Public resource
// ---------------------------------------------------------------------------

/// Tracks whether the title screen has been dismissed.
///
/// Other systems (audio, input) may read `dismissed` to suppress behaviour
/// while the title screen is up — though v0.4 does not enforce this yet.
#[derive(Resource, Default)]
pub struct MenuState {
    pub dismissed: bool,
}

// ---------------------------------------------------------------------------
// Component markers
// ---------------------------------------------------------------------------

#[derive(Component)]
struct TitleScreenRoot;

#[derive(Component)]
struct StartButton;

// ---------------------------------------------------------------------------
// Colour constants
// ---------------------------------------------------------------------------

const TITLE_BG:    Color = Color::srgba(0.05, 0.07, 0.12, 1.0);
const COLOR_TITLE: Color = Color::srgb(1.0, 0.85, 0.3);
const COLOR_SUB:   Color = Color::srgb(0.78, 0.78, 0.82);
const COLOR_HINT:  Color = Color::srgb(0.55, 0.60, 0.65);

// ---------------------------------------------------------------------------
// Startup: build title-screen tree
// ---------------------------------------------------------------------------

fn spawn_title_screen(mut commands: Commands) {
    // Full-screen opaque layer. Sits above HUD and game world.
    // ZIndex is not needed — Bevy renders UI nodes in tree order and this
    // overlay is spawned last, so it naturally appears on top.
    let root = commands
        .spawn((
            TitleScreenRoot,
            Node {
                width:           Val::Percent(100.0),
                height:          Val::Percent(100.0),
                position_type:   PositionType::Absolute,
                flex_direction:  FlexDirection::Column,
                align_items:     AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap:         Val::Px(16.0),
                ..default()
            },
            BackgroundColor(TITLE_BG),
        ))
        .id();

    // Large game title — lowercase wordmark with the brand subtitle just below.
    let title = commands
        .spawn((
            Text::new("skoffroad"),
            TextFont { font_size: 84.0, ..default() },
            TextColor(COLOR_TITLE),
        ))
        .id();

    // Brand subtitle — the white-label public-facing name.
    let brand = commands
        .spawn((
            Text::new("S&K  OFFROAD"),
            TextFont { font_size: 22.0, ..default() },
            TextColor(Color::srgb(0.85, 0.78, 0.55)),
            Node { margin: UiRect::top(Val::Px(-6.0)), ..default() },
        ))
        .id();

    // Version / engine subtitle — auto-pulled from CARGO_PKG_VERSION at build time.
    let subtitle = commands
        .spawn((
            Text::new(format!("v{}  —  A Bevy + Avian off-road sim", env!("CARGO_PKG_VERSION"))),
            TextFont { font_size: 16.0, ..default() },
            TextColor(COLOR_SUB),
        ))
        .id();

    // Spacer to push the keybinds toward the middle
    let spacer1 = commands
        .spawn(Node { height: Val::Px(28.0), ..default() })
        .id();

    // Two-column keybind grid
    let kb_root = commands
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            column_gap:     Val::Px(48.0),
            ..default()
        })
        .id();

    let col_left  = build_kb_column(&mut commands, &[
        ("DRIVING", ""),
        ("W A S D", "Throttle / Steer"),
        ("Space",   "Brake"),
        ("Shift",   "Boost / nitrous"),
        ("R",       "Reset to spawn"),
        ("J",       "Auto-flip recovery"),
        ("N",       "Horn"),
        ("U",       "Winch retract (recovery)"),
        ("",        ""),
        ("VEHICLE & MODS", ""),
        ("Shift+M", "Mods panel (long-arm/tires/bumper/winch)"),
        ("1-5",     "Cycle paint livery"),
        ("\\",      "Cycle silhouette (5 trucks)"),
        ("Y",       "Headlights (Shift+Y auto)"),
        ("",        ""),
        ("CAMERA", ""),
        ("V",       "Chase / cockpit"),
        ("Q E / RMB", "Orbit (chase)"),
        ("P",       "Photo mode"),
        (".",       "Replay last 10 s"),
        ("F12",     "Screenshot"),
        ("O",       "Toggle drone"),
    ]);
    let col_right = build_kb_column(&mut commands, &[
        ("HUD", ""),
        ("H M C E", "HUD / map / compass / events"),
        ("Shift+M", "Mods panel"),
        ("L",       "Hillclimb leaderboard"),
        ("G Z X",   "Speedo / wind / speedlines"),
        ("Tab (hold)", "Stats screen"),
        ("?",       "Keybind help overlay"),
        ("",        ""),
        ("MULTIPLAYER", ""),
        ("I",       "Peers panel + buddy recovery"),
        ("F",       "Voice PTT (Shift+F = always-on)"),
        ("Q",       "Webcam toggle"),
        ("",        ""),
        ("CUSTOM MAPS (drag onto canvas)", ""),
        (".png",    "Heightmap terrain"),
        (".glb",    "GLB scene (Polycam/Luma)"),
        (".gpx",    "GPS trail overlay"),
        ("",        ""),
        ("WORLD & SAVE", ""),
        ("T  [ ]",  "Pause / scrub time of day"),
        ("F5/F6/F7", "Save slots 1/2/3"),
        ("F1/F2/F4", "Load slots 1/2/3"),
        ("Esc",     "Pause / settings"),
    ]);
    commands.entity(kb_root).add_children(&[col_left, col_right]);

    let spacer2 = commands
        .spawn(Node { height: Val::Px(28.0), ..default() })
        .id();

    // Big tap/click target for mobile and click users; also dismissable via keys.
    let start_btn = commands
        .spawn((
            Button,
            StartButton,
            Node {
                width:           Val::Px(260.0),
                height:          Val::Px(64.0),
                align_items:     AlignItems::Center,
                justify_content: JustifyContent::Center,
                border:          UiRect::all(Val::Px(2.0)),
                ..default()
            },
            BorderColor::all(Color::srgb(1.0, 0.85, 0.30)),
            BackgroundColor(Color::srgba(0.20, 0.16, 0.05, 0.95)),
        ))
        .id();
    let start_label = commands
        .spawn((
            Text::new("▶  TAP / PRESS TO START"),
            TextFont { font_size: 22.0, ..default() },
            TextColor(Color::srgb(1.0, 0.92, 0.55)),
        ))
        .id();
    commands.entity(start_btn).add_children(&[start_label]);

    // Bottom prompt
    let prompt = commands
        .spawn((
            Text::new("Or press SPACE / W / Enter / Arrow.  Press ? in-game for full keybinds."),
            TextFont { font_size: 14.0, ..default() },
            TextColor(COLOR_HINT),
        ))
        .id();

    commands
        .entity(root)
        .add_children(&[title, brand, subtitle, spacer1, kb_root, spacer2, start_btn, prompt]);
}

/// Build a vertical column of (key, description) rows for the title screen.
/// A row with empty description is rendered as a section header (amber, larger).
fn build_kb_column(commands: &mut Commands, rows: &[(&str, &str)]) -> Entity {
    let col = commands
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            row_gap:        Val::Px(2.0),
            ..default()
        })
        .id();

    let mut children: Vec<Entity> = Vec::with_capacity(rows.len());
    for (key, desc) in rows {
        if key.is_empty() && desc.is_empty() {
            // Visual gap between sections.
            children.push(commands.spawn(Node { height: Val::Px(8.0), ..default() }).id());
            continue;
        }
        if desc.is_empty() {
            // Section header.
            let h = commands.spawn((
                Text::new(*key),
                TextFont { font_size: 13.0, ..default() },
                TextColor(Color::srgb(0.95, 0.72, 0.20)),
            )).id();
            children.push(h);
            continue;
        }
        // Row: key + description.
        let row = commands.spawn(Node {
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(8.0),
            ..default()
        }).id();
        let k = commands.spawn((
            Text::new(*key),
            TextFont { font_size: 12.0, ..default() },
            TextColor(Color::srgb(0.92, 0.92, 0.95)),
            Node { width: Val::Px(110.0), ..default() },
        )).id();
        let d = commands.spawn((
            Text::new(*desc),
            TextFont { font_size: 12.0, ..default() },
            TextColor(Color::srgb(0.65, 0.68, 0.72)),
        )).id();
        commands.entity(row).add_children(&[k, d]);
        children.push(row);
    }

    commands.entity(col).add_children(&children);
    col
}

// ---------------------------------------------------------------------------
// Update: dismiss on any movement / confirm key
// ---------------------------------------------------------------------------

fn dismiss_title_screen(
    keys:      Res<ButtonInput<KeyCode>>,
    mouse:     Res<ButtonInput<MouseButton>>,
    btn_q:     Query<&Interaction, (Changed<Interaction>, With<StartButton>)>,
    mut state: ResMut<MenuState>,
    mut roots: Query<&mut Node, With<TitleScreenRoot>>,
) {
    // Already gone — nothing to do.
    if state.dismissed {
        return;
    }

    let key_pressed = keys.just_pressed(KeyCode::Space)
        || keys.just_pressed(KeyCode::KeyW)
        || keys.just_pressed(KeyCode::Enter)
        || keys.just_pressed(KeyCode::ArrowUp)
        || keys.just_pressed(KeyCode::ArrowDown)
        || keys.just_pressed(KeyCode::ArrowLeft)
        || keys.just_pressed(KeyCode::ArrowRight);

    // Mobile / click: tap on the start button (Interaction goes Pressed) OR
    // a left-mouse click anywhere on the screen.
    let btn_pressed = btn_q.iter().any(|i| matches!(i, Interaction::Pressed));
    let mouse_clicked = mouse.just_pressed(MouseButton::Left);

    let pressed = key_pressed || btn_pressed || mouse_clicked;

    if pressed {
        state.dismissed = true;
        for mut node in &mut roots {
            node.display = Display::None;
        }
    }
}
