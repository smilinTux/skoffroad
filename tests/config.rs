use std::sync::Once;

static INIT: Once = Once::new();

/// Initialize test environment
pub fn setup() {
    INIT.call_once(|| {
        // Initialize logging for tests
        let subscriber = tracing_subscriber::FmtSubscriber::builder()
            .with_max_level(tracing::Level::DEBUG)
            .with_test_writer()
            .with_thread_ids(true)
            .with_file(true)
            .with_line_number(true)
            .with_target(false)
            .with_ansi(false)
            .with_env_filter("warn,sandk_offroad=debug,test=debug")
            .init();
    });
}

/// Test configuration settings
pub struct TestConfig {
    /// Whether to run slow tests
    pub run_slow_tests: bool,
    /// Whether to run network tests
    pub run_network_tests: bool,
    /// Whether to run GPU tests
    pub run_gpu_tests: bool,
}

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            run_slow_tests: std::env::var("RUN_SLOW_TESTS").is_ok(),
            run_network_tests: std::env::var("RUN_NETWORK_TESTS").is_ok(),
            run_gpu_tests: std::env::var("RUN_GPU_TESTS").is_ok(),
        }
    }
}

/// Helper macro to skip tests based on configuration
#[macro_export]
macro_rules! skip_if {
    ($condition:expr, $message:expr) => {
        if $condition {
            eprintln!("Skipping test: {}", $message);
            return;
        }
    };
}

/// Helper macro for tests that require specific features
#[macro_export]
macro_rules! require_feature {
    ($feature:expr) => {
        if !cfg!(feature = $feature) {
            eprintln!("Skipping test: requires feature '{}'", $feature);
            return;
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_defaults() {
        let config = TestConfig::default();
        assert!(!config.run_slow_tests, "Slow tests should be disabled by default");
        assert!(!config.run_network_tests, "Network tests should be disabled by default");
        assert!(!config.run_gpu_tests, "GPU tests should be disabled by default");
    }

    #[test]
    fn test_skip_if_macro() {
        let slow_test = true;
        skip_if!(slow_test, "slow test");
        // If we get here, the test didn't skip properly
        assert!(false, "Test should have been skipped");
    }
} 