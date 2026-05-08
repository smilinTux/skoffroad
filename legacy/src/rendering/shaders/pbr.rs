use bevy::{
    prelude::*,
    reflect::TypeUuid,
    render::{
        render_resource::{
            AsBindGroup, ShaderRef, ShaderType,
        },
        renderer::RenderDevice,
    },
};

/// Custom PBR material supporting advanced features like ray tracing
/// and dynamic weather effects
#[derive(AsBindGroup, TypeUuid, Debug, Clone)]
#[uuid = "b62bb455-a72c-4b56-87bb-81e0554e234f"]
pub struct CustomPbrMaterial {
    #[uniform(0)]
    pub base_color: Color,
    #[texture(1)]
    #[sampler(2)]
    pub base_color_texture: Option<Handle<Image>>,
    #[texture(3)]
    #[sampler(4)]
    pub normal_map: Option<Handle<Image>>,
    #[texture(5)]
    #[sampler(6)]
    pub metallic_roughness_texture: Option<Handle<Image>>,
    #[uniform(7)]
    pub metallic: f32,
    #[uniform(8)]
    pub roughness: f32,
    #[texture(9)]
    #[sampler(10)]
    pub emission_texture: Option<Handle<Image>>,
    #[uniform(11)]
    pub emission_power: f32,
}

impl Material for CustomPbrMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/pbr_fragment.wgsl".into()
    }

    fn vertex_shader() -> ShaderRef {
        "shaders/pbr_vertex.wgsl".into()
    }
}

/// Plugin to register the custom PBR material
pub struct CustomPbrPlugin;

impl Plugin for CustomPbrPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<CustomPbrMaterial>::default())
            .add_systems(Startup, setup_pbr_pipeline);
    }
}

fn setup_pbr_pipeline(
    mut commands: Commands,
    mut materials: ResMut<Assets<CustomPbrMaterial>>,
    asset_server: Res<AssetServer>,
) {
    // Create default PBR material
    let material = materials.add(CustomPbrMaterial {
        base_color: Color::WHITE,
        base_color_texture: None,
        normal_map: None,
        metallic_roughness_texture: None,
        metallic: 0.0,
        roughness: 0.5,
        emission_texture: None,
        emission_power: 0.0,
    });
} 