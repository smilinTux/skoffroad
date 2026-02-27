use bevy::prelude::*;
use bevy::gizmos::*;
use super::cb_radio::{CBRadio, CBRadioManager};
use super::cb_chatter::AITrucker;
use super::terrain_interference::calculate_terrain_interference;

/// Component for enabling CB radio debug visualization
#[derive(Component)]
pub struct CBRadioDebug {
    pub show_truckers: bool,
    pub show_routes: bool,
    pub show_signal_strength: bool,
    pub show_interference: bool,
}

impl Default for CBRadioDebug {
    fn default() -> Self {
        Self {
            show_truckers: true,
            show_routes: true,
            show_signal_strength: true,
            show_interference: true,
        }
    }
}

/// Plugin for CB radio debug visualization
pub struct CBRadioDebugPlugin;

impl Plugin for CBRadioDebugPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (
            draw_trucker_positions,
            draw_routes,
            draw_signal_strength,
            draw_interference_map,
            update_debug_text,
        ));
    }
}

/// Colors for debug visualization
const TRUCKER_COLOR: Color = Color::YELLOW;
const ROUTE_COLOR: Color = Color::rgba(0.5, 0.5, 1.0, 0.3);
const SIGNAL_COLOR: Color = Color::GREEN;
const INTERFERENCE_COLOR: Color = Color::RED;

/// System to draw AI trucker positions
fn draw_trucker_positions(
    mut gizmos: Gizmos,
    truckers: Query<(&AITrucker, &Transform)>,
    debug_settings: Query<&CBRadioDebug>,
) {
    if let Ok(debug) = debug_settings.get_single() {
        if !debug.show_truckers {
            return;
        }

        for (trucker, transform) in truckers.iter() {
            let pos = transform.translation;
            // Draw trucker icon
            gizmos.sphere(pos, Quat::IDENTITY, 2.0, TRUCKER_COLOR);
            
            // Draw handle text
            let text_pos = pos + Vec3::new(0.0, 3.0, 0.0);
            gizmos.text(text_pos, Color::WHITE, &trucker.handle);
        }
    }
}

/// System to draw trucker routes
fn draw_routes(
    mut gizmos: Gizmos,
    truckers: Query<(&AITrucker, &Transform)>,
    debug_settings: Query<&CBRadioDebug>,
) {
    if let Ok(debug) = debug_settings.get_single() {
        if !debug.show_routes {
            return;
        }

        for (trucker, transform) in truckers.iter() {
            let current_pos = transform.translation;
            
            // Draw route lines
            for i in 0..trucker.route.len() {
                let next_i = (i + 1) % trucker.route.len();
                let start = get_location_position(&trucker.route[i]);
                let end = get_location_position(&trucker.route[next_i]);
                
                gizmos.line(start, end, ROUTE_COLOR);
                
                // Draw waypoint markers
                gizmos.sphere(start, Quat::IDENTITY, 1.0, ROUTE_COLOR);
            }
            
            // Draw line from current position to next waypoint
            let next_pos = get_location_position(&trucker.route[trucker.route_index]);
            gizmos.line(current_pos, next_pos, ROUTE_COLOR);
        }
    }
}

/// System to visualize signal strength
fn draw_signal_strength(
    mut gizmos: Gizmos,
    radios: Query<(&CBRadio, &Transform)>,
    debug_settings: Query<&CBRadioDebug>,
) {
    if let Ok(debug) = debug_settings.get_single() {
        if !debug.show_signal_strength {
            return;
        }

        for (radio, transform) in radios.iter() {
            if radio.powered {
                let pos = transform.translation;
                let radius = 50.0 * radio.signal_strength;
                
                // Draw signal radius
                gizmos.circle(
                    pos,
                    Vec3::Y,
                    radius,
                    Color::rgba(0.0, 1.0, 0.0, 0.2),
                );
                
                // Draw signal strength indicator
                let strength_text = format!("Signal: {:.1}", radio.signal_strength);
                gizmos.text(pos + Vec3::new(0.0, 5.0, 0.0), SIGNAL_COLOR, &strength_text);
            }
        }
    }
}

/// System to visualize terrain interference
fn draw_interference_map(
    mut gizmos: Gizmos,
    radios: Query<(&CBRadio, &Transform)>,
    terrain_manager: Res<TerrainManager>,
    interference_config: Res<TerrainInterferenceConfig>,
    debug_settings: Query<&CBRadioDebug>,
    time: Res<Time>,
) {
    if let Ok(debug) = debug_settings.get_single() {
        if !debug.show_interference {
            return;
        }

        // Sample interference in a grid around active radios
        for (radio, transform) in radios.iter().filter(|(r, _)| r.powered) {
            let center = transform.translation;
            let grid_size = 100.0;
            let samples = 10;
            
            for x in -samples..=samples {
                for z in -samples..=samples {
                    let sample_pos = center + Vec3::new(
                        x as f32 * grid_size / samples as f32,
                        0.0,
                        z as f32 * grid_size / samples as f32,
                    );
                    
                    let interference = calculate_terrain_interference(
                        center,
                        sample_pos,
                        &terrain_manager,
                        &interference_config,
                        "clear", // TODO: Get actual weather
                        time.elapsed_seconds() % 24.0,
                    );
                    
                    let color = Color::rgb(
                        interference.max(0.0),
                        (1.0 - interference).max(0.0),
                        0.0,
                    ).with_a(0.3);
                    
                    gizmos.sphere(sample_pos, Quat::IDENTITY, 1.0, color);
                }
            }
        }
    }
}

/// System to update debug text overlay
fn update_debug_text(
    mut commands: Commands,
    debug_settings: Query<&CBRadioDebug>,
    radio_manager: Res<CBRadioManager>,
    truckers: Query<&AITrucker>,
) {
    if let Ok(debug) = debug_settings.get_single() {
        // Remove existing debug text
        commands.remove_resource::<DebugText>();
        
        // Create new debug text
        let mut text = String::new();
        text.push_str("CB Radio Debug Info:\n");
        text.push_str(&format!("Active Truckers: {}\n", truckers.iter().count()));
        text.push_str(&format!("Active Transmissions: {}\n", radio_manager.active_transmission_count()));
        text.push_str(&format!("Emergency Channel Active: {}\n", radio_manager.emergency_active()));
        
        commands.insert_resource(DebugText(text));
    }
}

/// Helper function to get world position for a location name
fn get_location_position(location: &str) -> Vec3 {
    // TODO: Replace with actual location data from game world
    match location {
        "Flying J" => Vec3::new(100.0, 0.0, 100.0),
        "Love's Truck Stop" => Vec3::new(-100.0, 0.0, 100.0),
        "Rest Area" => Vec3::new(0.0, 0.0, 200.0),
        "Weigh Station" => Vec3::new(200.0, 0.0, 0.0),
        "Toll Plaza" => Vec3::new(-200.0, 0.0, 0.0),
        _ => Vec3::ZERO,
    }
}

/// Resource for debug text overlay
#[derive(Resource)]
struct DebugText(String); 