use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// Configuration for a vehicle's suspension system
#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct SuspensionConfig {
    /// Spring stiffness in N/m
    pub spring_stiffness: f32,
    /// Damping coefficient in Ns/m
    pub damping: f32,
    /// Maximum suspension travel in meters
    pub travel: f32,
    /// Preload force in N
    pub preload: f32,
    /// Anti-roll bar stiffness in N/m
    pub anti_roll: f32,
}

/// Configuration for a vehicle's engine
#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct EngineConfig {
    /// Maximum power output in horsepower
    pub max_power: f32,
    /// Maximum torque in Nm
    pub max_torque: f32,
    /// Maximum engine RPM
    pub redline: f32,
    /// Idle RPM
    pub idle_rpm: f32,
    /// Power curve as array of [rpm, power] points
    pub power_curve: Vec<[f32; 2]>,
}

/// Configuration for a vehicle's wheels
#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct WheelConfig {
    /// Wheel radius in meters
    pub radius: f32,
    /// Wheel width in meters
    pub width: f32,
    /// Wheel mass in kg
    pub mass: f32,
    /// Rolling resistance coefficient
    pub rolling_resistance: f32,
    /// Tire grip coefficient
    pub grip_coefficient: f32,
    /// Maximum steering angle in degrees
    pub max_steering_angle: f32,
}

/// Configuration for a vehicle's transmission
#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct TransmissionConfig {
    /// Gear ratios for each gear
    pub gear_ratios: Vec<f32>,
    /// Final drive ratio
    pub final_drive: f32,
    /// Time to shift gears in seconds
    pub shift_time: f32,
}

/// Configuration for a vehicle's aerodynamics
#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct AerodynamicsConfig {
    /// Drag coefficient
    pub drag_coefficient: f32,
    /// Frontal area in square meters
    pub frontal_area: f32,
    /// Lift coefficient
    pub lift_coefficient: f32,
}

/// Complete vehicle configuration
#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct VehicleConfig {
    /// Vehicle name
    pub name: String,
    /// Vehicle mass in kg
    pub mass: f32,
    /// Suspension configuration
    pub suspension_config: SuspensionConfig,
    /// Engine configuration
    pub engine_config: EngineConfig,
    /// Wheel configuration
    pub wheel_config: WheelConfig,
    /// Transmission configuration
    pub transmission_config: TransmissionConfig,
    /// Aerodynamics configuration
    pub aerodynamics: AerodynamicsConfig,
}

impl Default for VehicleConfig {
    fn default() -> Self {
        Self {
            name: "Default Vehicle".to_string(),
            mass: 1500.0,
            suspension_config: SuspensionConfig {
                spring_stiffness: 50000.0,
                damping: 5000.0,
                travel: 0.3,
                preload: 4000.0,
                anti_roll: 1500.0,
            },
            engine_config: EngineConfig {
                max_power: 300.0,
                max_torque: 400.0,
                redline: 6000.0,
                idle_rpm: 800.0,
                power_curve: vec![
                    [1000.0, 80.0],
                    [3000.0, 200.0],
                    [5000.0, 300.0],
                    [6000.0, 280.0],
                ],
            },
            wheel_config: WheelConfig {
                radius: 0.35,
                width: 0.275,
                mass: 25.0,
                rolling_resistance: 0.015,
                grip_coefficient: 0.85,
                max_steering_angle: 35.0,
            },
            transmission_config: TransmissionConfig {
                gear_ratios: vec![3.5, 2.5, 1.8, 1.3, 1.0],
                final_drive: 3.73,
                shift_time: 0.2,
            },
            aerodynamics: AerodynamicsConfig {
                drag_coefficient: 0.4,
                frontal_area: 2.5,
                lift_coefficient: -0.1,
            },
        }
    }
} 