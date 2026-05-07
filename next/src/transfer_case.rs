// Transfer case selector: 2HI → 4HI → 4LO → 2HI (KeyV, 0.4 s cooldown).
//
// Sits between transmission and axles. Controls torque split to front axle
// and a final-drive multiplier that multiplies effective gear torque in 4LO.
//
// Architecture:
//   cycle_transfer_mode   (Update)  — KeyV cycles mode with 0.4 s cooldown
//   update_split_values   (Update)  — sync torque_split_front + final_drive_multiplier
//   display_hud           (Startup) — spawn top-left text + green dot indicator
//   refresh_hud           (Update)  — update text content and dot visibility
//
// Public API:
//   TransferCasePlugin
//   TransferCase  (resource)
//   TransferMode  (enum)

use bevy::prelude::*;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Key that cycles through transfer modes.
const CYCLE_KEY: KeyCode = KeyCode::KeyV;

/// Minimum seconds between mode changes.
const COOLDOWN_SECS: f32 = 0.4;

/// Final-drive multiplier in 4LO (simulates ~2.5× torque through low-range
/// transfer gears). 2HI and 4HI use 1.0.
const FINAL_DRIVE_4LO: f32 = 2.5;

/// Fraction of torque sent to the front axle when 4WD is engaged.
const SPLIT_4WD: f32 = 0.5;

// ---------------------------------------------------------------------------
// Colours
// ---------------------------------------------------------------------------

const WHITE:     Color = Color::srgb(0.93, 0.93, 0.93);
const AMBER:     Color = Color::srgb(0.95, 0.75, 0.10);
const RED_LO:    Color = Color::srgb(0.95, 0.25, 0.10);
const GREEN_DOT: Color = Color::srgb(0.15, 0.90, 0.35);

// ---------------------------------------------------------------------------
// Public enum + resource
// ---------------------------------------------------------------------------

/// Operating mode of the transfer case.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum TransferMode {
    /// Two-wheel drive, high range. Default road mode.
    #[default]
    TwoHi,
    /// Four-wheel drive, high range. Traction on loose surfaces.
    FourHi,
    /// Four-wheel drive, low range. Maximum torque, reduced speed.
    FourLo,
}

impl TransferMode {
    /// Human-readable HUD label.
    pub fn label(self) -> &'static str {
        match self {
            TransferMode::TwoHi  => "[2HI]",
            TransferMode::FourHi => "[4HI]",
            TransferMode::FourLo => "[4LO]",
        }
    }

    /// Advance to the next mode in the cycle.
    pub fn next(self) -> Self {
        match self {
            TransferMode::TwoHi  => TransferMode::FourHi,
            TransferMode::FourHi => TransferMode::FourLo,
            TransferMode::FourLo => TransferMode::TwoHi,
        }
    }

    /// Returns true when 4WD is engaged (either hi or lo range).
    pub fn is_4wd(self) -> bool {
        matches!(self, TransferMode::FourHi | TransferMode::FourLo)
    }
}

impl std::fmt::Display for TransferMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

/// Central transfer-case state exposed as a Bevy resource.
///
/// Downstream systems (e.g. vehicle physics) should read `torque_split_front`
/// and `final_drive_multiplier` to apply the appropriate forces. This resource
/// is always present after `TransferCasePlugin` is added.
#[derive(Resource)]
pub struct TransferCase {
    /// Current selector mode.
    pub mode: TransferMode,

    /// Fraction of drive torque directed to the front axle.
    /// `0.0` in `TwoHi` (rear-wheel only), `0.5` in `FourHi`/`FourLo`.
    pub torque_split_front: f32,

    /// Multiplier applied to effective gear torque.
    /// `1.0` in `TwoHi`/`FourHi`, `2.5` in `FourLo`.
    pub final_drive_multiplier: f32,

    /// Seconds remaining before the next mode change is accepted.
    pub(crate) cooldown: f32,
}

impl Default for TransferCase {
    fn default() -> Self {
        Self {
            mode:                   TransferMode::TwoHi,
            torque_split_front:     0.0,
            final_drive_multiplier: 1.0,
            cooldown:               0.0,
        }
    }
}

// ---------------------------------------------------------------------------
// HUD marker components
// ---------------------------------------------------------------------------

/// Marks the text node that displays the current mode label.
#[derive(Component)]
struct XferModeText;

/// Marks the small green dot shown when 4WD is active.
#[derive(Component)]
struct XferActiveDot;

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct TransferCasePlugin;

impl Plugin for TransferCasePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TransferCase>()
            .add_systems(Startup, spawn_xfer_hud)
            .add_systems(
                Update,
                (
                    cycle_transfer_mode,
                    update_split_values,
                    refresh_hud,
                )
                    .chain(),
            );
    }
}

// ---------------------------------------------------------------------------
// Startup: spawn HUD nodes
// ---------------------------------------------------------------------------

/// Spawns a small top-left panel that shows the current transfer-case mode.
///
/// Positioning: `left: 20 px`, `top: 50 px` — sits just below the
/// transmission gear text (which typically occupies ~0–44 px at the top).
fn spawn_xfer_hud(mut commands: Commands) {
    // Outer panel
    let panel = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left:   Val::Px(20.0),
                top:    Val::Px(50.0),
                height: Val::Px(26.0),
                align_items: AlignItems::Center,
                padding: UiRect {
                    left:   Val::Px(8.0),
                    right:  Val::Px(8.0),
                    top:    Val::Px(3.0),
                    bottom: Val::Px(3.0),
                },
                column_gap: Val::Px(6.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.04, 0.04, 0.06, 0.82)),
        ))
        .id();

    // Mode label text
    let label = commands
        .spawn((
            XferModeText,
            Text::new("[2HI]"),
            TextFont { font_size: 13.0, ..default() },
            TextColor(WHITE),
        ))
        .id();

    // Green activity dot (hidden by default; shown when 4WD active)
    let dot = commands
        .spawn((
            XferActiveDot,
            Node {
                width:  Val::Px(8.0),
                height: Val::Px(8.0),
                ..default()
            },
            BackgroundColor(GREEN_DOT),
            Visibility::Hidden,
        ))
        .id();

    commands.entity(panel).add_children(&[label, dot]);
}

// ---------------------------------------------------------------------------
// System: cycle mode on KeyV with cooldown
// ---------------------------------------------------------------------------

fn cycle_transfer_mode(
    keys:    Res<ButtonInput<KeyCode>>,
    time:    Res<Time>,
    mut tc:  ResMut<TransferCase>,
) {
    // Tick down cooldown regardless of input.
    tc.cooldown = (tc.cooldown - time.delta_secs()).max(0.0);

    if keys.just_pressed(CYCLE_KEY) && tc.cooldown <= 0.0 {
        let next = tc.mode.next();
        info!("transfer case: {} → {}", tc.mode, next);
        tc.mode = next;
        tc.cooldown = COOLDOWN_SECS;
    }
}

// ---------------------------------------------------------------------------
// System: keep split values in sync with mode
// ---------------------------------------------------------------------------

fn update_split_values(mut tc: ResMut<TransferCase>) {
    let (split, multiplier) = match tc.mode {
        TransferMode::TwoHi  => (0.0,      1.0),
        TransferMode::FourHi => (SPLIT_4WD, 1.0),
        TransferMode::FourLo => (SPLIT_4WD, FINAL_DRIVE_4LO),
    };

    // Avoid marking the resource changed when nothing actually changed.
    if (tc.torque_split_front - split).abs() > f32::EPSILON
        || (tc.final_drive_multiplier - multiplier).abs() > f32::EPSILON
    {
        tc.torque_split_front     = split;
        tc.final_drive_multiplier = multiplier;
    }
}

// ---------------------------------------------------------------------------
// System: refresh HUD text + dot visibility
// ---------------------------------------------------------------------------

fn refresh_hud(
    tc:         Res<TransferCase>,
    mut text_q: Query<(&mut Text, &mut TextColor), With<XferModeText>>,
    mut dot_q:  Query<(&mut Visibility, &mut BackgroundColor), With<XferActiveDot>>,
) {
    // Text label and colour.
    let (label, color) = match tc.mode {
        TransferMode::TwoHi  => ("[2HI]", WHITE),
        TransferMode::FourHi => ("[4HI]", AMBER),
        TransferMode::FourLo => ("[4LO]", RED_LO),
    };

    for (mut text, mut fg) in &mut text_q {
        if text.0 != label {
            text.0 = label.to_string();
        }
        fg.0 = color;
    }

    // Green dot — visible only when 4WD is engaged.
    let dot_vis = if tc.mode.is_4wd() {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };

    for (mut vis, mut bg) in &mut dot_q {
        *vis = dot_vis;
        bg.0 = GREEN_DOT;
    }
}
