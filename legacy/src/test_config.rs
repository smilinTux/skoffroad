use bevy::prelude::*;
use std::path::PathBuf;
use std::sync::Once;

static INIT: Once = Once::new();

/// Initialize test environment
pub fn initialize_test_env() {
    INIT.call_once(|| {
        // Set up any global test configuration here
        std::env::set_var("RUST_BACKTRACE", "1");
        std::env::set_var("RUST_LOG", "debug");
    });
}

/// Test configuration settings
#[derive(Debug, Clone)]
pub struct TestConfig {
    pub test_assets_dir: PathBuf,
    pub test_output_dir: PathBuf,
    pub enable_logging: bool,
    pub enable_timing: bool,
}

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            test_assets_dir: PathBuf::from("tests/assets"),
            test_output_dir: std::env::temp_dir().join("sandk_test_output"),
            enable_logging: true,
            enable_timing: true,
        }
    }
}

impl TestConfig {
    pub fn new() -> Self {
        initialize_test_env();
        Self::default()
    }

    pub fn with_assets_dir(mut self, dir: PathBuf) -> Self {
        self.test_assets_dir = dir;
        self
    }

    pub fn with_output_dir(mut self, dir: PathBuf) -> Self {
        self.test_output_dir = dir;
        self
    }

    pub fn with_logging(mut self, enable: bool) -> Self {
        self.enable_logging = enable;
        self
    }

    pub fn with_timing(mut self, enable: bool) -> Self {
        self.enable_timing = enable;
        self
    }

    pub fn setup(&self) {
        if !self.test_assets_dir.exists() {
            std::fs::create_dir_all(&self.test_assets_dir).unwrap();
        }
        if !self.test_output_dir.exists() {
            std::fs::create_dir_all(&self.test_output_dir).unwrap();
        }
    }

    pub fn cleanup(&self) {
        if self.test_output_dir.exists() {
            let _ = std::fs::remove_dir_all(&self.test_output_dir);
        }
    }
}

/// Test fixture for common test scenarios
pub struct TestFixture {
    pub config: TestConfig,
    pub app: App,
}

impl TestFixture {
    pub fn new() -> Self {
        let config = TestConfig::new();
        config.setup();
        
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        
        Self { config, app }
    }

    pub fn with_config(config: TestConfig) -> Self {
        config.setup();
        
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        
        Self { config, app }
    }

    pub fn with_plugins(mut self, plugins: impl IntoIterator<Item = impl Plugin>) -> Self {
        self.app.add_plugins(plugins);
        self
    }

    pub fn with_resource<T: Resource>(mut self, resource: T) -> Self {
        self.app.insert_resource(resource);
        self
    }

    pub fn with_system<Params>(mut self, system: impl IntoSystem<(), (), Params>) -> Self {
        self.app.add_system(system);
        self
    }
}

impl Drop for TestFixture {
    fn drop(&mut self) {
        self.config.cleanup();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_initialization() {
        let config = TestConfig::new();
        assert!(config.test_assets_dir.exists());
        assert!(config.test_output_dir.exists());
        config.cleanup();
    }

    #[test]
    fn test_config_customization() {
        let custom_assets = PathBuf::from("custom/assets");
        let custom_output = PathBuf::from("custom/output");

        let config = TestConfig::new()
            .with_assets_dir(custom_assets.clone())
            .with_output_dir(custom_output.clone())
            .with_logging(false)
            .with_timing(false);

        assert_eq!(config.test_assets_dir, custom_assets);
        assert_eq!(config.test_output_dir, custom_output);
        assert!(!config.enable_logging);
        assert!(!config.enable_timing);
    }

    #[test]
    fn test_fixture_creation() {
        let fixture = TestFixture::new();
        assert!(fixture.app.world.contains_resource::<Time>());
        assert!(fixture.config.test_assets_dir.exists());
    }

    #[test]
    fn test_fixture_customization() {
        #[derive(Resource)]
        struct TestResource(i32);

        let fixture = TestFixture::new()
            .with_resource(TestResource(42))
            .with_system(|res: Res<TestResource>| {
                assert_eq!(res.0, 42);
            });

        assert!(fixture.app.world.contains_resource::<TestResource>());
    }
} 