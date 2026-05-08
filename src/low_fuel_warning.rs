// Low-fuel warning: blinking red text "FUEL LOW" near fuel gauge when
// fuel < 20%. Reads Fuel resource from fuel.rs.
//
// Public API:
//   LowFuelWarningPlugin

use bevy::prelude::*;

use crate::fuel::Fuel;

// ---- Component ---------------------------------------------------------------

#[derive(Component)]
pub struct LowFuelWarningText;

// ---- Plugin ------------------------------------------------------------------

pub struct LowFuelWarningPlugin;

impl Plugin for LowFuelWarningPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_warning)
           .add_systems(Update, update_warning);
    }
}

// ---- Startup: spawn the warning node -----------------------------------------

fn spawn_warning(mut commands: Commands) {
    commands.spawn((
        LowFuelWarningText,
        Text::new("FUEL LOW"),
        TextFont { font_size: 18.0, ..default() },
        TextColor(Color::srgba(1.0, 0.2, 0.2, 0.0)),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(14.0),
            top:  Val::Px(200.0),
            display: Display::None,
            ..default()
        },
    ));
}

// ---- Update: pulse alpha when fuel < 20% ------------------------------------

fn update_warning(
    time:     Res<Time>,
    fuel:     Res<Fuel>,
    mut query: Query<(&mut Node, &mut TextColor), With<LowFuelWarningText>>,
) {
    let frac = if fuel.capacity_l > 0.0 {
        fuel.current_l / fuel.capacity_l
    } else {
        1.0
    };

    for (mut node, mut color) in &mut query {
        if frac < 0.20 {
            node.display = Display::Flex;
            let alpha = 0.4 + (time.elapsed_secs() * 4.0).sin() * 0.6;
            color.0 = Color::srgba(1.0, 0.2, 0.2, alpha.clamp(0.0, 1.0));
        } else {
            node.display = Display::None;
            color.0 = Color::srgba(1.0, 0.2, 0.2, 0.0);
        }
    }
}
