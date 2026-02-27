use bevy::{
    prelude::*,
    render::{
        camera::Camera,
        mesh::{Indices, Mesh},
        render_resource::PrimitiveTopology,
        render_resource::WgpuFeatures,
        renderer::RenderDevice,
    },
    input::mouse::MouseMotion,
    text::{Text2dBundle, TextAlignment, TextStyle},
    diagnostic::{
        DiagnosticsStore, FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin,
        EntityCountDiagnosticsPlugin, SystemInformationDiagnosticsPlugin,
    },
};
use std::f32::consts::PI;
use std::{
    fs::{self, File},
    io::{Read, Write},
    path::PathBuf,
    collections::HashMap,
};
use serde::{Deserialize, Serialize};

use super::settings::{PostProcessSettings, ToneMapping};
// use crate::game::plugins::wgpu_settings::WgpuSettings; // TODO: Fix or implement wgpu_settings module

/// Plugin that sets up a test scene for demonstrating post-processing effects
pub struct PostProcessTestPlugin;

#[derive(Component)]
struct TestCube;

#[derive(Component)]
struct TestTorus;

#[derive(Component)]
struct TestSphere;

#[derive(Component)]
struct OrbitingLight {
    radius: f32,
    speed: f32,
    phase: f32,
    height: f32,
}

#[derive(Component)]
struct CameraController {
    orbit_radius: f32,
    orbit_speed: f32,
    height: f32,
}

#[derive(Component)]
struct HelpText;

#[derive(Resource)]
struct EffectPresets {
    presets: Vec<(String, PostProcessSettings)>,
    current: usize,
    custom_slots: [Option<(String, PostProcessSettings)>; 3], // 3 slots for custom presets
    auto_animate: bool,
    animation_time: f32,
}

#[derive(Resource)]
struct MouseControlState {
    active_control: Option<PostProcessControl>,
    start_value: f32,
    start_position: Vec2,
}

#[derive(Resource)]
struct ComparisonMode {
    enabled: bool,
    split_position: f32,
    left_preset: usize,
    right_preset: usize,
}

#[derive(Component)]
struct PerformanceText;

#[derive(PartialEq, Clone, Copy)]
enum PostProcessControl {
    Exposure,
    Bloom,
    ChromaticAberration,
    Contrast,
    Saturation,
    Brightness,
    VignetteRadius,
}

#[derive(Resource)]
struct PerformanceMetrics {
    gpu_time: f32,
    memory_usage: u64,
    draw_calls: u32,
    last_update: f32,
}

#[derive(Resource, Default)]
struct SceneConfigs {
    save_directory: PathBuf,
    current_config: Option<String>,
}

#[derive(Resource)]
struct PresetManager {
    save_directory: PathBuf,
    current_preset: Option<String>,
    categories: Vec<String>,
    tags: HashMap<String, Vec<String>>,
    rename_mode: Option<(String, String)>, // (old_name, new_name)
    blend_source: Option<(String, PostProcessSettings, f32)>, // (name, settings, blend_factor)
    import_path: Option<PathBuf>,
}

#[derive(Serialize, Deserialize)]
struct SceneConfiguration {
    name: String,
    camera_position: Vec3,
    camera_rotation: Quat,
    post_process_settings: PostProcessSettings,
    lights: Vec<LightConfig>,
}

#[derive(Serialize, Deserialize)]
struct LightConfig {
    position: Vec3,
    color: [f32; 4],
    intensity: f32,
}

#[derive(Component)]
struct TestPattern;

#[derive(Component)]
struct TestCheckerboard;

#[derive(Serialize, Deserialize)]
struct PresetMetadata {
    name: String,
    category: String,
    tags: Vec<String>,
    settings: PostProcessSettings,
}

impl Default for MouseControlState {
    fn default() -> Self {
        Self {
            active_control: None,
            start_value: 0.0,
            start_position: Vec2::ZERO,
        }
    }
}

impl Default for EffectPresets {
    fn default() -> Self {
        let mut presets = Vec::new();
        presets.extend([
            (
                "HDR Test".to_string(),
                PostProcessSettings {
                    exposure: 1.0,
                    gamma: 2.2,
                    contrast: 1.0,
                    saturation: 1.0,
                    brightness: 1.0,
                    bloom_intensity: 0.0,
                    bloom_threshold: 1.0,
                    chromatic_aberration: 0.0,
                    vignette_strength: 0.0,
                    vignette_radius: 1.0,
                    tone_mapping: ToneMapping::ACES as u32,
                }
            ),
            (
                "Bloom Test".to_string(),
                PostProcessSettings {
                    exposure: 1.0,
                    gamma: 2.2,
                    contrast: 1.0,
                    saturation: 1.0,
                    brightness: 1.0,
                    bloom_intensity: 1.0,
                    bloom_threshold: 1.0,
                    chromatic_aberration: 0.0,
                    vignette_strength: 0.0,
                    vignette_radius: 1.0,
                    tone_mapping: ToneMapping::None as u32,
                }
            ),
            (
                "Aberration Test".to_string(),
                PostProcessSettings {
                    exposure: 1.0,
                    gamma: 2.2,
                    contrast: 1.0,
                    saturation: 1.0,
                    brightness: 1.0,
                    bloom_intensity: 0.0,
                    bloom_threshold: 1.0,
                    chromatic_aberration: 0.05,
                    vignette_strength: 0.0,
                    vignette_radius: 1.0,
                    tone_mapping: ToneMapping::None as u32,
                }
            ),
        ]);

        Self {
            presets,
            current: 0,
            custom_slots: [None, None, None],
            auto_animate: false,
            animation_time: 0.0,
        }
    }
}

impl Default for ComparisonMode {
    fn default() -> Self {
        Self {
            enabled: false,
            split_position: 0.5,
            left_preset: 0,
            right_preset: 1,
        }
    }
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self {
            gpu_time: 0.0,
            memory_usage: 0,
            draw_calls: 0,
            last_update: 0.0,
        }
    }
}

impl Default for PresetManager {
    fn default() -> Self {
        Self {
            save_directory: PathBuf::from("presets"),
            current_preset: None,
            categories: vec!["Default".to_string(), "Custom".to_string(), "Imported".to_string()],
            tags: HashMap::new(),
            rename_mode: None,
            blend_source: None,
            import_path: None,
        }
    }
}

impl Plugin for PostProcessTestPlugin {
    fn build(&self, app: &mut App) {
        // Create save directories if they don't exist
        let save_dir = PathBuf::from("scene_configs");
        let preset_dir = PathBuf::from("presets");
        fs::create_dir_all(&save_dir).unwrap_or_default();
        fs::create_dir_all(&preset_dir).unwrap_or_default();

        app.init_resource::<EffectPresets>()
            .init_resource::<MouseControlState>()
            .init_resource::<ComparisonMode>()
            .init_resource::<PerformanceMetrics>()
            .init_resource::<PresetManager>()
            .insert_resource(SceneConfigs {
                save_directory: save_dir,
                current_config: None,
            })
            .add_plugins((
                FrameTimeDiagnosticsPlugin::default(),
                LogDiagnosticsPlugin::default(),
                EntityCountDiagnosticsPlugin::default(),
                SystemInformationDiagnosticsPlugin::default(),
            ))
            .add_systems(Startup, (setup_test_scene, setup_help_text, setup_performance_text))
            .add_systems(Update, (
                update_settings,
                rotate_objects,
                orbit_lights,
                handle_user_controls,
                handle_mouse_controls,
                handle_comparison_controls,
                handle_scene_configs,
                handle_preset_manager,
                update_camera.after(handle_user_controls),
                update_help_text,
                update_performance_text,
                update_performance_metrics,
                update_motion_blur_tests,
            ));
    }
}

/// Updates post-processing settings over time to demonstrate different effects
fn update_settings(
    time: Res<Time>,
    mut settings: ResMut<PostProcessSettings>,
    keyboard: Res<Input<KeyCode>>,
) {
    let t = time.elapsed_seconds();
    
    // Only auto-animate if not in manual control mode
    if !keyboard.any_pressed([KeyCode::Key1, KeyCode::Key2, KeyCode::Key3, KeyCode::Key4]) {
        // Smoothly cycle through different effects
        settings.exposure = 1.0 + (t * 0.5).sin() * 0.5;
        settings.bloom_intensity = 0.5 + (t * 0.3).sin() * 0.3;
        settings.chromatic_aberration = ((t * 0.2).sin() * 0.5 + 0.5).max(0.0);
        settings.vignette_radius = ((t * 0.15).sin() * 0.3 + 0.3).max(0.0);
        settings.tone_mapping = (((t / 5.0).floor() as u32) % 3) + 1;
        settings.contrast = 1.0 + (t * 0.25).sin() * 0.2;
        settings.saturation = 1.0 + (t * 0.4).sin() * 0.3;
        settings.brightness = 1.0 + (t * 0.35).sin() * 0.2;
    }
}

/// Rotates test objects with different patterns
fn rotate_objects(
    time: Res<Time>,
    mut cubes: Query<&mut Transform, (With<TestCube>, Without<TestTorus>, Without<TestSphere>)>,
    mut tori: Query<&mut Transform, (With<TestTorus>, Without<TestCube>, Without<TestSphere>)>,
    mut spheres: Query<&mut Transform, (With<TestSphere>, Without<TestCube>, Without<TestTorus>)>,
) {
    let dt = time.delta_seconds();
    let t = time.elapsed_seconds();

    // Rotate cubes
    for mut transform in cubes.iter_mut() {
        transform.rotate_y(dt * 0.5);
        transform.rotate_x(dt * 0.3);
    }

    // Rotate tori with wave motion
    for mut transform in tori.iter_mut() {
        transform.rotate_z(dt * 0.2);
        transform.translation.y = 1.0 + (t * 0.8).sin() * 0.2;
    }

    // Rotate spheres in figure-8 pattern
    for mut transform in spheres.iter_mut() {
        let figure8_x = (t * 0.5).sin() * 0.5;
        let figure8_z = (t * 1.0).sin() * 0.25;
        transform.translation.x = figure8_x;
        transform.translation.z = figure8_z;
        transform.rotate_y(dt);
    }
}

/// Updates orbiting lights
fn orbit_lights(
    time: Res<Time>,
    mut lights: Query<(&mut Transform, &OrbitingLight)>,
) {
    let t = time.elapsed_seconds();
    
    for (mut transform, light) in lights.iter_mut() {
        let angle = t * light.speed + light.phase;
        transform.translation.x = angle.cos() * light.radius;
        transform.translation.z = angle.sin() * light.radius;
        transform.translation.y = light.height + (t * 0.5).sin() * 0.5;
    }
}

/// Updates camera position based on controller
fn update_camera(
    time: Res<Time>,
    mut cameras: Query<(&mut Transform, &CameraController)>,
) {
    let t = time.elapsed_seconds();
    
    for (mut transform, controller) in cameras.iter_mut() {
        let angle = t * controller.orbit_speed;
        transform.translation.x = angle.cos() * controller.orbit_radius;
        transform.translation.z = angle.sin() * controller.orbit_radius;
        transform.translation.y = controller.height;
        transform.look_at(Vec3::ZERO, Vec3::Y);
    }
}

/// Creates a torus mesh
fn create_torus(radius: f32, tube_radius: f32, segments: usize, sides: usize) -> Mesh {
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut indices = Vec::new();

    for segment in 0..segments {
        let segment_angle = segment as f32 * 2.0 * PI / segments as f32;
        let segment_cos = segment_angle.cos();
        let segment_sin = segment_angle.sin();

        for side in 0..sides {
            let side_angle = side as f32 * 2.0 * PI / sides as f32;
            let side_cos = side_angle.cos();
            let side_sin = side_angle.sin();

            let x = (radius + tube_radius * side_cos) * segment_cos;
            let y = tube_radius * side_sin;
            let z = (radius + tube_radius * side_cos) * segment_sin;

            positions.push([x, y, z]);
            
            let normal = Vec3::new(
                segment_cos * side_cos,
                side_sin,
                segment_sin * side_cos,
            ).normalize();
            normals.push(normal.to_array());
            
            uvs.push([
                segment as f32 / segments as f32,
                side as f32 / sides as f32,
            ]);

            if segment < segments - 1 && side < sides - 1 {
                let current = segment * sides + side;
                let next_segment = (segment + 1) * sides + side;
                let next_side = segment * sides + (side + 1);
                let next_both = (segment + 1) * sides + (side + 1);

                indices.extend_from_slice(&[
                    current as u32,
                    next_segment as u32,
                    next_both as u32,
                    current as u32,
                    next_both as u32,
                    next_side as u32,
                ]);
            }
        }
    }

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.set_indices(Some(Indices::U32(indices)));
    mesh
}

/// Sets up the test scene with a camera, lighting, and test objects
fn setup_test_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Camera with controller
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(-2.0, 2.5, 5.0)
                .looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        },
        PostProcessSettings::default(),
        CameraController {
            orbit_radius: 6.0,
            orbit_speed: 0.1,
            height: 3.0,
        },
    ));

    // Orbiting lights with different colors and patterns
    for i in 0..5 {
        let (color, height, radius, speed) = match i {
            0 => (Color::rgb(1.0, 0.5, 0.5), 2.0, 3.0, 0.5),  // Red, medium
            1 => (Color::rgb(0.5, 1.0, 0.5), 3.0, 4.0, 0.4),  // Green, high
            2 => (Color::rgb(0.5, 0.5, 1.0), 1.5, 3.5, 0.6),  // Blue, low
            3 => (Color::rgb(1.0, 1.0, 0.5), 2.5, 4.5, 0.3),  // Yellow, medium-high
            _ => (Color::rgb(1.0, 0.5, 1.0), 1.8, 5.0, 0.2),  // Purple, medium-low
        };

        commands.spawn((
            PointLightBundle {
                point_light: PointLight {
                    intensity: 1000.0,
                    shadows_enabled: true,
                    color,
                    ..default()
                },
                transform: Transform::from_xyz(0.0, height, 0.0),
                ..default()
            },
            OrbitingLight {
                radius,
                speed,
                phase: i as f32 * PI * 0.4,
                height,
            },
        ));
    }

    // Add more test objects with varied materials
    spawn_test_objects(&mut commands, &mut meshes, &mut materials);
}

fn create_test_patterns(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    // Checkerboard pattern for chromatic aberration testing
    let mut checkerboard = Mesh::new(PrimitiveTopology::TriangleList);
    let size = 4.0;
    let segments = 16;
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut colors = Vec::new();
    let mut indices = Vec::new();

    for z in 0..=segments {
        for x in 0..=segments {
            let px = size * (x as f32 / segments as f32 - 0.5);
            let pz = size * (z as f32 / segments as f32 - 0.5);
            let is_white = (x + z) % 2 == 0;
            
            positions.push([px, 0.0, pz]);
            normals.push([0.0, 1.0, 0.0]);
            uvs.push([x as f32 / segments as f32, z as f32 / segments as f32]);
            colors.push(if is_white {
                [1.0, 1.0, 1.0, 1.0]
            } else {
                [0.0, 0.0, 0.0, 1.0]
            });

            if x < segments && z < segments {
                let i = z * (segments + 1) + x;
                indices.extend_from_slice(&[
                    i as u32,
                    (i + segments + 1) as u32,
                    (i + 1) as u32,
                    (i + 1) as u32,
                    (i + segments + 1) as u32,
                    (i + segments + 2) as u32,
                ]);
            }
        }
    }

    checkerboard.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    checkerboard.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    checkerboard.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    checkerboard.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    checkerboard.set_indices(Some(Indices::U32(indices)));

    commands.spawn((
        PbrBundle {
            mesh: meshes.add(checkerboard),
            material: materials.add(StandardMaterial {
                base_color: Color::WHITE,
                unlit: true,
                ..default()
            }),
            transform: Transform::from_xyz(-6.0, 0.1, -6.0),
            ..default()
        },
        TestCheckerboard,
    ));

    // Concentric circles for vignette testing
    let mut circles = Mesh::new(PrimitiveTopology::TriangleList);
    let radius = 2.0;
    let rings = 12;
    let segments = 32;
    
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut colors = Vec::new();
    let mut indices = Vec::new();

    for ring in 0..=rings {
        let r = radius * (ring as f32 / rings as f32);
        for segment in 0..=segments {
            let angle = segment as f32 * 2.0 * PI / segments as f32;
            let x = r * angle.cos();
            let z = r * angle.sin();
            
            positions.push([x, 0.0, z]);
            normals.push([0.0, 1.0, 0.0]);
            uvs.push([
                (x / radius + 1.0) * 0.5,
                (z / radius + 1.0) * 0.5,
            ]);
            
            let brightness = 1.0 - (ring as f32 / rings as f32);
            colors.push([brightness, brightness, brightness, 1.0]);

            if ring < rings && segment < segments {
                let current = ring * (segments + 1) + segment;
                let next_ring = (ring + 1) * (segments + 1) + segment;
                indices.extend_from_slice(&[
                    current as u32,
                    next_ring as u32,
                    (current + 1) as u32,
                    (current + 1) as u32,
                    next_ring as u32,
                    (next_ring + 1) as u32,
                ]);
            }
        }
    }

    circles.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    circles.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    circles.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    circles.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    circles.set_indices(Some(Indices::U32(indices)));

    commands.spawn((
        PbrBundle {
            mesh: meshes.add(circles),
            material: materials.add(StandardMaterial {
                base_color: Color::WHITE,
                unlit: true,
                ..default()
            }),
            transform: Transform::from_xyz(6.0, 0.1, -6.0),
            ..default()
        },
        TestPattern,
    ));

    // Color gradient for saturation/contrast testing
    let mut gradient = Mesh::new(PrimitiveTopology::TriangleList);
    let size = 4.0;
    let segments = 16;
    
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut colors = Vec::new();
    let mut indices = Vec::new();

    for z in 0..=segments {
        for x in 0..=segments {
            let px = size * (x as f32 / segments as f32 - 0.5);
            let pz = size * (z as f32 / segments as f32 - 0.5);
            
            positions.push([px, 0.0, pz]);
            normals.push([0.0, 1.0, 0.0]);
            uvs.push([x as f32 / segments as f32, z as f32 / segments as f32]);
            
            let hue = x as f32 / segments as f32 * 360.0;
            let saturation = z as f32 / segments as f32;
            let color = Color::hsl(hue, saturation, 0.5);
            colors.push([color.r(), color.g(), color.b(), 1.0]);

            if x < segments && z < segments {
                let i = z * (segments + 1) + x;
                indices.extend_from_slice(&[
                    i as u32,
                    (i + segments + 1) as u32,
                    (i + 1) as u32,
                    (i + 1) as u32,
                    (i + segments + 1) as u32,
                    (i + segments + 2) as u32,
                ]);
            }
        }
    }

    gradient.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    gradient.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    gradient.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    gradient.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    gradient.set_indices(Some(Indices::U32(indices)));

    commands.spawn((
        PbrBundle {
            mesh: meshes.add(gradient),
            material: materials.add(StandardMaterial {
                base_color: Color::WHITE,
                unlit: true,
                ..default()
            }),
            transform: Transform::from_xyz(6.0, 0.1, 6.0),
            ..default()
        },
        TestPattern,
    ));
}

/// Creates additional test patterns for HDR and bloom testing
fn create_hdr_test_patterns(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    // HDR intensity strips for testing tone mapping
    let mut hdr_strips = Mesh::new(PrimitiveTopology::TriangleList);
    let size = 4.0;
    let strips = 8;
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut colors = Vec::new();
    let mut indices = Vec::new();

    for i in 0..=strips {
        let x = size * (i as f32 / strips as f32 - 0.5);
        
        // Create vertical strip
        positions.extend_from_slice(&[
            [x, 0.0, -size/2.0],
            [x, 0.0, size/2.0],
            [x + size/strips as f32, 0.0, size/2.0],
            [x + size/strips as f32, 0.0, -size/2.0],
        ]);

        let intensity = (2.0_f32).powf(i as f32 - strips as f32 / 2.0);
        let color = [intensity, intensity, intensity, 1.0];
        colors.extend_from_slice(&[color, color, color, color]);

        normals.extend_from_slice(&[[0.0, 1.0, 0.0]; 4]);
        uvs.extend_from_slice(&[
            [0.0, 0.0],
            [0.0, 1.0],
            [1.0, 1.0],
            [1.0, 0.0],
        ]);

        let base_idx = (i * 4) as u32;
        indices.extend_from_slice(&[
            base_idx, base_idx + 1, base_idx + 2,
            base_idx, base_idx + 2, base_idx + 3,
        ]);
    }

    hdr_strips.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    hdr_strips.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    hdr_strips.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    hdr_strips.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    hdr_strips.set_indices(Some(Indices::U32(indices)));

    commands.spawn((
        PbrBundle {
            mesh: meshes.add(hdr_strips),
            material: materials.add(StandardMaterial {
                base_color: Color::WHITE,
                emissive: Color::WHITE * 5.0,
                unlit: true,
                ..default()
            }),
            transform: Transform::from_xyz(-6.0, 0.1, 6.0),
            ..default()
        },
        TestPattern,
    ));

    // Bloom test pattern with bright spots
    let mut bloom_test = Mesh::new(PrimitiveTopology::TriangleList);
    let size = 4.0;
    let spots = 5;
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut colors = Vec::new();
    let mut indices = Vec::new();

    for i in 0..spots {
        let angle = i as f32 * 2.0 * PI / spots as f32;
        let radius = size * 0.3;
        let x = radius * angle.cos();
        let z = radius * angle.sin();
        
        // Create small bright quad
        let quad_size = 0.2;
        positions.extend_from_slice(&[
            [x - quad_size, 0.0, z - quad_size],
            [x - quad_size, 0.0, z + quad_size],
            [x + quad_size, 0.0, z + quad_size],
            [x + quad_size, 0.0, z - quad_size],
        ]);

        let intensity = 10.0 + i as f32 * 5.0;
        let hue = i as f32 / spots as f32 * 360.0;
        let color = Color::hsl(hue, 1.0, 0.5);
        let color_array = [color.r() * intensity, color.g() * intensity, color.b() * intensity, 1.0];
        colors.extend_from_slice(&[color_array; 4]);

        normals.extend_from_slice(&[[0.0, 1.0, 0.0]; 4]);
        uvs.extend_from_slice(&[
            [0.0, 0.0],
            [0.0, 1.0],
            [1.0, 1.0],
            [1.0, 0.0],
        ]);

        let base_idx = (i * 4) as u32;
        indices.extend_from_slice(&[
            base_idx, base_idx + 1, base_idx + 2,
            base_idx, base_idx + 2, base_idx + 3,
        ]);
    }

    bloom_test.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    bloom_test.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    bloom_test.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    bloom_test.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    bloom_test.set_indices(Some(Indices::U32(indices)));

    commands.spawn((
        PbrBundle {
            mesh: meshes.add(bloom_test),
            material: materials.add(StandardMaterial {
                base_color: Color::WHITE,
                emissive: Color::WHITE * 10.0,
                unlit: true,
                ..default()
            }),
            transform: Transform::from_xyz(0.0, 0.1, -6.0),
            ..default()
        },
        TestPattern,
    ));
}

fn spawn_test_objects(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    // Add test patterns first
    create_test_patterns(commands, meshes, materials);
    create_hdr_test_patterns(commands, meshes, materials);
    
    // Add new test patterns
    create_resolution_test_pattern(commands, meshes, materials);
    create_noise_test_pattern(commands, meshes, materials);
    create_depth_test_pattern(commands, meshes, materials);
    create_motion_blur_test_pattern(commands, meshes, materials);
    create_specular_test_pattern(commands, meshes, materials);

    // Central rotating cube with emissive material
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
            material: materials.add(StandardMaterial {
                base_color: Color::rgb(0.8, 0.2, 0.2),
                emissive: Color::rgb(1.0, 0.2, 0.2) * 2.0,
                metallic: 0.7,
                perceptual_roughness: 0.2,
                ..default()
            }),
            transform: Transform::from_xyz(0.0, 0.5, 0.0),
            ..default()
        },
        TestCube,
    ));

    // Floating torus with chrome-like material
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(create_torus(1.0, 0.2, 32, 16)),
            material: materials.add(StandardMaterial {
                base_color: Color::rgb(0.2, 0.8, 0.2),
                emissive: Color::rgb(0.2, 1.0, 0.2),
                metallic: 0.9,
                perceptual_roughness: 0.1,
                ..default()
            }),
            transform: Transform::from_xyz(2.0, 1.0, 0.0),
            ..default()
        },
        TestTorus,
    ));

    // Multiple orbiting spheres with different materials
    for i in 0..5 {
        let (color, metallic, roughness, emissive_strength) = match i {
            0 => (Color::rgb(0.2, 0.2, 0.8), 0.9, 0.1, 1.0),  // Chrome blue
            1 => (Color::rgb(0.8, 0.8, 0.2), 0.0, 0.8, 0.5),  // Matte yellow
            2 => (Color::rgb(0.8, 0.2, 0.8), 0.5, 0.5, 1.5),  // Mixed purple
            3 => (Color::rgb(0.2, 0.8, 0.8), 0.8, 0.2, 2.0),  // Metallic cyan
            _ => (Color::rgb(0.9, 0.5, 0.3), 0.3, 0.6, 0.8),  // Copper orange
        };

        commands.spawn((
            PbrBundle {
                mesh: meshes.add(Mesh::from(shape::UVSphere {
                    radius: 0.3,
                    sectors: 32,
                    stacks: 16,
                })),
                material: materials.add(StandardMaterial {
                    base_color: color,
                    emissive: color * emissive_strength,
                    metallic,
                    perceptual_roughness: roughness,
                    ..default()
                }),
                transform: Transform::from_xyz(
                    -2.0 + i as f32 * 0.8,
                    1.0 + (i as f32 * 0.3).sin(),
                    (i as f32 * 0.5).cos(),
                ),
                ..default()
            },
            TestSphere,
        ));
    }

    // Floating icosahedron with glass-like material
    commands.spawn(PbrBundle {
        mesh: meshes.add(create_icosahedron()),
        material: materials.add(StandardMaterial {
            base_color: Color::rgba(0.8, 0.9, 1.0, 0.5),
            metallic: 0.0,
            perceptual_roughness: 0.1,
            alpha_mode: AlphaMode::Blend,
            ..default()
        }),
        transform: Transform::from_xyz(-1.5, 2.0, -1.0),
        ..default()
    });

    // Textured ground plane with normal mapping
    commands.spawn(PbrBundle {
        mesh: meshes.add(create_detailed_ground_plane()),
        material: materials.add(StandardMaterial {
            base_color: Color::rgb(0.3, 0.5, 0.3),
            metallic: 0.0,
            perceptual_roughness: 0.8,
            ..default()
        }),
        transform: Transform::from_xyz(0.0, -0.5, 0.0),
        ..default()
    });
}

fn create_icosahedron() -> Mesh {
    let phi = (1.0 + 5.0_f32.sqrt()) / 2.0;
    let vertices = vec![
        Vec3::new(0.0, 1.0, phi),
        Vec3::new(0.0, -1.0, phi),
        // ... Add more vertices for icosahedron
    ];
    
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
    // ... Set up vertices, indices, normals
    mesh
}

fn create_detailed_ground_plane() -> Mesh {
    let size = 20.0;
    let segments = 32;
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
    
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut indices = Vec::new();
    
    // Generate vertices with slight height variation
    for z in 0..=segments {
        for x in 0..=segments {
            let px = size * (x as f32 / segments as f32 - 0.5);
            let pz = size * (z as f32 / segments as f32 - 0.5);
            let py = (px * 0.1).sin() * (pz * 0.1).cos() * 0.1; // Subtle waves
            
            positions.push([px, py, pz]);
            normals.push([0.0, 1.0, 0.0]);
            uvs.push([x as f32 / segments as f32, z as f32 / segments as f32]);
            
            if x < segments && z < segments {
                let i = z * (segments + 1) + x;
                indices.extend_from_slice(&[
                    i,
                    i + segments + 1,
                    i + 1,
                    i + 1,
                    i + segments + 1,
                    i + segments + 2,
                ]);
            }
        }
    }
    
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.set_indices(Some(Indices::U32(indices)));
    
    mesh
}

/// Sets up help text overlay
fn setup_help_text(mut commands: Commands) {
    commands.spawn((
        Text2dBundle {
            text: Text::from_section(
                "Loading...",
                TextStyle {
                    font_size: 20.0,
                    color: Color::WHITE,
                    ..default()
                },
            ).with_alignment(TextAlignment::Left),
            transform: Transform::from_xyz(-580.0, 320.0, 0.0),
            ..default()
        },
        HelpText,
    ));
}

/// Sets up performance text overlay
fn setup_performance_text(mut commands: Commands) {
    commands.spawn((
        Text2dBundle {
            text: Text::from_section(
                "FPS: --",
                TextStyle {
                    font_size: 20.0,
                    color: Color::GREEN,
                    ..default()
                },
            ).with_alignment(TextAlignment::Right),
            transform: Transform::from_xyz(580.0, 320.0, 0.0),
            ..default()
        },
        PerformanceText,
    ));
}

/// Updates the help text with current controls and settings
fn update_help_text(
    settings: Res<PostProcessSettings>,
    presets: Res<EffectPresets>,
    mouse_state: Res<MouseControlState>,
    comparison: Res<ComparisonMode>,
    scene_configs: Res<SceneConfigs>,
    preset_manager: Res<PresetManager>,
    mut query: Query<&mut Text, With<HelpText>>,
) {
    if let Ok(mut text) = query.get_single_mut() {
        let preset_name = &presets.presets[presets.current].0;
        let custom_slots: Vec<String> = presets.custom_slots.iter()
            .enumerate()
            .map(|(i, slot)| {
                if let Some((name, _)) = slot {
                    format!("F{}: {}", i + 1, name)
                } else {
                    format!("F{}: Empty", i + 1)
                }
            })
            .collect();
        
        let mouse_control = if let Some(control) = mouse_state.active_control {
            match control {
                PostProcessControl::Exposure => "Exposure",
                PostProcessControl::Bloom => "Bloom",
                PostProcessControl::ChromaticAberration => "Chromatic",
                PostProcessControl::Contrast => "Contrast",
                PostProcessControl::Saturation => "Saturation",
                PostProcessControl::Brightness => "Brightness",
                PostProcessControl::VignetteRadius => "Vignette Radius",
            }
        } else {
            "None"
        };

        let comparison_status = if comparison.enabled {
            format!(
                "\nComparison Mode (Tab to disable):\n\
                Left (Q): {}\n\
                Right (E): {}\n\
                Split: {:.0}% (Ctrl + Drag)",
                presets.presets[comparison.left_preset].0,
                presets.presets[comparison.right_preset].0,
                comparison.split_position * 100.0
            )
        } else {
            "\nTab: Enable Comparison Mode".to_string()
        };

        let scene_config_status = if let Some(name) = &scene_configs.current_config {
            format!("\n\nActive Scene Config: {}", name)
        } else {
            "\n\nNo Scene Config Loaded".to_string()
        };

        let preset_status = if let Some(name) = &preset_manager.current_preset {
            let tags = preset_manager.tags.get(name)
                .map(|t| t.join(", "))
                .unwrap_or_else(|| "No tags".to_string());
                
            let blend_info = if let Some((source_name, _, factor)) = &preset_manager.blend_source {
                format!("\nBlending with {} ({:.0}%)", source_name, factor * 100.0)
            } else {
                String::new()
            };

            let rename_info = if let Some((old_name, new_name)) = &preset_manager.rename_mode {
                format!("\nRenaming '{}' to '{}'", old_name, new_name)
            } else {
                String::new()
            };

            format!("\nActive Preset: {} ({}){}{}", 
                name, tags, blend_info, rename_info)
        } else {
            "\nNo Custom Preset Active".to_string()
        };

        let slot_name = |slot: &Option<(String, PostProcessSettings)>| -> String {
            slot.as_ref().map(|(n, _)| n.clone()).unwrap_or_else(|| "".to_string())
        };
        let slot0 = slot_name(&presets.custom_slots[0]);
        let slot1 = slot_name(&presets.custom_slots[1]);
        let slot2 = slot_name(&presets.custom_slots[2]);
        text.sections[0].value = format!(
            "Controls:\n\
            Hold number + Up/Down:\n\
            1: Exposure ({:.2})\n\
            2: Bloom ({:.2})\n\
            3: Chromatic ({:.2})\n\
            4: Contrast ({:.2})\n\
            5: Saturation ({:.2})\n\
            6: Brightness ({:.2})\n\
            7: Vignette Radius ({:.2})\n\
            \nMouse Controls:\n\
            Hold number + Left Click: Fine-tune value\n\
            Active Control: {}\n\
            \nPresets:\n\
            P: Cycle Presets ({})\n\
            Ctrl + F1-F3: Save custom preset\n\
            Custom Slot 1: {}\n\
            Custom Slot 2: {}\n\
            Custom Slot 3: {}\n\
            \nT: Cycle Tone Mapping ({})\n\
            Space: Toggle Auto-Animate\n\
            R: Reset to Default\n\
            {}\n\
            Scene Configuration:\n\
            Ctrl + S: Save current setup\n\
            Ctrl + L: Load next config\n\
            {}\n\
            Preset Controls:\n\
            Ctrl + Shift + 1-9: Save preset\n\
            Alt + 1-9: Load preset\n\
            Ctrl + R: Rename preset\n\
            Ctrl + T: Add/edit tags\n\
            Ctrl + E: Export preset\n\
            Ctrl + I: Import preset\n\
            Ctrl + B: Start preset blending\n\
            Left/Right: Adjust blend\n\
            Escape: Cancel blend",
            settings.exposure,
            settings.bloom_intensity,
            settings.chromatic_aberration,
            settings.contrast,
            settings.saturation,
            settings.brightness,
            settings.vignette_radius,
            mouse_control,
            preset_name,
            slot0,
            slot1,
            slot2,
            match settings.tone_mapping {
                x if x == ToneMapping::ACES as u32 => "ACES",
                x if x == ToneMapping::Reinhard as u32 => "Reinhard",
                x if x == ToneMapping::Uncharted2 as u32 => "Uncharted2",
                _ => "None",
            },
            scene_config_status,
            preset_status,
        );
    }
}

/// Handle user controls for post-processing settings
fn handle_user_controls(
    keyboard: Res<Input<KeyCode>>,
    mut settings: ResMut<PostProcessSettings>,
    mut presets: ResMut<EffectPresets>,
    time: Res<Time>,
    mut auto_animate: Local<bool>,
) {
    let dt = time.delta_seconds();

    // Save custom preset (Ctrl + F1-F3)
    if keyboard.pressed(KeyCode::ControlLeft) || keyboard.pressed(KeyCode::ControlRight) {
        let slot = if keyboard.just_pressed(KeyCode::F1) {
            Some(0)
        } else if keyboard.just_pressed(KeyCode::F2) {
            Some(1)
        } else if keyboard.just_pressed(KeyCode::F3) {
            Some(2)
        } else {
            None
        };

        if let Some(slot) = slot {
            presets.custom_slots[slot] = Some((
                format!("Custom {}", slot + 1),
                settings.clone(),
            ));
            return;
        }
    }

    // Load custom preset (F1-F3)
    if !keyboard.pressed(KeyCode::ControlLeft) && !keyboard.pressed(KeyCode::ControlRight) {
        let slot = if keyboard.just_pressed(KeyCode::F1) {
            Some(0)
        } else if keyboard.just_pressed(KeyCode::F2) {
            Some(1)
        } else if keyboard.just_pressed(KeyCode::F3) {
            Some(2)
        } else {
            None
        };

        if let Some(slot) = slot {
            if let Some((_, preset)) = &presets.custom_slots[slot] {
                *settings = preset.clone();
                return;
            }
        }
    }

    // Toggle auto-animation
    if keyboard.just_pressed(KeyCode::Space) {
        *auto_animate = !*auto_animate;
    }

    // Reset to default
    if keyboard.just_pressed(KeyCode::R) {
        *settings = PostProcessSettings::default();
        presets.current = 0;
        return;
    }

    // Cycle through presets
    if keyboard.just_pressed(KeyCode::P) {
        presets.current = (presets.current + 1) % presets.presets.len();
        *settings = presets.presets[presets.current].1.clone();
        return;
    }

    // Only process manual controls if auto-animate is off
    if !*auto_animate {
        // Exposure controls (1)
        if keyboard.pressed(KeyCode::Key1) {
            if keyboard.pressed(KeyCode::Up) { settings.exposure += dt; }
            if keyboard.pressed(KeyCode::Down) { settings.exposure -= dt; }
        }
        
        // Bloom controls (2)
        if keyboard.pressed(KeyCode::Key2) {
            if keyboard.pressed(KeyCode::Up) { settings.bloom_intensity += dt; }
            if keyboard.pressed(KeyCode::Down) { settings.bloom_intensity -= dt; }
        }
        
        // Chromatic aberration controls (3)
        if keyboard.pressed(KeyCode::Key3) {
            if keyboard.pressed(KeyCode::Up) { settings.chromatic_aberration += dt; }
            if keyboard.pressed(KeyCode::Down) { settings.chromatic_aberration -= dt; }
        }
        
        // Contrast controls (4)
        if keyboard.pressed(KeyCode::Key4) {
            if keyboard.pressed(KeyCode::Up) { settings.contrast += dt; }
            if keyboard.pressed(KeyCode::Down) { settings.contrast -= dt; }
        }

        // Saturation controls (5)
        if keyboard.pressed(KeyCode::Key5) {
            if keyboard.pressed(KeyCode::Up) { settings.saturation += dt; }
            if keyboard.pressed(KeyCode::Down) { settings.saturation -= dt; }
        }

        // Brightness controls (6)
        if keyboard.pressed(KeyCode::Key6) {
            if keyboard.pressed(KeyCode::Up) { settings.brightness += dt; }
            if keyboard.pressed(KeyCode::Down) { settings.brightness -= dt; }
        }

        // Vignette radius controls (7)
        if keyboard.pressed(KeyCode::Key7) {
            if keyboard.pressed(KeyCode::Up) { settings.vignette_radius += dt; }
            if keyboard.pressed(KeyCode::Down) { settings.vignette_radius -= dt; }
        }
        
        // Tone mapping cycling (T)
        if keyboard.just_pressed(KeyCode::T) {
            settings.tone_mapping = (settings.tone_mapping + 1) % 4;
        }

        // Clamp values
        settings.exposure = settings.exposure.clamp(0.0, 3.0);
        settings.bloom_intensity = settings.bloom_intensity.clamp(0.0, 2.0);
        settings.chromatic_aberration = settings.chromatic_aberration.clamp(0.0, 1.0);
        settings.vignette_radius = settings.vignette_radius.clamp(0.0, 1.0);
        settings.contrast = settings.contrast.clamp(0.5, 2.0);
        settings.saturation = settings.saturation.clamp(0.0, 2.0);
        settings.brightness = settings.brightness.clamp(0.0, 2.0);
    } else {
        // Auto-animate settings
        let t = time.elapsed_seconds();
        settings.exposure = 1.0 + (t * 0.5).sin() * 0.5;
        settings.bloom_intensity = 0.5 + (t * 0.3).sin() * 0.3;
        settings.chromatic_aberration = ((t * 0.2).sin() * 0.5 + 0.5).max(0.0);
        settings.vignette_radius = ((t * 0.15).sin() * 0.3 + 0.3).max(0.0);
        settings.tone_mapping = (settings.tone_mapping + 1) % 4;
        settings.contrast = 1.0 + (t * 0.25).sin() * 0.2;
        settings.saturation = 1.0 + (t * 0.4).sin() * 0.3;
        settings.brightness = 1.0 + (t * 0.35).sin() * 0.2;
    }
}

fn handle_mouse_controls(
    mut mouse_state: ResMut<MouseControlState>,
    mut settings: ResMut<PostProcessSettings>,
    keyboard: Res<Input<KeyCode>>,
    mouse: Res<Input<MouseButton>>,
    mut mouse_motion: EventReader<MouseMotion>,
    windows: Query<&Window>,
) {
    let window = windows.single();

    // Start mouse control
    if mouse.just_pressed(MouseButton::Left) {
        let control = if keyboard.pressed(KeyCode::Key1) {
            Some(PostProcessControl::Exposure)
        } else if keyboard.pressed(KeyCode::Key2) {
            Some(PostProcessControl::Bloom)
        } else if keyboard.pressed(KeyCode::Key3) {
            Some(PostProcessControl::ChromaticAberration)
        } else if keyboard.pressed(KeyCode::Key4) {
            Some(PostProcessControl::Contrast)
        } else if keyboard.pressed(KeyCode::Key5) {
            Some(PostProcessControl::Saturation)
        } else if keyboard.pressed(KeyCode::Key6) {
            Some(PostProcessControl::Brightness)
        } else if keyboard.pressed(KeyCode::Key7) {
            Some(PostProcessControl::VignetteRadius)
        } else {
            None
        };

        if let Some(control) = control {
            if let Some(position) = window.cursor_position() {
                mouse_state.active_control = Some(control);
                mouse_state.start_position = position;
                mouse_state.start_value = match control {
                    PostProcessControl::Exposure => settings.exposure,
                    PostProcessControl::Bloom => settings.bloom_intensity,
                    PostProcessControl::ChromaticAberration => settings.chromatic_aberration,
                    PostProcessControl::Contrast => settings.contrast,
                    PostProcessControl::Saturation => settings.saturation,
                    PostProcessControl::Brightness => settings.brightness,
                    PostProcessControl::VignetteRadius => settings.vignette_radius,
                };
            }
        }
    }

    // End mouse control
    if mouse.just_released(MouseButton::Left) {
        mouse_state.active_control = None;
    }

    // Apply mouse control
    if let Some(control) = mouse_state.active_control {
        let motion: Vec2 = mouse_motion.read().map(|m| Vec2::new(m.delta.x, m.delta.y)).sum();
        let sensitivity = 0.005;
        let delta = motion.x * sensitivity;

        let new_value = (mouse_state.start_value + delta).clamp(0.0, 3.0);
        match control {
            PostProcessControl::Exposure => settings.exposure = new_value.clamp(0.0, 3.0),
            PostProcessControl::Bloom => settings.bloom_intensity = new_value.clamp(0.0, 2.0),
            PostProcessControl::ChromaticAberration => settings.chromatic_aberration = new_value.clamp(0.0, 1.0),
            PostProcessControl::Contrast => settings.contrast = new_value.clamp(0.5, 2.0),
            PostProcessControl::Saturation => settings.saturation = new_value.clamp(0.0, 2.0),
            PostProcessControl::Brightness => settings.brightness = new_value.clamp(0.0, 2.0),
            PostProcessControl::VignetteRadius => settings.vignette_radius = new_value.clamp(0.0, 1.0),
        }
    }
}

fn handle_comparison_controls(
    mut comparison: ResMut<ComparisonMode>,
    keyboard: Res<Input<KeyCode>>,
    mouse: Res<Input<MouseButton>>,
    mut mouse_motion: EventReader<MouseMotion>,
    windows: Query<&Window>,
) {
    // Toggle comparison mode
    if keyboard.just_pressed(KeyCode::Tab) {
        comparison.enabled = !comparison.enabled;
    }

    if comparison.enabled {
        // Cycle left preset
        if keyboard.just_pressed(KeyCode::Q) {
            comparison.left_preset = (comparison.left_preset + 1) % 6;
        }
        // Cycle right preset
        if keyboard.just_pressed(KeyCode::E) {
            comparison.right_preset = (comparison.right_preset + 1) % 6;
        }

        // Adjust split position with mouse drag
        if mouse.pressed(MouseButton::Left) && keyboard.pressed(KeyCode::ControlLeft) {
            let motion: Vec2 = mouse_motion.read().map(|m| Vec2::new(m.delta.x, m.delta.y)).sum();
            comparison.split_position = (comparison.split_position + motion.x * 0.001).clamp(0.1, 0.9);
        }
    }
}

fn handle_scene_configs(
    keyboard: Res<Input<KeyCode>>,
    camera_query: Query<(&Transform, &PostProcessSettings), With<Camera>>,
    light_query: Query<(&Transform, &PointLight), With<OrbitingLight>>,
    mut scene_configs: ResMut<SceneConfigs>,
) {
    // Save current configuration (Ctrl + S)
    if keyboard.pressed(KeyCode::ControlLeft) && keyboard.just_pressed(KeyCode::S) {
        if let Ok((camera_transform, settings)) = camera_query.get_single() {
            let config = SceneConfiguration {
                name: "scene_config".to_string(),
                camera_position: camera_transform.translation,
                camera_rotation: camera_transform.rotation,
                post_process_settings: settings.clone(),
                lights: light_query
                    .iter()
                    .map(|(transform, light)| LightConfig {
                        position: transform.translation,
                        color: [
                            light.color.r(),
                            light.color.g(),
                            light.color.b(),
                            light.color.a(),
                        ],
                        intensity: light.intensity,
                    })
                    .collect(),
            };

            let file_path = scene_configs.save_directory.join(format!("{}.json", config.name));
            if let Ok(mut file) = File::create(&file_path) {
                if let Ok(json) = serde_json::to_string_pretty(&config) {
                    let _ = file.write_all(json.as_bytes());
                    scene_configs.current_config = Some(config.name);
                }
            }
        }
    }

    // Load previous configuration (Ctrl + L)
    if keyboard.pressed(KeyCode::ControlLeft) && keyboard.just_pressed(KeyCode::L) {
        if let Ok(entries) = fs::read_dir(&scene_configs.save_directory) {
            let mut configs: Vec<_> = entries
                .filter_map(|entry| entry.ok())
                .filter(|entry| entry.path().extension().map_or(false, |ext| ext == "json"))
                .collect();
            
            configs.sort_by_key(|entry| entry.path());

            if let Some(current_name) = &scene_configs.current_config {
                if let Some(current_idx) = configs.iter().position(|entry| {
                    entry.path().file_stem().map_or(false, |stem| {
                        stem.to_string_lossy() == *current_name
                    })
                }) {
                    let next_idx = (current_idx + 1) % configs.len();
                    if let Ok(mut file) = File::open(configs[next_idx].path()) {
                        let mut contents = String::new();
                        if file.read_to_string(&mut contents).is_ok() {
                            if let Ok(config) = serde_json::from_str::<SceneConfiguration>(&contents) {
                                // Apply configuration (handled by other systems)
                                scene_configs.current_config = Some(config.name);
                            }
                        }
                    }
                }
            }
        }
    }
}

fn handle_preset_manager(
    keyboard: Res<Input<KeyCode>>,
    mut preset_manager: ResMut<PresetManager>,
    mut settings: ResMut<PostProcessSettings>,
    mut presets: ResMut<EffectPresets>,
    time: Res<Time>,
) {
    // Rename mode (Ctrl + R while hovering over preset)
    if keyboard.pressed(KeyCode::ControlLeft) && keyboard.just_pressed(KeyCode::R) {
        if let Some(current) = &preset_manager.current_preset {
            preset_manager.rename_mode = Some((current.clone(), String::new()));
            return;
        }
    }

    // Confirm rename (Enter)
    let rename_mode = preset_manager.rename_mode.clone();
    if let Some((old_name, new_name)) = rename_mode {
        if keyboard.just_pressed(KeyCode::Return) {
            if !new_name.is_empty() {
                // Rename file
                let old_path = preset_manager.save_directory.join(format!("{}.json", old_name));
                let new_path = preset_manager.save_directory.join(format!("{}.json", new_name));
                if let Ok(_) = std::fs::rename(&old_path, &new_path) {
                    // Update in-memory presets
                    if let Some(index) = presets.presets.iter().position(|(name, _)| name == &old_name) {
                        presets.presets[index].0 = new_name.clone();
                        preset_manager.current_preset = Some(new_name.clone());
                        // Update tags
                        let tags = preset_manager.tags.remove(&old_name);
                        if let Some(tags) = tags {
                            preset_manager.tags.insert(new_name.clone(), tags);
                        }
                    }
                }
            }
            preset_manager.rename_mode = None;
        }
    }

    // Add/remove tags (Ctrl + T)
    let current_preset = preset_manager.current_preset.clone();
    if keyboard.pressed(KeyCode::ControlLeft) && keyboard.just_pressed(KeyCode::T) {
        if let Some(current) = current_preset {
            let tags = preset_manager.tags.entry(current.clone()).or_insert_with(Vec::new);
            // TODO: Show tag input UI
            tags.push("new_tag".to_string());
        }
    }

    // Export preset (Ctrl + E)
    if keyboard.pressed(KeyCode::ControlLeft) && keyboard.just_pressed(KeyCode::E) {
        if let Some(current) = &preset_manager.current_preset {
            if let Some((_, preset)) = presets.presets.iter().find(|(name, _)| name == current) {
                let metadata = PresetMetadata {
                    name: current.clone(),
                    category: "Exported".to_string(),
                    tags: preset_manager.tags.get(current).cloned().unwrap_or_default(),
                    settings: preset.clone(),
                };
                let export_path = preset_manager.save_directory.join("exports");
                std::fs::create_dir_all(&export_path).unwrap_or_default();
                let file_path = export_path.join(format!("{}_export.json", current));
                if let Ok(mut file) = std::fs::File::create(&file_path) {
                    if let Ok(json) = serde_json::to_string_pretty(&metadata) {
                        let _ = file.write_all(json.as_bytes());
                    }
                }
            }
        }
    }

    // Import preset (Ctrl + I)
    if keyboard.pressed(KeyCode::ControlLeft) && keyboard.just_pressed(KeyCode::I) {
        let import_path = preset_manager.save_directory.join("imports");
        if let Ok(entries) = std::fs::read_dir(import_path) {
            for entry in entries.filter_map(|e| e.ok()) {
                if let Ok(mut file) = std::fs::File::open(entry.path()) {
                    let mut contents = String::new();
                    if file.read_to_string(&mut contents).is_ok() {
                        if let Ok(metadata) = serde_json::from_str::<PresetMetadata>(&contents) {
                            // Add to presets
                            presets.presets.push((metadata.name.clone(), metadata.settings));
                            preset_manager.tags.insert(metadata.name.clone(), metadata.tags);
                            preset_manager.current_preset = Some(metadata.name);
                        }
                    }
                }
            }
        }
    }

    // Start preset blending (Ctrl + B)
    if keyboard.pressed(KeyCode::ControlLeft) && keyboard.just_pressed(KeyCode::B) {
        if let Some(current) = &preset_manager.current_preset {
            if let Some((_, preset)) = presets.presets.iter().find(|(name, _)| name == current) {
                preset_manager.blend_source = Some((current.clone(), preset.clone(), 0.0));
            }
        }
    }

    // Update blend factor
    let current_preset = preset_manager.current_preset.clone();
    if let Some((_, source, factor)) = &mut preset_manager.blend_source {
        if keyboard.pressed(KeyCode::Left) {
            *factor = (*factor - time.delta_seconds()).max(0.0);
        }
        if keyboard.pressed(KeyCode::Right) {
            *factor = (*factor + time.delta_seconds()).min(1.0);
        }
        // Apply blend
        if let Some(current) = current_preset {
            if let Some((_, target)) = presets.presets.iter().find(|(n, _)| n == &current) {
                let mut blended = source.clone();
                blend_settings(&mut blended, target, *factor);
                *settings = blended;
            }
        }
        // End blend mode
        if keyboard.just_pressed(KeyCode::Escape) {
            preset_manager.blend_source = None;
        }
    }

    // Save preset (Ctrl + Shift + 1-9)
    if keyboard.pressed(KeyCode::ControlLeft) && keyboard.pressed(KeyCode::ShiftLeft) {
        for i in 1..=9 {
            if keyboard.just_pressed(match i {
                1 => KeyCode::Key1,
                2 => KeyCode::Key2,
                3 => KeyCode::Key3,
                4 => KeyCode::Key4,
                5 => KeyCode::Key5,
                6 => KeyCode::Key6,
                7 => KeyCode::Key7,
                8 => KeyCode::Key8,
                9 => KeyCode::Key9,
                _ => continue,
            }) {
                let preset_name = format!("Custom Preset {}", i);
                let file_path = preset_manager.save_directory.join(format!("{}.json", preset_name));
                if let Ok(mut file) = std::fs::File::create(&file_path) {
                    if let Ok(json) = serde_json::to_string_pretty(&*settings) {
                        let _ = file.write_all(json.as_bytes());
                        preset_manager.current_preset = Some(preset_name.clone());
                        // Add to presets list if not already present
                        if !presets.presets.iter().any(|(name, _)| name == &preset_name) {
                            presets.presets.push((preset_name.clone(), settings.clone()));
                        }
                    }
                }
                break;
            }
        }
    }

    // Load preset (Alt + 1-9)
    if keyboard.pressed(KeyCode::AltLeft) {
        for i in 1..=9 {
            if keyboard.just_pressed(match i {
                1 => KeyCode::Key1,
                2 => KeyCode::Key2,
                3 => KeyCode::Key3,
                4 => KeyCode::Key4,
                5 => KeyCode::Key5,
                6 => KeyCode::Key6,
                7 => KeyCode::Key7,
                8 => KeyCode::Key8,
                9 => KeyCode::Key9,
                _ => continue,
            }) {
                let preset_name = format!("Custom Preset {}", i);
                let file_path = preset_manager.save_directory.join(format!("{}.json", preset_name));
                if let Ok(mut file) = std::fs::File::open(&file_path) {
                    let mut contents = String::new();
                    if file.read_to_string(&mut contents).is_ok() {
                        if let Ok(loaded_settings) = serde_json::from_str::<PostProcessSettings>(&contents) {
                            // Find or add preset
                            if let Some(index) = presets.presets.iter().position(|(name, _)| name == &preset_name) {
                                presets.presets[index].1 = loaded_settings.clone();
                                presets.current = index;
                            } else {
                                presets.presets.push((preset_name.clone(), loaded_settings.clone()));
                                presets.current = presets.presets.len() - 1;
                            }
                            preset_manager.current_preset = Some(preset_name);
                            *settings = loaded_settings;
                        }
                    }
                }
                break;
            }
        }
    }
}

fn blend_settings(source: &mut PostProcessSettings, target: &PostProcessSettings, factor: f32) {
    source.exposure = lerp(source.exposure, target.exposure, factor);
    source.contrast = lerp(source.contrast, target.contrast, factor);
    source.saturation = lerp(source.saturation, target.saturation, factor);
    source.brightness = lerp(source.brightness, target.brightness, factor);
    source.bloom_intensity = lerp(source.bloom_intensity, target.bloom_intensity, factor);
    source.chromatic_aberration = lerp(source.chromatic_aberration, target.chromatic_aberration, factor);
    source.vignette_radius = lerp(source.vignette_radius, target.vignette_radius, factor);
    
    // Discrete values like tone_mapping use step function
    if factor > 0.5 {
        source.tone_mapping = target.tone_mapping;
    }
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn update_performance_metrics(
    time: Res<Time>,
    render_device: Res<RenderDevice>,
    mut metrics: ResMut<PerformanceMetrics>,
    diagnostics: Res<DiagnosticsStore>,
) {
    // Update every 0.5 seconds
    if time.elapsed_seconds() - metrics.last_update >= 0.5 {
        metrics.last_update = time.elapsed_seconds();

        // Get GPU time (if available)
        if render_device.features().contains(WgpuFeatures::TIMESTAMP_QUERY) {
            metrics.gpu_time = time.delta_seconds() * 1000.0; // Approximate for now
        }

        // Get memory usage from system information diagnostics
        if let Some(memory_diag) = diagnostics.get(SystemInformationDiagnosticsPlugin::MEM_USAGE) {
            if let Some(memory) = memory_diag.value() {
                metrics.memory_usage = memory as u64;
            }
        }

        // Draw calls: Bevy 0.12+ does not provide a built-in diagnostic for draw calls by default.
        // If you have a custom diagnostic, use it here. Otherwise, comment out or remove this section.
        // metrics.draw_calls = 0; // Placeholder or remove if not used.
    }
}

fn update_performance_text(
    diagnostics: Res<DiagnosticsStore>,
    metrics: Res<PerformanceMetrics>,
    settings: Res<PostProcessSettings>,
    mut query: Query<&mut Text, With<PerformanceText>>,
) {
    if let Ok(mut text) = query.get_single_mut() {
        if let Some(fps_diag) = diagnostics.get(FrameTimeDiagnosticsPlugin::FPS) {
            if let Some(fps) = fps_diag.smoothed() {
                text.sections[0].value = format!(
                    "Performance Metrics:\n\
                    FPS: {:.1} ({:.1}ms)\n\
                    GPU Time: {:.2}ms\n\
                    Memory: {:.1} MB\n\
                    Draw Calls: {}\n\
                    Entities: {}\n\n\
                    Active Effects:\n\
                    - Bloom: {}\n\
                    - Chromatic: {}\n\
                    - Vignette: {}\n\
                    - Tone Map: {}",
                    fps,
                    1000.0 / fps,
                    metrics.gpu_time,
                    metrics.memory_usage as f32 / (1024.0 * 1024.0),
                    metrics.draw_calls,
                    if let Some(count_diag) = diagnostics.get(EntityCountDiagnosticsPlugin::ENTITY_COUNT) {
                        if let Some(count) = count_diag.value() {
                            count as u32
                        } else {
                            0
                        }
                    } else {
                        0
                    },
                    settings.bloom_intensity > 0.0,
                    settings.chromatic_aberration > 0.0,
                    settings.vignette_radius > 0.0,
                    match settings.tone_mapping {
                        x if x == ToneMapping::ACES as u32 => "ACES",
                        x if x == ToneMapping::Reinhard as u32 => "Reinhard",
                        x if x == ToneMapping::Uncharted2 as u32 => "Uncharted2",
                        _ => "None",
                    },
                );
            }
        }
    }
}

/// Creates a resolution test pattern with fine lines and text
fn create_resolution_test_pattern(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    let mut resolution_test = Mesh::new(PrimitiveTopology::TriangleList);
    let size = 4.0;
    let segments = 64; // Higher resolution for fine details
    
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut colors = Vec::new();
    let mut indices = Vec::new();

    // Create fine line grid pattern
    for z in 0..=segments {
        for x in 0..=segments {
            let px = size * (x as f32 / segments as f32 - 0.5);
            let pz = size * (z as f32 / segments as f32 - 0.5);
            
            positions.push([px, 0.0, pz]);
            normals.push([0.0, 1.0, 0.0]);
            uvs.push([x as f32 / segments as f32, z as f32 / segments as f32]);
            
            // Create alternating line pattern
            let is_line = x % 4 == 0 || z % 4 == 0;
            let is_text_region = (x > segments / 4 && x < segments * 3 / 4) && 
                               (z > segments / 4 && z < segments * 3 / 4);
            
            colors.push(if is_line {
                [0.0, 0.0, 0.0, 1.0]
            } else if is_text_region {
                [0.8, 0.8, 0.8, 1.0]
            } else {
                [1.0, 1.0, 1.0, 1.0]
            });

            if x < segments && z < segments {
                let i = z * (segments + 1) + x;
                indices.extend_from_slice(&[
                    i as u32,
                    (i + segments + 1) as u32,
                    (i + 1) as u32,
                    (i + 1) as u32,
                    (i + segments + 1) as u32,
                    (i + segments + 2) as u32,
                ]);
            }
        }
    }

    resolution_test.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    resolution_test.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    resolution_test.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    resolution_test.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    resolution_test.set_indices(Some(Indices::U32(indices)));

    commands.spawn((
        PbrBundle {
            mesh: meshes.add(resolution_test),
            material: materials.add(StandardMaterial {
                base_color: Color::WHITE,
                unlit: true,
                ..default()
            }),
            transform: Transform::from_xyz(-6.0, 0.1, 0.0),
            ..default()
        },
        TestPattern,
    ));
}

/// Creates a noise pattern for testing temporal effects
fn create_noise_test_pattern(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    let mut noise_test = Mesh::new(PrimitiveTopology::TriangleList);
    let size = 4.0;
    let segments = 32;
    
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut colors = Vec::new();
    let mut indices = Vec::new();

    use rand::{Rng, SeedableRng};
    let mut rng = rand::rngs::StdRng::seed_from_u64(42);

    for z in 0..=segments {
        for x in 0..=segments {
            let px = size * (x as f32 / segments as f32 - 0.5);
            let pz = size * (z as f32 / segments as f32 - 0.5);
            
            // Add slight height variation for visual interest
            let py = rng.gen_range(-0.1..0.1);
            
            positions.push([px, py, pz]);
            normals.push([0.0, 1.0, 0.0]);
            uvs.push([x as f32 / segments as f32, z as f32 / segments as f32]);
            
            // Generate noise pattern
            let noise_value = rng.gen_range(0.0..1.0);
            colors.push([noise_value, noise_value, noise_value, 1.0]);

            if x < segments && z < segments {
                let i = z * (segments + 1) + x;
                indices.extend_from_slice(&[
                    i as u32,
                    (i + segments + 1) as u32,
                    (i + 1) as u32,
                    (i + 1) as u32,
                    (i + segments + 1) as u32,
                    (i + segments + 2) as u32,
                ]);
            }
        }
    }

    noise_test.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    noise_test.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    noise_test.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    noise_test.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    noise_test.set_indices(Some(Indices::U32(indices)));

    commands.spawn((
        PbrBundle {
            mesh: meshes.add(noise_test),
            material: materials.add(StandardMaterial {
                base_color: Color::WHITE,
                unlit: true,
                ..default()
            }),
            transform: Transform::from_xyz(0.0, 0.1, 0.0),
            ..default()
        },
        TestPattern,
    ));
}

/// Creates a depth complexity test with overlapping transparent objects
fn create_depth_test_pattern(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    // Create multiple overlapping transparent planes at different heights
    for i in 0..5 {
        let height = 0.2 + i as f32 * 0.2;
        let color = match i {
            0 => Color::rgba(1.0, 0.0, 0.0, 0.3), // Red
            1 => Color::rgba(0.0, 1.0, 0.0, 0.3), // Green
            2 => Color::rgba(0.0, 0.0, 1.0, 0.3), // Blue
            3 => Color::rgba(1.0, 1.0, 0.0, 0.3), // Yellow
            _ => Color::rgba(1.0, 0.0, 1.0, 0.3), // Magenta
        };

        commands.spawn((
            PbrBundle {
                mesh: meshes.add(Mesh::from(shape::Plane { size: 2.0, subdivisions: 1 })),
                material: materials.add(StandardMaterial {
                    base_color: color,
                    alpha_mode: AlphaMode::Blend,
                    ..default()
                }),
                transform: Transform::from_xyz(6.0, height, 0.0),
                ..default()
            },
            TestPattern,
        ));
    }
}

/// Creates a motion blur test pattern with varying velocities
fn create_motion_blur_test_pattern(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    // Create rotating rings with different velocities
    for i in 0..4 {
        let radius = 0.5 + i as f32 * 0.3;
        let speed = 1.0 + i as f32 * 0.5;
        let color = Color::hsla(i as f32 * 90.0, 1.0, 0.5, 1.0);

        commands.spawn((
            PbrBundle {
                mesh: meshes.add(create_torus(radius, 0.05, 32, 16)),
                material: materials.add(StandardMaterial {
                    base_color: color,
                    emissive: color * 2.0,
                    ..default()
                }),
                transform: Transform::from_xyz(-3.0, 0.1, 3.0),
                ..default()
            },
            TestPattern,
            MotionBlurTest { speed },
        ));
    }
}

/// Creates a specular highlight test pattern with different roughness values
fn create_specular_test_pattern(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    let sphere_mesh = meshes.add(Mesh::from(shape::UVSphere {
        radius: 0.3,
        sectors: 32,
        stacks: 16,
    }));

    // Create spheres with varying roughness and metallic values
    for i in 0..5 {
        for j in 0..5 {
            let roughness = i as f32 / 4.0;
            let metallic = j as f32 / 4.0;
            
            commands.spawn((
                PbrBundle {
                    mesh: sphere_mesh.clone(),
                    material: materials.add(StandardMaterial {
                        base_color: Color::rgb(0.8, 0.8, 0.8),
                        metallic,
                        perceptual_roughness: roughness,
                        ..default()
                    }),
                    transform: Transform::from_xyz(
                        3.0 + i as f32 * 0.7 - 1.4,
                        0.3,
                        3.0 + j as f32 * 0.7 - 1.4
                    ),
                    ..default()
                },
                TestPattern,
            ));
        }
    }
}

#[derive(Component)]
struct MotionBlurTest {
    speed: f32,
}

// Add motion blur test system
fn update_motion_blur_tests(
    time: Res<Time>,
    mut query: Query<(&mut Transform, &MotionBlurTest)>,
) {
    for (mut transform, test) in query.iter_mut() {
        transform.rotate_y(time.delta_seconds() * test.speed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scene_setup() {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            PostProcessTestPlugin,
        ));

        // Run startup systems
        app.update();

        // Verify scene setup
        assert!(app.world.query::<&Camera>().iter(&app.world).count() == 1);
        assert!(app.world.query::<&PointLight>().iter(&app.world).count() == 5); // Now 5 lights
        assert!(app.world.query::<&TestCube>().iter(&app.world).count() == 1);
        assert!(app.world.query::<&TestTorus>().iter(&app.world).count() == 1);
        assert!(app.world.query::<&TestSphere>().iter(&app.world).count() == 3);
    }

    #[test]
    fn test_settings_update() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<PostProcessSettings>()
            .add_systems(Update, update_settings);

        // Run for a few frames
        for _ in 0..10 {
            app.update();
        }

        // Verify settings were updated
        let settings = app.world.resource::<PostProcessSettings>();
        assert!(settings.exposure != 1.0);
        assert!(settings.bloom_intensity != 0.5);
        assert!(settings.tone_mapping >= 1 && settings.tone_mapping <= 3);
    }

    #[test]
    fn test_user_controls() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<PostProcessSettings>()
            .init_resource::<Input<KeyCode>>()
            .add_systems(Update, handle_user_controls);

        let mut input = app.world.resource_mut::<Input<KeyCode>>();
        input.press(KeyCode::Key1);
        input.press(KeyCode::Up);

        // Run a frame
        app.update();

        // Verify exposure was increased
        let settings = app.world.resource::<PostProcessSettings>();
        assert!(settings.exposure > 1.0);
    }

    #[test]
    fn test_presets() {
        let presets = EffectPresets::default();
        assert_eq!(presets.presets.len(), 4); // Default + 3 custom presets
        assert_eq!(presets.current, 0);
        assert_eq!(presets.presets[0].0, "Default");
    }

    #[test]
    fn test_help_text() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_systems(Startup, setup_help_text);
        
        app.update();
        
        assert!(app.world.query::<&Text>().iter(&app.world).count() == 1);
    }

    #[test]
    fn test_custom_presets() {
        let mut presets = EffectPresets::default();
        assert!(presets.custom_slots.iter().all(|slot| slot.is_none()));

        // Test saving a custom preset
        let custom_settings = PostProcessSettings {
            exposure: 1.5,
            ..default()
        };
        presets.custom_slots[0] = Some(("Custom 1".to_string(), custom_settings.clone()));
        
        assert_eq!(presets.custom_slots[0].as_ref().unwrap().0, "Custom 1");
        assert_eq!(presets.custom_slots[0].as_ref().unwrap().1.exposure, 1.5);
    }

    #[test]
    fn test_mouse_control() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<PostProcessSettings>()
            .init_resource::<MouseControlState>()
            .add_systems(Update, handle_mouse_controls);

        let mut mouse_state = app.world.resource_mut::<MouseControlState>();
        mouse_state.active_control = Some(PostProcessControl::Exposure);
        mouse_state.start_value = 1.0;

        app.update();

        let settings = app.world.resource::<PostProcessSettings>();
        assert!(settings.exposure >= 0.0 && settings.exposure <= 3.0);
    }

    #[test]
    fn test_comparison_mode() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<ComparisonMode>()
            .add_systems(Update, handle_comparison_controls);

        let comparison = app.world.resource::<ComparisonMode>();
        assert!(!comparison.enabled);
        assert_eq!(comparison.split_position, 0.5);
        assert_eq!(comparison.left_preset, 0);
        assert_eq!(comparison.right_preset, 1);
    }

    #[test]
    fn test_performance_text() {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            FrameTimeDiagnosticsPlugin::default(),
        ))
        .add_systems(Startup, setup_performance_text)
        .add_systems(Update, update_performance_text);

        app.update();

        let mut query = app.world.query::<&Text>();
        assert_eq!(query.iter(&app.world).count(), 1);
    }
}