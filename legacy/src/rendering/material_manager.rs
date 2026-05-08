use bevy::{
    prelude::*,
    render::{
        render_resource::{
            BindGroup, BindGroupLayout, BindGroupLayoutEntry, BindGroupEntry,
            Buffer, BufferDescriptor, BufferUsages, BufferBindingType, BufferSize,
            BindGroupDescriptor, BindGroupLayoutDescriptor, BindingType, BindingResource,
            ShaderStages, TextureSampleType, TextureViewDimension, ShaderType,
            TextureView, Sampler, TextureDescriptor, TextureUsages, TextureFormat,
            TextureViewDescriptor, Extent3d,
        },
        renderer::RenderDevice,
        texture::BevyDefault,
    },
    asset::{AssetServer, Handle, Assets},
};
use std::sync::Arc;

#[derive(Debug, Clone, ShaderType)]
pub struct MaterialParams {
    pub albedo: Vec4,
    pub metallic: f32,
    pub roughness: f32,
    pub ior: f32,
    pub emission: Vec3,
    pub emission_strength: f32,
    pub alpha: f32,
    pub subsurface_radius: Vec3,
    pub subsurface_strength: f32,
    pub dispersion_strength: f32,
    pub dispersion_bias: f32,
    pub flags: u32,
}

#[derive(Component, Clone, PartialEq)]
pub struct PbrMaterial {
    pub params: MaterialParams,
    pub albedo_texture: Option<Handle<Image>>,
    pub normal_texture: Option<Handle<Image>>,
    pub metallic_roughness_texture: Option<Handle<Image>>,
    pub emission_texture: Option<Handle<Image>>,
    bind_group: Option<Arc<BindGroup>>,
}

pub struct MaterialManager {
    device: RenderDevice,
    queue: RenderQueue,
    bind_group_layout: BindGroupLayout,
    material_buffer: Buffer,
    materials: Vec<PbrMaterial>,
    asset_server: AssetServer,
    images: Assets<Image>,
    default_white_texture: Handle<Image>,
}

impl MaterialManager {
    pub fn new(
        device: RenderDevice,
        queue: RenderQueue,
        asset_server: AssetServer,
        images: &mut Assets<Image>
    ) -> Self {
        // Create bind group layout for material data
        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("material_bind_group_layout"),
            entries: &[
                // Material buffer
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: BufferSize::new(std::mem::size_of::<MaterialParams>() as u64),
                    },
                    count: None,
                },
                // Albedo texture
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                // Normal texture
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                // Metallic-Roughness texture
                BindGroupLayoutEntry {
                    binding: 3,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                // Emission texture
                BindGroupLayoutEntry {
                    binding: 4,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                // Sampler
                BindGroupLayoutEntry {
                    binding: 5,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        // Create material buffer
        let material_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("material_buffer"),
            size: std::mem::size_of::<MaterialParams>() as u64 * MAX_MATERIALS,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Create default white texture
        let default_white_texture = create_default_white_texture(&device, &queue, images);

        Self {
            device,
            queue,
            bind_group_layout,
            material_buffer,
            materials: Vec::new(),
            asset_server,
            images: images.clone(),
            default_white_texture,
        }
    }

    pub fn create_material(&mut self, params: MaterialParams) -> PbrMaterial {
        let material = PbrMaterial {
            params,
            albedo_texture: None,
            normal_texture: None,
            metallic_roughness_texture: None,
            emission_texture: None,
            bind_group: None,
        };

        self.materials.push(material.clone());
        material
    }

    pub fn update_material(&mut self, material: &mut PbrMaterial) {
        // Update material buffer
        let buffer_offset = self.materials.iter().position(|m| m == material).unwrap() as u64
            * std::mem::size_of::<MaterialParams>() as u64;

        self.queue.write_buffer(
            &self.material_buffer,
            buffer_offset,
            bytemuck::cast_slice(&[material.params]),
        );

        // Create or update bind group if textures changed
        if material.bind_group.is_none() {
            let bind_group = self.device.create_bind_group(&BindGroupDescriptor {
                label: Some("material_bind_group"),
                layout: &self.bind_group_layout,
                entries: &[
                    // Material buffer
                    BindGroupEntry {
                        binding: 0,
                        resource: self.material_buffer.as_entire_binding(),
                    },
                    // Albedo texture
                    BindGroupEntry {
                        binding: 1,
                        resource: BindingResource::TextureView(&self.get_texture_view(&material.albedo_texture)),
                    },
                    // Normal texture
                    BindGroupEntry {
                        binding: 2,
                        resource: BindingResource::TextureView(&self.get_texture_view(&material.normal_texture)),
                    },
                    // Metallic-Roughness texture
                    BindGroupEntry {
                        binding: 3,
                        resource: BindingResource::TextureView(&self.get_texture_view(&material.metallic_roughness_texture)),
                    },
                    // Emission texture
                    BindGroupEntry {
                        binding: 4,
                        resource: BindingResource::TextureView(&self.get_texture_view(&material.emission_texture)),
                    },
                    // Sampler
                    BindGroupEntry {
                        binding: 5,
                        resource: BindingResource::Sampler(&self.create_default_sampler()),
                    },
                ],
            });
            material.bind_group = Some(Arc::new(bind_group));
        }
    }

    fn get_texture_view(&self, texture_handle: &Option<Handle<Image>>) -> TextureView {
        if let Some(handle) = texture_handle {
            if let Some(image) = self.images.get(handle) {
                image.texture_view.clone()
            } else {
                self.images.get(&self.default_white_texture).unwrap().texture_view.clone()
            }
        } else {
            self.images.get(&self.default_white_texture).unwrap().texture_view.clone()
        }
    }

    fn create_default_sampler(&self) -> Sampler {
        self.device.create_sampler(&SamplerDescriptor {
            address_mode_u: AddressMode::Repeat,
            address_mode_v: AddressMode::Repeat,
            address_mode_w: AddressMode::Repeat,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Linear,
            ..default()
        })
    }

    pub fn get_bind_group_layout(&self) -> &BindGroupLayout {
        &self.bind_group_layout
    }

    pub fn get_material_bind_group(&self, material: &PbrMaterial) -> Option<Arc<BindGroup>> {
        material.bind_group.clone()
    }
}

fn create_default_white_texture(
    device: &RenderDevice,
    queue: &RenderQueue,
    images: &mut Assets<Image>,
) -> Handle<Image> {
    let size = Extent3d {
        width: 1,
        height: 1,
        depth_or_array_layers: 1,
    };

    let texture = device.create_texture(&TextureDescriptor {
        label: Some("default_white_texture"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: TextureFormat::bevy_default(),
        usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
    });

    // White pixel data
    let data = vec![255u8; 4];
    queue.write_texture(
        ImageCopyTexture {
            texture: &texture,
            mip_level: 0,
            origin: Origin3d::ZERO,
            aspect: TextureAspect::All,
        },
        &data,
        ImageDataLayout {
            offset: 0,
            bytes_per_row: 4,
            rows_per_image: 1,
        },
        size,
    );

    let image = Image {
        texture,
        texture_descriptor: TextureDescriptor {
            label: Some("default_white_texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::bevy_default(),
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
        },
        sampler_descriptor: SamplerDescriptor::default(),
        texture_view: texture.create_view(&TextureViewDescriptor::default()),
    };

    images.add(image)
}

// Constants
const MAX_MATERIALS: u64 = 1024; // Maximum number of materials in the buffer 