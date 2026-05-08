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
use crate::rendering::light_manager::{LightManager, Light, LightType};

pub struct LightPlugin;

impl Plugin for LightPlugin {
    fn build(&self, app: &mut App) {
        // Register light component
        app.register_type::<Light>()
            // Add systems to the main app
            .add_systems(Update, update_lights);

        // Setup render-world systems
        let render_app = app.sub_app_mut(RenderApp);
        render_app
            .init_resource::<LightManager>()
            .add_systems(ExtractSchedule, extract_lights)
            .add_systems(Render, prepare_lights.in_set(RenderSet::Prepare));
    }
}

// System to update lights in the main world
fn update_lights(
    mut lights: Query<(&mut Light, &GlobalTransform)>,
    time: Res<Time>,
) {
    for (mut light, transform) in lights.iter_mut() {
        // Update light position and direction from transform
        light.params.position = transform.translation();
        light.params.direction = -transform.forward();

        // Update shadow matrices
        if light.params.cast_shadows == 1 {
            let view = Mat4::look_at_rh(
                transform.translation(),
                transform.translation() + transform.forward(),
                transform.up(),
            );

            let proj = match light.params.light_type {
                0 => { // Directional light
                    Mat4::orthographic_rh(-50.0, 50.0, -50.0, 50.0, -50.0, 50.0)
                },
                1 => { // Point light
                    Mat4::perspective_infinite_rh(
                        90.0_f32.to_radians(),
                        1.0,
                        0.1,
                    )
                },
                2 => { // Spot light
                    let fov = light.params.spot_angle_cos.acos() * 2.0;
                    Mat4::perspective_rh(
                        fov,
                        1.0,
                        0.1,
                        light.params.range,
                    )
                },
                _ => Mat4::IDENTITY,
            };

            light.shadow_view = view;
            light.shadow_proj = proj;
        }
    }
}

// System to extract lights to the render world
fn extract_lights(
    mut commands: Commands,
    lights: Query<(Entity, &Light)>,
) {
    for (entity, light) in lights.iter() {
        commands.get_or_spawn(entity)
            .insert(light.clone());
    }
}

// System to prepare lights for rendering
fn prepare_lights(
    mut light_manager: ResMut<LightManager>,
    lights: Query<&Light>,
    camera: Query<&Camera>,
) {
    if let Ok(camera) = camera.get_single() {
        light_manager.update_lights(camera);
    }
}

// Initialize the LightManager as a resource
impl FromWorld for LightManager {
    fn from_world(world: &mut World) -> Self {
        let device = world.resource::<RenderDevice>();
        let queue = world.resource::<RenderQueue>();
        let shadow_maps = world.resource::<Assets<Image>>().clone();

        LightManager::new(
            device.clone(),
            queue.clone(),
            shadow_maps,
        )
    }
}

// Helper functions to create different types of lights
pub fn create_directional_light(
    commands: &mut Commands,
    color: Vec3,
    intensity: f32,
    transform: Transform,
) -> Entity {
    let mut light_manager = LightManager::new(
        commands.world.resource::<RenderDevice>().clone(),
        commands.world.resource::<RenderQueue>().clone(),
        commands.world.resource::<Assets<Image>>().clone(),
    );

    let mut light = light_manager.create_light(LightType::Directional);
    light.params.color = color;
    light.params.intensity = intensity;

    commands.spawn((
        light,
        transform,
        GlobalTransform::default(),
    )).id()
}

pub fn create_point_light(
    commands: &mut Commands,
    color: Vec3,
    intensity: f32,
    range: f32,
    transform: Transform,
) -> Entity {
    let mut light_manager = LightManager::new(
        commands.world.resource::<RenderDevice>().clone(),
        commands.world.resource::<RenderQueue>().clone(),
        commands.world.resource::<Assets<Image>>().clone(),
    );

    let mut light = light_manager.create_light(LightType::Point);
    light.params.color = color;
    light.params.intensity = intensity;
    light.params.range = range;

    commands.spawn((
        light,
        transform,
        GlobalTransform::default(),
    )).id()
}

pub fn create_spot_light(
    commands: &mut Commands,
    color: Vec3,
    intensity: f32,
    range: f32,
    angle: f32,
    transform: Transform,
) -> Entity {
    let mut light_manager = LightManager::new(
        commands.world.resource::<RenderDevice>().clone(),
        commands.world.resource::<RenderQueue>().clone(),
        commands.world.resource::<Assets<Image>>().clone(),
    );

    let mut light = light_manager.create_light(LightType::Spot);
    light.params.color = color;
    light.params.intensity = intensity;
    light.params.range = range;
    light.params.spot_angle_cos = (angle.to_radians() / 2.0).cos();

    commands.spawn((
        light,
        transform,
        GlobalTransform::default(),
    )).id()
} 