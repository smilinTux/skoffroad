pub mod common;
pub mod config;

#[cfg(test)]
mod game {
    pub mod weather_tests;
}

// Re-export commonly used test utilities
pub use common::TestApp;
pub use common::assert::{assert_vec3_eq, assert_transform_eq};
pub use config::{TestConfig, setup};

// Initialize test environment when running tests
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_environment_setup() {
        setup();
        // Verify test environment is properly initialized
        let config = TestConfig::default();
        assert!(!config.run_slow_tests || std::env::var("RUN_SLOW_TESTS").is_ok());
    }
} 