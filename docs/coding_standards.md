# SandK Offroad Coding Standards

This document outlines the coding standards and best practices for the SandK Offroad game project.

## Rust Code Style

### Formatting
- Use `rustfmt` for consistent code formatting
- Maximum line length: 100 characters
- Use 4 spaces for indentation
- Run `cargo fmt` before committing

### Naming Conventions

1. **Variables and Functions**
   - Use snake_case for variables and functions
   - Be descriptive and clear
   - Avoid abbreviations unless widely known
   ```rust
   let vehicle_position = Vec3::new(0.0, 0.0, 0.0);
   fn update_vehicle_physics() { ... }
   ```

2. **Types and Traits**
   - Use PascalCase for types, traits, and enums
   - Be descriptive and specific
   ```rust
   struct VehicleComponent { ... }
   trait PhysicsObject { ... }
   enum SuspensionType { ... }
   ```

3. **Constants and Statics**
   - Use SCREAMING_SNAKE_CASE for constants
   - Group related constants in modules
   ```rust
   const MAX_VEHICLE_SPEED: f32 = 200.0;
   static DEFAULT_GRAVITY: Vec3 = Vec3::new(0.0, -9.81, 0.0);
   ```

4. **Modules**
   - Use snake_case for module names
   - Create meaningful hierarchies
```rust
   mod vehicle;
   mod physics;
   mod terrain;
   ```

### Code Organization

1. **File Structure**
   ```rust
   // 1. Module declarations
   mod submodule;
   
   // 2. Use statements
   use bevy::prelude::*;
   use crate::physics::*;
   
   // 3. Constants/Statics
   const CONFIG_PATH: &str = "config/vehicle.ron";
   
   // 4. Types/Traits
   struct Vehicle { ... }
   
   // 5. Implementations
impl Vehicle { ... }
   
   // 6. Tests module (if applicable)
   #[cfg(test)]
   mod tests { ... }
   ```

2. **Module Organization**
   - Keep modules focused and single-purpose
   - Use `pub(crate)` for internal APIs
   - Document module-level decisions

### Documentation

1. **Doc Comments**
   - Use `///` for public items
   - Include examples for complex functionality
   - Document panics and errors
```rust
   /// Calculates the suspension force based on compression and velocity.
   ///
   /// # Arguments
   /// * `compression` - Current suspension compression in meters
   /// * `velocity` - Compression velocity in m/s
   ///
   /// # Returns
   /// Force in Newtons
///
/// # Examples
/// ```
   /// let force = calculate_suspension_force(0.1, -0.5);
   /// assert!(force > 0.0);
/// ```
   fn calculate_suspension_force(compression: f32, velocity: f32) -> f32 { ... }
   ```

2. **Comments**
   - Use `//` for implementation details
   - Explain complex algorithms
   - Document temporary solutions or TODOs

### Error Handling

1. **Result and Option**
   - Use `Result` for recoverable errors
   - Use `Option` for optional values
   - Avoid unwrap() in production code
```rust
   fn load_vehicle_config(path: &Path) -> Result<VehicleConfig, ConfigError> { ... }
   ```

2. **Custom Errors**
   - Create specific error types
   - Implement std::error::Error
   - Provide helpful error messages
```rust
   #[derive(Debug, Error)]
pub enum VehicleError {
       #[error("Invalid wheel count: {0}")]
       InvalidWheelCount(u32),
       #[error("Configuration not found: {0}")]
       ConfigNotFound(String),
   }
   ```

### Performance Considerations

1. **Memory Management**
   - Minimize allocations in hot paths
   - Use appropriate data structures
   - Consider using object pools
   ```rust
   // Prefer
   let mut buffer = Vec::with_capacity(size);
   // Over
   let mut buffer = Vec::new();
   ```

2. **Concurrency**
   - Use Bevy's ECS for parallelism
   - Avoid blocking operations
   - Document thread safety requirements

### Testing

1. **Unit Tests**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
       fn test_suspension_force() {
           let force = calculate_suspension_force(0.1, -0.5);
           assert!(force > 0.0);
           assert!(force < MAX_SUSPENSION_FORCE);
       }
   }
   ```

2. **Integration Tests**
   - Test component interactions
   - Use realistic test data
   - Cover edge cases

3. **Benchmarks**
   - Benchmark performance-critical code
   - Use criterion for benchmarking
   - Track performance regressions

### Safety and Correctness

1. **Type Safety**
   - Use newtype pattern for strong typing
   - Leverage the type system for invariants
   ```rust
   #[derive(Debug, Clone, Copy)]
   pub struct Speed(f32);

   impl Speed {
       pub fn new(value: f32) -> Result<Self, VehicleError> {
           if value >= 0.0 {
               Ok(Speed(value))
           } else {
               Err(VehicleError::InvalidSpeed(value))
           }
    }
}
```

2. **Unsafe Code**
   - Minimize unsafe blocks
   - Document safety requirements
   - Explain why unsafe is necessary

### Dependencies

1. **Dependency Management**
   - Keep dependencies up to date
   - Review security advisories
   - Document version requirements

2. **Feature Flags**
   - Use features for optional functionality
   - Document feature combinations
   - Test all feature combinations

## Bevy-Specific Standards

### Components

1. **Component Design**
   - Keep components focused
   - Use appropriate derive macros
   - Document component relationships
   ```rust
   #[derive(Component, Debug)]
   pub struct Vehicle {
       pub mass: f32,
       pub center_of_mass: Vec3,
   }
   ```

2. **Systems**
   - Use appropriate system sets
   - Document system ordering
   - Handle edge cases
```rust
   fn update_vehicle_physics(
       mut query: Query<(&Vehicle, &mut Transform)>,
       time: Res<Time>,
   ) {
       // ...
   }
   ```

### Resources

1. **Resource Management**
   - Use appropriate resource types
   - Document resource dependencies
   - Handle resource initialization
   ```rust
   #[derive(Resource)]
   pub struct PhysicsConfig {
       pub timestep: f32,
       pub iterations: u32,
   }
   ```

### Plugin Organization

1. **Plugin Structure**
   - Group related functionality
   - Document plugin dependencies
   - Handle plugin ordering
   ```rust
   pub struct VehiclePlugin;

   impl Plugin for VehiclePlugin {
       fn build(&self, app: &mut App) {
           app.add_systems(Update, update_vehicle_physics)
              .init_resource::<PhysicsConfig>();
       }
   }
   ```

## Tools and Automation

1. **Required Tools**
   - rustfmt for formatting
   - clippy for linting
   - cargo-audit for security

2. **CI/CD Checks**
   - Run all tests
   - Check formatting
   - Run clippy
   - Check documentation

## Version Control

1. **Commit Guidelines**
   - Follow conventional commits
   - Keep commits focused
   - Reference issues/tasks

2. **Branch Management**
   - Keep branches up to date
   - Clean up merged branches
   - Use meaningful branch names

## Additional Resources

- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [Bevy Engine Style Guide](https://bevyengine.org/learn/book/introduction)
- [Project Development Workflow](./development_workflow.md)
- [Contribution Guidelines](./contribution_guidelines.md)