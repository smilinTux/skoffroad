use bevy::{
    prelude::*,
    render::{
        render_resource::{Extent3d, TextureDescriptor, TextureFormat, TextureUsages},
        texture::BevyDefault,
    },
    asset::AssetPath,
};
use noise::{NoiseFn, Perlin};

/// Plugin for generating particle effect textures
pub struct ParticleTextureGenPlugin;

impl Plugin for ParticleTextureGenPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, generate_particle_textures);
    }
}

/// Generates all particle textures and adds them to assets
fn generate_particle_textures(
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Generate each atlas
    let fire_handle = generate_fire_atlas(&mut images);
    let smoke_handle = generate_smoke_atlas(&mut images);
    let sparkle_handle = generate_sparkle_atlas(&mut images);
    let magic_handle = generate_magic_atlas(&mut images);
    let portal_handle = generate_portal_atlas(&mut images);

    // Store handles in the asset server
    images.set_untracked(AssetPath::new_ref("textures/fire_atlas.png", None), fire_handle);
    images.set_untracked(AssetPath::new_ref("textures/smoke_atlas.png", None), smoke_handle);
    images.set_untracked(AssetPath::new_ref("textures/sparkle_atlas.png", None), sparkle_handle);
    images.set_untracked(AssetPath::new_ref("textures/magic_atlas.png", None), magic_handle);
    images.set_untracked(AssetPath::new_ref("textures/portal_atlas.png", None), portal_handle);
}

/// Creates a new 256x256 RGBA8 texture
fn create_texture() -> Image {
    let size = Extent3d {
        width: 256,
        height: 256,
        depth_or_array_layers: 1,
    };

    let mut image = Image {
        texture_descriptor: TextureDescriptor {
            label: None,
            size,
            dimension: bevy::render::render_resource::TextureDimension::D2,
            format: TextureFormat::bevy_default(),
            mip_level_count: 1,
            sample_count: 1,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            view_formats: &[],
        },
        ..default()
    };

    image.resize(size);
    image
}

/// Generates a radial gradient
fn generate_radial_gradient(x: f32, y: f32, center_x: f32, center_y: f32, radius: f32) -> f32 {
    let dist = ((x - center_x).powi(2) + (y - center_y).powi(2)).sqrt();
    (1.0 - (dist / radius).min(1.0)).max(0.0)
}

/// Generates fire atlas texture
fn generate_fire_atlas(images: &mut Assets<Image>) -> Handle<Image> {
    let mut image = create_texture();
    let cell_size = 128;

    for y in 0..256 {
        for x in 0..256 {
            let cell_x = x / cell_size;
            let cell_y = y / cell_size;
            let local_x = (x % cell_size) as f32;
            let local_y = (y % cell_size) as f32;
            
            let pixel_idx = (y * 256 + x) * 4;
            
            match (cell_x, cell_y) {
                // Bright core
                (0, 0) => {
                    let intensity = generate_radial_gradient(local_x, local_y, 64.0, 64.0, 50.0);
                    image.data[pixel_idx] = (255.0 * intensity) as u8;     // R
                    image.data[pixel_idx + 1] = (230.0 * intensity) as u8; // G
                    image.data[pixel_idx + 2] = (180.0 * intensity) as u8; // B
                    image.data[pixel_idx + 3] = (255.0 * intensity) as u8; // A
                },
                // Medium flame
                (1, 0) => {
                    let intensity = generate_radial_gradient(local_x, local_y, 64.0, 64.0, 60.0);
                    image.data[pixel_idx] = (255.0 * intensity) as u8;     // R
                    image.data[pixel_idx + 1] = (140.0 * intensity) as u8; // G
                    image.data[pixel_idx + 2] = (50.0 * intensity) as u8;  // B
                    image.data[pixel_idx + 3] = (200.0 * intensity) as u8; // A
                },
                // Soft flame
                (0, 1) => {
                    let intensity = generate_radial_gradient(local_x, local_y, 64.0, 64.0, 70.0);
                    image.data[pixel_idx] = (255.0 * intensity) as u8;     // R
                    image.data[pixel_idx + 1] = (180.0 * intensity) as u8; // G
                    image.data[pixel_idx + 2] = (100.0 * intensity) as u8; // B
                    image.data[pixel_idx + 3] = (150.0 * intensity) as u8; // A
                },
                // Smoke
                (1, 1) => {
                    let intensity = generate_radial_gradient(local_x, local_y, 64.0, 64.0, 80.0);
                    let value = (180.0 * intensity) as u8;
                    image.data[pixel_idx] = value;     // R
                    image.data[pixel_idx + 1] = value; // G
                    image.data[pixel_idx + 2] = value; // B
                    image.data[pixel_idx + 3] = (100.0 * intensity) as u8; // A
                },
                _ => unreachable!(),
            }
        }
    }
    
    images.add(image)
}

/// Generates smoke atlas texture using Perlin noise
fn generate_smoke_atlas(images: &mut Assets<Image>) -> Handle<Image> {
    let mut image = create_texture();
    let cell_size = 128;
    let perlin = Perlin::new(1);
    let scale = 0.05;

    for y in 0..256 {
        for x in 0..256 {
            let cell_x = x / cell_size;
            let cell_y = y / cell_size;
            let local_x = (x % cell_size) as f32;
            let local_y = (y % cell_size) as f32;
            
            let pixel_idx = (y * 256 + x) * 4;
            
            let base_noise = (perlin.get([x as f64 * scale, y as f64 * scale]) + 1.0) / 2.0;
            let radial = generate_radial_gradient(local_x, local_y, 64.0, 64.0, 60.0);
            let intensity = (base_noise as f32 * radial).min(1.0);
            
            let (color, alpha) = match (cell_x, cell_y) {
                (0, 0) => (180, 200), // Dense smoke
                (1, 0) => (160, 150), // Medium smoke
                (0, 1) => (140, 100), // Light smoke
                (1, 1) => (120, 50),  // Wispy smoke
                _ => unreachable!(),
            };
            
            let value = (color as f32 * intensity) as u8;
            image.data[pixel_idx] = value;     // R
            image.data[pixel_idx + 1] = value; // G
            image.data[pixel_idx + 2] = value; // B
            image.data[pixel_idx + 3] = (alpha as f32 * intensity) as u8; // A
        }
    }
    
    images.add(image)
}

/// Generates sparkle atlas texture
fn generate_sparkle_atlas(images: &mut Assets<Image>) -> Handle<Image> {
    let mut image = create_texture();
    let cell_size = 128;

    for y in 0..256 {
        for x in 0..256 {
            let cell_x = x / cell_size;
            let cell_y = y / cell_size;
            let local_x = (x % cell_size) as f32 - 64.0;
            let local_y = (y % cell_size) as f32 - 64.0;
            
            let pixel_idx = (y * 256 + x) * 4;
            
            let intensity = match (cell_x, cell_y) {
                // 4-point star
                (0, 0) => {
                    let star = (1.0 - (local_x.abs() * 0.1).min(1.0)) * 
                             (1.0 - (local_y.abs() * 0.1).min(1.0));
                    let glow = generate_radial_gradient(local_x + 64.0, local_y + 64.0, 64.0, 64.0, 40.0);
                    (star + glow * 0.5).min(1.0)
                },
                // Small twinkle
                (1, 0) => {
                    let dist = (local_x.powi(2) + local_y.powi(2)).sqrt();
                    let ring = (1.0 - (dist - 20.0).abs() * 0.1).max(0.0);
                    ring * generate_radial_gradient(local_x + 64.0, local_y + 64.0, 64.0, 64.0, 30.0)
                },
                // Diamond
                (0, 1) => {
                    let diamond = (1.0 - (local_x.abs() + local_y.abs()) * 0.04).max(0.0);
                    diamond * generate_radial_gradient(local_x + 64.0, local_y + 64.0, 64.0, 64.0, 40.0)
                },
                // Simple dot
                (1, 1) => generate_radial_gradient(local_x + 64.0, local_y + 64.0, 64.0, 64.0, 20.0),
                _ => unreachable!(),
            };
            
            image.data[pixel_idx] = (255.0 * intensity) as u8;     // R
            image.data[pixel_idx + 1] = (255.0 * intensity) as u8; // G
            image.data[pixel_idx + 2] = (200.0 * intensity) as u8; // B
            image.data[pixel_idx + 3] = (255.0 * intensity) as u8; // A
        }
    }
    
    images.add(image)
}

/// Generates magic atlas texture
fn generate_magic_atlas(images: &mut Assets<Image>) -> Handle<Image> {
    let mut image = create_texture();
    let cell_size = 128;

    for y in 0..256 {
        for x in 0..256 {
            let cell_x = x / cell_size;
            let cell_y = y / cell_size;
            let local_x = (x % cell_size) as f32 - 64.0;
            let local_y = (y % cell_size) as f32 - 64.0;
            let angle = local_y.atan2(local_x);
            let dist = (local_x.powi(2) + local_y.powi(2)).sqrt();
            
            let pixel_idx = (y * 256 + x) * 4;
            
            let intensity = match (cell_x, cell_y) {
                // Rune symbol
                (0, 0) => {
                    let rune_lines = ((angle * 4.0).sin().abs() * 0.5 + 0.5) * 
                                   (1.0 - (dist - 30.0).abs() * 0.1).max(0.0);
                    let glow = generate_radial_gradient(local_x + 64.0, local_y + 64.0, 64.0, 64.0, 50.0);
                    (rune_lines + glow * 0.3).min(1.0)
                },
                // Energy swirl
                (1, 0) => {
                    let spiral = ((angle * 3.0 + dist * 0.1).sin() * 0.5 + 0.5) * 
                               generate_radial_gradient(local_x + 64.0, local_y + 64.0, 64.0, 64.0, 60.0);
                    spiral
                },
                // Energy burst
                (0, 1) => {
                    let rays = ((angle * 8.0).cos().abs() * 0.8 + 0.2) * 
                             generate_radial_gradient(local_x + 64.0, local_y + 64.0, 64.0, 64.0, 70.0);
                    rays
                },
                // Glow orb
                (1, 1) => generate_radial_gradient(local_x + 64.0, local_y + 64.0, 64.0, 64.0, 40.0),
                _ => unreachable!(),
            };
            
            // Purple-blue gradient
            image.data[pixel_idx] = (180.0 * intensity) as u8;     // R
            image.data[pixel_idx + 1] = (100.0 * intensity) as u8; // G
            image.data[pixel_idx + 2] = (255.0 * intensity) as u8; // B
            image.data[pixel_idx + 3] = (255.0 * intensity) as u8; // A
        }
    }
    
    images.add(image)
}

/// Generates portal atlas texture
fn generate_portal_atlas(images: &mut Assets<Image>) -> Handle<Image> {
    let mut image = create_texture();
    let cell_size = 128;
    let perlin = Perlin::new(2);
    let scale = 0.03;

    for y in 0..256 {
        for x in 0..256 {
            let cell_x = x / cell_size;
            let cell_y = y / cell_size;
            let local_x = (x % cell_size) as f32 - 64.0;
            let local_y = (y % cell_size) as f32 - 64.0;
            let angle = local_y.atan2(local_x);
            let dist = (local_x.powi(2) + local_y.powi(2)).sqrt();
            
            let pixel_idx = (y * 256 + x) * 4;
            
            let noise = (perlin.get([x as f64 * scale, y as f64 * scale]) + 1.0) / 2.0;
            
            let intensity = match (cell_x, cell_y) {
                // Portal ring
                (0, 0) => {
                    let ring = (1.0 - (dist - 50.0).abs() * 0.1).max(0.0);
                    let pattern = ((angle * 12.0).cos() * 0.5 + 0.5) * ring;
                    (pattern + ring * 0.5).min(1.0)
                },
                // Energy ripple
                (1, 0) => {
                    let ripples = ((dist * 0.2).cos() * 0.5 + 0.5) * 
                                generate_radial_gradient(local_x + 64.0, local_y + 64.0, 64.0, 64.0, 60.0);
                    ripples * (noise as f32)
                },
                // Void swirl
                (0, 1) => {
                    let swirl = ((angle * 4.0 + dist * 0.1).sin() * 0.5 + 0.5) * 
                              (1.0 - generate_radial_gradient(local_x + 64.0, local_y + 64.0, 64.0, 64.0, 30.0));
                    swirl * (noise as f32)
                },
                // Particle burst
                (1, 1) => {
                    let burst = ((angle * 16.0).cos().abs() * 0.5 + 0.5) * 
                              generate_radial_gradient(local_x + 64.0, local_y + 64.0, 64.0, 64.0, 70.0);
                    burst * (noise as f32)
                },
                _ => unreachable!(),
            };
            
            // Blue-purple portal colors
            image.data[pixel_idx] = (100.0 * intensity) as u8;     // R
            image.data[pixel_idx + 1] = (150.0 * intensity) as u8; // G
            image.data[pixel_idx + 2] = (255.0 * intensity) as u8; // B
            image.data[pixel_idx + 3] = (255.0 * intensity) as u8; // A
        }
    }
    
    images.add(image)
} 