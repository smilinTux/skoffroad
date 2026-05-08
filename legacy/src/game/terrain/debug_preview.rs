use std::sync::Arc;
use rayon::prelude::*;
use bevy::prelude::*;
use bevy_egui::{egui, EguiContext};
use crate::game::terrain::{
    chunk::TerrainNoiseConfig,
    heightmap::Heightmap,
    BiomeType,
    debug::*,
    material::MaterialWeights,
};

/// Resource for preview texture with double buffering
#[derive(Resource)]
pub struct TerrainPreviewTexture {
    // Double buffered textures for smooth updates
    front_buffer: Option<egui::TextureHandle>,
    back_buffer: Option<egui::TextureHandle>,
    needs_swap: bool,
    last_update: f32,
    
    // Cached data for visualization
    cached_data: PreviewCache,
    chunk_updates: Vec<(usize, usize)>,
    current_chunk: usize,
    
    // Performance metrics
    perf_metrics: PreviewPerformanceMetrics,
    
    // New visualization settings
    normal_map_scale: f32,
    slope_sensitivity: f32,
    visualization_blend: f32,
}

/// Cached data for visualization modes
#[derive(Default)]
struct PreviewCache {
    normals: Option<Arc<Vec<Vec3>>>,
    slopes: Option<Arc<Vec<f32>>>,
    curvature: Option<Arc<Vec<f32>>>,
    flow: Option<Arc<Vec<Vec2>>>,
    material_weights: Option<Arc<Vec<MaterialWeights>>>,
    biome_boundaries: Option<Arc<Vec<bool>>>,
    erosion_impact: Option<Arc<Vec<f32>>>,
    cache_hits: usize,
    cache_misses: usize,
    
    // New cached data
    tangents: Option<Arc<Vec<Vec3>>>,
    bitangents: Option<Arc<Vec<Vec3>>>,
    ambient_occlusion: Option<Arc<Vec<f32>>>,
    roughness: Option<Arc<Vec<f32>>>,
}

/// Performance tracking for the preview system
#[derive(Default)]
struct PreviewPerformanceMetrics {
    generation_times: Vec<f32>,
    max_generation_time: f32,
    avg_generation_time: f32,
    memory_usage: usize,
    frame_times: Vec<f32>,
    cache_hit_rate: f32,
    chunks_per_second: f32,
    
    // New metrics
    gpu_time: f32,
    cpu_time: f32,
    cache_memory_usage: usize,
    texture_memory_usage: usize,
    draw_calls: usize,
}

impl Default for TerrainPreviewTexture {
    fn default() -> Self {
        Self {
            front_buffer: None,
            back_buffer: None,
            needs_swap: false,
            last_update: 0.0,
            cached_data: PreviewCache::default(),
            chunk_updates: Vec::new(),
            current_chunk: 0,
            perf_metrics: PreviewPerformanceMetrics::default(),
            normal_map_scale: 1.0,
            slope_sensitivity: 1.0,
            visualization_blend: 0.5,
        }
    }
}

/// System to render the preview texture
pub fn update_preview_texture(
    mut preview: ResMut<TerrainPreviewTexture>,
    mut egui_context: ResMut<EguiContext>,
    time: Res<Time>,
    debug_settings: Res<TerrainDebugSettings>,
    noise_config: Res<TerrainNoiseConfig>,
) {
    let start_time = time.elapsed_seconds();
    
    // Check if update is needed
    if time.elapsed_seconds() - preview.last_update < 1.0 / 30.0 {
        return;
    }
    
    let size = debug_settings.preview_size as usize;
    
    // Initialize or resize textures if needed
    if preview.front_buffer.is_none() || preview.back_buffer.is_none() {
        let pixels = vec![0u8; size * size * 4];
        preview.front_buffer = Some(egui_context.ctx_mut().load_texture(
            "terrain_preview_front",
            egui::ColorImage::from_rgba_unmultiplied([size, size], &pixels),
            egui::TextureOptions::default(),
        ));
        preview.back_buffer = Some(egui_context.ctx_mut().load_texture(
            "terrain_preview_back",
            egui::ColorImage::from_rgba_unmultiplied([size, size], &pixels),
            egui::TextureOptions::default(),
        ));
    }
    
    // Generate heightmap for preview
    let heightmap = Heightmap::generate_preview(
        UVec2::new(size as u32, size as u32),
        &noise_config,
    );
    
    // Update cached data
    update_cached_data(&heightmap, &debug_settings, &mut preview.cached_data);
    
    // Process chunks in parallel
    let mut chunk_pixels: Vec<((usize, usize), Vec<u8>)> = Vec::new();
    let chunk_size = 32;
    
    preview.chunk_updates.par_iter().for_each_with(&chunk_pixels, |pixels, &(chunk_x, chunk_y)| {
        let mut chunk_data = vec![0u8; chunk_size * chunk_size * 4];
        
        for y in 0..chunk_size {
            for x in 0..chunk_size {
                let px = chunk_x * chunk_size + x;
                let py = chunk_y * chunk_size + y;
                
                if px >= size || py >= size {
                    continue;
                }
                
                let color = get_visualization_color(
                    px,
                    py,
                    &heightmap,
                    &debug_settings,
                    &preview,
                );
                
                let idx = (y * chunk_size + x) * 4;
                chunk_data[idx] = (color.r() * 255.0) as u8;
                chunk_data[idx + 1] = (color.g() * 255.0) as u8;
                chunk_data[idx + 2] = (color.b() * 255.0) as u8;
                chunk_data[idx + 3] = (color.a() * 255.0) as u8;
            }
        }
        
        pixels.push(((chunk_x, chunk_y), chunk_data));
    });
    
    // Update back buffer
    if let Some(texture) = &preview.back_buffer {
        for ((chunk_x, chunk_y), pixels) in chunk_pixels {
            let rect = [
                chunk_x * chunk_size,
                chunk_y * chunk_size,
                chunk_size.min(size - chunk_x * chunk_size),
                chunk_size.min(size - chunk_y * chunk_size),
            ];
            texture.set_partial(rect, egui::ColorImage::from_rgba_unmultiplied(
                [rect[2], rect[3]],
                &pixels,
            ));
        }
    }
    
    // Swap buffers if needed
    if !preview.chunk_updates.is_empty() {
        std::mem::swap(&mut preview.front_buffer, &mut preview.back_buffer);
        preview.needs_swap = true;
    }
    
    // Track performance metrics
    let generation_time = time.elapsed_seconds() - start_time;
    preview.perf_metrics.generation_times.push(generation_time);
    if preview.perf_metrics.generation_times.len() > 100 {
        preview.perf_metrics.generation_times.remove(0);
    }
    
    preview.perf_metrics.max_generation_time = preview.perf_metrics.max_generation_time.max(generation_time);
    preview.perf_metrics.avg_generation_time = preview.perf_metrics.generation_times.iter().sum::<f32>() 
        / preview.perf_metrics.generation_times.len() as f32;
    
    // Update cache hit rate
    let total_cache_ops = preview.cached_data.cache_hits + preview.cached_data.cache_misses;
    if total_cache_ops > 0 {
        preview.perf_metrics.cache_hit_rate = preview.cached_data.cache_hits as f32 / total_cache_ops as f32;
    }
    
    // Update chunks per second
    let processed_chunks = preview.chunk_updates.len();
    preview.perf_metrics.chunks_per_second = processed_chunks as f32 / generation_time;
    
    // Reset chunk updates when all chunks are processed
    if preview.current_chunk >= size * size / (chunk_size * chunk_size) {
        preview.chunk_updates.clear();
        preview.current_chunk = 0;
        preview.last_update = time.elapsed_seconds();
        
        // Clear cached data if visualization mode changed
        if !debug_settings.show_normals { preview.cached_data.normals = None; }
        if !debug_settings.show_slope_overlay { preview.cached_data.slopes = None; }
        if !debug_settings.show_curvature { preview.cached_data.curvature = None; }
        if !debug_settings.show_flow_paths { preview.cached_data.flow = None; }
        if !debug_settings.show_material_weights { preview.cached_data.material_weights = None; }
        if !debug_settings.show_biome_boundaries { preview.cached_data.biome_boundaries = None; }
        if !debug_settings.show_erosion_impact { preview.cached_data.erosion_impact = None; }
    }
}

fn get_visualization_color(
    x: usize,
    y: usize,
    heightmap: &Heightmap,
    debug_settings: &TerrainDebugSettings,
    preview: &TerrainPreviewTexture,
) -> Color {
    let height = heightmap.get_height(x, y);
    let mut final_color = Color::BLACK;
    let mut blend_weight = 0.0;
    let idx = y * heightmap.width() + x;

    // Base height color
    if debug_settings.show_height_colormap {
        let height_color = get_height_color(height, &debug_settings.height_gradient);
        final_color = blend_colors(final_color, height_color, preview.visualization_blend);
        blend_weight += preview.visualization_blend;
    }

    // Normal map visualization
    if debug_settings.show_normals && preview.cached_data.normals.is_some() {
        let normal = preview.cached_data.normals.as_ref().unwrap()[idx];
        let normal_color = normal_to_color(normal * preview.normal_map_scale);
        final_color = blend_colors(final_color, normal_color, preview.visualization_blend);
        blend_weight += preview.visualization_blend;
    }

    // Slope overlay
    if debug_settings.show_slope_overlay && preview.cached_data.slopes.is_some() {
        let slope = preview.cached_data.slopes.as_ref().unwrap()[idx];
        let slope_color = slope_to_color(slope * preview.slope_sensitivity, &debug_settings.slope_gradient);
        final_color = blend_colors(final_color, slope_color, preview.visualization_blend);
        blend_weight += preview.visualization_blend;
    }

    // Curvature visualization
    if debug_settings.show_curvature && preview.cached_data.curvature.is_some() {
        let curvature = preview.cached_data.curvature.as_ref().unwrap()[idx];
        let curvature_color = curvature_to_color(curvature, &debug_settings.curvature_gradient);
        final_color = blend_colors(final_color, curvature_color, preview.visualization_blend);
        blend_weight += preview.visualization_blend;
    }

    // Flow field visualization
    if debug_settings.show_flow_paths && preview.cached_data.flow.is_some() {
        let flow = preview.cached_data.flow.as_ref().unwrap()[idx];
        let flow_color = flow_to_color(flow, &debug_settings.flow_gradient);
        final_color = blend_colors(final_color, flow_color, preview.visualization_blend);
        blend_weight += preview.visualization_blend;
    }

    // Material weights
    if debug_settings.show_material_weights && preview.cached_data.material_weights.is_some() {
        let weights = &preview.cached_data.material_weights.as_ref().unwrap()[idx];
        let material_color = material_weights_to_color(weights);
        final_color = blend_colors(final_color, material_color, preview.visualization_blend);
        blend_weight += preview.visualization_blend;
    }

    // Biome boundaries
    if debug_settings.show_biome_boundaries && preview.cached_data.biome_boundaries.is_some() {
        let is_boundary = preview.cached_data.biome_boundaries.as_ref().unwrap()[idx];
        if is_boundary {
            let boundary_color = Color::WHITE;
            final_color = blend_colors(final_color, boundary_color, preview.visualization_blend);
            blend_weight += preview.visualization_blend;
        }
    }

    // Erosion impact
    if debug_settings.show_erosion_impact && preview.cached_data.erosion_impact.is_some() {
        let impact = preview.cached_data.erosion_impact.as_ref().unwrap()[idx];
        let erosion_color = erosion_to_color(impact);
        final_color = blend_colors(final_color, erosion_color, preview.visualization_blend);
        blend_weight += preview.visualization_blend;
    }

    // Ambient occlusion
    if debug_settings.show_ambient_occlusion && preview.cached_data.ambient_occlusion.is_some() {
        let ao = preview.cached_data.ambient_occlusion.as_ref().unwrap()[idx];
        let ao_color = Color::rgb(ao, ao, ao);
        final_color = blend_colors(final_color, ao_color, preview.visualization_blend);
        blend_weight += preview.visualization_blend;
    }

    // Roughness
    if debug_settings.show_roughness && preview.cached_data.roughness.is_some() {
        let roughness = preview.cached_data.roughness.as_ref().unwrap()[idx];
        let roughness_color = Color::rgb(roughness, roughness, roughness);
        final_color = blend_colors(final_color, roughness_color, preview.visualization_blend);
        blend_weight += preview.visualization_blend;
    }

    // Normalize final color if multiple layers were blended
    if blend_weight > 1.0 {
        final_color = Color::rgb(
            final_color.r() / blend_weight,
            final_color.g() / blend_weight,
            final_color.b() / blend_weight,
        );
    }

    final_color
}

fn blend_colors(base: Color, overlay: Color, alpha: f32) -> Color {
    Color::rgb(
        base.r() * (1.0 - alpha) + overlay.r() * alpha,
        base.g() * (1.0 - alpha) + overlay.g() * alpha,
        base.b() * (1.0 - alpha) + overlay.b() * alpha,
    )
}

fn material_weights_to_color(weights: &MaterialWeights) -> Color {
    // Assuming MaterialWeights has fields like rock, sand, grass, etc.
    Color::rgb(
        weights.rock,
        weights.grass,
        weights.sand,
    )
}

fn erosion_impact_to_color(impact: f32) -> Color {
    let normalized = impact.clamp(0.0, 1.0);
    Color::rgb(
        normalized,
        0.0,
        1.0 - normalized,
    )
}

fn calculate_normals_parallel(heightmap: &Heightmap) -> Vec<Vec3> {
    let size = heightmap.dimensions.x as usize;
    let mut normals = vec![Vec3::ZERO; size * size];
    
    normals.par_chunks_mut(size).enumerate().for_each(|(y, row)| {
        if y > 0 && y < size - 1 {
            for x in 1..size-1 {
                let h = heightmap.get_height(x, y);
                let h_right = heightmap.get_height(x + 1, y);
                let h_up = heightmap.get_height(x, y + 1);
                
                let dx = Vec3::new(1.0, h_right - h, 0.0).normalize();
                let dy = Vec3::new(0.0, h_up - h, 1.0).normalize();
                row[x] = dx.cross(dy).normalize();
            }
        }
    });
    
    normals
}

fn calculate_slopes_parallel(heightmap: &Heightmap) -> Vec<f32> {
    let size = heightmap.dimensions.x as usize;
    let mut slopes = vec![0.0; size * size];
    
    slopes.par_chunks_mut(size).enumerate().for_each(|(y, row)| {
        if y > 0 && y < size - 1 {
            for x in 1..size-1 {
                let h = heightmap.get_height(x, y);
                let h_right = heightmap.get_height(x + 1, y);
                let h_up = heightmap.get_height(x, y + 1);
                
                let dx = (h_right - h).abs();
                let dy = (h_up - h).abs();
                row[x] = (dx * dx + dy * dy).sqrt().atan() * (180.0 / std::f32::consts::PI);
            }
        }
    });
    
    slopes
}

fn calculate_curvature(heightmap: &Heightmap) -> Vec<f32> {
    let size = heightmap.dimensions.x as usize;
    let mut curvature = vec![0.0; size * size];
    
    for y in 1..size-1 {
        for x in 1..size-1 {
            let h = heightmap.get_height(x, y);
            let h_left = heightmap.get_height(x - 1, y);
            let h_right = heightmap.get_height(x + 1, y);
            let h_up = heightmap.get_height(x, y + 1);
            let h_down = heightmap.get_height(x, y - 1);
            
            let dx = h_right - 2.0 * h + h_left;
            let dy = h_up - 2.0 * h + h_down;
            curvature[y * size + x] = dx + dy;
        }
    }
    
    curvature
}

fn calculate_flow_field(heightmap: &Heightmap) -> Vec<Vec2> {
    let size = heightmap.dimensions.x as usize;
    let mut flow = vec![Vec2::ZERO; size * size];
    
    for y in 1..size-1 {
        for x in 1..size-1 {
            let h = heightmap.get_height(x, y);
            let h_right = heightmap.get_height(x + 1, y);
            let h_up = heightmap.get_height(x, y + 1);
            
            flow[y * size + x] = Vec2::new(
                h_right - h,
                h_up - h,
            ).normalize_or_zero();
        }
    }
    
    flow
}

fn curvature_to_color(curvature: f32, gradient: &CurvatureGradient) -> Color {
    let t = ((curvature - gradient.min_curvature) / (gradient.max_curvature - gradient.min_curvature))
        .clamp(0.0, 1.0);
    
    if t < 0.5 {
        gradient.concave_color.lerp(gradient.flat_color, t * 2.0)
    } else {
        gradient.flat_color.lerp(gradient.convex_color, (t - 0.5) * 2.0)
    }
}

fn flow_to_color(flow: Vec2, gradient: &FlowGradient) -> Color {
    let magnitude = flow.length();
    let t = ((magnitude - gradient.min_flow) / (gradient.max_flow - gradient.min_flow))
        .clamp(0.0, 1.0);
    gradient.min_color.lerp(gradient.max_color, t)
}

fn get_height_color(height: f32, gradient: &HeightGradient) -> Color {
    let normalized_height = (height + 1.0) * 0.5;
    
    match gradient.interpolation_mode {
        GradientMode::Step => {
            // Find the appropriate color stop
            for i in 0..gradient.stops.len() - 1 {
                if normalized_height <= gradient.stops[i + 1].0 {
                    return gradient.stops[i].1;
                }
            }
            gradient.stops.last().unwrap().1
        },
        GradientMode::Linear => {
            // Find the stops to interpolate between
            for i in 0..gradient.stops.len() - 1 {
                if normalized_height <= gradient.stops[i + 1].0 {
                    let t = (normalized_height - gradient.stops[i].0) / 
                           (gradient.stops[i + 1].0 - gradient.stops[i].0);
                    return gradient.stops[i].1.lerp(gradient.stops[i + 1].1, t);
                }
            }
            gradient.stops.last().unwrap().1
        },
        GradientMode::Smooth => {
            // Similar to linear but with smoothstep interpolation
            for i in 0..gradient.stops.len() - 1 {
                if normalized_height <= gradient.stops[i + 1].0 {
                    let t = (normalized_height - gradient.stops[i].0) / 
                           (gradient.stops[i + 1].0 - gradient.stops[i].0);
                    let smoothed = t * t * (3.0 - 2.0 * t);
                    return gradient.stops[i].1.lerp(gradient.stops[i + 1].1, smoothed);
                }
            }
            gradient.stops.last().unwrap().1
        },
    }
}

fn calculate_normals(heightmap: &Heightmap) -> Vec<Vec3> {
    let size = heightmap.dimensions.x as usize;
    let mut normals = vec![Vec3::ZERO; size * size];
    
    for y in 1..size - 1 {
        for x in 1..size - 1 {
            let h = heightmap.get_height(x, y);
            let h_right = heightmap.get_height(x + 1, y);
            let h_up = heightmap.get_height(x, y + 1);
            
            let dx = Vec3::new(1.0, h_right - h, 0.0).normalize();
            let dy = Vec3::new(0.0, h_up - h, 1.0).normalize();
            normals[y * size + x] = dx.cross(dy).normalize();
        }
    }
    
    normals
}

fn normal_to_color(normal: Vec3) -> Color {
    Color::rgb(
        (normal.x + 1.0) * 0.5,
        (normal.y + 1.0) * 0.5,
        (normal.z + 1.0) * 0.5,
    )
}

fn calculate_slopes(heightmap: &Heightmap) -> Vec<f32> {
    let size = heightmap.dimensions.x as usize;
    let mut slopes = vec![0.0; size * size];
    
    for y in 1..size - 1 {
        for x in 1..size - 1 {
            let h = heightmap.get_height(x, y);
            let h_right = heightmap.get_height(x + 1, y);
            let h_up = heightmap.get_height(x, y + 1);
            
            let dx = (h_right - h).abs();
            let dy = (h_up - h).abs();
            slopes[y * size + x] = (dx * dx + dy * dy).sqrt().atan() * (180.0 / std::f32::consts::PI);
        }
    }
    
    slopes
}

fn slope_to_color(slope: f32, gradient: &SlopeGradient) -> Color {
    let t = ((slope - gradient.min_angle) / (gradient.max_angle - gradient.min_angle))
        .clamp(0.0, 1.0);
    gradient.min_color.lerp(gradient.max_color, t)
}

fn calculate_erosion_impact(heightmap: &Heightmap, x: usize, y: usize) -> f32 {
    let size = heightmap.dimensions.x as usize;
    if x == 0 || x >= size - 1 || y == 0 || y >= size - 1 {
        return 0.0;
    }
    
    let h = heightmap.get_height(x, y);
    let h_neighbors = [
        heightmap.get_height(x - 1, y),
        heightmap.get_height(x + 1, y),
        heightmap.get_height(x, y - 1),
        heightmap.get_height(x, y + 1),
    ];
    
    h_neighbors.iter().map(|&hn| (h - hn).abs()).sum::<f32>() / 4.0
}

fn erosion_to_color(impact: f32) -> Color {
    let intensity = (impact * 5.0).clamp(0.0, 1.0);
    Color::rgb(intensity, 0.0, 0.0)
}

/// System to render the preview window
pub fn render_preview_window(
    mut egui_context: ResMut<EguiContext>,
    preview: Res<TerrainPreviewTexture>,
    mut debug_settings: ResMut<TerrainDebugSettings>,
) {
    egui::Window::new("Terrain Preview")
        .default_size([400.0, 800.0])
        .show(egui_context.ctx_mut(), |ui| {
            // Preview texture display
            if let Some(texture) = &preview.front_buffer {
                let size = debug_settings.preview_size as f32;
                let aspect = size / size;
                let available_width = ui.available_width().min(400.0);
                let image_size = egui::vec2(available_width, available_width / aspect);
                
                ui.image(texture.id(), image_size);
            }

            ui.separator();

            // Visualization controls
            ui.collapsing("Visualization Settings", |ui| {
                ui.checkbox(&mut debug_settings.show_height_colormap, "Height Colormap");
                ui.checkbox(&mut debug_settings.show_normals, "Normal Map");
                ui.checkbox(&mut debug_settings.show_slope_overlay, "Slope Overlay");
                ui.checkbox(&mut debug_settings.show_curvature, "Curvature");
                ui.checkbox(&mut debug_settings.show_flow_paths, "Flow Paths");
                ui.checkbox(&mut debug_settings.show_material_weights, "Material Weights");
                ui.checkbox(&mut debug_settings.show_biome_boundaries, "Biome Boundaries");
                ui.checkbox(&mut debug_settings.show_erosion_impact, "Erosion Impact");
                ui.checkbox(&mut debug_settings.show_ambient_occlusion, "Ambient Occlusion");
                ui.checkbox(&mut debug_settings.show_roughness, "Roughness");

                ui.add(egui::Slider::new(&mut preview.visualization_blend, 0.0..=1.0)
                    .text("Blend Factor"));
                ui.add(egui::Slider::new(&mut preview.normal_map_scale, 0.1..=2.0)
                    .text("Normal Scale"));
                ui.add(egui::Slider::new(&mut preview.slope_sensitivity, 0.1..=2.0)
                    .text("Slope Sensitivity"));
            });

            // Height gradient settings
            ui.collapsing("Height Gradient", |ui| {
                ui.horizontal(|ui| {
                    ui.label("Interpolation:");
                    ui.radio_value(&mut debug_settings.height_gradient.interpolation, 
                        GradientInterpolation::Linear, "Linear");
                    ui.radio_value(&mut debug_settings.height_gradient.interpolation,
                        GradientInterpolation::Smooth, "Smooth");
                });

                for (i, stop) in debug_settings.height_gradient.stops.iter_mut().enumerate() {
                    ui.horizontal(|ui| {
                        ui.label(format!("Stop {}", i));
                        ui.color_edit_button_rgb(&mut stop.color);
                        ui.add(egui::Slider::new(&mut stop.position, 0.0..=1.0)
                            .text("Position"));
                    });
                }
            });

            // Slope gradient settings
            ui.collapsing("Slope Settings", |ui| {
                ui.add(egui::Slider::new(&mut debug_settings.slope_gradient.min_angle, 0.0..=90.0)
                    .text("Min Angle"));
                ui.add(egui::Slider::new(&mut debug_settings.slope_gradient.max_angle, 0.0..=90.0)
                    .text("Max Angle"));
                ui.color_edit_button_rgb(&mut debug_settings.slope_gradient.min_color)
                    .labelled_by(ui.label("Min Color").id);
                ui.color_edit_button_rgb(&mut debug_settings.slope_gradient.max_color)
                    .labelled_by(ui.label("Max Color").id);
            });

            // Performance metrics
            ui.collapsing("Performance Metrics", |ui| {
                ui.label(format!("Generation Time: {:.2} ms", 
                    preview.perf_metrics.avg_generation_time * 1000.0));
                ui.label(format!("Max Generation Time: {:.2} ms",
                    preview.perf_metrics.max_generation_time * 1000.0));
                ui.label(format!("Cache Hit Rate: {:.1}%",
                    preview.perf_metrics.cache_hit_rate * 100.0));
                ui.label(format!("Chunks/Second: {:.0}",
                    preview.perf_metrics.chunks_per_second));
                ui.label(format!("Memory Usage: {:.1} MB",
                    preview.perf_metrics.memory_usage as f32 / (1024.0 * 1024.0)));
                
                // Performance graph
                let plot = egui::plot::Plot::new("perf_plot")
                    .height(100.0)
                    .show_x(false)
                    .include_y(0.0)
                    .view_aspect(2.0);
                
                plot.show(ui, |plot_ui| {
                    let points: Vec<[f64; 2]> = preview.perf_metrics.generation_times
                        .iter()
                        .enumerate()
                        .map(|(i, &t)| [i as f64, t as f64 * 1000.0])
                        .collect();
                    
                    plot_ui.line(egui::plot::Line::new(points)
                        .name("Generation Time (ms)")
                        .color(egui::Color32::from_rgb(100, 200, 100)));
                });
            });

            // Export/Import settings
            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("Export Settings").clicked() {
                    if let Err(e) = export_debug_settings(&debug_settings) {
                        ui.label(format!("Export failed: {}", e));
                    }
                }
                if ui.button("Import Settings").clicked() {
                    if let Err(e) = import_debug_settings(&mut debug_settings) {
                        ui.label(format!("Import failed: {}", e));
                    }
                }
            });
        });
}

/// Update cached data based on current visualization modes
fn update_cached_data(
    heightmap: &Heightmap,
    debug_settings: &TerrainDebugSettings,
    cached_data: &mut PreviewCache,
) {
    let start = std::time::Instant::now();
    let mut cache_hits = 0;
    let mut cache_misses = 0;

    // Calculate normals if needed
    if debug_settings.show_normals && cached_data.normals.is_none() {
        cached_data.normals = Some(Arc::new(calculate_normals_parallel(heightmap)));
        cache_misses += 1;
    } else if debug_settings.show_normals {
        cache_hits += 1;
    }

    // Calculate slopes if needed
    if debug_settings.show_slope_overlay && cached_data.slopes.is_none() {
        cached_data.slopes = Some(Arc::new(calculate_slopes_parallel(heightmap)));
        cache_misses += 1;
    } else if debug_settings.show_slope_overlay {
        cache_hits += 1;
    }

    // Calculate curvature if needed
    if debug_settings.show_curvature && cached_data.curvature.is_none() {
        cached_data.curvature = Some(Arc::new(calculate_curvature_parallel(heightmap)));
        cache_misses += 1;
    } else if debug_settings.show_curvature {
        cache_hits += 1;
    }

    // Calculate flow field if needed
    if debug_settings.show_flow_paths && cached_data.flow.is_none() {
        cached_data.flow = Some(Arc::new(calculate_flow_field_parallel(heightmap)));
        cache_misses += 1;
    } else if debug_settings.show_flow_paths {
        cache_hits += 1;
    }

    // Calculate material weights if needed
    if debug_settings.show_material_weights && cached_data.material_weights.is_none() {
        cached_data.material_weights = Some(Arc::new(calculate_material_weights_parallel(heightmap)));
        cache_misses += 1;
    } else if debug_settings.show_material_weights {
        cache_hits += 1;
    }

    // Calculate biome boundaries if needed
    if debug_settings.show_biome_boundaries && cached_data.biome_boundaries.is_none() {
        cached_data.biome_boundaries = Some(Arc::new(detect_biome_boundaries_parallel(heightmap)));
        cache_misses += 1;
    } else if debug_settings.show_biome_boundaries {
        cache_hits += 1;
    }

    // Calculate erosion impact if needed
    if debug_settings.show_erosion_impact && cached_data.erosion_impact.is_none() {
        cached_data.erosion_impact = Some(Arc::new(calculate_erosion_impact_parallel(heightmap)));
        cache_misses += 1;
    } else if debug_settings.show_erosion_impact {
        cache_hits += 1;
    }

    // Calculate tangent space if needed for normal mapping
    if (debug_settings.show_normals || debug_settings.show_material_weights) && 
       (cached_data.tangents.is_none() || cached_data.bitangents.is_none()) {
        if let Some(normals) = &cached_data.normals {
            let (tangents, bitangents) = calculate_tangent_space(heightmap, normals);
            cached_data.tangents = Some(Arc::new(tangents));
            cached_data.bitangents = Some(Arc::new(bitangents));
            cache_misses += 1;
        }
    } else if debug_settings.show_normals || debug_settings.show_material_weights {
        cache_hits += 1;
    }

    // Calculate ambient occlusion if needed
    if debug_settings.show_ambient_occlusion && cached_data.ambient_occlusion.is_none() {
        if let Some(normals) = &cached_data.normals {
            cached_data.ambient_occlusion = Some(Arc::new(calculate_ambient_occlusion(heightmap, normals)));
            cache_misses += 1;
        }
    } else if debug_settings.show_ambient_occlusion {
        cache_hits += 1;
    }

    // Calculate roughness if needed
    if debug_settings.show_roughness && cached_data.roughness.is_none() {
        if let Some(normals) = &cached_data.normals {
            let roughness = calculate_roughness(heightmap, normals);
            cached_data.roughness = Some(Arc::new(roughness));
            cache_misses += 1;
        }
    } else if debug_settings.show_roughness {
        cache_hits += 1;
    }

    // Update cache statistics
    cached_data.cache_hits = cache_hits;
    cached_data.cache_misses = cache_misses;
}

fn calculate_material_weights_parallel(heightmap: &Heightmap) -> Vec<MaterialWeights> {
    let size = (heightmap.dimensions.x * heightmap.dimensions.y) as usize;
    let mut weights = vec![MaterialWeights::default(); size];

    weights.par_chunks_mut(heightmap.dimensions.x as usize).enumerate().for_each(|(y, row)| {
        for (x, weight) in row.iter_mut().enumerate() {
            let height = heightmap.get_height(x, y);
            let slope = calculate_slope(heightmap, x, y);
            
            // Calculate material weights based on height and slope
            weight.rock = (slope * 2.0).min(1.0); // More rock on steep slopes
            weight.grass = ((1.0 - slope) * (height + 0.3)).max(0.0).min(1.0); // Grass on gentle slopes
            weight.sand = ((0.3 - height) * 2.0).max(0.0).min(1.0); // Sand in lower areas
        }
    });

    weights
}

fn detect_biome_boundaries_parallel(heightmap: &Heightmap) -> Vec<bool> {
    let size = (heightmap.dimensions.x * heightmap.dimensions.y) as usize;
    let mut boundaries = vec![false; size];

    boundaries.par_chunks_mut(heightmap.dimensions.x as usize).enumerate().for_each(|(y, row)| {
        for (x, is_boundary) in row.iter_mut().enumerate() {
            if x == 0 || y == 0 || x >= heightmap.dimensions.x as usize - 1 || y >= heightmap.dimensions.y as usize - 1 {
                continue;
            }

            let current_height = heightmap.get_height(x, y);
            let current_biome = BiomeType::from_height(current_height);

            // Check neighboring cells for biome transitions
            let neighbors = [
                (x - 1, y), (x + 1, y),
                (x, y - 1), (x, y + 1),
            ];

            for (nx, ny) in neighbors {
                let neighbor_height = heightmap.get_height(nx, ny);
                let neighbor_biome = BiomeType::from_height(neighbor_height);

                if current_biome != neighbor_biome {
                    *is_boundary = true;
                    break;
                }
            }
        }
    });

    boundaries
}

fn calculate_erosion_impact_parallel(heightmap: &Heightmap) -> Vec<f32> {
    let size = (heightmap.dimensions.x * heightmap.dimensions.y) as usize;
    let mut impact = vec![0.0; size];

    impact.par_chunks_mut(heightmap.dimensions.x as usize).enumerate().for_each(|(y, row)| {
        for (x, impact_value) in row.iter_mut().enumerate() {
            if x == 0 || y == 0 || x >= heightmap.dimensions.x as usize - 1 || y >= heightmap.dimensions.y as usize - 1 {
                continue;
            }

            let current_height = heightmap.get_height(x, y);
            let slope = calculate_slope(heightmap, x, y);
            let flow_accumulation = calculate_flow_accumulation(heightmap, x, y);

            // Calculate erosion impact based on slope and flow accumulation
            *impact_value = (slope * 0.6 + flow_accumulation * 0.4).min(1.0);
        }
    });

    impact
}

fn calculate_flow_accumulation(heightmap: &Heightmap, x: usize, y: usize) -> f32 {
    let mut accumulation = 0.0;
    let current_height = heightmap.get_height(x, y);

    // Check 8 neighboring cells
    for dy in -1..=1 {
        for dx in -1..=1 {
            if dx == 0 && dy == 0 {
                continue;
            }

            let nx = x as i32 + dx;
            let ny = y as i32 + dy;

            if nx < 0 || ny < 0 || nx >= heightmap.dimensions.x as i32 || ny >= heightmap.dimensions.y as i32 {
                continue;
            }

            let neighbor_height = heightmap.get_height(nx as usize, ny as usize);
            if neighbor_height > current_height {
                // Add flow contribution from higher neighbors
                let height_diff = neighbor_height - current_height;
                accumulation += height_diff;
            }
        }
    }

    // Normalize accumulation
    accumulation / 8.0
}

fn calculate_curvature_parallel(heightmap: &Heightmap) -> Vec<f32> {
    let size = (heightmap.dimensions.x * heightmap.dimensions.y) as usize;
    let mut curvature = vec![0.0; size];

    curvature.par_chunks_mut(heightmap.dimensions.x as usize).enumerate().for_each(|(y, row)| {
        for (x, curve) in row.iter_mut().enumerate() {
            if x == 0 || y == 0 || x >= heightmap.dimensions.x as usize - 1 || y >= heightmap.dimensions.y as usize - 1 {
                continue;
            }

            *curve = calculate_curvature_at_point(heightmap, x, y);
        }
    });

    curvature
}

fn calculate_curvature_at_point(heightmap: &Heightmap, x: usize, y: usize) -> f32 {
    let h = heightmap.get_height(x, y);
    let h_left = heightmap.get_height(x - 1, y);
    let h_right = heightmap.get_height(x + 1, y);
    let h_up = heightmap.get_height(x, y - 1);
    let h_down = heightmap.get_height(x, y + 1);

    // Second derivatives
    let d2x = h_left - 2.0 * h + h_right;
    let d2y = h_up - 2.0 * h + h_down;

    // Mean curvature (simplified)
    (d2x + d2y) * 0.5
}

fn calculate_flow_field_parallel(heightmap: &Heightmap) -> Vec<Vec2> {
    let size = (heightmap.dimensions.x * heightmap.dimensions.y) as usize;
    let mut flow = vec![Vec2::ZERO; size];

    flow.par_chunks_mut(heightmap.dimensions.x as usize).enumerate().for_each(|(y, row)| {
        for (x, flow_vec) in row.iter_mut().enumerate() {
            if x == 0 || y == 0 || x >= heightmap.dimensions.x as usize - 1 || y >= heightmap.dimensions.y as usize - 1 {
                continue;
            }

            let h = heightmap.get_height(x, y);
            let h_left = heightmap.get_height(x - 1, y);
            let h_right = heightmap.get_height(x + 1, y);
            let h_up = heightmap.get_height(x, y - 1);
            let h_down = heightmap.get_height(x, y + 1);

            // Calculate flow direction based on steepest descent
            let dx = h_right - h_left;
            let dy = h_down - h_up;
            *flow_vec = Vec2::new(-dx, -dy).normalize_or_zero();
        }
    });

    flow
}

/// Calculate tangent and bitangent vectors for normal mapping
fn calculate_tangent_space(
    heightmap: &Heightmap,
    normals: &[Vec3],
) -> (Vec<Vec3>, Vec<Vec3>) {
    let size = heightmap.dimensions.x as usize;
    let mut tangents = vec![Vec3::ZERO; size * size];
    let mut bitangents = vec![Vec3::ZERO; size * size];

    for y in 1..size - 1 {
        for x in 1..size - 1 {
            let idx = y * size + x;
            let du = heightmap.get_height(x + 1, y) - heightmap.get_height(x - 1, y);
            let dv = heightmap.get_height(x, y + 1) - heightmap.get_height(x, y - 1);
            
            let tangent = Vec3::new(2.0, du, 0.0).normalize();
            let bitangent = Vec3::new(0.0, dv, 2.0).normalize();
            
            tangents[idx] = tangent;
            bitangents[idx] = bitangent;
        }
    }

    (tangents, bitangents)
}

/// Calculate ambient occlusion for terrain surface
fn calculate_ambient_occlusion(
    heightmap: &Heightmap,
    normals: &[Vec3],
) -> Vec<f32> {
    let size = heightmap.dimensions.x as usize;
    let mut ao = vec![1.0; size * size];
    let sample_radius = 3;
    let samples = 8;
    
    for y in sample_radius..size - sample_radius {
        for x in sample_radius..size - sample_radius {
            let idx = y * size + x;
            let center_height = heightmap.get_height(x, y);
            let normal = normals[idx];
            
            let mut occlusion = 0.0;
            for i in 0..samples {
                let angle = (i as f32 / samples as f32) * std::f32::consts::TAU;
                let mut ray_occlusion = 0.0;
                
                for r in 1..=sample_radius {
                    let sample_x = (x as f32 + angle.cos() * r as f32) as usize;
                    let sample_y = (y as f32 + angle.sin() * r as f32) as usize;
                    let sample_height = heightmap.get_height(sample_x, sample_y);
                    
                    let height_diff = sample_height - center_height;
                    if height_diff > 0.0 {
                        ray_occlusion += height_diff / (r as f32);
                    }
                }
                
                occlusion += ray_occlusion;
            }
            
            ao[idx] = (-occlusion / samples as f32).exp();
        }
    }
    
    ao
}

fn calculate_roughness(heightmap: &Heightmap, normals: &[Vec3]) -> Vec<f32> {
    let width = heightmap.width();
    let height = heightmap.height();
    let mut roughness = vec![0.0; width * height];

    // Calculate roughness as variance in normal directions
    for y in 1..height - 1 {
        for x in 1..width - 1 {
            let idx = y * width + x;
            let center = normals[idx];
            
            // Sample neighboring normals
            let neighbors = [
                normals[idx - width - 1], normals[idx - width], normals[idx - width + 1],
                normals[idx - 1],                               normals[idx + 1],
                normals[idx + width - 1], normals[idx + width], normals[idx + width + 1],
            ];

            // Calculate variance
            let mut variance = 0.0;
            for n in neighbors.iter() {
                let diff = 1.0 - center.dot(*n);
                variance += diff * diff;
            }
            variance /= 8.0;

            roughness[idx] = variance.sqrt();
        }
    }

    roughness
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

    #[test]
    fn test_height_gradient_interpolation() {
        let gradient = HeightGradient {
            stops: vec![
                (0.0, Color::RED),
                (1.0, Color::BLUE),
            ],
            interpolation_mode: GradientMode::Linear,
        };
        
        let mid_color = get_height_color(0.0, &gradient); // height 0.0 maps to 0.5 normalized
        assert!((mid_color.r() - 0.5).abs() < 0.01);
        assert!((mid_color.b() - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_slope_calculation() {
        let mut heightmap = Heightmap::new(UVec2::new(3, 3), Vec2::new(2.0, 2.0));
        heightmap.set_height(0, 0, 0.0).unwrap();
        heightmap.set_height(1, 0, 1.0).unwrap();
        heightmap.set_height(1, 1, 0.0).unwrap();
        
        let slopes = calculate_slopes(&heightmap);
        assert!(slopes[4] > 0.0); // Center point should have non-zero slope
    }

    #[test]
    fn test_performance_metrics() {
        let mut preview = TerrainPreviewTexture::default();
        preview.perf_metrics.generation_times.extend_from_slice(&[0.1, 0.2, 0.3]);
        assert_eq!(preview.perf_metrics.generation_times.len(), 3);
        assert!((preview.perf_metrics.avg_generation_time - 0.2).abs() < f32::EPSILON);
        assert!((preview.perf_metrics.max_generation_time - 0.3).abs() < f32::EPSILON);
    }

    #[test]
    fn test_tangent_space_calculation() {
        let dimensions = UVec2::new(4, 4);
        let mut heightmap = Heightmap::new(dimensions);
        
        // Set up a simple slope
        for y in 0..4 {
            for x in 0..4 {
                heightmap.set_height(x, y, (x + y) as f32 * 0.1);
            }
        }
        
        let normals = calculate_normals_parallel(&heightmap);
        let (tangents, bitangents) = calculate_tangent_space(&heightmap, &normals);
        
        // Check that tangents and bitangents are perpendicular
        let idx = 5; // Check middle point
        assert!((tangents[idx].dot(bitangents[idx])).abs() < 0.01);
    }

    #[test]
    fn test_ambient_occlusion() {
        let dimensions = UVec2::new(10, 10);
        let mut heightmap = Heightmap::new(dimensions);
        
        // Create a valley
        for y in 0..10 {
            for x in 0..10 {
                let dist = ((x as f32 - 5.0).powi(2) + (y as f32 - 5.0).powi(2)).sqrt();
                heightmap.set_height(x, y, dist * 0.1);
            }
        }
        
        let normals = calculate_normals_parallel(&heightmap);
        let ao = calculate_ambient_occlusion(&heightmap, &normals);
        
        // Valley bottom should have more occlusion (lower value)
        assert!(ao[5 * 10 + 5] < ao[0]);
    }

    #[test]
    fn test_preview_texture_initialization() {
        let preview = TerrainPreviewTexture::default();
        assert!(preview.front_buffer.is_none());
        assert!(preview.back_buffer.is_none());
        assert!(!preview.needs_swap);
        assert_eq!(preview.last_update, 0.0);
        assert_eq!(preview.visualization_blend, 0.5);
        assert_eq!(preview.normal_map_scale, 1.0);
        assert_eq!(preview.slope_sensitivity, 1.0);
    }

    #[test]
    fn test_cached_data_management() {
        let mut cache = PreviewCache::default();
        assert_eq!(cache.cache_hits, 0);
        assert_eq!(cache.cache_misses, 0);
        assert!(cache.normals.is_none());
        assert!(cache.slopes.is_none());
        assert!(cache.curvature.is_none());
        assert!(cache.flow.is_none());
        assert!(cache.material_weights.is_none());
        assert!(cache.biome_boundaries.is_none());
        assert!(cache.erosion_impact.is_none());
        assert!(cache.tangents.is_none());
        assert!(cache.bitangents.is_none());
        assert!(cache.ambient_occlusion.is_none());
        assert!(cache.roughness.is_none());
    }

    #[test]
    fn test_visualization_color_blending() {
        let base = Color::rgb(0.5, 0.5, 0.5);
        let overlay = Color::rgb(1.0, 0.0, 0.0);
        
        // Test 50% blend
        let blended = blend_colors(base, overlay, 0.5);
        assert!((blended.r() - 0.75).abs() < 0.001);
        assert!((blended.g() - 0.25).abs() < 0.001);
        assert!((blended.b() - 0.25).abs() < 0.001);
        
        // Test full overlay
        let full = blend_colors(base, overlay, 1.0);
        assert_eq!(full, overlay);
        
        // Test no blend
        let none = blend_colors(base, overlay, 0.0);
        assert_eq!(none, base);
    }

    #[test]
    fn test_material_weights_visualization() {
        let weights = MaterialWeights {
            rock: 0.8,
            grass: 0.3,
            sand: 0.1,
        };
        let color = material_weights_to_color(&weights);
        assert!((color.r() - 0.8).abs() < 0.001); // Rock
        assert!((color.g() - 0.3).abs() < 0.001); // Grass
        assert!((color.b() - 0.1).abs() < 0.001); // Sand
    }

    #[test]
    fn test_normal_calculation() {
        let mut heightmap = Heightmap::new(UVec2::new(3, 3));
        // Create a 45-degree slope along x-axis
        heightmap.set_height(0, 1, 0.0).unwrap();
        heightmap.set_height(1, 1, 1.0).unwrap();
        heightmap.set_height(2, 1, 2.0).unwrap();
        
        let normals = calculate_normals_parallel(&heightmap);
        let center_normal = normals[4]; // Center point
        
        // For a 45-degree slope, normal should be (-1, √2, 0) normalized
        let expected_x = -1.0 / 2.0_f32.sqrt();
        let expected_y = 1.0 / 2.0_f32.sqrt();
        assert!((center_normal.x - expected_x).abs() < 0.01);
        assert!((center_normal.y - expected_y).abs() < 0.01);
        assert!(center_normal.z.abs() < 0.01);
    }

    #[test]
    fn test_roughness_calculation() {
        let mut heightmap = Heightmap::new(UVec2::new(4, 4));
        // Create alternating heights to generate high roughness
        for y in 0..4 {
            for x in 0..4 {
                let height = if (x + y) % 2 == 0 { 0.0 } else { 1.0 };
                heightmap.set_height(x, y, height).unwrap();
            }
        }
        
        let normals = calculate_normals_parallel(&heightmap);
        let roughness = calculate_roughness(&heightmap, &normals);
        
        // Center points should have high roughness due to varying normals
        let center_roughness = roughness[5];
        assert!(center_roughness > 0.5);
    }

    #[test]
    fn test_flow_field_calculation() {
        let mut heightmap = Heightmap::new(UVec2::new(3, 3));
        // Create a slope towards bottom-right
        for y in 0..3 {
            for x in 0..3 {
                heightmap.set_height(x, y, (x + y) as f32).unwrap();
            }
        }
        
        let flow = calculate_flow_field_parallel(&heightmap);
        let center_flow = flow[4]; // Center point
        
        // Flow should point down the slope (positive x and y)
        assert!(center_flow.x > 0.0);
        assert!(center_flow.y > 0.0);
        assert!((center_flow.length() - 1.0).abs() < 0.01); // Should be normalized
    }

    #[test]
    fn test_ambient_occlusion_calculation() {
        let mut heightmap = Heightmap::new(UVec2::new(7, 7));
        // Create a bowl shape
        for y in 0..7 {
            for x in 0..7 {
                let dx = x as f32 - 3.0;
                let dy = y as f32 - 3.0;
                let height = (dx * dx + dy * dy) * 0.1;
                heightmap.set_height(x, y, height).unwrap();
            }
        }
        
        let normals = calculate_normals_parallel(&heightmap);
        let ao = calculate_ambient_occlusion(&heightmap, &normals);
        
        // Center should have more occlusion (lower value) than edges
        let center_ao = ao[24]; // Center point
        let edge_ao = ao[0];    // Corner point
        assert!(center_ao < edge_ao);
    }

    #[test]
    fn test_erosion_impact_calculation() {
        let mut heightmap = Heightmap::new(UVec2::new(5, 5));
        // Create a sharp ridge
        for x in 0..5 {
            heightmap.set_height(x, 2, if x == 2 { 2.0 } else { 0.0 }).unwrap();
        }
        
        let impact = calculate_erosion_impact_parallel(&heightmap);
        
        // Ridge points should show high erosion impact
        let ridge_impact = impact[2 * 5 + 2]; // Center ridge point
        let flat_impact = impact[0 * 5 + 0];  // Corner flat point
        assert!(ridge_impact > flat_impact);
    }

    #[test]
    fn test_performance_metrics_tracking() {
        let mut metrics = PreviewPerformanceMetrics::default();
        
        // Add some generation times
        metrics.generation_times = vec![0.1, 0.2, 0.3];
        metrics.max_generation_time = 0.3;
        metrics.avg_generation_time = 0.2;
        metrics.cache_hit_rate = 0.75;
        metrics.chunks_per_second = 1000.0;
        
        assert_eq!(metrics.generation_times.len(), 3);
        assert!((metrics.max_generation_time - 0.3).abs() < f32::EPSILON);
        assert!((metrics.avg_generation_time - 0.2).abs() < f32::EPSILON);
        assert!((metrics.cache_hit_rate - 0.75).abs() < f32::EPSILON);
        assert!((metrics.chunks_per_second - 1000.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_biome_boundary_detection() {
        let mut heightmap = Heightmap::new(UVec2::new(4, 4));
        // Create a sharp height transition
        for y in 0..4 {
            for x in 0..4 {
                heightmap.set_height(x, y, if x < 2 { -0.5 } else { 0.5 }).unwrap();
            }
        }
        
        let boundaries = detect_biome_boundaries_parallel(&heightmap);
        
        // Check boundary points along the transition
        for y in 1..3 {
            assert!(boundaries[y * 4 + 1]); // Left side of boundary
            assert!(boundaries[y * 4 + 2]); // Right side of boundary
        }
    }
}