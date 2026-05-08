use bevy::{
    prelude::*,
    render::{
        render_resource::*,
        renderer::RenderDevice,
        RenderApp,
        RenderSet,
        Extract,
        Render,
    },
    asset::Assets,
};
use crate::rendering::material_manager::{MaterialManager, PbrMaterial, MaterialParams};

pub struct PbrMaterialPlugin;

impl Plugin for PbrMaterialPlugin {
    fn build(&self, app: &mut App) {
        // Register the material type as a component
        app.register_type::<PbrMaterial>()
            // Add systems to the main app
            .add_systems(Update, update_materials);

        // Setup render-world systems
        let render_app = app.sub_app_mut(RenderApp);
        render_app
            .init_resource::<MaterialManager>()
            .add_systems(ExtractSchedule, extract_materials)
            .add_systems(Render, prepare_materials.in_set(RenderSet::Prepare));
    }
}

// System to update materials in the main world
fn update_materials(
    mut materials: Query<&mut PbrMaterial>,
    time: Res<Time>,
) {
    for mut material in materials.iter_mut() {
        // Update any time-based material parameters
        if material.params.flags & MATERIAL_FLAG_ANIMATED != 0 {
            // Example: Update emission strength based on time
            material.params.emission_strength = 
                (time.elapsed_seconds() * 2.0).sin() * 0.5 + 0.5;
        }
    }
}

// System to extract materials to the render world
fn extract_materials(
    mut commands: Commands,
    materials: Query<(Entity, &PbrMaterial)>,
) {
    for (entity, material) in materials.iter() {
        commands.get_or_spawn(entity)
            .insert(material.clone());
    }
}

// System to prepare materials for rendering
fn prepare_materials(
    mut material_manager: ResMut<MaterialManager>,
    materials: Query<&mut PbrMaterial>,
) {
    for mut material in materials.iter_mut() {
        material_manager.update_material(&mut material);
    }
}

// Initialize the MaterialManager as a resource
impl FromWorld for MaterialManager {
    fn from_world(world: &mut World) -> Self {
        let device = world.resource::<RenderDevice>();
        let queue = world.resource::<RenderQueue>();
        let asset_server = world.resource::<AssetServer>();
        let mut images = world.resource_mut::<Assets<Image>>();

        MaterialManager::new(
            device.clone(),
            queue.clone(),
            asset_server.clone(),
            &mut images,
        )
    }
}

// Material flags
pub const MATERIAL_FLAG_ANIMATED: u32 = 1 << 0;
pub const MATERIAL_FLAG_SUBSURFACE: u32 = 1 << 1;
pub const MATERIAL_FLAG_DISPERSION: u32 = 1 << 2; 