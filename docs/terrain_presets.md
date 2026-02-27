# Terrain Presets Documentation

This document details the available terrain presets and their characteristics for the terrain generation system.

## Overview

The terrain generation system provides several pre-configured terrain presets that can be used to quickly generate different types of terrain. Each preset is carefully tuned to produce realistic and visually interesting landscapes.

## Available Presets

### Mountains
- **Description**: Dramatic mountainous terrain with sharp peaks and deep valleys
- **Characteristics**:
  - High elevation range (200.0 height scale)
  - Sharp ridge formations with significant erosion
  - Prominent peaks with varied heights
  - Deep valleys between mountain ranges
- **Best for**: Alpine environments, mountain ranges, challenging off-road terrain

### Desert
- **Description**: Arid terrain with sand dunes and deep canyons
- **Characteristics**:
  - Moderate elevation range (100.0 height scale)
  - Large dune formations with smooth transitions
  - Deep canyon systems with eroded walls
  - High erosion on canyon features
- **Best for**: Desert environments, dune racing, canyon exploration

### RiverValley
- **Description**: Terrain with prominent river channels and elevated plateaus
- **Characteristics**:
  - Significant elevation range (150.0 height scale)
  - Meandering river channels with smooth banks
  - Large plateau formations
  - Natural transitions between features
- **Best for**: River-based gameplay, mixed terrain challenges

### Volcanic
- **Description**: Dramatic volcanic terrain with peaks and impact craters
- **Characteristics**:
  - Extreme elevation range (250.0 height scale)
  - Sharp volcanic peaks
  - Impact craters of varying sizes
  - Minimal warping for more geometric features
- **Best for**: Alien landscapes, volcanic regions, extreme terrain

### Coastal
- **Description**: Coastal terrain with cliffs and beach areas
- **Characteristics**:
  - Moderate elevation range (120.0 height scale)
  - Coastal cliff formations
  - Gentle valleys leading to water
  - Natural erosion patterns
- **Best for**: Coastal racing, beach environments

### Arctic
- **Description**: Cold region terrain with glacial features and ice formations
- **Characteristics**:
  - High elevation range (180.0 height scale)
  - Glacial ridge formations
  - Smooth plateau areas
  - Moderate erosion for ice-worn features
- **Best for**: Winter environments, ice racing, arctic exploration

### Canyonlands
- **Description**: Extensive canyon networks with mesas and gorges
- **Characteristics**:
  - Significant elevation range (160.0 height scale)
  - Deep canyon systems
  - Elevated plateau regions
  - Heavy erosion modeling
- **Best for**: Technical driving challenges, canyon exploration

### Hills
- **Description**: Gentle rolling hills with smooth transitions
- **Characteristics**:
  - Low elevation range (80.0 height scale)
  - Smooth, rolling terrain
  - Minimal sharp features
  - Light erosion
- **Best for**: Beginner areas, relaxed driving, open terrain

### Islands
- **Description**: Tropical island terrain with varied elevation
- **Characteristics**:
  - Moderate elevation range (140.0 height scale)
  - Central peak formations
  - Coastal valleys
  - Natural erosion patterns
- **Best for**: Island environments, coastal exploration

### Badlands
- **Description**: Eroded formations with ridges and canyons
- **Characteristics**:
  - Moderate elevation range (130.0 height scale)
  - Sharp ridge formations
  - Eroded canyon features
  - Heavy erosion modeling
- **Best for**: Technical driving, challenging terrain

## Usage Example

```rust
use bevy::prelude::*;
use your_game::terrain::{TerrainConfig, TerrainPreset};

// Create a new terrain configuration using a preset
fn setup_terrain(mut commands: Commands) {
    // Create mountainous terrain
    let mountain_config = TerrainConfig::new_preset(TerrainPreset::Mountains);
    
    // Or create desert terrain
    let desert_config = TerrainConfig::new_preset(TerrainPreset::Desert);
    
    // Spawn the terrain with the chosen configuration
    commands.spawn((
        TerrainBundle::new(mountain_config),
        Name::new("Mountain Terrain"),
    ));
}

// Example of creating multiple terrain regions
fn create_mixed_terrain(mut commands: Commands) {
    // Create an arctic region
    let arctic_config = TerrainConfig::new_preset(TerrainPreset::Arctic);
    arctic_config.size = Vec2::new(1000.0, 1000.0);
    commands.spawn((
        TerrainBundle::new(arctic_config),
        Transform::from_xyz(-1000.0, 0.0, 0.0),
        Name::new("Arctic Region"),
    ));
    
    // Create a volcanic region
    let volcanic_config = TerrainConfig::new_preset(TerrainPreset::Volcanic);
    volcanic_config.size = Vec2::new(1000.0, 1000.0);
    commands.spawn((
        TerrainBundle::new(volcanic_config),
        Transform::from_xyz(1000.0, 0.0, 0.0),
        Name::new("Volcanic Region"),
    ));
}
```

## Customizing Presets

While presets provide good starting points, you can further customize them:

```rust
fn create_custom_mountains(mut commands: Commands) {
    // Start with the mountain preset
    let mut config = TerrainConfig::new_preset(TerrainPreset::Mountains);
    
    // Customize the configuration
    config.height_scale = 250.0; // Make mountains taller
    config.frequency = 0.002; // Increase feature frequency
    
    // Add an additional noise layer
    config.additional_layers.push(NoiseLayer {
        feature_type: TerrainFeatureType::Peak,
        frequency: 0.005,
        amplitude: 0.3,
        octaves: 3,
        persistence: 0.5,
        lacunarity: 2.0,
        enable_warping: true,
        warp_strength: 20.0,
        mask_frequency: 0.002,
        threshold: 0.6,
        smoothing: 0.2,
        erosion_iterations: 2,
    });
    
    // Spawn the customized terrain
    commands.spawn((
        TerrainBundle::new(config),
        Name::new("Custom Mountains"),
    ));
}
```

## Performance Considerations

- Higher resolution and more noise layers will increase generation time
- Erosion iterations significantly impact performance
- Consider using lower resolution for distant terrain
- LOD thresholds can be adjusted per preset for optimal performance

## Best Practices

1. **Choosing a Preset**:
   - Consider the gameplay requirements
   - Match the terrain to the environment theme
   - Think about the desired difficulty level

2. **Customization**:
   - Start with the closest matching preset
   - Make incremental adjustments
   - Test with different erosion levels
   - Consider the performance impact

3. **Terrain Composition**:
   - Use multiple presets for varied landscapes
   - Blend between different terrain types
   - Add custom features for unique areas

4. **Testing**:
   - Test with different vehicle types
   - Verify performance at different LOD levels
   - Check terrain features at various scales 