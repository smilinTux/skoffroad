use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use crate::physics::{
    wheel::Wheel,
    suspension::Suspension,
    tire_temperature::TireTemperature,
};

// Constants for dynamics calculations
const GRAVITY: f32 = 9.81;
const AIR_DENSITY: f32 = 1.225; // kg/m³ at sea level, 15°C

#[derive(Component, Default)]
pub struct VehicleDynamics {
    // Weight transfer
    pub center_of_mass_height: f32,
    pub track_width: f32,
    pub wheelbase: f32,
    
    // Aerodynamics
    pub drag_coefficient: f32,
    pub frontal_area: f32,
    pub downforce_coefficient: f32,
    
    // Stability control
    pub stability_control_enabled: bool,
    pub abs_enabled: bool,
    pub tcs_enabled: bool,
    
    // Torque distribution
    pub front_torque_bias: f32,
    pub rear_torque_bias: f32,
    pub differential_locking: f32,
}

impl VehicleDynamics {
    pub fn new() -> Self {
        Self {
            center_of_mass_height: 0.5, // meters
            track_width: 1.8,           // meters
            wheelbase: 2.7,             // meters
            drag_coefficient: 0.4,
            frontal_area: 2.5,          // m²
            downforce_coefficient: 0.1,
            stability_control_enabled: true,
            abs_enabled: true,
            tcs_enabled: true,
            front_torque_bias: 0.4,     // 40% front, 60% rear
            rear_torque_bias: 0.6,
            differential_locking: 0.0,   // 0 = open, 1 = locked
        }
    }
}

// System to calculate and apply weight transfer
pub fn update_weight_transfer(
    mut query: Query<(
        &VehicleDynamics,
        &Velocity,
        &mut ExternalForce,
        &Transform,
        &Children,
    )>,
    wheel_query: Query<(&Wheel, &mut ExternalForce)>,
    time: Res<Time>,
) {
    for (dynamics, velocity, mut vehicle_force, transform, children) in query.iter_mut() {
        let forward_accel = transform.forward().dot(velocity.linvel);
        let lateral_accel = transform.right().dot(velocity.linvel);
        
        // Calculate longitudinal weight transfer
        let long_weight_transfer = dynamics.center_of_mass_height * forward_accel / dynamics.wheelbase;
        
        // Calculate lateral weight transfer
        let lat_weight_transfer = dynamics.center_of_mass_height * lateral_accel / dynamics.track_width;
        
        // Apply weight transfer to wheels
        for &child in children.iter() {
            if let Ok((wheel, mut wheel_force)) = wheel_query.get_mut(child) {
                // Adjust normal force based on position and weight transfer
                let is_front = wheel.is_front;
                let is_left = wheel.is_left;
                
                let long_transfer = if is_front { long_weight_transfer } else { -long_weight_transfer };
                let lat_transfer = if is_left { lat_weight_transfer } else { -lat_weight_transfer };
                
                let total_transfer = Vec3::new(0.0, long_transfer + lat_transfer, 0.0);
                wheel_force.force += total_transfer;
            }
        }
    }
}

// System to calculate and apply aerodynamic forces
pub fn update_aerodynamics(
    mut query: Query<(&VehicleDynamics, &Velocity, &mut ExternalForce, &Transform)>,
    time: Res<Time>,
) {
    for (dynamics, velocity, mut force, transform) in query.iter_mut() {
        let speed = velocity.linvel.length();
        let forward_speed = transform.forward().dot(velocity.linvel);
        
        // Calculate drag force
        let drag_force = -0.5 * AIR_DENSITY * dynamics.drag_coefficient * dynamics.frontal_area 
            * speed * speed * transform.forward();
            
        // Calculate downforce
        let downforce = -0.5 * AIR_DENSITY * dynamics.downforce_coefficient * dynamics.frontal_area 
            * forward_speed * forward_speed * transform.up();
            
        force.force += drag_force + downforce;
    }
}

// System to implement stability control
pub fn update_stability_control(
    mut query: Query<(&VehicleDynamics, &Velocity, &Transform, &Children)>,
    mut wheel_query: Query<(&mut Wheel, &mut ExternalForce)>,
    time: Res<Time>,
) {
    for (dynamics, velocity, transform, children) in query.iter_mut() {
        if !dynamics.stability_control_enabled {
            continue;
        }
        
        let forward_speed = transform.forward().dot(velocity.linvel);
        let lateral_speed = transform.right().dot(velocity.linvel);
        let yaw_rate = velocity.angvel.y;
        
        // Calculate desired yaw rate based on steering angle and speed
        for &child in children.iter() {
            if let Ok((mut wheel, mut wheel_force)) = wheel_query.get_mut(child) {
                if dynamics.abs_enabled {
                    // Implement ABS logic
                    let slip_ratio = wheel.get_slip_ratio();
                    if slip_ratio.abs() > 0.2 {  // Threshold for ABS intervention
                        wheel.brake_torque *= 0.5;  // Reduce brake torque
                    }
                }
                
                if dynamics.tcs_enabled {
                    // Implement TCS logic
                    let slip_ratio = wheel.get_slip_ratio();
                    if slip_ratio > 0.1 {  // Threshold for TCS intervention
                        wheel.drive_torque *= 0.7;  // Reduce drive torque
                    }
                }
                
                // Apply torque distribution
                if wheel.is_front {
                    wheel.drive_torque *= dynamics.front_torque_bias;
                } else {
                    wheel.drive_torque *= dynamics.rear_torque_bias;
                }
            }
        }
    }
}

pub struct VehicleDynamicsPlugin;

impl Plugin for VehicleDynamicsPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(Update, (
                update_weight_transfer,
                update_aerodynamics,
                update_stability_control,
            ));
    }
} 