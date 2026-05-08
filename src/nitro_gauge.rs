// Nitro gauge: dedicated boost-energy bar with refill mechanic. Distinct
// from boost.rs (which already exists as a force module). This module just
// renders a HUD bar showing nitro from 0..1, refilling when not boosting.
//
// Public API:
//   NitroGaugePlugin
//   NitroGaugeState (resource)

use bevy::prelude::*;

// ---- Public resource ---------------------------------------------------------

/// Energy level for the nitro gauge, ranging 0.0 (empty) to 1.0 (full).
/// Decreases at 1/5 s⁻¹ while the boost key is held, refills at 1/8 s⁻¹
/// when released. Purely visual — does not gate boost.rs.
#[derive(Resource, Clone, Copy)]
pub struct NitroGaugeState {
    pub level: f32,
}

impl Default for NitroGaugeState {
    fn default() -> Self {
        Self { level: 1.0 }
    }
}

// ---- HUD marker components ---------------------------------------------------

#[derive(Component)]
struct NitroBarFill;

#[derive(Component)]
struct NitroHudRoot;

// ---- Plugin ------------------------------------------------------------------

pub struct NitroGaugePlugin;

impl Plugin for NitroGaugePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<NitroGaugeState>()
            .add_systems(Startup, spawn_nitro_hud)
            .add_systems(Update, (tick_nitro, update_bar).chain());
    }
}

// ---- Startup: build HUD panel ------------------------------------------------

fn spawn_nitro_hud(mut commands: Commands) {
    // Outer panel: bottom-right at (right: 14, bottom: 60), 200×18 px.
    // Sits visually below the boost bar and above the screen edge.
    let panel = commands
        .spawn((
            NitroHudRoot,
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(14.0),
                bottom: Val::Px(60.0),
                width: Val::Px(200.0),
                height: Val::Px(18.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(3.0),
                padding: UiRect {
                    left: Val::Px(6.0),
                    right: Val::Px(6.0),
                    top: Val::Px(2.0),
                    bottom: Val::Px(2.0),
                },
                display: Display::Flex,
                ..default()
            },
            BackgroundColor(Color::srgba(0.04, 0.04, 0.06, 0.82)),
        ))
        .id();

    // Title label row.
    let title = commands
        .spawn((
            Text::new("NITRO"),
            TextFont {
                font_size: 9.0,
                ..default()
            },
            TextColor(Color::srgb(0.4, 0.95, 1.0)),
        ))
        .id();

    // Bar background container.
    let bar_bg = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Px(5.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.12, 0.12, 0.14, 1.0)),
        ))
        .id();

    // Bar fill — width is driven every frame by update_bar.
    let bar_fill = commands
        .spawn((
            NitroBarFill,
            Node {
                width: Val::Px(200.0), // full width; overwritten immediately
                height: Val::Percent(100.0),
                ..default()
            },
            BackgroundColor(Color::srgb(0.4, 0.95, 1.0)),
        ))
        .id();

    commands.entity(bar_bg).add_child(bar_fill);
    commands.entity(panel).add_children(&[title, bar_bg]);
}

// ---- Update: drain / refill level -------------------------------------------

/// Decreases `NitroGaugeState::level` while KeyB is pressed (5 s to empty),
/// refills when released (8 s to full). Clamped to [0, 1].
fn tick_nitro(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut state: ResMut<NitroGaugeState>,
) {
    let dt = time.delta_secs();

    if keys.pressed(KeyCode::KeyB) && state.level > 0.0 {
        state.level = (state.level - dt / 5.0).max(0.0);
    } else {
        state.level = (state.level + dt / 8.0).min(1.0);
    }
}

// ---- Update: sync bar width and colour --------------------------------------

/// Adjusts the fill bar width (px) and colour to match the current level.
/// Colour zones: cyan (>0.3), orange (0.1–0.3), red (<0.1).
fn update_bar(
    state: Res<NitroGaugeState>,
    mut bar_q: Query<(&mut Node, &mut BackgroundColor), With<NitroBarFill>>,
) {
    let level = state.level.clamp(0.0, 1.0);
    let fill_px = level * 200.0;

    let color = if level > 0.3 {
        Color::srgb(0.4, 0.95, 1.0) // cyan
    } else if level > 0.1 {
        Color::srgb(1.0, 0.55, 0.1) // orange
    } else {
        Color::srgb(0.95, 0.15, 0.15) // red
    };

    for (mut node, mut bg) in &mut bar_q {
        node.width = Val::Px(fill_px);
        bg.0 = color;
    }
}
