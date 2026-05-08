use image::{ImageBuffer, Rgb};
use noise::{NoiseFn, Perlin};

fn generate_texture(name: &str, color: Rgb<u8>, noise_scale: f64) {
    let perlin = Perlin::new();
    let size = 512;
    
    let mut img = ImageBuffer::new(size, size);
    
    for x in 0..size {
        for y in 0..size {
            let nx = x as f64 / size as f64;
            let ny = y as f64 / size as f64;
            
            let noise_val = (perlin.get([nx * noise_scale, ny * noise_scale]) + 1.0) / 2.0;
            
            let pixel = Rgb([
                (color[0] as f64 * noise_val) as u8,
                (color[1] as f64 * noise_val) as u8,
                (color[2] as f64 * noise_val) as u8,
            ]);
            
            img.put_pixel(x as u32, y as u32, pixel);
        }
    }
    
    img.save(format!("assets/textures/{}.png", name)).unwrap();
}

fn main() {
    // Create assets/textures directory if it doesn't exist
    std::fs::create_dir_all("assets/textures").unwrap();
    
    // Generate test textures
    generate_texture("grass", Rgb([34, 139, 34]), 8.0);  // Forest green
    generate_texture("rock", Rgb([169, 169, 169]), 16.0); // Dark gray
    generate_texture("snow", Rgb([255, 250, 250]), 4.0);  // Snow white
    generate_texture("dirt", Rgb([139, 69, 19]), 12.0);   // Saddle brown
    
    println!("Test textures generated successfully!");
} 