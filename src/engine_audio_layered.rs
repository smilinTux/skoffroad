// Multi-layer engine audio HUD — Sprint 39.
//
// Procedural visual approach: since bevy_kira_audio requires asset files for
// runtime audio, we implement the layer-blending logic as a HUD overlay that
// visualises the three audio layers (Idle / Cruise / Redline) as animated
// horizontal bars whose widths track the RPM-driven volume weights.
//
// Layer volumes (RPM-driven):
//   idle_vol    = max(0, 1 - rpm / 2000.0)
//   cruise_vol  = gaussian peak at 3500 RPM, σ=1500
//   redline_vol = max(0, (rpm - 4500) / 2500.0).min(1.0)
//
// HUD position: top-right corner.  Three rows, each row = label + bar.
// Bar width = layer_vol * MAX_BAR_PX.

use bevy::prelude::*;
use crate::engine_torque::EngineState;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Maximum bar width in logical pixels (100% volume).
const MAX_BAR_PX: f32 = 200.0;

/// Width of the panel holding all three rows.
const PANEL_W: f32 = 280.0;

/// Height of a single layer row (label + bar).
const ROW_H: f32 = 22.0;

/// Vertical gap between rows.
const ROW_GAP: f32 = 6.0;

/// Total panel height: 3 rows + 2 gaps + top/bottom padding (10 px each).
const PANEL_H: f32 = ROW_H * 3.0 + ROW_GAP * 2.0 + 20.0;

// ---------------------------------------------------------------------------
// Public plugin
// ---------------------------------------------------------------------------

pub struct EngineAudioLayeredPlugin;

impl Plugin for EngineAudioLayeredPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_audio_bars_hud)
           .add_systems(Update, update_audio_bars_hud);
    }
}

// ---------------------------------------------------------------------------
// Layer enum & math
// ---------------------------------------------------------------------------

/// The three blended engine audio layers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Layer {
    Idle,
    Cruise,
    Redline,
}

impl Layer {
    /// Display label.
    fn label(self) -> &'static str {
        match self {
            Layer::Idle    => "Idle",
            Layer::Cruise  => "Cruise",
            Layer::Redline => "Redline",
        }
    }

    /// Bar fill colour.
    fn color(self) -> Color {
        match self {
            Layer::Idle    => Color::srgb(0.20, 0.45, 0.85),  // blue
            Layer::Cruise  => Color::srgb(0.20, 0.80, 0.25),  // green
            Layer::Redline => Color::srgb(0.90, 0.18, 0.18),  // red
        }
    }

    /// Compute the [0, 1] volume weight for this layer given an RPM value.
    fn volume(self, rpm: f32) -> f32 {
        match self {
            Layer::Idle => {
                // Linear falloff: full at 0 RPM, zero at 2000 RPM.
                (1.0 - rpm / 2000.0).max(0.0)
            }
            Layer::Cruise => {
                // Gaussian centred at 3500 RPM, σ = 1500 RPM.
                let z = (rpm - 3500.0) / 1500.0;
                (-z * z).exp()
            }
            Layer::Redline => {
                // Linear ramp: zero at 4500 RPM, full at 7000 RPM.
                ((rpm - 4500.0) / 2500.0).clamp(0.0, 1.0)
            }
        }
    }
}

// ---------------------------------------------------------------------------
// HUD components
// ---------------------------------------------------------------------------

/// Marks the root panel entity.
#[derive(Component)]
struct AudioBarsRoot;

/// Marks a bar-fill entity.  The `layer` field lets the update system find
/// the correct fill without a separate query per layer.
#[derive(Component)]
struct AudioBarFill {
    layer: Layer,
}

// ---------------------------------------------------------------------------
// Startup system: build the HUD hierarchy
// ---------------------------------------------------------------------------

fn spawn_audio_bars_hud(mut commands: Commands) {
    // Root panel — top-right corner.
    let panel = commands.spawn((
        AudioBarsRoot,
        Node {
            position_type:   PositionType::Absolute,
            right:           Val::Px(12.0),
            top:             Val::Px(12.0),
            width:           Val::Px(PANEL_W),
            height:          Val::Px(PANEL_H),
            flex_direction:  FlexDirection::Column,
            justify_content: JustifyContent::FlexStart,
            padding: UiRect::all(Val::Px(10.0)),
            row_gap:         Val::Px(ROW_GAP),
            display:         Display::Flex,
            ..default()
        },
        BackgroundColor(Color::srgba(0.04, 0.04, 0.06, 0.82)),
    )).id();

    // One row per layer.
    for layer in [Layer::Idle, Layer::Cruise, Layer::Redline] {
        let row = spawn_layer_row(&mut commands, layer);
        commands.entity(panel).add_child(row);
    }
}

/// Spawns a single label + bar row for `layer`; returns the row entity id.
fn spawn_layer_row(commands: &mut Commands, layer: Layer) -> Entity {
    // Row container (horizontal flex).
    let row = commands.spawn((
        Node {
            width:           Val::Percent(100.0),
            height:          Val::Px(ROW_H),
            flex_direction:  FlexDirection::Row,
            align_items:     AlignItems::Center,
            column_gap:      Val::Px(8.0),
            display:         Display::Flex,
            ..default()
        },
    )).id();

    // Label.
    let label = commands.spawn((
        Text::new(layer.label()),
        TextFont { font_size: 11.0, ..default() },
        TextColor(layer.color()),
        Node {
            width: Val::Px(50.0),
            ..default()
        },
    )).id();

    // Bar track (background).
    let track = commands.spawn((
        Node {
            width:  Val::Px(MAX_BAR_PX),
            height: Val::Px(8.0),
            ..default()
        },
        BackgroundColor(Color::srgba(0.15, 0.15, 0.15, 1.0)),
    )).id();

    // Bar fill — width driven by volume each frame.
    let fill = commands.spawn((
        AudioBarFill { layer },
        Node {
            width:  Val::Px(0.0),   // starts empty; update system sets this
            height: Val::Percent(100.0),
            ..default()
        },
        BackgroundColor(layer.color()),
    )).id();

    commands.entity(track).add_child(fill);
    commands.entity(row).add_children(&[label, track]);

    row
}

// ---------------------------------------------------------------------------
// Update system: drive bar widths from EngineState::rpm
// ---------------------------------------------------------------------------

fn update_audio_bars_hud(
    engine: Option<Res<EngineState>>,
    mut bar_q: Query<(&AudioBarFill, &mut Node)>,
) {
    // If EngineState hasn't been inserted yet (e.g. early frames, headless),
    // leave bars at their default zero width.
    let rpm = match engine {
        Some(ref e) => e.rpm,
        None        => return,
    };

    for (fill, mut node) in &mut bar_q {
        let vol = fill.layer.volume(rpm);
        node.width = Val::Px((vol * MAX_BAR_PX).clamp(0.0, MAX_BAR_PX));
    }
}
