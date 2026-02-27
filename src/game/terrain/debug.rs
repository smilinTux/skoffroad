use bevy::prelude::*;
use bevy_egui::{egui, EguiContext};
use crate::game::terrain::{
    chunk::TerrainNoiseConfig,
    BiomeType,
};
use super::debug_preview::{TerrainPreviewTexture, update_preview_texture, render_preview_window};

#[derive(Resource)]
pub struct TerrainDebugSettings {
    // Visualization Options
    pub show_wireframe: bool,
    pub show_chunk_bounds: bool,
    pub show_lod_levels: bool,
    pub show_biome_overlay: bool,
    pub height_colormap: bool,
    pub show_slope_overlay: bool,
    pub show_normals: bool,
    pub show_erosion_impact: bool,
    pub show_performance_metrics: bool,
    pub show_curvature: bool,
    pub show_material_weights: bool,
    pub show_flow_paths: bool,
    pub show_performance_heatmap: bool,
    
    // Visualization Parameters
    pub noise_preview_size: f32,
    pub selected_biome: BiomeType,
    pub update_interval: f32,
    pub chunk_size: usize,
    
    // Height Gradient Settings
    pub height_gradient: HeightGradient,
    pub slope_gradient: SlopeGradient,
    pub curvature_gradient: CurvatureGradient,
    pub flow_gradient: FlowGradient,
    
    // Performance Metrics
    pub perf_metrics: PerfMetrics,
    
    // Export/Import
    pub settings_path: String,
}

#[derive(Default)]
pub struct HeightGradient {
    pub stops: Vec<(f32, Color)>,
    pub interpolation_mode: GradientMode,
    pub preset: GradientPreset,
}

#[derive(Default)]
pub struct SlopeGradient {
    pub min_angle: f32,
    pub max_angle: f32,
    pub min_color: Color,
    pub max_color: Color,
}

#[derive(Default)]
pub struct CurvatureGradient {
    pub min_curvature: f32,
    pub max_curvature: f32,
    pub concave_color: Color,
    pub flat_color: Color,
    pub convex_color: Color,
}

#[derive(Default)]
pub struct FlowGradient {
    pub min_flow: f32,
    pub max_flow: f32,
    pub min_color: Color,
    pub max_color: Color,
    pub arrow_scale: f32,
}

#[derive(Default)]
pub enum GradientMode {
    #[default]
    Linear,
    Step,
    Smooth,
    Bezier,
    CatmullRom,
    Custom(Box<dyn Fn(f32) -> f32>),
}

#[derive(Default)]
pub enum GradientPreset {
    #[default]
    Terrain,
    Topographic,
    Satellite,
    Heatmap,
    Custom,
}

#[derive(Default)]
pub struct PerfMetrics {
    pub chunk_gen_times: Vec<f32>,
    pub mesh_gen_times: Vec<f32>,
    pub erosion_times: Vec<f32>,
    pub frame_times: Vec<f32>,
    pub max_samples: usize,
}

impl Default for TerrainDebugSettings {
    fn default() -> Self {
        Self {
            show_wireframe: false,
            show_chunk_bounds: false,
            show_lod_levels: false,
            show_biome_overlay: false,
            height_colormap: true,
            show_slope_overlay: false,
            show_normals: false,
            show_erosion_impact: false,
            show_performance_metrics: false,
            show_curvature: false,
            show_material_weights: false,
            show_flow_paths: false,
            show_performance_heatmap: false,
            noise_preview_size: 256.0,
            selected_biome: BiomeType::Plains,
            update_interval: 0.5,
            chunk_size: 32,
            height_gradient: HeightGradient {
                stops: vec![
                    (0.0, Color::BLUE),
                    (0.3, Color::GREEN),
                    (0.6, Color::YELLOW),
                    (0.8, Color::rgb(0.6, 0.4, 0.2)), // Brown
                    (1.0, Color::WHITE),
                ],
                interpolation_mode: GradientMode::default(),
                preset: GradientPreset::default(),
            },
            slope_gradient: SlopeGradient {
                min_angle: 0.0,
                max_angle: 60.0,
                min_color: Color::GREEN,
                max_color: Color::RED,
            },
            curvature_gradient: CurvatureGradient {
                min_curvature: -1.0,
                max_curvature: 1.0,
                concave_color: Color::BLUE,
                flat_color: Color::GREEN,
                convex_color: Color::RED,
            },
            flow_gradient: FlowGradient {
                min_flow: 0.0,
                max_flow: 1.0,
                min_color: Color::WHITE,
                max_color: Color::BLUE,
                arrow_scale: 1.0,
            },
            perf_metrics: PerfMetrics {
                chunk_gen_times: Vec::new(),
                mesh_gen_times: Vec::new(),
                erosion_times: Vec::new(),
                frame_times: Vec::new(),
                max_samples: 100,
            },
            settings_path: "debug_settings.ron".to_string(),
        }
    }
}

pub fn terrain_debug_ui(
    mut egui_context: ResMut<EguiContext>,
    mut debug_settings: ResMut<TerrainDebugSettings>,
    mut noise_config: ResMut<TerrainNoiseConfig>,
) {
    egui::Window::new("Terrain Debug")
        .show(egui_context.ctx_mut(), |ui| {
            ui.heading("Visualization");
            ui.checkbox(&mut debug_settings.show_wireframe, "Show Wireframe");
            ui.checkbox(&mut debug_settings.show_chunk_bounds, "Show Chunk Bounds");
            ui.checkbox(&mut debug_settings.show_lod_levels, "Show LOD Levels");
            ui.checkbox(&mut debug_settings.show_biome_overlay, "Show Biome Overlay");
            ui.checkbox(&mut debug_settings.height_colormap, "Height Colormap");
            ui.checkbox(&mut debug_settings.show_slope_overlay, "Slope Overlay");
            ui.checkbox(&mut debug_settings.show_normals, "Show Normals");
            ui.checkbox(&mut debug_settings.show_erosion_impact, "Show Erosion Impact");
            ui.checkbox(&mut debug_settings.show_curvature, "Show Curvature");
            ui.checkbox(&mut debug_settings.show_material_weights, "Show Material Weights");
            ui.checkbox(&mut debug_settings.show_flow_paths, "Show Flow Paths");
            ui.checkbox(&mut debug_settings.show_performance_heatmap, "Show Performance Heatmap");
            ui.checkbox(&mut debug_settings.show_performance_metrics, "Show Performance Metrics");
            
            ui.separator();
            ui.heading("Height Gradient");
            ui.horizontal(|ui| {
                ui.label("Interpolation:");
                ui.radio_value(&mut debug_settings.height_gradient.interpolation_mode, GradientMode::Linear, "Linear");
                ui.radio_value(&mut debug_settings.height_gradient.interpolation_mode, GradientMode::Step, "Step");
                ui.radio_value(&mut debug_settings.height_gradient.interpolation_mode, GradientMode::Smooth, "Smooth");
                ui.radio_value(&mut debug_settings.height_gradient.interpolation_mode, GradientMode::Bezier, "Bezier");
                ui.radio_value(&mut debug_settings.height_gradient.interpolation_mode, GradientMode::CatmullRom, "Catmull-Rom");
            });
            
            ui.horizontal(|ui| {
                ui.label("Preset:");
                ui.radio_value(&mut debug_settings.height_gradient.preset, GradientPreset::Terrain, "Terrain");
                ui.radio_value(&mut debug_settings.height_gradient.preset, GradientPreset::Topographic, "Topographic");
                ui.radio_value(&mut debug_settings.height_gradient.preset, GradientPreset::Satellite, "Satellite");
                ui.radio_value(&mut debug_settings.height_gradient.preset, GradientPreset::Heatmap, "Heatmap");
                ui.radio_value(&mut debug_settings.height_gradient.preset, GradientPreset::Custom, "Custom");
            });
            
            ui.separator();
            ui.heading("Slope Overlay");
            ui.add(egui::Slider::new(&mut debug_settings.slope_gradient.min_angle, 0.0..=90.0).text("Min Angle"));
            ui.add(egui::Slider::new(&mut debug_settings.slope_gradient.max_angle, 0.0..=90.0).text("Max Angle"));
            
            ui.separator();
            ui.heading("Noise Settings");
            ui.add(egui::Slider::new(&mut noise_config.frequency, 0.1..=10.0).text("Frequency"));
            ui.add(egui::Slider::new(&mut noise_config.octaves, 1..=8).text("Octaves"));
            ui.add(egui::Slider::new(&mut noise_config.persistence, 0.0..=1.0).text("Persistence"));
            ui.add(egui::Slider::new(&mut noise_config.lacunarity, 1.0..=4.0).text("Lacunarity"));
            ui.add(egui::Slider::new(&mut noise_config.height_scale, 0.1..=10.0).text("Height Scale"));
            
            if ui.button("Randomize Seed").clicked() {
                use rand::Rng;
                noise_config.seed = rand::thread_rng().gen();
            }
            
            ui.separator();
            ui.heading("Biome Settings");
            egui::ComboBox::from_label("Selected Biome")
                .selected_text(format!("{:?}", debug_settings.selected_biome))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut debug_settings.selected_biome, BiomeType::Plains, "Plains");
                    ui.selectable_value(&mut debug_settings.selected_biome, BiomeType::Mountains, "Mountains");
                    ui.selectable_value(&mut debug_settings.selected_biome, BiomeType::Desert, "Desert");
                    ui.selectable_value(&mut debug_settings.selected_biome, BiomeType::Hills, "Hills");
                });
            
            ui.add(egui::Slider::new(&mut noise_config.biome_blend_distance, 0.0..=1.0).text("Biome Blend"));
            
            ui.separator();
            ui.heading("Erosion");
            ui.add(egui::Slider::new(&mut noise_config.erosion_iterations, 0..=10).text("Iterations"));
            
            ui.separator();
            ui.heading("Preview");
            ui.add(egui::Slider::new(&mut debug_settings.noise_preview_size, 128.0..=512.0).text("Preview Size"));
            
            ui.separator();
            ui.heading("Performance");
            ui.add(egui::Slider::new(&mut debug_settings.update_interval, 0.1..=2.0).text("Update Interval"));
            ui.add(egui::Slider::new(&mut debug_settings.chunk_size, 16..=64).text("Chunk Size"));
            
            ui.separator();
            ui.heading("Debug Settings");
            ui.horizontal(|ui| {
                if ui.button("Export Settings").clicked() {
                    export_debug_settings(&debug_settings);
                }
                if ui.button("Import Settings").clicked() {
                    if let Some(imported) = import_debug_settings() {
                        *debug_settings = imported;
                    }
                }
            });
            
            if debug_settings.show_performance_metrics {
                ui.separator();
                ui.heading("Performance Metrics");
                plot_performance_metrics(ui, &debug_settings.perf_metrics);
            }
        });
}

pub fn terrain_debug_render(
    debug_settings: Res<TerrainDebugSettings>,
    chunks: Query<(&Transform, &TerrainChunk)>,
    mut gizmos: Gizmos,
) {
    if debug_settings.show_chunk_bounds {
        for (transform, chunk) in chunks.iter() {
            let bounds = chunk.bounds;
            let min = transform.transform_point(bounds.min.extend(0.0));
            let max = transform.transform_point(bounds.max.extend(0.0));
            
            gizmos.cuboid(
                Transform::from_translation(min.lerp(max, 0.5))
                    .with_scale((max - min).abs()),
                Color::YELLOW.with_a(0.2),
            );
        }
    }

    if debug_settings.show_lod_levels {
        for (transform, chunk) in chunks.iter() {
            let pos = transform.translation;
            let lod = chunk.lod_level;
            
            gizmos.text_3d(
                pos,
                Color::GREEN,
                format!("LOD {}", lod),
            );
        }
    }
}

pub struct TerrainDebugPlugin;

impl Plugin for TerrainDebugPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<TerrainDebugSettings>()
            .init_resource::<TerrainPreviewTexture>()
            .add_systems(Update, (
                terrain_debug_ui,
                terrain_debug_render,
                update_preview_texture,
                render_preview_window,
            ));
    }
}

fn plot_performance_metrics(ui: &mut egui::Ui, metrics: &PerfMetrics) {
    use egui::plot::{Plot, Line, PlotPoints};
    
    let plot = Plot::new("performance_plot")
        .height(120.0)
        .allow_zoom(false)
        .allow_drag(false)
        .include_y(0.0);
        
    plot.show(ui, |plot_ui| {
        if !metrics.chunk_gen_times.is_empty() {
            let points: PlotPoints = metrics.chunk_gen_times.iter()
                .enumerate()
                .map(|(i, &t)| [i as f64, t as f64])
                .collect();
            plot_ui.line(Line::new(points).name("Chunk Gen Time (ms)"));
        }
        // Add similar plots for mesh_gen_times, erosion_times, and frame_times
    });
}

fn export_debug_settings(settings: &TerrainDebugSettings) {
    use std::fs::File;
    use ron::ser::to_writer_pretty;
    
    if let Ok(file) = File::create(&settings.settings_path) {
        let _ = to_writer_pretty(file, settings, ron::ser::PrettyConfig::default());
    }
}

fn import_debug_settings() -> Option<TerrainDebugSettings> {
    use std::fs::File;
    use ron::de::from_reader;
    
    if let Ok(file) = File::open("debug_settings.ron") {
        from_reader(file).ok()
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_debug_settings_default() {
        let settings = TerrainDebugSettings::default();
        assert!(!settings.show_wireframe);
        assert!(!settings.show_chunk_bounds);
        assert!(!settings.show_lod_levels);
        assert!(!settings.show_biome_overlay);
        assert!(settings.height_colormap);
        assert_eq!(settings.noise_preview_size, 256.0);
        assert!(matches!(settings.selected_biome, BiomeType::Plains));
    }
}