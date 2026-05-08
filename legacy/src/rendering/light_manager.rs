use bevy::{
    prelude::*,
    render::{
        render_resource::*,
        renderer::RenderDevice,
        camera::CameraProjection,
    },
};
use std::sync::Arc;

// Light types supported by our system
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LightType {
    Directional,
    Point,
    Spot,
}

// Common light parameters shared by all light types
#[derive(Component, Debug, Clone, ShaderType)]
pub struct LightParams {
    pub position: Vec3,
    pub direction: Vec3,
    pub color: Vec3,
    pub intensity: f32,
    pub range: f32,
    pub shadow_bias: f32,
    pub shadow_normal_bias: f32,
    pub spot_angle_cos: f32,  // For spot lights: cos(angle/2)
    pub light_type: u32,      // 0: Directional, 1: Point, 2: Spot
    pub cast_shadows: u32,    // 0: No shadows, 1: Cast shadows
}

// Component for entities with lights
#[derive(Component)]
pub struct Light {
    pub params: LightParams,
    pub shadow_map: Option<Handle<Image>>,
    pub shadow_view: Mat4,
    pub shadow_proj: Mat4,
    bind_group: Option<Arc<BindGroup>>,
}

// Shadow cascade data for directional lights
#[derive(Debug, Clone, ShaderType)]
pub struct CascadeData {
    pub split_depths: Vec4,           // Cascade split depths
    pub view_proj: [Mat4; 4],        // View-projection matrices for each cascade
    pub shadow_map_space: [Mat4; 4],  // Matrices to transform from world to shadow map space
}

// GPU buffer for light data
#[derive(ShaderType)]
struct LightBuffer {
    lights: [LightParams; MAX_LIGHTS],
    num_lights: u32,
    cascade_data: CascadeData,
    _padding: [u32; 3],  // Keep aligned to 16 bytes
}

pub struct LightManager {
    device: RenderDevice,
    queue: RenderQueue,
    bind_group_layout: BindGroupLayout,
    light_buffer: Buffer,
    lights: Vec<Light>,
    shadow_maps: Assets<Image>,
    cascade_data: CascadeData,
}

impl LightManager {
    pub fn new(device: RenderDevice, queue: RenderQueue, shadow_maps: Assets<Image>) -> Self {
        // Create bind group layout for light data
        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("light_bind_group_layout"),
            entries: &[
                // Light buffer
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: BufferSize::new(std::mem::size_of::<LightBuffer>() as u64),
                    },
                    count: None,
                },
                // Shadow maps array
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Depth,
                        view_dimension: TextureViewDimension::D2Array,
                        multisampled: false,
                    },
                    count: None,
                },
                // Shadow sampler
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Comparison),
                    count: None,
                },
            ],
        });

        // Create light buffer
        let light_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("light_buffer"),
            size: std::mem::size_of::<LightBuffer>() as u64,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            device,
            queue,
            bind_group_layout,
            light_buffer,
            lights: Vec::new(),
            shadow_maps,
            cascade_data: CascadeData {
                split_depths: Vec4::new(0.1, 0.3, 0.6, 1.0),
                view_proj: [Mat4::IDENTITY; 4],
                shadow_map_space: [Mat4::IDENTITY; 4],
            },
        }
    }

    pub fn create_light(&mut self, light_type: LightType) -> Light {
        let params = match light_type {
            LightType::Directional => LightParams {
                position: Vec3::ZERO,
                direction: -Vec3::Y,
                color: Vec3::ONE,
                intensity: 1.0,
                range: f32::INFINITY,
                shadow_bias: 0.005,
                shadow_normal_bias: 0.0,
                spot_angle_cos: -1.0,
                light_type: 0,
                cast_shadows: 1,
            },
            LightType::Point => LightParams {
                position: Vec3::ZERO,
                direction: -Vec3::Y,
                color: Vec3::ONE,
                intensity: 1.0,
                range: 10.0,
                shadow_bias: 0.005,
                shadow_normal_bias: 0.0,
                spot_angle_cos: -1.0,
                light_type: 1,
                cast_shadows: 1,
            },
            LightType::Spot => LightParams {
                position: Vec3::ZERO,
                direction: -Vec3::Y,
                color: Vec3::ONE,
                intensity: 1.0,
                range: 10.0,
                shadow_bias: 0.005,
                shadow_normal_bias: 0.0,
                spot_angle_cos: (45.0_f32.to_radians() / 2.0).cos(),
                light_type: 2,
                cast_shadows: 1,
            },
        };

        let light = Light {
            params,
            shadow_map: None,
            shadow_view: Mat4::IDENTITY,
            shadow_proj: Mat4::IDENTITY,
            bind_group: None,
        };

        self.lights.push(light.clone());
        light
    }

    pub fn update_lights(&mut self, camera: &Camera) {
        // Update shadow matrices for directional lights with cascaded shadow maps
        self.update_cascade_data(camera);

        // Create light buffer data
        let mut light_data = LightBuffer {
            lights: [LightParams::default(); MAX_LIGHTS],
            num_lights: self.lights.len() as u32,
            cascade_data: self.cascade_data.clone(),
            _padding: [0; 3],
        };

        // Copy light data to buffer
        for (i, light) in self.lights.iter().enumerate() {
            if i >= MAX_LIGHTS {
                break;
            }
            light_data.lights[i] = light.params.clone();
        }

        // Update GPU buffer
        self.queue.write_buffer(
            &self.light_buffer,
            0,
            bytemuck::cast_slice(&[light_data]),
        );
    }

    fn update_cascade_data(&mut self, camera: &Camera) {
        let camera_pos = camera.transform.translation;
        let camera_dir = -camera.transform.forward();
        let near = camera.near;
        let far = camera.far;

        // Calculate cascade split depths using PSSM (Parallel-Split Shadow Maps)
        let lambda = 0.5; // Balance between uniform and logarithmic splitting
        for i in 0..4 {
            let p = (i + 1) as f32 / 4.0;
            let log_split = near * (far / near).powf(p);
            let uniform_split = near + (far - near) * p;
            let d = lambda * log_split + (1.0 - lambda) * uniform_split;
            self.cascade_data.split_depths[i] = d;
        }

        // Calculate view-projection matrices for each cascade
        for i in 0..4 {
            let cascade_near = if i == 0 { near } else { self.cascade_data.split_depths[i - 1] };
            let cascade_far = self.cascade_data.split_depths[i];

            // Create orthographic projection for this cascade
            let size = cascade_far * 2.0; // Size of the shadow map frustum
            let proj = Mat4::orthographic_rh(-size, size, -size, size, 0.0, cascade_far - cascade_near);

            // Create view matrix looking from light direction
            let light_pos = camera_pos - camera_dir * ((cascade_near + cascade_far) * 0.5);
            let view = Mat4::look_at_rh(light_pos, light_pos + camera_dir, Vec3::Y);

            self.cascade_data.view_proj[i] = proj * view;
            self.cascade_data.shadow_map_space[i] = Mat4::from_scale(Vec3::new(0.5, -0.5, 1.0))
                * Mat4::from_translation(Vec3::new(0.5, 0.5, 0.0))
                * self.cascade_data.view_proj[i];
        }
    }

    pub fn get_bind_group_layout(&self) -> &BindGroupLayout {
        &self.bind_group_layout
    }
}

impl Default for LightParams {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            direction: -Vec3::Y,
            color: Vec3::ONE,
            intensity: 1.0,
            range: 10.0,
            shadow_bias: 0.005,
            shadow_normal_bias: 0.0,
            spot_angle_cos: -1.0,
            light_type: 0,
            cast_shadows: 0,
        }
    }
}

// Constants
const MAX_LIGHTS: usize = 16; // Maximum number of lights supported 