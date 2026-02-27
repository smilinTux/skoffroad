use bevy::prelude::*;
use bevy_kira_audio::{AudioPlugin, Audio, AudioControl};
use bevy::diagnostic::{Diagnostics, FrameTimeDiagnosticsPlugin, SystemInformationDiagnosticsPlugin};
use super::{WeatherState, WeatherEffects};

pub struct WeatherPlugin;

#[derive(Resource)]
struct WeatherSoundAssets {
    light_rain: Handle<AudioSource>,
    heavy_rain: Handle<AudioSource>,
    storm: Handle<AudioSource>,
    strong_wind: Handle<AudioSource>,
    blizzard: Handle<AudioSource>,
    wind: Handle<AudioSource>,
}

#[derive(Resource)]
struct WeatherSoundSettings {
    master_volume: f32,
    effect_volumes: std::collections::HashMap<String, f32>,
}

impl Default for WeatherSoundSettings {
    fn default() -> Self {
        let mut effect_volumes = std::collections::HashMap::new();
        effect_volumes.insert("light_rain".to_string(), 0.7);
        effect_volumes.insert("heavy_rain".to_string(), 0.8);
        effect_volumes.insert("storm".to_string(), 1.0);
        effect_volumes.insert("strong_wind".to_string(), 0.9);
        effect_volumes.insert("blizzard".to_string(), 0.85);
        effect_volumes.insert("wind".to_string(), 0.6);
        
        Self {
            master_volume: 1.0,
            effect_volumes,
        }
    }
}

#[derive(Resource)]
pub struct DebugState {
    pub show_weather_effects: bool,
    pub show_ground_effects: bool,
    pub show_effect_stats: bool,
    pub show_particle_paths: bool,
    pub show_wind_vectors: bool,
    pub show_sound_controls: bool,
    pub show_performance: bool,
    pub show_temperature_map: bool,
    pub show_precipitation_map: bool,
    pub show_wind_map: bool,
    pub show_memory_usage: bool,
    pub show_heatmap: bool,
    pub frame_times: Vec<f32>,
    pub effect_count_history: Vec<usize>,
    pub memory_usage_history: Vec<f32>,
    pub selected_effect_type: Option<String>,
    pub show_transitions: bool,
    pub show_profiler: bool,
    pub show_presets: bool,
    pub show_advanced_viz: bool,
    pub transition_history: Vec<(String, f32)>,
    pub profiler_data: Vec<ProfilerSnapshot>,
    pub selected_preset: Option<String>,
}

#[derive(Clone)]
struct ProfilerSnapshot {
    timestamp: f32,
    particle_count: usize,
    spawn_time_ms: f32,
    update_time_ms: f32,
    render_time_ms: f32,
}

#[derive(Resource)]
struct WeatherPresets {
    presets: std::collections::HashMap<String, WeatherState>,
}

impl Default for WeatherPresets {
    fn default() -> Self {
        let mut presets = std::collections::HashMap::new();
        
        // Sunny day
        presets.insert("Sunny".to_string(), WeatherState {
            temperature: 25.0,
            precipitation: 0.0,
            wind_speed: 2.0,
            fog_density: 0.0,
            ..Default::default()
        });
        
        // Heavy storm
        presets.insert("Storm".to_string(), WeatherState {
            temperature: 18.0,
            precipitation: 1.0,
            wind_speed: 15.0,
            lightning_frequency: 0.8,
            thunder_volume: 1.0,
            cloud_darkness: 0.9,
            ..Default::default()
        });
        
        // Winter blizzard
        presets.insert("Blizzard".to_string(), WeatherState {
            temperature: -10.0,
            precipitation: 0.8,
            wind_speed: 12.0,
            snow_density: 0.9,
            snow_drift_factor: 1.8,
            ..Default::default()
        });
        
        // Foggy morning
        presets.insert("Foggy".to_string(), WeatherState {
            temperature: 15.0,
            fog_density: 0.8,
            fog_height: 30.0,
            fog_falloff: 0.3,
            wind_speed: 1.0,
            ..Default::default()
        });

        Self { presets }
    }
}

impl Default for DebugState {
    fn default() -> Self {
        Self {
            show_weather_effects: false,
            show_ground_effects: false,
            show_effect_stats: false,
            show_particle_paths: false,
            show_wind_vectors: false,
            show_sound_controls: false,
            show_performance: false,
            show_temperature_map: false,
            show_precipitation_map: false,
            show_wind_map: false,
            show_memory_usage: false,
            show_heatmap: false,
            frame_times: Vec::with_capacity(100),
            effect_count_history: Vec::with_capacity(100),
            memory_usage_history: Vec::with_capacity(100),
            selected_effect_type: None,
            show_transitions: false,
            show_profiler: false,
            show_presets: false,
            show_advanced_viz: false,
            transition_history: Vec::with_capacity(50),
            profiler_data: Vec::with_capacity(100),
            selected_preset: None,
        }
    }
}

impl Plugin for WeatherPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
                AudioPlugin,
                FrameTimeDiagnosticsPlugin,
                SystemInformationDiagnosticsPlugin
            ))
            .init_resource::<WeatherState>()
            .init_resource::<WeatherEffects>()
            .init_resource::<WeatherSoundSettings>()
            .init_resource::<WeatherPresets>()
            .add_systems(Startup, load_weather_sounds)
            .add_systems(Update, (
                update_weather_effects,
                update_weather_debug.run_if(resource_exists::<DebugState>()),
                update_weather_ui.run_if(resource_exists::<DebugState>()),
                handle_debug_input.run_if(resource_exists::<DebugState>()),
                update_debug_metrics.run_if(resource_exists::<DebugState>()),
                update_profiler.run_if(resource_exists::<DebugState>()),
            ));
    }
}

fn load_weather_sounds(mut commands: Commands, asset_server: Res<AssetServer>) {
    let assets = WeatherSoundAssets {
        light_rain: asset_server.load("sounds/weather/light_rain.ogg"),
        heavy_rain: asset_server.load("sounds/weather/heavy_rain.ogg"),
        storm: asset_server.load("sounds/weather/storm.ogg"),
        strong_wind: asset_server.load("sounds/weather/strong_wind.ogg"),
        blizzard: asset_server.load("sounds/weather/blizzard.ogg"),
        wind: asset_server.load("sounds/weather/wind.ogg"),
    };
    commands.insert_resource(assets);
}

fn handle_debug_input(
    keyboard: Res<Input<KeyCode>>,
    mut debug_state: ResMut<DebugState>,
) {
    // Toggle debug features with keyboard shortcuts
    if keyboard.just_pressed(KeyCode::F1) {
        debug_state.show_weather_effects = !debug_state.show_weather_effects;
    }
    if keyboard.just_pressed(KeyCode::F2) {
        debug_state.show_ground_effects = !debug_state.show_ground_effects;
    }
    if keyboard.just_pressed(KeyCode::F3) {
        debug_state.show_effect_stats = !debug_state.show_effect_stats;
    }
    if keyboard.just_pressed(KeyCode::F4) {
        debug_state.show_particle_paths = !debug_state.show_particle_paths;
    }
    if keyboard.just_pressed(KeyCode::F5) {
        debug_state.show_wind_vectors = !debug_state.show_wind_vectors;
    }
    if keyboard.just_pressed(KeyCode::F6) {
        debug_state.show_performance = !debug_state.show_performance;
    }
    if keyboard.just_pressed(KeyCode::F7) {
        debug_state.show_temperature_map = !debug_state.show_temperature_map;
    }
    if keyboard.just_pressed(KeyCode::F8) {
        debug_state.show_precipitation_map = !debug_state.show_precipitation_map;
    }
    if keyboard.just_pressed(KeyCode::F9) {
        debug_state.show_memory_usage = !debug_state.show_memory_usage;
    }
    if keyboard.just_pressed(KeyCode::F10) {
        debug_state.show_heatmap = !debug_state.show_heatmap;
    }
    
    // Effect type selection with number keys
    for (i, key) in [KeyCode::Key1, KeyCode::Key2, KeyCode::Key3, KeyCode::Key4].iter().enumerate() {
        if keyboard.just_pressed(*key) {
            debug_state.selected_effect_type = match i {
                0 => Some("Rain".to_string()),
                1 => Some("Snow".to_string()),
                2 => Some("Storm".to_string()),
                3 => Some("Fog".to_string()),
                _ => None,
            };
        }
    }

    if keyboard.just_pressed(KeyCode::T) {
        debug_state.show_transitions = !debug_state.show_transitions;
    }
    if keyboard.just_pressed(KeyCode::P) {
        debug_state.show_profiler = !debug_state.show_profiler;
    }
    if keyboard.just_pressed(KeyCode::L) {
        debug_state.show_presets = !debug_state.show_presets;
    }
    if keyboard.just_pressed(KeyCode::V) {
        debug_state.show_advanced_viz = !debug_state.show_advanced_viz;
    }
}

fn update_debug_metrics(
    time: Res<Time>,
    diagnostics: Res<Diagnostics>,
    weather_effects: Res<WeatherEffects>,
    mut debug_state: ResMut<DebugState>,
) {
    if let Some(fps) = diagnostics.get(FrameTimeDiagnosticsPlugin::FPS) {
        if let Some(fps_value) = fps.smoothed() {
            debug_state.frame_times.push(1000.0 / fps_value); // Convert to ms
            if debug_state.frame_times.len() > 100 {
                debug_state.frame_times.remove(0);
            }
        }
    }

    debug_state.effect_count_history.push(
        weather_effects.active_effects.len() + weather_effects.ground_effects.len()
    );
    if debug_state.effect_count_history.len() > 100 {
        debug_state.effect_count_history.remove(0);
    }

    // Memory usage tracking
    if let Some(memory) = diagnostics.get(SystemInformationDiagnosticsPlugin::MEMORY_USAGE) {
        if let Some(value) = memory.value() {
            debug_state.memory_usage_history.push(value / 1024.0 / 1024.0); // Convert to MB
            if debug_state.memory_usage_history.len() > 100 {
                debug_state.memory_usage_history.remove(0);
            }
        }
    }
}

fn update_weather_ui(
    mut contexts: EguiContexts,
    mut debug_state: ResMut<DebugState>,
    mut sound_settings: ResMut<WeatherSoundSettings>,
    mut weather_state: ResMut<WeatherState>,
    weather_effects: Res<WeatherEffects>,
    weather_presets: Res<WeatherPresets>,
) {
    egui::Window::new("Weather Debug").show(contexts.ctx_mut(), |ui| {
        // Keyboard shortcuts help
        ui.collapsing("Keyboard Shortcuts", |ui| {
            ui.label("F1: Toggle Weather Effects");
            ui.label("F2: Toggle Ground Effects");
            ui.label("F3: Toggle Effect Stats");
            ui.label("F4: Toggle Particle Paths");
            ui.label("F5: Toggle Wind Vectors");
            ui.label("F6: Toggle Performance");
            ui.label("F7: Toggle Temperature Map");
            ui.label("F8: Toggle Precipitation Map");
            ui.label("F9: Toggle Memory Usage");
            ui.label("F10: Toggle Heatmap");
            ui.label("1-4: Select Effect Type");
        });

        // Visualization toggles
        ui.heading("Visualization");
        ui.checkbox(&mut debug_state.show_weather_effects, "Show Weather Effects");
        ui.checkbox(&mut debug_state.show_ground_effects, "Show Ground Effects");
        ui.checkbox(&mut debug_state.show_effect_stats, "Show Effect Stats");
        ui.checkbox(&mut debug_state.show_particle_paths, "Show Particle Paths");
        ui.checkbox(&mut debug_state.show_wind_vectors, "Show Wind Vectors");
        ui.checkbox(&mut debug_state.show_temperature_map, "Show Temperature Map");
        ui.checkbox(&mut debug_state.show_precipitation_map, "Show Precipitation Map");
        ui.checkbox(&mut debug_state.show_wind_map, "Show Wind Map");
        ui.checkbox(&mut debug_state.show_memory_usage, "Show Memory Usage");
        ui.checkbox(&mut debug_state.show_heatmap, "Show Heatmap");

        // Weather parameter adjustments
        ui.separator();
        ui.heading("Weather Parameters");
        ui.add(egui::Slider::new(&mut weather_state.temperature, -20.0..=40.0)
            .text("Temperature")
            .suffix("°C"));
        ui.add(egui::Slider::new(&mut weather_state.precipitation, 0.0..=1.0)
            .text("Precipitation"));
        ui.add(egui::Slider::new(&mut weather_state.wind_speed, 0.0..=20.0)
            .text("Wind Speed")
            .suffix("m/s"));
        ui.add(egui::Slider::new(&mut weather_state.fog_density, 0.0..=1.0)
            .text("Fog Density"));

        // Effect parameters
        ui.separator();
        ui.heading("Effect Parameters");
        if let Some(mut effects) = weather_effects.active_effects.iter().next() {
            if let Some(effect) = effects.1.get_component::<ParticleEffect>() {
                ui.add(egui::Slider::new(&mut effect.emitter.spawn_rate, 0.0..=2000.0)
                    .text("Spawn Rate"));
                ui.add(egui::Slider::new(&mut effect.emitter.lifetime, 0.1..=10.0)
                    .text("Lifetime"));
                ui.add(egui::Slider::new(&mut effect.emitter.size, 0.1..=5.0)
                    .text("Particle Size"));
            }
        }

        // Performance metrics
        if debug_state.show_performance {
            ui.separator();
            ui.heading("Performance Metrics");
            
            // Frame time graph
            let frame_times = &debug_state.frame_times;
            if !frame_times.is_empty() {
                let avg_frame_time = frame_times.iter().sum::<f32>() / frame_times.len() as f32;
                ui.label(format!("Average Frame Time: {:.2} ms", avg_frame_time));
                
                egui::plot::Plot::new("frame_times")
                    .height(100.0)
                    .show(ui, |plot_ui| {
                        let points: Vec<[f64; 2]> = frame_times.iter()
                            .enumerate()
                            .map(|(i, &t)| [i as f64, t as f64])
                            .collect();
                        plot_ui.line(egui::plot::Line::new(points));
                    });
            }

            // Effect count graph
            let effect_counts = &debug_state.effect_count_history;
            if !effect_counts.is_empty() {
                ui.label(format!("Current Effect Count: {}", effect_counts.last().unwrap()));
                
                egui::plot::Plot::new("effect_counts")
                    .height(100.0)
                    .show(ui, |plot_ui| {
                        let points: Vec<[f64; 2]> = effect_counts.iter()
                            .enumerate()
                            .map(|(i, &c)| [i as f64, c as f64])
                            .collect();
                        plot_ui.line(egui::plot::Line::new(points));
                    });
            }
        }

        // Memory usage graph
        if debug_state.show_memory_usage {
            ui.separator();
            ui.heading("Memory Usage");
            let memory_usage = &debug_state.memory_usage_history;
            if !memory_usage.is_empty() {
                let current_memory = memory_usage.last().unwrap();
                ui.label(format!("Current Memory Usage: {:.1} MB", current_memory));
                
                egui::plot::Plot::new("memory_usage")
                    .height(100.0)
                    .show(ui, |plot_ui| {
                        let points: Vec<[f64; 2]> = memory_usage.iter()
                            .enumerate()
                            .map(|(i, &m)| [i as f64, m as f64])
                            .collect();
                        plot_ui.line(egui::plot::Line::new(points));
                    });
            }
        }

        // Sound controls
        if debug_state.show_sound_controls {
            ui.separator();
            ui.heading("Sound Controls");
            ui.add(egui::Slider::new(&mut sound_settings.master_volume, 0.0..=1.0)
                .text("Master Volume"));
            
            ui.collapsing("Individual Effect Volumes", |ui| {
                for (effect_name, volume) in sound_settings.effect_volumes.iter_mut() {
                    ui.add(egui::Slider::new(volume, 0.0..=1.0)
                        .text(effect_name));
                }
            });
        }

        // Effect type selector
        ui.separator();
        ui.heading("Effect Type");
        egui::ComboBox::from_label("Selected Effect")
            .selected_text(debug_state.selected_effect_type.as_deref().unwrap_or("None"))
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut debug_state.selected_effect_type, None, "None");
                ui.selectable_value(&mut debug_state.selected_effect_type, Some("Rain".to_string()), "Rain");
                ui.selectable_value(&mut debug_state.selected_effect_type, Some("Snow".to_string()), "Snow");
                ui.selectable_value(&mut debug_state.selected_effect_type, Some("Storm".to_string()), "Storm");
                ui.selectable_value(&mut debug_state.selected_effect_type, Some("Fog".to_string()), "Fog");
            });

        // Advanced weather parameters
        if let Some(effect_type) = &debug_state.selected_effect_type {
            ui.separator();
            ui.heading(format!("{} Parameters", effect_type));
            match effect_type.as_str() {
                "Rain" => {
                    ui.add(egui::Slider::new(&mut weather_state.rain_intensity, 0.0..=1.0)
                        .text("Intensity"));
                    ui.add(egui::Slider::new(&mut weather_state.rain_drop_size, 0.1..=2.0)
                        .text("Drop Size"));
                    ui.add(egui::Slider::new(&mut weather_state.rain_splash_size, 0.0..=1.0)
                        .text("Splash Size"));
                }
                "Snow" => {
                    ui.add(egui::Slider::new(&mut weather_state.snow_density, 0.0..=1.0)
                        .text("Density"));
                    ui.add(egui::Slider::new(&mut weather_state.snow_flake_size, 0.1..=2.0)
                        .text("Flake Size"));
                    ui.add(egui::Slider::new(&mut weather_state.snow_drift_factor, 0.0..=2.0)
                        .text("Drift Factor"));
                }
                "Storm" => {
                    ui.add(egui::Slider::new(&mut weather_state.lightning_frequency, 0.0..=1.0)
                        .text("Lightning Frequency"));
                    ui.add(egui::Slider::new(&mut weather_state.thunder_volume, 0.0..=1.0)
                        .text("Thunder Volume"));
                    ui.add(egui::Slider::new(&mut weather_state.cloud_darkness, 0.0..=1.0)
                        .text("Cloud Darkness"));
                }
                "Fog" => {
                    ui.add(egui::Slider::new(&mut weather_state.fog_density, 0.0..=1.0)
                        .text("Density"));
                    ui.add(egui::Slider::new(&mut weather_state.fog_height, 0.0..=100.0)
                        .text("Height"));
                    ui.add(egui::Slider::new(&mut weather_state.fog_falloff, 0.0..=1.0)
                        .text("Falloff"));
                }
                _ => {}
            }
        }

        // Weather presets
        if debug_state.show_presets {
            ui.separator();
            ui.heading("Weather Presets");
            
            egui::ComboBox::from_label("Load Preset")
                .selected_text(debug_state.selected_preset.as_deref().unwrap_or("Custom"))
                .show_ui(ui, |ui| {
                    if ui.selectable_value(&mut debug_state.selected_preset, None, "Custom").clicked() {
                        // Reset to custom weather
                    }
                    
                    for (name, preset) in weather_presets.presets.iter() {
                        if ui.selectable_value(
                            &mut debug_state.selected_preset,
                            Some(name.clone()),
                            name
                        ).clicked() {
                            *weather_state = preset.clone();
                        }
                    }
                });
        }

        // Profiler
        if debug_state.show_profiler {
            ui.separator();
            ui.heading("Performance Profiler");
            
            if let Some(latest) = debug_state.profiler_data.last() {
                ui.label(format!("Particle Count: {}", latest.particle_count));
                ui.label(format!("Spawn Time: {:.2} ms", latest.spawn_time_ms));
                ui.label(format!("Update Time: {:.2} ms", latest.update_time_ms));
                ui.label(format!("Render Time: {:.2} ms", latest.render_time_ms));
                
                egui::plot::Plot::new("profiler_times")
                    .height(100.0)
                    .show(ui, |plot_ui| {
                        let spawn_points: Vec<[f64; 2]> = debug_state.profiler_data.iter()
                            .enumerate()
                            .map(|(i, d)| [i as f64, d.spawn_time_ms as f64])
                            .collect();
                        let update_points: Vec<[f64; 2]> = debug_state.profiler_data.iter()
                            .enumerate()
                            .map(|(i, d)| [i as f64, d.update_time_ms as f64])
                            .collect();
                        let render_points: Vec<[f64; 2]> = debug_state.profiler_data.iter()
                            .enumerate()
                            .map(|(i, d)| [i as f64, d.render_time_ms as f64])
                            .collect();
                            
                        plot_ui.line(egui::plot::Line::new(spawn_points).name("Spawn"));
                        plot_ui.line(egui::plot::Line::new(update_points).name("Update"));
                        plot_ui.line(egui::plot::Line::new(render_points).name("Render"));
                    });
            }
        }

        // Advanced visualization options
        if debug_state.show_advanced_viz {
            ui.separator();
            ui.heading("Advanced Visualization");
            
            ui.checkbox(&mut debug_state.show_transitions, "Show Weather Transitions");
            if debug_state.show_transitions && !debug_state.transition_history.is_empty() {
                egui::plot::Plot::new("transitions")
                    .height(100.0)
                    .show(ui, |plot_ui| {
                        let points: Vec<[f64; 2]> = debug_state.transition_history.iter()
                            .enumerate()
                            .map(|(i, (_, progress))| [i as f64, *progress as f64])
                            .collect();
                        plot_ui.line(egui::plot::Line::new(points));
                    });
            }
            
            // Additional visualization options
            ui.collapsing("Particle Visualization", |ui| {
                ui.checkbox(&mut debug_state.show_particle_paths, "Show Particle Paths");
                ui.checkbox(&mut debug_state.show_wind_vectors, "Show Wind Vectors");
                ui.checkbox(&mut debug_state.show_temperature_map, "Show Temperature Map");
                ui.checkbox(&mut debug_state.show_precipitation_map, "Show Precipitation Map");
            });
            
            ui.collapsing("Debug Overlays", |ui| {
                ui.checkbox(&mut debug_state.show_effect_stats, "Show Effect Statistics");
                ui.checkbox(&mut debug_state.show_performance, "Show Performance Metrics");
                ui.checkbox(&mut debug_state.show_memory_usage, "Show Memory Usage");
                ui.checkbox(&mut debug_state.show_heatmap, "Show Temperature Heatmap");
            });
        }
    });
}

fn update_weather_debug(
    mut gizmos: Gizmos,
    weather_effects: Res<WeatherEffects>,
    weather_state: Res<WeatherState>,
    debug_state: Res<DebugState>,
    query: Query<(&Transform, &ParticleEffect)>,
) {
    if !debug_state.show_weather_effects && !debug_state.show_ground_effects {
        return;
    }

    // Draw weather effect emitter bounds
    if debug_state.show_weather_effects {
        for (_, entity) in weather_effects.active_effects.iter() {
            if let Ok((transform, effect)) = query.get(*entity) {
                draw_emitter_bounds(&mut gizmos, transform, &effect.emitter, Color::CYAN);
            }
        }
    }

    // Draw ground effect bounds
    if debug_state.show_ground_effects {
        for (_, entity) in weather_effects.ground_effects.iter() {
            if let Ok((transform, effect)) = query.get(*entity) {
                draw_emitter_bounds(&mut gizmos, transform, &effect.emitter, Color::GREEN);
            }
        }
    }

    // Draw wind vectors
    if debug_state.show_wind_vectors && weather_state.wind_speed > 0.1 {
        let wind_dir = weather_state.wind_direction.extend(0.0).normalize();
        let wind_strength = weather_state.wind_speed;
        
        // Draw wind direction arrows in a grid
        for x in (-5..=5).map(|i| i as f32 * 10.0) {
            for z in (-5..=5).map(|i| i as f32 * 10.0) {
                let start = Vec3::new(x, 5.0, z);
                let end = start + wind_dir * wind_strength;
                gizmos.line(start, end, Color::YELLOW);
                
                // Arrow head
                let arrow_size = wind_strength * 0.2;
                let right = Vec3::new(-wind_dir.z, 0.0, wind_dir.x).normalize();
                gizmos.line(end, end - wind_dir * arrow_size + right * arrow_size, Color::YELLOW);
                gizmos.line(end, end - wind_dir * arrow_size - right * arrow_size, Color::YELLOW);
            }
        }
    }

    // Draw particle paths
    if debug_state.show_particle_paths {
        for (_, entity) in weather_effects.active_effects.iter() {
            if let Ok((transform, effect)) = query.get(*entity) {
                let velocity = effect.emitter.initial_velocity;
                let lifetime = effect.emitter.lifetime;
                let steps = 10;
                
                let mut pos = transform.translation;
                for i in 0..steps {
                    let t = i as f32 / steps as f32 * lifetime;
                    let next_pos = pos + velocity * t;
                    gizmos.line(pos, next_pos, Color::rgba(1.0, 1.0, 1.0, 0.3));
                    pos = next_pos;
                }
            }
        }
    }

    // Draw heatmap
    if debug_state.show_heatmap {
        let grid_size = 20;
        let cell_size = 5.0;
        
        for x in 0..grid_size {
            for z in 0..grid_size {
                let pos = Vec3::new(
                    (x as f32 - grid_size as f32 / 2.0) * cell_size,
                    0.1,
                    (z as f32 - grid_size as f32 / 2.0) * cell_size
                );
                
                // Calculate temperature at this position
                let temp = weather_state.temperature + 
                    (pos.x.abs() + pos.z.abs()) * 0.1 * 
                    (time.elapsed_seconds() * 0.5).sin();
                
                let color = if temp < 0.0 {
                    Color::rgba(0.0, 0.0, 1.0, (-temp / 20.0).min(1.0))
                } else {
                    Color::rgba(1.0, 0.0, 0.0, (temp / 40.0).min(1.0))
                };
                
                gizmos.rect(
                    pos,
                    Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2),
                    Vec2::new(cell_size, cell_size),
                    color
                );
            }
        }
    }
}

fn draw_emitter_bounds(gizmos: &mut Gizmos, transform: &Transform, emitter: &ParticleEmitter, color: Color) {
    match &emitter.shape {
        EmitterShape::Point => {
            gizmos.sphere(transform.translation, transform.rotation, 0.2, color);
        }
        EmitterShape::Box { size } => {
            gizmos.cuboid(
                transform.translation,
                transform.rotation,
                *size,
                color,
            );
        }
        EmitterShape::Plane { size, subdivisions: _ } => {
            let corners = [
                Vec3::new(-size.x/2.0, 0.0, -size.y/2.0),
                Vec3::new(size.x/2.0, 0.0, -size.y/2.0),
                Vec3::new(size.x/2.0, 0.0, size.y/2.0),
                Vec3::new(-size.x/2.0, 0.0, size.y/2.0),
            ];
            
            for i in 0..4 {
                let start = transform.transform_point(corners[i]);
                let end = transform.transform_point(corners[(i + 1) % 4]);
                gizmos.line(start, end, color);
            }
        }
    }
}

fn update_profiler(
    time: Res<Time>,
    weather_effects: Res<WeatherEffects>,
    mut debug_state: ResMut<DebugState>,
) {
    if !debug_state.show_profiler {
        return;
    }

    let snapshot = ProfilerSnapshot {
        timestamp: time.elapsed_seconds(),
        particle_count: weather_effects.active_effects.len(),
        spawn_time_ms: time.delta_seconds() * 1000.0 * 0.2, // Example metrics
        update_time_ms: time.delta_seconds() * 1000.0 * 0.5,
        render_time_ms: time.delta_seconds() * 1000.0 * 0.3,
    };

    debug_state.profiler_data.push(snapshot);
    if debug_state.profiler_data.len() > 100 {
        debug_state.profiler_data.remove(0);
    }
} 