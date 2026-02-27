# Asset Management System

This document outlines the asset management system for the SandK Offroad game project.

## Directory Structure

```
assets/
├── vehicles/
│   ├── configs/           # Vehicle configuration files
│   ├── models/           # 3D models for vehicles
│   └── textures/         # Vehicle textures
├── terrain/
│   ├── heightmaps/      # Terrain height data
│   ├── textures/        # Terrain textures
│   └── props/           # Environmental props
├── ui/
│   ├── fonts/           # UI fonts
│   ├── icons/           # UI icons
│   └── themes/          # UI theme configurations
└── audio/
    ├── music/           # Background music
    ├── sfx/             # Sound effects
    └── ambient/         # Ambient sounds
```

## Asset Types

### Vehicle Assets

#### VehicleConfig
```rust
struct VehicleConfig {
    name: String,
    mass: f32,
    suspension: SuspensionConfig,
    engine: EngineConfig,
    // ... other vehicle properties
}
```

- Location: `assets/vehicles/configs/*.json`
- Format: JSON configuration files
- Validation: Automated validation during loading
- Hot-reloading: Supported for quick iteration

#### Vehicle Models
- Location: `assets/vehicles/models/*.glb`
- Format: GLTF/GLB (preferred) or FBX
- Requirements:
  - Proper scale (1 unit = 1 meter)
  - Named bones for suspension points
  - Collision mesh marked with "_col" suffix

### Terrain Assets

#### Heightmaps
- Location: `assets/terrain/heightmaps/*.r16`
- Format: 16-bit raw heightmap data
- Size: Must be power of 2 + 1 (e.g., 513x513)
- Scale: Height values 0-65535 mapped to terrain height

#### Terrain Textures
- Location: `assets/terrain/textures/*.ktx2`
- Format: KTX2 with mipmap chain
- Requirements:
  - Power of 2 dimensions
  - Splatmap alpha channels
  - Normal maps for each texture

### UI Assets

#### Fonts
- Location: `assets/ui/fonts/*.ttf`
- Format: TrueType fonts
- Requirements:
  - Include required character ranges
  - Multiple weights when needed

#### UI Themes
- Location: `assets/ui/themes/*.json`
- Format: JSON theme configuration
- Hot-reloading: Supported
- Validation: Schema-based validation

### Audio Assets

#### Sound Effects
- Location: `assets/audio/sfx/*.ogg`
- Format: Ogg Vorbis
- Requirements:
  - 44.1kHz sample rate
  - Normalized volume levels
  - Proper looping points if needed

## Asset Loading System

### Loading Process

1. Asset Discovery
   - Scan asset directories
   - Match files to registered loaders
   - Build asset dependency graph

2. Validation
   - Check file formats
   - Validate configurations
   - Verify dependencies

3. Loading
   - Parallel asset loading
   - Progress tracking
   - Error handling

4. Hot Reloading
   - Watch for file changes
   - Reload modified assets
   - Update dependent systems

### Usage Example

```rust
// Register asset types
app.add_asset::<VehicleConfig>();
app.init_asset_loader::<VehicleConfigLoader>();

// Load assets
fn load_vehicle(asset_server: Res<AssetServer>) -> Handle<VehicleConfig> {
    asset_server.load("vehicles/configs/jeep.json")
}

// Use loaded assets
fn spawn_vehicle(
    vehicles: Res<Assets<VehicleConfig>>,
    handle: Handle<VehicleConfig>
) {
    if let Some(config) = vehicles.get(&handle) {
        // Use the config to spawn vehicle
    }
}
```

## Asset Optimization

### Texture Optimization
- Use KTX2 format with supercompression
- Generate mipmaps
- Compress normal maps
- Atlas small textures

### Model Optimization
- Remove unused vertices/faces
- Optimize mesh for GPU
- Use LOD levels
- Share materials

### Audio Optimization
- Proper compression settings
- Stream large audio files
- Use spatial audio zones
- Pool sound instances

## Asset Creation Guidelines

### Textures
- Power of 2 dimensions
- Maximum size: 4096x4096
- Proper UV mapping
- PBR material workflow

### Models
- Clean topology
- Proper UV unwrapping
- Named materials
- Collision meshes
- LOD setup

### Audio
- Normalized levels
- Proper looping points
- Consistent sample rates
- Spatial audio setup

## Version Control

### Large File Storage
- Use Git LFS for:
  - Textures
  - Models
  - Audio files
  - Large configs

### Asset Tracking
- Track binary files
- Version assets with code
- Document major changes

## Tools and Utilities

### Asset Processing
- Texture compression
- Model optimization
- Audio conversion
- Config validation

### Development Tools
- Asset preview
- Hot reload system
- Validation checks
- Asset browser

## Troubleshooting

### Common Issues

1. **Missing Assets**
   - Check file paths
   - Verify asset registration
   - Check case sensitivity
   - Validate dependencies

2. **Loading Errors**
   - Verify file format
   - Check asset validation
   - Review error logs
   - Test dependencies

3. **Performance Issues**
   - Monitor memory usage
   - Check asset sizes
   - Review loading order
   - Optimize assets

## Best Practices

1. **Asset Creation**
   - Follow naming conventions
   - Maintain consistent scale
   - Document special requirements
   - Test before commit

2. **Asset Usage**
   - Load assets asynchronously
   - Implement proper cleanup
   - Handle loading errors
   - Use asset references

3. **Maintenance**
   - Regular cleanup
   - Version control
   - Documentation updates
   - Performance monitoring