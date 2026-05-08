# SandK Offroad Game Architecture

## Overview
SandK Offroad is built using the Bevy game engine, following Entity Component System (ECS) architecture. The game is structured into modular plugins, each responsible for a specific aspect of functionality.

## Core Structure

### Modules
- `game/mod.rs`: Main game module and plugin initialization
- `game/plugins/`: Custom Bevy plugins for major game systems
- `game/systems/`: Game logic and update systems
- `game/components/`: ECS components for game entities
- `game/resources/`: Game-wide resources and state
- `game/assets/`: Asset management and loading
- `game/physics/`: Physics simulation and collision handling
- `game/rendering/`: Custom rendering and graphics systems
- `game/entities/`: Entity spawning and management

### Plugin Architecture
The game uses a modular plugin architecture through `GamePluginGroup`:
- `StatePlugin`: Game state management
- `DebugPlugin`: Debug visualization and tools
- `InputPlugin`: Input handling and mapping
- `VehiclePlugin`: Vehicle physics and controls
- `PhysicsPlugin`: Physics world and simulation
- `CameraPlugin`: Camera controls and following
- `UiPlugin`: User interface and HUD

## Core Systems

### State Management
- Game states are managed through `GameState` resource
- State transitions trigger appropriate system changes
- Supports pausing, menus, and gameplay states

### Physics Integration
- Uses `bevy_rapier3d` for physics simulation
- Custom vehicle physics implementation
- Terrain collision and deformation

### Input Handling
- Configurable input mapping
- Support for keyboard, mouse, and controllers
- Context-sensitive input states

### Asset Management
- Centralized asset loading through `AssetServer`
- Custom asset types for game-specific resources
- Asset hot-reloading for development

### Rendering Pipeline
- PBR materials for realistic rendering
- Custom shaders for special effects
- Dynamic lighting and shadows

## Development Guidelines

### Adding New Features
1. Create a new plugin in `game/plugins/` if adding major functionality
2. Implement components in `game/components/`
3. Add systems in `game/systems/`
4. Register plugin in `GamePluginGroup`

### Testing
- Unit tests for components and systems
- Integration tests for plugin functionality
- Performance benchmarks for critical systems

### Best Practices
- Follow Rust naming conventions
- Document public APIs and complex systems
- Use type-safe interfaces and strong typing
- Minimize unsafe code and document when necessary

## Performance Considerations
- Use ECS queries efficiently
- Batch similar operations
- Profile and optimize hot paths
- Use appropriate system ordering

## Future Expansion
- Modular vehicle customization
- Advanced terrain generation
- Multiplayer networking
- Additional game modes

## Directory Structure
```
src/game/
├── mod.rs                 # Main game module and plugin registration
├── components/           # ECS components
├── systems/             # Game systems
├── plugins/            # Custom Bevy plugins
├── resources/         # Game resources and state
├── assets/           # Asset management
├── entities/         # Entity spawning and management
├── physics/          # Physics simulation
├── rendering/        # Rendering pipeline
├── debug.rs          # Debug visualization and tools
├── state.rs          # Game state management
├── ui.rs             # User interface components
└── constants.rs      # Game constants and configuration
```

## Plugin Architecture

### Core Plugins
1. **State Plugin**
   - Manages game state transitions
   - Handles loading, menu, and gameplay states

2. **Debug Plugin**
   - Debug visualization tools
   - Performance metrics
   - Debug controls

3. **Input Plugin**
   - Input handling and mapping
   - Controller support

4. **Vehicle Plugin**
   - Vehicle physics
   - Vehicle components
   - Vehicle systems

5. **Physics Plugin**
   - Physics world management
   - Collision detection
   - Physics constraints

6. **Camera Plugin**
   - Camera movement
   - View modes
   - Camera effects

7. **UI Plugin**
   - HUD elements
   - Menu systems
   - Debug overlays

### Feature Plugins

1. **Particle System Plugin**
   - GPU-accelerated particle simulation
   - Particle effects and emitters
   - Material system with texture atlas

2. **Post-Processing Plugin**
   - Post-processing effects chain
   - Screen-space effects
   - Visual effects management

3. **Lighting Plugin**
   - Dynamic lighting system
   - Shadow mapping
   - Light types and management

## Asset Management
- Uses Bevy's AssetServer for resource loading
- Custom asset types for game-specific resources
- Asset validation and error handling

## Testing Strategy
- Unit tests for components and systems
- Integration tests for plugin interactions
- Performance benchmarks
- Visual validation tools

## Debug Tools
- FPS display
- Physics debug visualization
- Vehicle debug information
- Particle system debugging

## Performance Considerations
- GPU-based particle system
- Efficient physics simulation
- Asset loading optimization
- Memory management

## Future Extensions
- Multiplayer networking
- Advanced vehicle customization
- Additional game modes
- Community features

## Development Guidelines
1. Follow Rust best practices and idioms
2. Use Bevy's ECS patterns consistently
3. Keep plugins focused and modular
4. Document public APIs and systems
5. Write tests for new functionality
6. Profile performance regularly 