use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

mod wheel;
mod terrain_properties;
mod tire_temperature;

pub use wheel::{Wheel, WheelForces, WheelPhysicsPlugin};
pub use terrain_properties::{TerrainProperties, PhysicsTerrainType};
pub use tire_temperature::{TireTemperature, TireTemperatureState};

pub struct PhysicsPlugin;

impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(RapierPhysicsPlugin::<NoUserData>::default())
            .add_plugins(RapierDebugRenderPlugin::default())
            .insert_resource(RapierConfiguration {
                gravity: Vec3::new(0.0, -9.81, 0.0),
                ..default()
            })
            .add_systems(Startup, setup_physics);
    }
}

#[derive(Component)]
pub struct Terrain {
    pub width: f32,
    pub height: f32,
}

fn setup_physics(mut commands: Commands) {
    // Create ground plane with friction
    commands.spawn((
        Collider::cuboid(50.0, 0.1, 50.0),
        TransformBundle::from(Transform::from_xyz(0.0, -0.1, 0.0)),
        RigidBody::Fixed,
        Friction::coefficient(1.0),
    ));
} 