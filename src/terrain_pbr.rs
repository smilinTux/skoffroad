// Triplanar terrain material — Sprint 41 commit 3.
//
// Wraps StandardMaterial with a small extension that samples world-space
// triplanar projections of CC0 PBR textures from `assets/materials/terrain/`.
// Currently single-layer (dirt). Commit 4 extends this to a 4-way splat
// blend (dirt / grass / rock / mud) keyed off slope/height/the existing
// terrain_splatmap data.
//
// Used only when `GraphicsQuality::triplanar_terrain()` is true (Medium+).
// Low keeps the legacy vertex-color StandardMaterial path in `terrain.rs`.

use bevy::pbr::{ExtendedMaterial, MaterialExtension, MaterialPlugin};
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::shader::ShaderRef;

use crate::graphics_quality::GraphicsQuality;
use crate::storm::StormState;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

pub struct TerrainPbrPlugin;

impl Plugin for TerrainPbrPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<TriplanarTerrainMaterial>::default())
            .add_systems(Startup, load_terrain_assets)
            .add_systems(Update, drive_wetness);
    }
}

pub type TriplanarTerrainMaterial =
    ExtendedMaterial<StandardMaterial, TriplanarTerrainExt>;

/// Resource holding the dirt-layer texture handles + the assembled
/// extended material that `terrain.rs` clones onto the terrain entity.
/// `material` is `None` until Startup completes.
#[derive(Resource, Default)]
pub struct TerrainPbrAssets {
    pub material: Option<Handle<TriplanarTerrainMaterial>>,
}

#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
pub struct TriplanarTerrainExt {
    #[uniform(100)]
    pub uniforms: TriplanarUniforms,

    #[texture(101)]
    #[sampler(102)]
    pub dirt_albedo: Handle<Image>,
    #[texture(103)]
    #[sampler(104)]
    pub dirt_roughness: Handle<Image>,

    #[texture(105)]
    #[sampler(106)]
    pub grass_albedo: Handle<Image>,
    #[texture(107)]
    #[sampler(108)]
    pub grass_roughness: Handle<Image>,

    #[texture(109)]
    #[sampler(110)]
    pub rock_albedo: Handle<Image>,
    #[texture(111)]
    #[sampler(112)]
    pub rock_roughness: Handle<Image>,

    #[texture(113)]
    #[sampler(114)]
    pub mud_albedo: Handle<Image>,
    #[texture(115)]
    #[sampler(116)]
    pub mud_roughness: Handle<Image>,
}

#[derive(ShaderType, Reflect, Debug, Clone, Copy)]
pub struct TriplanarUniforms {
    /// World-units → UV scale for the close-up sample.
    /// 0.25 = 4 m repeat, 0.5 = 2 m repeat. ~0.25 looks natural.
    pub tile_scale:    f32,
    /// Multiplier applied to tile_scale for the detail sample.
    pub detail_scale:  f32,
    /// Mix factor close↔detail (0..1).
    pub detail_blend:  f32,
    /// 1.0 = full 4-layer splat blend, 0.0 = dirt-only.
    /// Medium tier ramps this down to keep cost lower while keeping triplanar.
    pub blend_strength: f32,
    /// 0..1; eased value driven by StormState. 1 = soaking wet (darker albedo,
    /// lower roughness so headlights pop on the puddles).
    pub wetness:        f32,
    pub _pad0:          f32,
    pub _pad1:          f32,
    pub _pad2:          f32,
}

impl Default for TriplanarUniforms {
    fn default() -> Self {
        Self {
            tile_scale:     0.25,
            detail_scale:   4.0,
            detail_blend:   0.35,
            blend_strength: 1.0,
            wetness:        0.0,
            _pad0: 0.0,
            _pad1: 0.0,
            _pad2: 0.0,
        }
    }
}

impl MaterialExtension for TriplanarTerrainExt {
    fn fragment_shader() -> ShaderRef {
        "shaders/triplanar_terrain.wgsl".into()
    }
}

// ---------------------------------------------------------------------------
// Startup: load textures + build the material if quality permits
// ---------------------------------------------------------------------------

fn load_terrain_assets(
    mut commands: Commands,
    quality: Res<GraphicsQuality>,
    assets: Res<AssetServer>,
    mut materials: ResMut<Assets<TriplanarTerrainMaterial>>,
) {
    let assets_resource = if quality.triplanar_terrain() {
        // Splat strength scales with quality tier:
        //   High   -> 1.0 (full 4-layer blend)
        //   Medium -> 0.55 (toward dirt-only, but still varied)
        let blend_strength = match *quality {
            GraphicsQuality::High => 1.0,
            _                     => 0.55,
        };

        let mat = TriplanarTerrainMaterial {
            base: StandardMaterial {
                base_color: Color::WHITE,
                perceptual_roughness: 0.9,
                ..default()
            },
            extension: TriplanarTerrainExt {
                uniforms: TriplanarUniforms {
                    blend_strength,
                    ..default()
                },
                dirt_albedo:     assets.load("materials/terrain/dirt/albedo.jpg"),
                dirt_roughness:  assets.load("materials/terrain/dirt/roughness.jpg"),
                grass_albedo:    assets.load("materials/terrain/grass/albedo.jpg"),
                grass_roughness: assets.load("materials/terrain/grass/roughness.jpg"),
                rock_albedo:     assets.load("materials/terrain/rock/albedo.jpg"),
                rock_roughness:  assets.load("materials/terrain/rock/roughness.jpg"),
                mud_albedo:      assets.load("materials/terrain/mud/albedo.jpg"),
                mud_roughness:   assets.load("materials/terrain/mud/roughness.jpg"),
            },
        };

        TerrainPbrAssets {
            material: Some(materials.add(mat)),
        }
    } else {
        // Low quality: leave material as None. terrain.rs falls back
        // to the plain vertex-color StandardMaterial path.
        TerrainPbrAssets::default()
    };

    commands.insert_resource(assets_resource);
}

// ---------------------------------------------------------------------------
// Drive the wetness uniform from StormState
// ---------------------------------------------------------------------------
//
// We exponentially ease `wetness` toward StormState.active (0 or 1) so dry/wet
// transitions take a few seconds rather than snapping. The terrain material is
// shared, so a single mutation each frame is enough.

fn drive_wetness(
    quality: Res<GraphicsQuality>,
    storm: Option<Res<StormState>>,
    pbr_assets: Option<Res<TerrainPbrAssets>>,
    mut materials: ResMut<Assets<TriplanarTerrainMaterial>>,
    time: Res<Time>,
) {
    if !quality.wet_shader() {
        return;
    }
    let Some(handle) = pbr_assets.as_ref().and_then(|a| a.material.clone()) else {
        return;
    };
    let Some(mat) = materials.get_mut(&handle) else {
        return;
    };

    let target = storm.map(|s| if s.active { 1.0 } else { 0.0 }).unwrap_or(0.0);
    let dt = time.delta_secs();
    // Time constant ~2 s for full transition.
    let ease = 1.0 - (-dt / 2.0).exp();
    mat.extension.uniforms.wetness +=
        (target - mat.extension.uniforms.wetness) * ease;
}
