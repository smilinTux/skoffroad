use bevy::prelude::*;
use bevy_prototype_debug_lines::*;
use crate::physics::wheel::{Wheel, WheelForces};
use crate::physics::vehicle::Vehicle;

pub struct DebugVisualizationPlugin;

impl Plugin for DebugVisualizationPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(DebugLinesPlugin::default())
           .add_systems(Update, visualize_wheel_forces);
    }
}

/// Visualizes forces acting on each wheel for debugging purposes
fn visualize_wheel_forces(
    wheels: Query<(&Transform, &Wheel, &WheelForces)>,
    mut lines: ResMut<DebugLines>,
) {
    for (transform, wheel, forces) in wheels.iter() {
        let wheel_pos = transform.translation;
        
        // Normal force (up/down) - White
        let normal_dir = forces.normal_force.normalize_or_zero();
        let normal_magnitude = forces.normal_force.length();
        lines.line_colored(
            wheel_pos,
            wheel_pos + normal_dir * normal_magnitude * 0.1, // Scale for visualization
            0.0,
            Color::WHITE,
        );

        // Lateral force (side-to-side) - Red
        let lateral_dir = forces.lateral_force.normalize_or_zero();
        let lateral_magnitude = forces.lateral_force.length();
        lines.line_colored(
            wheel_pos,
            wheel_pos + lateral_dir * lateral_magnitude * 0.1,
            0.0,
            Color::RED,
        );

        // Longitudinal force (forward/backward) - Green
        let longitudinal_dir = forces.longitudinal_force.normalize_or_zero();
        let longitudinal_magnitude = forces.longitudinal_force.length();
        lines.line_colored(
            wheel_pos,
            wheel_pos + longitudinal_dir * longitudinal_magnitude * 0.1,
            0.0,
            Color::GREEN,
        );

        // Contact point - Yellow dot
        if let Some(contact_point) = forces.ground_contact_point {
            lines.line_colored(
                contact_point,
                contact_point + Vec3::new(0.1, 0.1, 0.1),
                0.0,
                Color::YELLOW,
            );
        }
    }
}

/// Helper function to draw suspension visualization
pub fn draw_suspension_debug(
    vehicle: &Vehicle,
    lines: &mut DebugLines,
    transform: &Transform,
) {
    for wheel in &vehicle.wheels {
        let mount_point = transform.transform_point(wheel.mount_point);
        let wheel_pos = transform.transform_point(wheel.position);
        
        // Draw suspension line - Blue
        lines.line_colored(
            mount_point,
            wheel_pos,
            0.0,
            Color::BLUE,
        );
        
        // Draw mount point - Cyan dot
        lines.line_colored(
            mount_point,
            mount_point + Vec3::new(0.05, 0.05, 0.05),
            0.0,
            Color::CYAN,
        );
    }
} 