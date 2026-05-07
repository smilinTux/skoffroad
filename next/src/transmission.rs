// Multi-speed transmission: 6 forward gears + reverse + neutral.
//
// Sprint 39 — PRD v3 priority 1 (drivetrain realism).
//
// Resources:
//   Transmission         — current gear, gear ratios, auto/manual flag, cooldown
//   TransmissionOutput   — effective gear ratio for other systems to read
//
// Plugin: TransmissionPlugin
//   auto_shift_logic     (Update, auto mode only)
//   manual_shift_input   (Update, manual mode + T-toggle)
//   display_hud          (Update, top-left text overlay)
//
// Gear index mapping:
//   current_gear + 1 → index into gear_ratios
//   -1  → index 0  (reverse)
//    0  → index 1  (neutral)
//    1  → index 2  (1st)
//    …
//    6  → index 7  (6th)

use bevy::prelude::*;

use crate::engine_torque::EngineState;
use crate::vehicle::DriveInput;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const UPSHIFT_RPM:       f32 = 5500.0;
const DOWNSHIFT_RPM:     f32 = 1500.0;
const AUTO_COOLDOWN:     f32 = 0.5;
const MANUAL_COOLDOWN:   f32 = 0.3;

// ---------------------------------------------------------------------------
// Resources
// ---------------------------------------------------------------------------

/// Transmission state shared with the rest of the drivetrain.
#[derive(Resource, Clone)]
pub struct Transmission {
    /// -1 = reverse, 0 = neutral, 1..=6 = forward gears.
    pub current_gear: i8,

    /// Gear ratios indexed by `current_gear + 1`:
    ///   [0] reverse  (-3.5)
    ///   [1] neutral  ( 0.0)
    ///   [2] 1st      ( 3.8)
    ///   [3] 2nd      ( 2.4)
    ///   [4] 3rd      ( 1.6)
    ///   [5] 4th      ( 1.2)
    ///   [6] 5th      ( 0.95)
    ///   [7] 6th      ( 0.75)
    pub gear_ratios: [f32; 8],

    /// true = automatic shifting; false = manual (KeyK/KeyL).
    pub auto: bool,

    /// Seconds remaining before the next shift is allowed.
    pub shift_cooldown: f32,
}

impl Default for Transmission {
    fn default() -> Self {
        Self {
            current_gear:  0,
            gear_ratios:   [-3.5, 0.0, 3.8, 2.4, 1.6, 1.2, 0.95, 0.75],
            auto:          true,
            shift_cooldown: 0.0,
        }
    }
}

impl Transmission {
    /// Returns the gear ratio for `current_gear`.
    #[inline]
    pub fn ratio(&self) -> f32 {
        let idx = (self.current_gear + 1).clamp(0, 7) as usize;
        self.gear_ratios[idx]
    }

    /// Human-readable gear label.
    fn gear_label(&self) -> String {
        match self.current_gear {
            -1 => "R".to_string(),
             0 => "N".to_string(),
             n => n.to_string(),
        }
    }
}

// ---------------------------------------------------------------------------

/// Effective gear ratio for this frame; other systems multiply against this.
/// Set to `gear_ratios[current_gear + 1]` each frame by `auto_shift_logic` /
/// `manual_shift_input`, then exposed here so vehicle.rs can stay untouched.
#[derive(Resource, Default)]
pub struct TransmissionOutput {
    pub ratio: f32,
}

// ---------------------------------------------------------------------------
// HUD marker components
// ---------------------------------------------------------------------------

#[derive(Component)]
struct TransHudRoot;

#[derive(Component)]
struct TransHudText;

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct TransmissionPlugin;

impl Plugin for TransmissionPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<Transmission>()
            .init_resource::<TransmissionOutput>()
            .add_systems(Startup,  spawn_hud)
            .add_systems(Update, (
                tick_cooldown,
                auto_shift_logic.run_if(|t: Res<Transmission>| t.auto),
                manual_shift_input.run_if(|t: Res<Transmission>| !t.auto),
                toggle_auto,       // T-key toggle works in both modes
                sync_output,       // keep TransmissionOutput in sync
                display_hud,
            ).chain());
    }
}

// ---------------------------------------------------------------------------
// Startup: HUD
// ---------------------------------------------------------------------------

fn spawn_hud(mut commands: Commands) {
    let root = commands.spawn((
        TransHudRoot,
        Node {
            position_type:   PositionType::Absolute,
            left:            Val::Px(12.0),
            top:             Val::Px(12.0),
            width:           Val::Px(180.0),
            height:          Val::Px(30.0),
            align_items:     AlignItems::Center,
            padding: UiRect {
                left:   Val::Px(8.0),
                right:  Val::Px(8.0),
                top:    Val::Px(4.0),
                bottom: Val::Px(4.0),
            },
            ..default()
        },
        BackgroundColor(Color::srgba(0.04, 0.04, 0.06, 0.82)),
    )).id();

    let label = commands.spawn((
        TransHudText,
        Text::new("Gear: N (Auto)"),
        TextFont { font_size: 14.0, ..default() },
        TextColor(Color::srgb(0.9, 0.85, 0.25)),
    )).id();

    commands.entity(root).add_child(label);
}

// ---------------------------------------------------------------------------
// System: tick cooldown (always runs first in the chain)
// ---------------------------------------------------------------------------

fn tick_cooldown(mut trans: ResMut<Transmission>, time: Res<Time>) {
    trans.shift_cooldown = (trans.shift_cooldown - time.delta_secs()).max(0.0);
}

// ---------------------------------------------------------------------------
// System: auto_shift_logic  (runs only when auto = true)
// ---------------------------------------------------------------------------

fn auto_shift_logic(
    engine:    Res<EngineState>,
    input:     Res<DriveInput>,
    mut trans: ResMut<Transmission>,
) {
    if trans.shift_cooldown > 0.0 { return; }

    let rpm = engine.rpm;
    let gear = trans.current_gear;

    // Reverse from neutral when driver pushes backward.
    if gear == 0 && input.drive < 0.0 {
        trans.current_gear  = -1;
        trans.shift_cooldown = AUTO_COOLDOWN;
        info!("transmission: auto → REVERSE");
        return;
    }

    // Return to neutral / 1st when in reverse and driver pushes forward.
    if gear == -1 && input.drive > 0.0 {
        trans.current_gear  = 1;
        trans.shift_cooldown = AUTO_COOLDOWN;
        info!("transmission: auto → 1st");
        return;
    }

    // Upshift.
    if rpm > UPSHIFT_RPM && gear >= 1 && gear < 6 {
        trans.current_gear  += 1;
        trans.shift_cooldown = AUTO_COOLDOWN;
        info!("transmission: auto upshift → {}", trans.current_gear);
        return;
    }

    // Downshift (don't drop into neutral automatically).
    if rpm < DOWNSHIFT_RPM && gear > 1 {
        trans.current_gear  -= 1;
        trans.shift_cooldown = AUTO_COOLDOWN;
        info!("transmission: auto downshift → {}", trans.current_gear);
    }
}

// ---------------------------------------------------------------------------
// System: manual_shift_input  (runs only when auto = false)
// ---------------------------------------------------------------------------

fn manual_shift_input(
    keys:      Res<ButtonInput<KeyCode>>,
    mut trans: ResMut<Transmission>,
) {
    if trans.shift_cooldown > 0.0 { return; }

    // KeyK = upshift.
    if keys.just_pressed(KeyCode::KeyK) {
        let new = (trans.current_gear + 1).min(6);
        if new != trans.current_gear {
            trans.current_gear   = new;
            trans.shift_cooldown = MANUAL_COOLDOWN;
            info!("transmission: manual upshift → {}", trans.current_gear);
        }
        return;
    }

    // KeyL = downshift.
    if keys.just_pressed(KeyCode::KeyL) {
        let new = (trans.current_gear - 1).max(-1);
        if new != trans.current_gear {
            trans.current_gear   = new;
            trans.shift_cooldown = MANUAL_COOLDOWN;
            info!("transmission: manual downshift → {}", trans.current_gear);
        }
        return;
    }

    // KeyN = neutral.
    if keys.just_pressed(KeyCode::KeyN) && trans.current_gear != 0 {
        trans.current_gear   = 0;
        trans.shift_cooldown = MANUAL_COOLDOWN;
        info!("transmission: manual → NEUTRAL");
    }
}

// ---------------------------------------------------------------------------
// System: toggle_auto  (T key, always active)
// ---------------------------------------------------------------------------

fn toggle_auto(keys: Res<ButtonInput<KeyCode>>, mut trans: ResMut<Transmission>) {
    if keys.just_pressed(KeyCode::KeyT) {
        trans.auto = !trans.auto;
        info!(
            "transmission: mode → {}",
            if trans.auto { "AUTO" } else { "MANUAL" }
        );
    }
}

// ---------------------------------------------------------------------------
// System: sync_output — keep TransmissionOutput current
// ---------------------------------------------------------------------------

fn sync_output(trans: Res<Transmission>, mut out: ResMut<TransmissionOutput>) {
    out.ratio = trans.ratio();
}

// ---------------------------------------------------------------------------
// System: display_hud — update top-left text each frame
// ---------------------------------------------------------------------------

fn display_hud(
    trans:      Res<Transmission>,
    mut text_q: Query<(&mut Text, &mut TextColor), With<TransHudText>>,
) {
    let mode  = if trans.auto { "Auto" } else { "Manual" };
    let label = format!("Gear: {} ({})", trans.gear_label(), mode);

    // Colour: green in a forward gear, amber in neutral, red in reverse.
    let color = match trans.current_gear {
        g if g > 0  => Color::srgb(0.25, 0.90, 0.35),
        0            => Color::srgb(0.95, 0.85, 0.20),
        _            => Color::srgb(0.95, 0.30, 0.25),
    };

    for (mut text, mut fg) in &mut text_q {
        text.0 = label.clone();
        fg.0   = color;
    }
}
