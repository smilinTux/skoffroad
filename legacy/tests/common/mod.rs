use bevy::prelude::*;
use bevy::app::App;
use bevy_rapier3d::prelude::*;

/// Test app configuration for running isolated tests
pub struct TestApp {
    app: App,
}

impl TestApp {
    /// Create a new test app with minimal plugins
    pub fn new() -> Self {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        Self { app }
    }

    /// Create a new test app with physics enabled
    pub fn with_physics() -> Self {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
           .add_plugins(RapierPhysicsPlugin::<NoUserData>::default());
        Self { app }
    }

    /// Add a plugin to the test app
    pub fn add_plugin<T: Plugin>(&mut self, plugin: T) -> &mut Self {
        self.app.add_plugin(plugin);
        self
    }

    /// Add a resource to the test app
    pub fn add_resource<T: Resource>(&mut self, resource: T) -> &mut Self {
        self.app.insert_resource(resource);
        self
    }

    /// Run the app for a specified number of update cycles
    pub fn update_cycles(&mut self, cycles: usize) -> &mut Self {
        for _ in 0..cycles {
            self.app.update();
        }
        self
    }

    /// Get a resource from the app
    pub fn get_resource<T: Resource>(&self) -> Option<&T> {
        self.app.world.get_resource::<T>()
    }

    /// Get a resource mutably from the app
    pub fn get_resource_mut<T: Resource>(&mut self) -> Option<&mut T> {
        self.app.world.get_resource_mut::<T>()
    }
}

/// Helper function to create a test entity with basic components
pub fn spawn_test_entity(app: &mut App) -> Entity {
    app.world.spawn((
        Transform::default(),
        GlobalTransform::default(),
    )).id()
}

/// Helper function to create a test physics entity
pub fn spawn_test_physics_entity(app: &mut App) -> Entity {
    app.world.spawn((
        Transform::default(),
        GlobalTransform::default(),
        RigidBody::Dynamic,
        Collider::cuboid(1.0, 1.0, 1.0),
    )).id()
}

/// Assertion helpers for common test cases
pub mod assert {
    use bevy::math::Vec3;
    use pretty_assertions::assert_eq;

    pub fn assert_vec3_eq(a: Vec3, b: Vec3, epsilon: f32) {
        assert!(
            (a.x - b.x).abs() < epsilon &&
            (a.y - b.y).abs() < epsilon &&
            (a.z - b.z).abs() < epsilon,
            "Vector comparison failed: {:?} != {:?} (epsilon: {})",
            a, b, epsilon
        );
    }

    pub fn assert_transform_eq(a: &Transform, b: &Transform, epsilon: f32) {
        assert_vec3_eq(a.translation, b.translation, epsilon);
        assert_vec3_eq(a.rotation.to_euler(EulerRot::XYZ).into(), 
                      b.rotation.to_euler(EulerRot::XYZ).into(), 
                      epsilon);
        assert_vec3_eq(a.scale, b.scale, epsilon);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_creation() {
        let app = TestApp::new();
        assert!(app.app.world.contains_resource::<Time>());
    }

    #[test]
    fn test_physics_app_creation() {
        let app = TestApp::with_physics();
        assert!(app.app.world.contains_resource::<RapierConfiguration>());
    }

    #[test]
    fn test_resource_management() {
        let mut app = TestApp::new();
        app.add_resource(42_i32);
        assert_eq!(app.get_resource::<i32>(), Some(&42));
    }

    #[test]
    fn test_update_cycles() {
        let mut app = TestApp::new();
        let initial_time = app.get_resource::<Time>().unwrap().elapsed_seconds();
        app.update_cycles(5);
        let final_time = app.get_resource::<Time>().unwrap().elapsed_seconds();
        assert!(final_time > initial_time);
    }
} 