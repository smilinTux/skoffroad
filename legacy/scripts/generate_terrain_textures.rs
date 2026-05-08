use image::{ImageBuffer, Rgba, RgbaImage};
use noise::{NoiseFn, Perlin, Seedable};
use std::fs;
use std::path::Path;

const TEXTURE_SIZE: u32 = 1024;
const OUTPUT_DIR: &str = "../assets/textures/terrain";

struct TextureGenerator {
    perlin: Perlin,
    detail_perlin: Perlin,
}

impl TextureGenerator {
    fn new(seed: u32) -> Self {
        let perlin = Perlin::new(seed);
        let detail_perlin = Perlin::new(seed + 1);
        
        TextureGenerator {
            perlin,
            detail_perlin,
        }
    }

    fn generate_albedo(&self, terrain_type: &str) -> RgbaImage {
        let mut image = ImageBuffer::new(TEXTURE_SIZE, TEXTURE_SIZE);

        for y in 0..TEXTURE_SIZE {
            for x in 0..TEXTURE_SIZE {
                let noise_val = self.perlin.get([
                    x as f64 * 0.05,
                    y as f64 * 0.05,
                    0.0,
                ]) * 0.5 + 0.5;

                let detail = self.detail_perlin.get([
                    x as f64 * 0.2,
                    y as f64 * 0.2,
                    0.0,
                ]) * 0.25;

                let color = match terrain_type {
                    "grass" => Rgba([
                        (34.0 + (noise_val + detail) * 40.0) as u8,
                        (139.0 + (noise_val + detail) * 40.0) as u8,
                        (34.0 + (noise_val + detail) * 30.0) as u8,
                        255,
                    ]),
                    "rock" => Rgba([
                        (128.0 + (noise_val + detail) * 30.0) as u8,
                        (128.0 + (noise_val + detail) * 30.0) as u8,
                        (128.0 + (noise_val + detail) * 30.0) as u8,
                        255,
                    ]),
                    "sand" => Rgba([
                        (194.0 + (noise_val + detail) * 20.0) as u8,
                        (178.0 + (noise_val + detail) * 20.0) as u8,
                        (128.0 + (noise_val + detail) * 20.0) as u8,
                        255,
                    ]),
                    "snow" => Rgba([
                        (235.0 + (noise_val + detail) * 20.0) as u8,
                        (235.0 + (noise_val + detail) * 20.0) as u8,
                        (235.0 + (noise_val + detail) * 20.0) as u8,
                        255,
                    ]),
                    "water" => Rgba([
                        (0.0 + (noise_val + detail) * 30.0) as u8,
                        (105.0 + (noise_val + detail) * 30.0) as u8,
                        (148.0 + (noise_val + detail) * 30.0) as u8,
                        200,
                    ]),
                    "forest" => Rgba([
                        (45.0 + (noise_val + detail) * 35.0) as u8,
                        (82.0 + (noise_val + detail) * 35.0) as u8,
                        (45.0 + (noise_val + detail) * 25.0) as u8,
                        255,
                    ]),
                    "plains" => Rgba([
                        (153.0 + (noise_val + detail) * 30.0) as u8,
                        (204.0 + (noise_val + detail) * 30.0) as u8,
                        (102.0 + (noise_val + detail) * 25.0) as u8,
                        255,
                    ]),
                    "tundra" => Rgba([
                        (204.0 + (noise_val + detail) * 25.0) as u8,
                        (204.0 + (noise_val + detail) * 25.0) as u8,
                        (204.0 + (noise_val + detail) * 25.0) as u8,
                        255,
                    ]),
                    _ => Rgba([128, 128, 128, 255]),
                };

                image.put_pixel(x, y, color);
            }
        }

        image
    }

    fn generate_normal(&self, terrain_type: &str) -> RgbaImage {
        let mut image = ImageBuffer::new(TEXTURE_SIZE, TEXTURE_SIZE);
        let scale = match terrain_type {
            "rock" => 2.0,
            "grass" => 1.0,
            "sand" => 0.5,
            "snow" => 0.3,
            "water" => 0.8,
            "forest" => 1.2,
            "plains" => 0.7,
            "tundra" => 0.4,
            _ => 1.0,
        };

        for y in 0..TEXTURE_SIZE {
            for x in 0..TEXTURE_SIZE {
                let x_noise = self.perlin.get([
                    x as f64 * 0.1,
                    y as f64 * 0.1,
                    1.0,
                ]) * scale;
                let y_noise = self.perlin.get([
                    x as f64 * 0.1,
                    y as f64 * 0.1,
                    2.0,
                ]) * scale;

                // Add detail noise
                let detail_scale = scale * 0.3;
                let x_detail = self.detail_perlin.get([
                    x as f64 * 0.3,
                    y as f64 * 0.3,
                    1.0,
                ]) * detail_scale;
                let y_detail = self.detail_perlin.get([
                    x as f64 * 0.3,
                    y as f64 * 0.3,
                    2.0,
                ]) * detail_scale;

                let normal = normalize_vector(
                    (x_noise + x_detail) as f32,
                    (y_noise + y_detail) as f32,
                    1.0
                );
                let color = Rgba([
                    ((normal.0 * 0.5 + 0.5) * 255.0) as u8,
                    ((normal.1 * 0.5 + 0.5) * 255.0) as u8,
                    ((normal.2 * 0.5 + 0.5) * 255.0) as u8,
                    255,
                ]);

                image.put_pixel(x, y, color);
            }
        }

        image
    }

    fn generate_roughness(&self, terrain_type: &str) -> RgbaImage {
        let mut image = ImageBuffer::new(TEXTURE_SIZE, TEXTURE_SIZE);
        let base_roughness = match terrain_type {
            "rock" => 0.7,
            "grass" => 0.5,
            "sand" => 0.3,
            "snow" => 0.2,
            "water" => 0.1,
            "forest" => 0.6,
            "plains" => 0.4,
            "tundra" => 0.3,
            _ => 0.5,
        };

        for y in 0..TEXTURE_SIZE {
            for x in 0..TEXTURE_SIZE {
                let noise_val = self.perlin.get([
                    x as f64 * 0.1,
                    y as f64 * 0.1,
                    3.0,
                ]) * 0.3;

                let detail = self.detail_perlin.get([
                    x as f64 * 0.4,
                    y as f64 * 0.4,
                    3.0,
                ]) * 0.15;

                let value = ((noise_val + detail + base_roughness).min(1.0).max(0.0) * 255.0) as u8;
                image.put_pixel(x, y, Rgba([value, value, value, 255]));
            }
        }

        image
    }

    fn generate_metallic(&self, terrain_type: &str) -> RgbaImage {
        let mut image = ImageBuffer::new(TEXTURE_SIZE, TEXTURE_SIZE);
        let base_metallic = match terrain_type {
            "rock" => 0.1,
            "grass" => 0.0,
            "sand" => 0.0,
            "snow" => 0.05,
            "water" => 0.0,
            "forest" => 0.0,
            "plains" => 0.0,
            "tundra" => 0.05,
            _ => 0.0,
        };

        for y in 0..TEXTURE_SIZE {
            for x in 0..TEXTURE_SIZE {
                let noise_val = self.perlin.get([
                    x as f64 * 0.2,
                    y as f64 * 0.2,
                    6.0,
                ]) * 0.1;

                let value = ((noise_val + base_metallic).min(1.0).max(0.0) * 255.0) as u8;
                image.put_pixel(x, y, Rgba([value, value, value, 255]));
            }
        }

        image
    }

    fn generate_height(&self, terrain_type: &str) -> RgbaImage {
        let mut image = ImageBuffer::new(TEXTURE_SIZE, TEXTURE_SIZE);
        let scale = match terrain_type {
            "rock" => 1.0,
            "grass" => 0.5,
            "sand" => 0.2,
            "snow" => 0.3,
            "water" => 0.1,
            _ => 0.5,
        };

        for y in 0..TEXTURE_SIZE {
            for x in 0..TEXTURE_SIZE {
                let noise_val = self.perlin.get([
                    x as f64 * 0.05,
                    y as f64 * 0.05,
                    4.0,
                ]) * scale;

                let value = ((noise_val * 0.5 + 0.5).min(1.0).max(0.0) * 255.0) as u8;
                image.put_pixel(x, y, Rgba([value, value, value, 255]));
            }
        }

        image
    }

    fn generate_ao(&self, terrain_type: &str) -> RgbaImage {
        let mut image = ImageBuffer::new(TEXTURE_SIZE, TEXTURE_SIZE);
        let intensity = match terrain_type {
            "rock" => 0.3,
            "grass" => 0.2,
            "sand" => 0.1,
            "snow" => 0.15,
            "water" => 0.05,
            _ => 0.2,
        };

        for y in 0..TEXTURE_SIZE {
            for x in 0..TEXTURE_SIZE {
                let noise_val = self.perlin.get([
                    x as f64 * 0.1,
                    y as f64 * 0.1,
                    5.0,
                ]) * intensity + (1.0 - intensity);

                let value = ((noise_val).min(1.0).max(0.0) * 255.0) as u8;
                image.put_pixel(x, y, Rgba([value, value, value, 255]));
            }
        }

        image
    }
}

fn normalize_vector(x: f32, y: f32, z: f32) -> (f32, f32, f32) {
    let length = (x * x + y * y + z * z).sqrt();
    (x / length, y / length, z / length)
}

fn main() {
    let texture_types = vec![
        "grass", "rock", "sand", "snow", "water",
        "forest", "plains", "tundra"
    ];
    let generator = TextureGenerator::new(42);

    // Create output directory if it doesn't exist
    fs::create_dir_all(OUTPUT_DIR).expect("Failed to create output directory");

    for terrain_type in texture_types {
        let map_types = vec!["albedo", "normal", "roughness", "metallic", "height", "ao"];
        
        for map_type in map_types {
            let image = match map_type {
                "albedo" => generator.generate_albedo(terrain_type),
                "normal" => generator.generate_normal(terrain_type),
                "roughness" => generator.generate_roughness(terrain_type),
                "metallic" => generator.generate_metallic(terrain_type),
                "height" => generator.generate_height(terrain_type),
                "ao" => generator.generate_ao(terrain_type),
                _ => continue,
            };

            let filename = format!("{}_{}.png", terrain_type, map_type);
            let path = Path::new(OUTPUT_DIR).join(&filename);
            image.save(&path).expect(&format!("Failed to save {}", filename));
            println!("Generated {}", filename);
        }
    }

    println!("Texture generation complete!");
} 