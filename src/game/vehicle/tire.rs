use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// Tire friction model parameters based on Pacejka's Magic Formula
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TireModel {
    // Lateral force coefficients (Magic Formula)
    pub b_lat: f32,  // Stiffness factor
    pub c_lat: f32,  // Shape factor
    pub d_lat: f32,  // Peak factor
    pub e_lat: f32,  // Curvature factor

    // Longitudinal force coefficients
    pub b_long: f32,
    pub c_long: f32,
    pub d_long: f32,
    pub e_long: f32,

    // Load sensitivity
    pub load_influence: f32,
    pub optimal_load: f32,

    // Temperature model
    pub temp_optimal: f32,
    pub temp_range: f32,
    pub heating_rate: f32,
    pub cooling_rate: f32,

    // Wear model
    pub wear_rate: f32,
    pub grip_loss_per_wear: f32,
}

impl Default for TireModel {
    fn default() -> Self {
        Self {
            // Typical values for an off-road tire
            b_lat: 10.0,
            c_lat: 1.9,
            d_lat: 1.0,
            e_lat: -1.0,

            b_long: 12.0,
            c_long: 1.9,
            d_long: 1.0,
            e_long: -1.0,

            load_influence: 0.2,
            optimal_load: 4000.0,

            temp_optimal: 60.0,  // Celsius
            temp_range: 30.0,    // ±30°C from optimal
            heating_rate: 0.1,
            cooling_rate: 0.05,

            wear_rate: 0.001,
            grip_loss_per_wear: 0.1,
        }
    }
}

/// Component for tracking tire state
#[derive(Component)]
pub struct TireState {
    pub temperature: f32,
    pub wear: f32,
    pub surface_type: SurfaceType,
    pub surface_grip_multiplier: f32,
}

impl Default for TireState {
    fn default() -> Self {
        Self {
            temperature: 20.0,  // Ambient temperature
            wear: 0.0,         // 0.0 = new, 1.0 = completely worn
            surface_type: SurfaceType::Tarmac,
            surface_grip_multiplier: 1.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SurfaceType {
    Tarmac,
    Dirt,
    Gravel,
    Sand,
    Mud,
    Snow,
    Ice,
}

impl SurfaceType {
    pub fn get_base_friction(&self) -> f32 {
        match self {
            SurfaceType::Tarmac => 1.0,
            SurfaceType::Dirt => 0.8,
            SurfaceType::Gravel => 0.6,
            SurfaceType::Sand => 0.4,
            SurfaceType::Mud => 0.3,
            SurfaceType::Snow => 0.2,
            SurfaceType::Ice => 0.1,
        }
    }
}

/// Calculate tire forces using the Pacejka Magic Formula
pub fn calculate_tire_forces(
    slip_angle: f32,
    slip_ratio: f32,
    normal_force: f32,
    tire_model: &TireModel,
    tire_state: &TireState,
) -> (Vec3, Vec3) {
    // Temperature effect on grip
    let temp_delta = (tire_state.temperature - tire_model.temp_optimal).abs();
    let temp_factor = 1.0 - (temp_delta / tire_model.temp_range).clamp(0.0, 1.0);

    // Load sensitivity
    let load_factor = {
        let normalized_load = normal_force / tire_model.optimal_load;
        1.0 - tire_model.load_influence * (normalized_load - 1.0).abs()
    };

    // Wear effect
    let wear_factor = 1.0 - tire_model.grip_loss_per_wear * tire_state.wear;

    // Surface grip
    let surface_grip = tire_state.surface_grip_multiplier;

    // Combined grip multiplier
    let grip_multiplier = temp_factor * load_factor * wear_factor * surface_grip;

    // Lateral force calculation (Magic Formula)
    let lateral_force = {
        let b = tire_model.b_lat;
        let c = tire_model.c_lat;
        let d = tire_model.d_lat * normal_force * grip_multiplier;
        let e = tire_model.e_lat;
        
        let b_slip = b * slip_angle;
        d * f32::sin(c * f32::atan(b_slip - e * (b_slip - f32::atan(b_slip))))
    };

    // Longitudinal force calculation
    let longitudinal_force = {
        let b = tire_model.b_long;
        let c = tire_model.c_long;
        let d = tire_model.d_long * normal_force * grip_multiplier;
        let e = tire_model.e_long;
        
        let b_slip = b * slip_ratio;
        d * f32::sin(c * f32::atan(b_slip - e * (b_slip - f32::atan(b_slip))))
    };

    // Return forces in local tire coordinates
    (
        Vec3::new(lateral_force, 0.0, 0.0),      // Lateral force (right)
        Vec3::new(0.0, 0.0, longitudinal_force), // Longitudinal force (forward)
    )
}

/// System to update tire temperature and wear
pub fn update_tire_state(
    mut tire_query: Query<(&mut TireState, &Wheel)>,
    time: Res<Time>,
) {
    let dt = time.delta_seconds();

    for (mut tire_state, wheel) in tire_query.iter_mut() {
        // Temperature update based on slip
        let slip_work = wheel.slip_angle.abs() + wheel.slip_ratio.abs();
        let temp_change = if wheel.ground_contact {
            // Heat generation from slip
            slip_work * tire_state.temperature * wheel.normal_force * 0.0001
        } else {
            // Cooling when not in contact
            -(tire_state.temperature - 20.0) * 0.1
        };

        tire_state.temperature += temp_change * dt;

        // Wear update based on slip and load
        if wheel.ground_contact {
            let wear_increment = slip_work * wheel.normal_force * 0.00001 * dt;
            tire_state.wear = (tire_state.wear + wear_increment).min(1.0);
        }
    }
}

/// System to detect surface type under the tire
pub fn detect_surface_type(
    mut tire_query: Query<(&mut TireState, &GlobalTransform)>,
    rapier_context: Res<RapierContext>,
) {
    for (mut tire_state, transform) in tire_query.iter_mut() {
        // Cast a short ray downward to detect surface
        let ray_pos = transform.translation() + Vec3::Y * 0.1;
        let ray_dir = -Vec3::Y;
        
        if let Some((entity, intersection)) = rapier_context.cast_ray(
            ray_pos,
            ray_dir,
            0.2,
            true,
            QueryFilter::default(),
        ) {
            // Here you would get the surface type from the collided entity
            // For now, we'll just use a default multiplier
            tire_state.surface_grip_multiplier = 1.0;
        } else {
            // No ground contact
            tire_state.surface_grip_multiplier = 0.0;
        }
    }
} 