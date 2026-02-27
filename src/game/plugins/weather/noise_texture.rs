use bevy::{
    prelude::*,
    render::{
        render_resource::{
            Extent3d,
            TextureUsages,
            TextureDimension,
            TextureFormat,
        },
        texture::Image,
    },
};
use noise::{NoiseFn, Perlin, Fbm, MultiFractal};

pub struct NoiseGenerator;

impl NoiseGenerator {
    /// Generates a 3D noise texture for cloud rendering
    /// 
    /// # Arguments
    /// * `size` - The size of the 3D texture (width, height, depth)
    /// * `scale` - The scale of the noise
    /// * `octaves` - Number of octaves for FBM noise
    /// * `persistence` - Persistence for FBM noise
    /// * `lacunarity` - Lacunarity for FBM noise
    pub fn generate_3d_noise(
        size: UVec3,
        scale: f64,
        octaves: usize,
        persistence: f64,
        lacunarity: f64,
    ) -> Image {
        let fbm = Fbm::<Perlin>::new(0)
            .set_octaves(octaves)
            .set_persistence(persistence)
            .set_lacunarity(lacunarity);

        let mut data = Vec::with_capacity((size.x * size.y * size.z * 4) as usize);

        for z in 0..size.z {
            for y in 0..size.y {
                for x in 0..size.x {
                    let point = [
                        x as f64 * scale / size.x as f64,
                        y as f64 * scale / size.y as f64,
                        z as f64 * scale / size.z as f64,
                    ];

                    let noise_val = (fbm.get(point) + 1.0) / 2.0;
                    let value = (noise_val * 255.0) as u8;

                    // RGBA format
                    data.push(value); // R
                    data.push(value); // G
                    data.push(value); // B
                    data.push(255);   // A
                }
            }
        }

        Image::new(
            Extent3d {
                width: size.x,
                height: size.y,
                depth_or_array_layers: size.z,
            },
            TextureDimension::D3,
            data,
            TextureFormat::Rgba8Unorm,
        )
    }

    /// Generates a set of noise textures for different cloud details
    pub fn generate_cloud_noise_set() -> CloudNoiseTextures {
        // Base shape noise (32x32x32)
        let mut base_shape = Self::generate_3d_noise(
            UVec3::new(32, 32, 32),
            4.0,
            4,
            0.5,
            2.0,
        );
        base_shape.texture_descriptor.usage = TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST;

        // Detail noise (64x64x64)
        let mut detail = Self::generate_3d_noise(
            UVec3::new(64, 64, 64),
            8.0,
            6,
            0.5,
            2.0,
        );
        detail.texture_descriptor.usage = TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST;

        // Weather noise (16x16x16)
        let mut weather = Self::generate_3d_noise(
            UVec3::new(16, 16, 16),
            2.0,
            2,
            0.5,
            2.0,
        );
        weather.texture_descriptor.usage = TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST;

        CloudNoiseTextures {
            base_shape,
            detail,
            weather,
        }
    }
}

pub struct CloudNoiseTextures {
    pub base_shape: Image,
    pub detail: Image,
    pub weather: Image,
}

// Plugin to handle noise texture generation and updates
pub struct NoiseTexturePlugin;

impl Plugin for NoiseTexturePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_noise_textures);
    }
}

fn setup_noise_textures(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
) {
    let noise_textures = NoiseGenerator::generate_cloud_noise_set();
    
    let base_shape_handle = images.add(noise_textures.base_shape);
    let detail_handle = images.add(noise_textures.detail);
    let weather_handle = images.add(noise_textures.weather);

    commands.insert_resource(CloudNoiseTextureHandles {
        base_shape: base_shape_handle,
        detail: detail_handle,
        weather: weather_handle,
    });
}

#[derive(Resource)]
pub struct CloudNoiseTextureHandles {
    pub base_shape: Handle<Image>,
    pub detail: Handle<Image>,
    pub weather: Handle<Image>,
} 