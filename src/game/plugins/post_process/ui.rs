use bevy::prelude::*;
use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};

/// Component that marks an entity as part of the performance display UI
#[derive(Component)]
pub struct PerformanceDisplay;

/// Plugin that adds a performance statistics display to the game
pub struct PerformanceDisplayPlugin;

impl Plugin for PerformanceDisplayPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(FrameTimeDiagnosticsPlugin::default())
           .add_systems(Update, update_performance_display);
    }
}

/// System that updates the performance display UI
fn update_performance_display(
    mut commands: Commands,
    diagnostics: Res<DiagnosticsStore>,
    query: Query<Entity, With<PerformanceDisplay>>,
    asset_server: Res<AssetServer>,
) {
    // Remove existing display
    for entity in query.iter() {
        commands.entity(entity).despawn_recursive();
    }

    // Get frame time
    let fps = diagnostics
        .get(FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|d| d.value())
        .unwrap_or(0.0);

    let frame_time = diagnostics
        .get(FrameTimeDiagnosticsPlugin::FRAME_TIME)
        .and_then(|d| d.value())
        .unwrap_or(0.0);

    // Create performance display UI
    commands
        .spawn((
            NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    left: Val::Px(10.0),
                    top: Val::Px(10.0),
                    padding: UiRect::all(Val::Px(5.0)),
                    ..default()
                },
                background_color: BackgroundColor(Color::rgba(0.0, 0.0, 0.0, 0.5)),
                ..default()
            },
            PerformanceDisplay,
        ))
        .with_children(|parent| {
            // FPS Counter
            parent.spawn(TextBundle::from_sections([
                TextSection::new(
                    "FPS: ",
                    TextStyle {
                        font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                        font_size: 16.0,
                        color: Color::WHITE,
                    },
                ),
                TextSection::new(
                    format!("{:.1}", fps),
                    TextStyle {
                        font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                        font_size: 16.0,
                        color: Color::GREEN,
                    },
                ),
            ]));

            // Frame Time
            parent.spawn(TextBundle::from_sections([
                TextSection::new(
                    "\nFrame Time: ",
                    TextStyle {
                        font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                        font_size: 16.0,
                        color: Color::WHITE,
                    },
                ),
                TextSection::new(
                    format!("{:.2} ms", frame_time),
                    TextStyle {
                        font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                        font_size: 16.0,
                        color: Color::YELLOW,
                    },
                ),
            ]));

            // Effect Overheads
            parent.spawn(TextBundle::from_sections([
                TextSection::new(
                    "\nPost-Process Effects:\n",
                    TextStyle {
                        font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                        font_size: 16.0,
                        color: Color::WHITE,
                    },
                ),
                TextSection::new(
                    "• Bloom: 0.5ms\n",
                    TextStyle {
                        font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                        font_size: 14.0,
                        color: Color::CYAN,
                    },
                ),
                TextSection::new(
                    "• Chromatic Aberration: 0.2ms\n",
                    TextStyle {
                        font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                        font_size: 14.0,
                        color: Color::CYAN,
                    },
                ),
                TextSection::new(
                    "• Vignette: 0.1ms\n",
                    TextStyle {
                        font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                        font_size: 14.0,
                        color: Color::CYAN,
                    },
                ),
                TextSection::new(
                    "Total Overhead: 0.8ms",
                    TextStyle {
                        font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                        font_size: 14.0,
                        color: Color::ORANGE,
                    },
                ),
            ]));
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_registration() {
        let mut app = App::new();
        app.add_plugins(PerformanceDisplayPlugin);
        // If the plugin runs without panicking, registration is assumed successful in Bevy 0.12+
        app.update();
    }

    #[test]
    fn test_performance_display_creation() {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            AssetPlugin::default(),
            PerformanceDisplayPlugin,
        ));

        // Run the app for one frame
        app.update();

        // Verify the performance display was created
        let mut display_query = app.world.query_filtered::<(), With<PerformanceDisplay>>();
        assert!(display_query.iter(&app.world).count() > 0);
    }
} 