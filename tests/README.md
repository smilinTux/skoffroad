# SandK Offroad Testing Framework

This document outlines the testing practices and framework used in the SandK Offroad game project.

## Test Structure

The test suite is organized into several categories:

- `tests/common/`: Common test utilities and helpers
- `tests/game/`: Game system integration tests
- `tests/backend/`: Backend service tests
- `performance_tests.rs`: Performance benchmarks using Criterion
- `config.rs`: Test configuration and environment setup

## Test Categories

### Unit Tests

Unit tests are written alongside the code they test in the same module. They focus on testing individual components and functions in isolation.

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_component_behavior() {
        // Test implementation
    }
}
```

### Integration Tests

Integration tests verify the interaction between different systems and components. These are located in the `tests/` directory and use the `TestApp` utility for setup.

```rust
use sandk_offroad::game::MyPlugin;
use crate::common::TestApp;

#[test]
fn test_system_interaction() {
    let mut app = TestApp::new();
    app.add_plugin(MyPlugin::default())
       .update_cycles(5);
    // Test assertions
}
```

### Performance Tests

Performance benchmarks use the Criterion framework to measure and track performance metrics:

```rust
fn bench_my_feature(c: &mut Criterion) {
    let mut group = c.benchmark_group("Feature Name");
    group.bench_function("operation_name", |b| {
        b.iter(|| {
            // Code to benchmark
        });
    });
    group.finish();
}
```

## Test Utilities

### TestApp

The `TestApp` struct provides a convenient way to set up and run tests with Bevy:

```rust
let mut app = TestApp::new(); // Basic setup
// or
let mut app = TestApp::with_physics(); // Setup with physics enabled

app.add_plugin(MyPlugin::default())
   .add_resource(MyResource::default())
   .update_cycles(5);
```

### Assertion Helpers

Custom assertion helpers are available in `common/mod.rs`:

```rust
use crate::common::assert::assert_vec3_eq;

#[test]
fn test_vector_comparison() {
    let a = Vec3::new(1.0, 2.0, 3.0);
    let b = Vec3::new(1.0001, 2.0, 3.0);
    assert_vec3_eq(a, b, 0.001);
}
```

## Test Configuration

Tests can be configured using environment variables:

- `RUN_SLOW_TESTS=1`: Enable slow tests
- `RUN_NETWORK_TESTS=1`: Enable network-dependent tests
- `RUN_GPU_TESTS=1`: Enable GPU-dependent tests

Use the `skip_if!` macro to conditionally skip tests:

```rust
#[test]
fn test_gpu_feature() {
    let config = TestConfig::default();
    skip_if!(!config.run_gpu_tests, "GPU tests disabled");
    // Test implementation
}
```

## Running Tests

### Basic Test Run

```bash
cargo test
```

### Run with All Features

```bash
cargo test --all-features
```

### Run Performance Tests

```bash
cargo bench
```

### Run Specific Test Categories

```bash
# Run only unit tests
cargo test --lib

# Run only integration tests
cargo test --test '*'

# Run specific test file
cargo test --test weather_tests
```

## Best Practices

1. **Test Organization**
   - Keep unit tests close to the code they test
   - Use integration tests for system interactions
   - Group related tests in test modules

2. **Test Isolation**
   - Use `TestApp` to ensure clean test environment
   - Avoid test interdependencies
   - Clean up resources after tests

3. **Assertions**
   - Use custom assertion helpers for common comparisons
   - Include descriptive error messages
   - Use `pretty_assertions` for complex comparisons

4. **Performance Testing**
   - Write benchmarks for performance-critical code
   - Use realistic test data
   - Track performance changes over time

5. **Test Coverage**
   - Aim for high test coverage
   - Test edge cases and error conditions
   - Include both positive and negative test cases

## Adding New Tests

When adding new tests:

1. Determine the appropriate test category
2. Use existing test utilities and helpers
3. Follow the established naming conventions
4. Include documentation comments
5. Verify test isolation
6. Run the full test suite before committing

## Debugging Tests

For detailed test output:

```bash
RUST_LOG=debug cargo test -- --nocapture
```

For performance test analysis:

```bash
cargo bench --bench performance_tests -- --verbose
```

## Continuous Integration

Tests are automatically run in CI:

- Unit and integration tests on every PR
- Performance tests on main branch
- Coverage reports generated and tracked

See `.github/workflows/test.yml` for CI configuration details. 