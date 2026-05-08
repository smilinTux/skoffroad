use bevy::prelude::*;
use std::path::PathBuf;

/// Helper function to create a test app with minimal plugins
pub fn create_test_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app
}

/// Helper function to create a test app with rendering plugins
pub fn create_test_app_with_rendering() -> App {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        RenderPlugin::default(),
    ));
    app
}

/// Helper function to create a temporary test directory
pub fn create_test_dir(prefix: &str) -> PathBuf {
    let temp_dir = std::env::temp_dir().join(format!("sandk_test_{}", prefix));
    std::fs::create_dir_all(&temp_dir).unwrap();
    temp_dir
}

/// Helper function to cleanup a test directory
pub fn cleanup_test_dir(dir: PathBuf) {
    let _ = std::fs::remove_dir_all(dir);
}

/// Helper struct for test resource management
pub struct TestResources {
    pub temp_dir: PathBuf,
}

impl TestResources {
    pub fn new(prefix: &str) -> Self {
        Self {
            temp_dir: create_test_dir(prefix),
        }
    }
}

impl Drop for TestResources {
    fn drop(&mut self) {
        cleanup_test_dir(self.temp_dir.clone());
    }
}

/// Helper trait for test assertions
pub trait TestAssertions {
    fn assert_component_exists<T: Component>(&self);
    fn assert_resource_exists<T: Resource>(&self);
    fn assert_system_ran<T: IntoSystem<(), (), Marker>, Marker>(&mut self, system: T);
}

impl TestAssertions for App {
    fn assert_component_exists<T: Component>(&self) {
        assert!(self.world.components().get_id::<T>().is_some());
    }

    fn assert_resource_exists<T: Resource>(&self) {
        assert!(self.world.contains_resource::<T>());
    }

    fn assert_system_ran<T: IntoSystem<(), (), Marker>, Marker>(&mut self, system: T) {
        self.add_system(system);
        self.update();
    }
}

/// Helper macro for creating test fixtures
#[macro_export]
macro_rules! test_fixture {
    ($name:ident, $setup:expr) => {
        pub fn $name() -> App {
            let mut app = create_test_app();
            $setup(&mut app);
            app
        }
    };
}

/// Helper macro for creating parameterized tests
#[macro_export]
macro_rules! parameterized_test {
    ($name:ident, $params:expr, $test:expr) => {
        #[test_case($params)]
        fn $name(params: &[(&str, &str)]) {
            for (input, expected) in params {
                $test(input, expected);
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_test_app() {
        let app = create_test_app();
        assert!(app.world.contains_resource::<Time>());
    }

    #[test]
    fn test_create_test_dir() {
        let dir = create_test_dir("test");
        assert!(dir.exists());
        cleanup_test_dir(dir);
    }

    #[test]
    fn test_test_resources() {
        let resources = TestResources::new("test");
        assert!(resources.temp_dir.exists());
        drop(resources);
        assert!(!resources.temp_dir.exists());
    }

    #[test]
    fn test_assertions() {
        #[derive(Component)]
        struct TestComponent;

        #[derive(Resource)]
        struct TestResource;

        let mut app = create_test_app();
        app.world.spawn(TestComponent);
        app.insert_resource(TestResource);

        app.assert_component_exists::<TestComponent>();
        app.assert_resource_exists::<TestResource>();
    }
} 