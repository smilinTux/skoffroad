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

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

pub struct TerrainPbrPlugin;

impl Plugin for TerrainPbrPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<TriplanarTerrainMaterial>::default())
            .add_systems(Startup, load_terrain_assets);
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
}

#[derive(ShaderType, Reflect, Debug, Clone, Copy)]
pub struct TriplanarUniforms {
    /// World-units → UV scale for the close-up sample.
    /// 0.25 = 4 m repeat, 0.5 = 2 m repeat. ~0.25 looks natural.
    pub tile_scale:   f32,
    /// Multiplier applied to tile_scale for the detail sample.
    pub detail_scale: f32,
    /// Mix factor close↔detail (0..1).
    pub detail_blend: f32,
    pub _pad:         f32,
}

impl Default for TriplanarUniforms {
    fn default() -> Self {
        Self {
            tile_scale:   0.25,
            detail_scale: 4.0,
            detail_blend: 0.35,
            _pad:         0.0,
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
        let dirt_albedo    = assets.load("materials/terrain/dirt/albedo.jpg");
        let dirt_roughness = assets.load("materials/terrain/dirt/roughness.jpg");

        let mat = TriplanarTerrainMaterial {
            base: StandardMaterial {
                base_color: Color::WHITE,
                perceptual_roughness: 0.9,
                ..default()
            },
            extension: TriplanarTerrainExt {
                uniforms: TriplanarUniforms::default(),
                dirt_albedo,
                dirt_roughness,
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
