use std::path::{Path, PathBuf};
use std::{env, fs};

fn main() {
    println!("cargo:rerun-if-changed=assets/shaders");
    println!("cargo:rerun-if-changed=build.rs");

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let shader_dir = PathBuf::from("assets/shaders");

    // Ensure shader output directory exists
    fs::create_dir_all(&out_dir.join("shaders")).unwrap();

    // Process all WGSL shaders
    process_shaders(&shader_dir, &out_dir);
}

fn process_shaders(shader_dir: &Path, out_dir: &Path) {
    if !shader_dir.exists() {
        return;
    }

    for entry in fs::read_dir(shader_dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();

        if path.is_file() && path.extension().map_or(false, |ext| ext == "wgsl") {
            let shader_name = path.file_name().unwrap().to_str().unwrap();
            let dest_path = out_dir.join("shaders").join(shader_name);

            // Copy shader to output directory
            fs::copy(&path, &dest_path).unwrap();

            // Validate shader
            validate_shader(&path);

            println!("cargo:rerun-if-changed={}", path.display());
        }
    }
}

fn validate_shader(path: &Path) {
    // TODO: Add naga validation when stabilized
    // For now, we just ensure the file exists and is readable
    if let Err(e) = fs::read_to_string(path) {
        panic!("Failed to read shader {}: {}", path.display(), e);
    }
}

#[cfg(feature = "shader-hot-reload")]
fn setup_shader_hot_reload() {
    println!("cargo:rustc-cfg=shader_hot_reload");
    // Additional setup for hot reloading if needed
} 