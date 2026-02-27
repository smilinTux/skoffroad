use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use crate::physics::tire_temperature::{TireTemperature, TireTemperatureState};
use crate::physics::TerrainProperties;

#[derive(Component, Debug)]
pub struct Wheel {
    pub radius: f32,
    pub width: f32,
    pub mass: f32,
    pub angular_velocity: f32,
    pub steering_angle: f32,
    pub drive_torque: f32,
    pub brake_torque: f32,
    pub inertia: f32,
    pub suspension_mount: Vec3,
}

impl Default for Wheel {
    fn default() -> Self {
        Self {
            radius: 0.4,
            width: 0.3,
            mass: 25.0,
            angular_velocity: 0.0,
            steering_angle: 0.0,
            drive_torque: 0.0,
            brake_torque: 0.0,
            inertia: 2.5, // Calculated as mr²/2 for a cylinder
            suspension_mount: Vec3::ZERO,
        }
    }
}

#[derive(Component, Debug)]
pub struct WheelForces {
    pub longitudinal_force: f32,
    pub lateral_force: f32,
    pub normal_force: f32,
    pub contact_point: Option<Vec3>,
    pub slip_ratio: f32,
    pub slip_angle: f32,
    pub terrain_properties: Option<TerrainProperties>,
    pub slip_power: f32,  // New field to track power dissipated through slip
}

impl Default for WheelForces {
    fn default() -> Self {
        Self {
            longitudinal_force: 0.0,
            lateral_force: 0.0,
            normal_force: 0.0,
            contact_point: None,
            slip_ratio: 0.0,
            slip_angle: 0.0,
            terrain_properties: Some(TerrainProperties::default()),
            slip_power: 0.0,
        }
    }
}

#[derive(Debug)]
struct SlipParameters {
    longitudinal: f32,
    lateral: f32,
    combined: f32,
}

impl Wheel {
    pub fn calculate_slip(&self, vehicle_velocity: Vec3, contact_point: Vec3, forward: Vec3, right: Vec3) -> SlipParameters {
        let wheel_velocity = self.angular_velocity * self.radius;
        let velocity_at_contact = vehicle_velocity + forward.cross(contact_point);
        
        // Decompose velocity into longitudinal and lateral components
        let longitudinal_velocity = velocity_at_contact.dot(forward);
        let lateral_velocity = velocity_at_contact.dot(right);
        
        // Calculate slip ratio (longitudinal slip)
        let slip_ratio = if longitudinal_velocity.abs() < 0.1 {
            0.0
        } else {
            (wheel_velocity - longitudinal_velocity) / longitudinal_velocity.abs()
        };
        
        // Calculate slip angle (lateral slip)
        let slip_angle = if longitudinal_velocity.abs() < 0.1 {
            0.0
        } else {
            (lateral_velocity / longitudinal_velocity.abs()).atan()
        };
        
        // Combined slip
        let combined_slip = (slip_ratio.powi(2) + slip_angle.powi(2)).sqrt();
        
        SlipParameters {
            longitudinal: slip_ratio,
            lateral: slip_angle,
            combined: combined_slip,
        }
    }

    pub fn update_forces(&mut self, entity: Entity, forces: &mut WheelForces, vehicle_velocity: Vec3, dt: f32, tire_temp_query: &Query<&TireTemperature>) {
        if let Some(contact_point) = forces.contact_point {
            let forward = Quat::from_rotation_y(self.steering_angle) * Vec3::X;
            let right = forward.cross(Vec3::Y);
            
            let slip = self.calculate_slip(vehicle_velocity, contact_point, forward, right);
            forces.slip_ratio = slip.longitudinal;
            forces.slip_angle = slip.lateral;
            
            // Calculate slip power (work done by tire slip)
            forces.slip_power = slip.combined * vehicle_velocity.length() * forces.normal_force;
            
            // Get terrain properties or use default
            let default_terrain = TerrainProperties::default();
            let terrain = forces.terrain_properties.as_ref().unwrap_or(&default_terrain);
            
            // Calculate effective friction coefficient based on velocity and normal force
            let mut friction_coeff = terrain.get_effective_friction(vehicle_velocity.length(), forces.normal_force);
            
            // Apply tire temperature grip multiplier if component exists
            if let Ok(tire_temp) = tire_temp_query.get(entity) {
                friction_coeff *= tire_temp.get_grip_multiplier();
            }
            
            // Calculate rolling resistance
            let rolling_resistance = terrain.get_rolling_resistance(vehicle_velocity.length());
            
            // Magic Formula (Pacejka) tire model coefficients
            let b = 10.0; // Stiffness factor
            let c = 1.9;  // Shape factor
            let d = friction_coeff * forces.normal_force; // Peak force
            let e = 0.97; // Curvature factor
            
            // Calculate forces using Magic Formula
            let long_force = d * f32::sin(c * f32::atan(b * slip.longitudinal - e * (b * slip.longitudinal - f32::atan(b * slip.longitudinal))));
            let lat_force = d * f32::sin(c * f32::atan(b * slip.lateral - e * (b * slip.lateral - f32::atan(b * slip.lateral))));
            
            // Apply rolling resistance
            let rolling_force = -rolling_resistance * forces.normal_force * vehicle_velocity.normalize().dot(forward).signum();
            
            // Update forces
            forces.longitudinal_force = long_force + rolling_force;
            forces.lateral_force = lat_force;
            
            // Update wheel angular velocity based on forces and torques
            let net_torque = self.drive_torque - (forces.longitudinal_force * self.radius) - 
                           (self.brake_torque * self.angular_velocity.signum());
            
            self.angular_velocity += (net_torque / self.inertia) * dt;
        } else {
            forces.longitudinal_force = 0.0;
            forces.lateral_force = 0.0;
            forces.slip_power = 0.0;
            
            // Slow down wheel when not in contact
            self.angular_velocity *= 0.95;
        }
    }
}

pub struct WheelPhysicsPlugin;

impl Plugin for WheelPhysicsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (update_wheel_physics, update_tire_temperature));
    }
}

fn update_tire_temperature(
    mut query: Query<(&WheelForces, &mut TireTemperature)>,
    time: Res<Time>,
) {
    for (forces, mut tire_temp) in query.iter_mut() {
        tire_temp.update(
            forces.slip_power,
            forces.normal_force,
            time.delta_seconds(),
            20.0, // Ambient temperature
        );
    }
}

fn update_wheel_physics(
    mut wheels: Query<(Entity, &mut Wheel, &mut WheelForces, &GlobalTransform)>,
    tire_temp_query: Query<&TireTemperature>,
    rapier_context: Res<RapierContext>,
    terrain_query: Query<(&TerrainProperties, &GlobalTransform)>,
    time: Res<Time>,
) {
    for (entity, mut wheel, mut forces, transform) in wheels.iter_mut() {
        // Cast ray for ground detection
        let ray_pos = transform.translation();
        let ray_dir = -Vec3::Y;
        let max_distance = wheel.radius * 2.0;
        
        if let Some((entity, intersection)) = rapier_context.cast_ray(
            ray_pos,
            ray_dir,
            max_distance,
            true,
            QueryFilter::default(),
        ) {
            forces.contact_point = Some(ray_pos + ray_dir * intersection);
            forces.normal_force = (wheel.radius - intersection).max(0.0) * 10000.0; // Spring force
            
            // Try to get terrain properties from the hit entity
            if let Ok((terrain_props, _)) = terrain_query.get(entity) {
                forces.terrain_properties = Some(terrain_props.clone());
            }
            
            // Update forces based on terrain interaction
            wheel.update_forces(entity, &mut forces, Vec3::ZERO, time.delta_seconds(), &tire_temp_query); // Replace Vec3::ZERO with actual vehicle velocity
        } else {
            forces.contact_point = None;
            forces.normal_force = 0.0;
            forces.terrain_properties = None;
            wheel.update_forces(entity, &mut forces, Vec3::ZERO, time.delta_seconds(), &tire_temp_query);
        }
    }
}