// Title-screen overlay for SandK Offroad.
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

    // Large game title
    let title = commands
        .spawn((
            Text::new("SANDK OFFROAD"),
            TextFont { font_size: 72.0, ..default() },
            TextColor(COLOR_TITLE),
        ))
        .id();

    // Version / engine subtitle
    let subtitle = commands
        .spawn((
            Text::new("v0.4  —  A Bevy + Avian off-road sim"),
            TextFont { font_size: 18.0, ..default() },
            TextColor(COLOR_SUB),
        ))
        .id();

    // Spacer to push the prompt toward the bottom third
    let spacer = commands
        .spawn(Node {
            height: Val::Px(80.0),
            ..default()
        })
        .id();

    // Bottom prompt
    let prompt = commands
        .spawn((
            Text::new("Press SPACE or W to start.  Press ? for keybinds."),
            TextFont { font_size: 16.0, ..default() },
            TextColor(COLOR_HINT),
        ))
        .id();

    commands
        .entity(root)
        .add_children(&[title, subtitle, spacer, prompt]);
}

// ---------------------------------------------------------------------------
// Update: dismiss on any movement / confirm key
// ---------------------------------------------------------------------------

fn dismiss_title_screen(
    keys:      Res<ButtonInput<KeyCode>>,
    mut state: ResMut<MenuState>,
    mut roots: Query<&mut Node, With<TitleScreenRoot>>,
) {
    // Already gone — nothing to do.
    if state.dismissed {
        return;
    }

    let pressed = keys.just_pressed(KeyCode::Space)
        || keys.just_pressed(KeyCode::KeyW)
        || keys.just_pressed(KeyCode::Enter)
        || keys.just_pressed(KeyCode::ArrowUp)
        || keys.just_pressed(KeyCode::ArrowDown)
        || keys.just_pressed(KeyCode::ArrowLeft)
        || keys.just_pressed(KeyCode::ArrowRight);

    if pressed {
        state.dismissed = true;
        for mut node in &mut roots {
            node.display = Display::None;
        }
    }
}
