// First-run tutorial overlay for skoffroad.
//
// Displays a small top-centred panel that walks the player through the basics
// one step at a time, then auto-hides after course completion.
// Press H to toggle visibility at any time.

use bevy::prelude::*;

use crate::menu::MenuState;
use crate::course::CourseState;

// ---------------------------------------------------------------------------
// Public plugin
// ---------------------------------------------------------------------------

pub struct TutorialPlugin;

impl Plugin for TutorialPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TutorialState>()
            .add_systems(Startup, spawn_tutorial_panel)
            .add_systems(Update, update_tutorial);
    }
}

// ---------------------------------------------------------------------------
// Public resource
// ---------------------------------------------------------------------------

#[derive(Resource, Default)]
pub struct TutorialState {
    pub step: u32,
    pub elapsed_in_step: f32,
    pub dismissed_after_completion: bool,
}

// ---------------------------------------------------------------------------
// Components
// ---------------------------------------------------------------------------

#[derive(Component)]
struct TutorialRoot;

#[derive(Component)]
enum TutorialText {
    StepLabel,
    Message,
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const PANEL_BG:     Color = Color::srgba(0.05, 0.05, 0.07, 0.92);
const COLOR_WHITE:  Color = Color::WHITE;
const COLOR_GREY:   Color = Color::srgba(0.60, 0.60, 0.63, 1.0);
const PANEL_WIDTH:  f32   = 480.0;
const PANEL_TOP:    f32   = 90.0;
const AUTO_HIDE_S:  f32   = 8.0;   // seconds after step 3 before hiding

const MESSAGES: [&str; 4] = [
    "Welcome! Use W A S D to drive. Hold Shift for boost.",
    "Drive through the GREEN START GATE to begin a timed run. The cyan arrow points the way.",
    "Lap timer running! Hit each gate in order: yellow then red. Press F5 to save your run.",
    "Course complete! Press R to reset and try for a better time, or free-roam.",
];

// ---------------------------------------------------------------------------
// Startup: build tutorial panel tree
// ---------------------------------------------------------------------------

fn spawn_tutorial_panel(mut commands: Commands) {
    // Outer panel — top-centred, hidden until menu dismissed.
    // Centering trick: left 50% + negative left margin of half the width.
    let panel = commands
        .spawn((
            TutorialRoot,
            Node {
                position_type: PositionType::Absolute,
                top:           Val::Px(PANEL_TOP),
                left:          Val::Percent(50.0),
                margin: UiRect {
                    left: Val::Px(-(PANEL_WIDTH / 2.0)),
                    ..default()
                },
                width:           Val::Px(PANEL_WIDTH),
                flex_direction:  FlexDirection::Column,
                align_items:     AlignItems::Center,
                padding:         UiRect::axes(Val::Px(14.0), Val::Px(10.0)),
                row_gap:         Val::Px(4.0),
                border:          UiRect::all(Val::Px(1.0)),
                display:         Display::None, // hidden until step 0 fires
                ..default()
            },
            BackgroundColor(PANEL_BG),
            BorderColor::all(COLOR_WHITE),
        ))
        .id();

    // Small grey "tutorial: step N / 4" label at top.
    let step_label = commands
        .spawn((
            TutorialText::StepLabel,
            Text::new("tutorial: step 0 / 4"),
            TextFont { font_size: 11.0, ..default() },
            TextColor(COLOR_GREY),
        ))
        .id();

    // Main white message text.
    let msg_text = commands
        .spawn((
            TutorialText::Message,
            Text::new(""),
            TextFont { font_size: 14.0, ..default() },
            TextColor(COLOR_WHITE),
            Node {
                // Allow text to wrap inside the panel.
                max_width: Val::Px(PANEL_WIDTH - 28.0),
                ..default()
            },
        ))
        .id();

    commands.entity(panel).add_children(&[step_label, msg_text]);
}

// ---------------------------------------------------------------------------
// Per-frame logic
// ---------------------------------------------------------------------------

fn update_tutorial(
    time:         Res<Time>,
    keys:         Res<ButtonInput<KeyCode>>,
    menu:         Option<Res<MenuState>>,
    course:       Option<Res<CourseState>>,
    vehicle:      Option<Res<crate::vehicle::VehicleRoot>>,
    mut tut:      ResMut<TutorialState>,
    mut roots:    Query<(&mut Node, &mut Visibility), With<TutorialRoot>>,
    mut texts:    Query<(&TutorialText, &mut Text)>,
) {
    let dt = time.delta_secs();

    // Once dismissed after completion, do nothing.
    if tut.dismissed_after_completion {
        return;
    }

    // ---- H key: toggle visibility ------------------------------------------
    if keys.just_pressed(KeyCode::KeyH) && tut.step > 0 {
        for (_, mut vis) in roots.iter_mut() {
            *vis = match *vis {
                Visibility::Hidden => Visibility::Visible,
                _                  => Visibility::Hidden,
            };
        }
        // Toggle does not advance steps; return early so we don't flip it back.
        return;
    }

    // ---- Determine target step based on triggers ---------------------------
    let menu_dismissed = menu
        .as_ref()
        .map(|m| m.dismissed)
        .unwrap_or(false);

    let course_index = course
        .as_ref()
        .map(|c| c.current_index)
        .unwrap_or(0);

    let course_completed = course
        .as_ref()
        .map(|c| c.completed)
        .unwrap_or(false);

    // Vehicle present = player is in the world and can drive.
    let vehicle_present = vehicle.is_some();

    // Accumulate time while the player is in the game world (step 0 or 1).
    let driving_long_enough = if vehicle_present && tut.step <= 1 {
        tut.elapsed_in_step += dt;
        tut.elapsed_in_step > 3.0
    } else {
        false
    };

    // The highest step whose trigger is satisfied.
    let target_step: u32 = if course_completed {
        3
    } else if course_index >= 1 {
        2
    } else if driving_long_enough && tut.step == 1 {
        1 // already at 1, no change; handled below as a no-op
    } else if menu_dismissed && tut.step == 0 {
        0 // will fire the transition to step 1 in a moment
    } else {
        tut.step
    };

    // Compute the new step we should be showing.
    let new_step: u32 = if !menu_dismissed {
        // Menu still up — keep hidden at step 0 (pre-show).
        tut.step
    } else if tut.step == 0 {
        // Menu just dismissed: advance to step 1 (first real message).
        1
    } else if target_step > tut.step {
        target_step
    } else if driving_long_enough && tut.step == 1 {
        // Player has driven long enough — nudge to step 2 (go find start gate).
        2
    } else {
        tut.step
    };

    // Advance step and reset per-step timer.
    if new_step != tut.step {
        tut.step = new_step;
        tut.elapsed_in_step = 0.0;
    } else {
        tut.elapsed_in_step += dt;
    }

    // ---- Step 4: auto-hide 8 seconds after "course complete" message --------
    if tut.step == 3 {
        // elapsed_in_step counts from when we entered step 3.
        if tut.elapsed_in_step >= AUTO_HIDE_S {
            tut.step = 4;
            tut.dismissed_after_completion = true;
            for (mut node, _) in roots.iter_mut() {
                node.display = Display::None;
            }
            return;
        }
    }

    // ---- Show or hide panel -------------------------------------------------
    let should_show = tut.step >= 1 && tut.step <= 3;
    for (mut node, _) in roots.iter_mut() {
        node.display = if should_show { Display::Flex } else { Display::None };
    }

    // ---- Update text content ------------------------------------------------
    if should_show {
        let msg_index = (tut.step.saturating_sub(1)) as usize;
        let msg = MESSAGES.get(msg_index).copied().unwrap_or("");
        let label = format!("tutorial: step {} / 4", tut.step);

        for (kind, mut text) in texts.iter_mut() {
            match kind {
                TutorialText::StepLabel => text.0 = label.clone(),
                TutorialText::Message   => text.0 = msg.to_string(),
            }
        }
    }
}
