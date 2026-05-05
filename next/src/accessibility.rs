// Accessibility settings panel: colorblind mode, reduce-motion, HUD scale.
// Toggle panel with "0" (zero key).  Other modules read AccessibilityState
// directly — this module only manages state and its own UI.
//
// Public API:
//   AccessibilityPlugin
//   AccessibilityState (resource)
//   Scalable (marker component — HUD roots may opt in for hud_scale)
//   cb_swap(Color) -> Color

use bevy::prelude::*;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

pub struct AccessibilityPlugin;

impl Plugin for AccessibilityPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AccessibilityState>()
            .add_systems(Startup, spawn_panel)
            .add_systems(
                Update,
                (
                    toggle_with_zero,
                    cycle_options,
                    apply_hud_scale,
                    update_panel_view,
                )
                    .chain(),
            );
    }
}

/// Runtime accessibility state — readable by any system.
#[derive(Resource, Clone, Copy)]
pub struct AccessibilityState {
    /// Map red/green to deuteranopia-safe colours when true.
    pub colorblind: bool,
    /// Skip camera shake, speedlines, screen flashes when true.
    pub reduce_motion: bool,
    /// UI scale multiplier for opted-in `Scalable` roots (1.0, 1.25, 1.5).
    pub hud_scale: f32,
    // Internal: which menu row is selected (0, 1, 2).
    selected_row: usize,
    // Internal: panel open/closed.
    open: bool,
}

impl Default for AccessibilityState {
    fn default() -> Self {
        Self {
            colorblind: false,
            reduce_motion: false,
            hud_scale: 1.0,
            selected_row: 0,
            open: false,
        }
    }
}

/// Marker: attach to a top-level UI root node to opt in to HUD scaling.
/// Only nodes carrying this component will be resized by `apply_hud_scale`.
#[derive(Component)]
pub struct Scalable;

/// Deuteranopia-friendly colour helper.
/// Maps red → orange, green → blue when called from a colorblind context.
/// Leaves other colours unchanged.  The caller decides whether to invoke it
/// (typically by checking AccessibilityState.colorblind first).
pub fn cb_swap(c: Color) -> Color {
    let Srgba { red, green, blue, alpha } = c.to_srgba();

    // Detect "red-dominant": r > 0.5, r > g*2, r > b*2
    let is_red = red > 0.5 && red > green * 2.0 && red > blue * 2.0;
    // Detect "green-dominant": g > 0.5, g > r*2, g > b*2
    let is_green = green > 0.5 && green > red * 2.0 && green > blue * 2.0;

    if is_red {
        // red → orange (keep red channel, boost green, keep blue low)
        Color::srgba(red, red * 0.55, blue, alpha)
    } else if is_green {
        // green → blue (drop green, shift to blue)
        Color::srgba(red, green * 0.1, green * 0.9, alpha)
    } else {
        c
    }
}

// ---------------------------------------------------------------------------
// UI component markers
// ---------------------------------------------------------------------------

#[derive(Component)]
struct PanelRoot;

/// Which row's text entity this is.
#[derive(Component)]
enum RowText {
    Colorblind,
    ReduceMotion,
    HudScale,
}

// ---------------------------------------------------------------------------
// Colour constants
// ---------------------------------------------------------------------------

const OVERLAY_BG:   Color = Color::srgba(0.0,  0.0,  0.0,  0.55);
const PANEL_BG:     Color = Color::srgba(0.04, 0.04, 0.06, 0.92);
const COLOR_TITLE:  Color = Color::srgb(1.0,  0.9,  0.3);   // amber
const COLOR_NORMAL: Color = Color::srgb(0.82, 0.82, 0.82);
const COLOR_SEL:    Color = Color::srgb(1.0,  0.95, 0.2);   // yellow highlight
const COLOR_HINT:   Color = Color::srgb(0.5,  0.5,  0.5);

// HUD scale steps
const SCALE_STEPS: [f32; 3] = [1.0, 1.25, 1.5];

// ---------------------------------------------------------------------------
// Startup: build panel (hidden by default)
// ---------------------------------------------------------------------------

fn spawn_panel(mut commands: Commands) {
    // Full-screen dim backdrop.
    let root = commands
        .spawn((
            PanelRoot,
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
            ZIndex(120),
        ))
        .id();

    // Centred panel: 420 × 260 px.
    let panel = commands
        .spawn((
            Node {
                width:           Val::Px(420.0),
                height:          Val::Px(260.0),
                flex_direction:  FlexDirection::Column,
                align_items:     AlignItems::Stretch,
                padding:         UiRect::all(Val::Px(24.0)),
                row_gap:         Val::Px(16.0),
                ..default()
            },
            BackgroundColor(PANEL_BG),
        ))
        .id();

    // Title.
    let title = commands
        .spawn((
            Text::new("ACCESSIBILITY"),
            TextFont { font_size: 22.0, ..default() },
            TextColor(COLOR_TITLE),
            Node { margin: UiRect::bottom(Val::Px(4.0)), ..default() },
        ))
        .id();

    // Row 0 — colorblind
    let row0 = commands
        .spawn((
            RowText::Colorblind,
            Text::new(""),
            TextFont { font_size: 16.0, ..default() },
            TextColor(COLOR_SEL),
        ))
        .id();

    // Row 1 — reduce motion
    let row1 = commands
        .spawn((
            RowText::ReduceMotion,
            Text::new(""),
            TextFont { font_size: 16.0, ..default() },
            TextColor(COLOR_NORMAL),
        ))
        .id();

    // Row 2 — HUD scale
    let row2 = commands
        .spawn((
            RowText::HudScale,
            Text::new(""),
            TextFont { font_size: 16.0, ..default() },
            TextColor(COLOR_NORMAL),
        ))
        .id();

    // Hint footer.
    let hint = commands
        .spawn((
            Text::new("Up/Down: move row   Enter: toggle/cycle   0: close"),
            TextFont { font_size: 12.0, ..default() },
            TextColor(COLOR_HINT),
            Node {
                margin: UiRect::top(Val::Px(8.0)),
                ..default()
            },
        ))
        .id();

    commands.entity(panel).add_children(&[title, row0, row1, row2, hint]);
    commands.entity(root).add_child(panel);
}

// ---------------------------------------------------------------------------
// System: toggle panel with "0" (Digit0)
// ---------------------------------------------------------------------------

fn toggle_with_zero(
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<AccessibilityState>,
) {
    if keys.just_pressed(KeyCode::Digit0) {
        state.open = !state.open;
    }
}

// ---------------------------------------------------------------------------
// System: Up/Down to move row, Enter to toggle/cycle — only while open
// ---------------------------------------------------------------------------

fn cycle_options(
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<AccessibilityState>,
) {
    if !state.open {
        return;
    }

    // Row navigation.
    if keys.just_pressed(KeyCode::ArrowUp) {
        state.selected_row = state.selected_row.saturating_sub(1);
    }
    if keys.just_pressed(KeyCode::ArrowDown) {
        state.selected_row = (state.selected_row + 1).min(2);
    }

    // Activate selected row on Enter.
    if keys.just_pressed(KeyCode::Enter) {
        match state.selected_row {
            0 => state.colorblind = !state.colorblind,
            1 => state.reduce_motion = !state.reduce_motion,
            2 => {
                // Find current scale step and advance.
                let cur = SCALE_STEPS
                    .iter()
                    .position(|&s| (s - state.hud_scale).abs() < 0.01)
                    .unwrap_or(0);
                state.hud_scale = SCALE_STEPS[(cur + 1) % SCALE_STEPS.len()];
            }
            _ => {}
        }
    }
}

// ---------------------------------------------------------------------------
// System: apply hud_scale to opted-in Scalable roots
// ---------------------------------------------------------------------------

fn apply_hud_scale(
    state: Res<AccessibilityState>,
    mut scalable: Query<&mut Node, With<Scalable>>,
) {
    if !state.is_changed() {
        return;
    }
    for mut node in &mut scalable {
        node.width  = Val::Percent(state.hud_scale * 100.0);
        node.height = Val::Percent(state.hud_scale * 100.0);
    }
}

// ---------------------------------------------------------------------------
// System: show/hide panel and refresh row text + colours
// ---------------------------------------------------------------------------

fn update_panel_view(
    state: Res<AccessibilityState>,
    mut roots: Query<&mut Node, With<PanelRoot>>,
    mut rows: Query<(&RowText, &mut Text, &mut TextColor)>,
) {
    // Show or hide the backdrop.
    for mut node in &mut roots {
        node.display = if state.open { Display::Flex } else { Display::None };
    }

    if !state.open && !state.is_changed() {
        return;
    }

    for (row, mut text, mut color) in &mut rows {
        let selected = match row {
            RowText::Colorblind   => state.selected_row == 0,
            RowText::ReduceMotion => state.selected_row == 1,
            RowText::HudScale     => state.selected_row == 2,
        };

        color.0 = if selected { COLOR_SEL } else { COLOR_NORMAL };

        let prefix = if selected { "> " } else { "  " };

        match row {
            RowText::Colorblind => {
                text.0 = format!(
                    "{}Colorblind mode: {}",
                    prefix,
                    onoff(state.colorblind)
                );
            }
            RowText::ReduceMotion => {
                text.0 = format!(
                    "{}Reduce motion: {}",
                    prefix,
                    onoff(state.reduce_motion)
                );
            }
            RowText::HudScale => {
                text.0 = format!("{}HUD scale: {:.2}x", prefix, state.hud_scale);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tiny helper
// ---------------------------------------------------------------------------

#[inline]
fn onoff(v: bool) -> &'static str {
    if v { "ON" } else { "OFF" }
}
