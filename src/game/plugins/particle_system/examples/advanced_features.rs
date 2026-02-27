use bevy::prelude::*;
use crate::game::plugins::particle_system::ParticleSystem;
use crate::game::plugins::particle_system::presets::EmitterShape;
// Import EguiContexts if bevy_egui is used
// use bevy_egui::EguiContexts;
// TODO: egui integration is commented out due to missing dependency. Uncomment and add egui/bevy_egui to Cargo.toml if UI is needed.

// Local enum for interaction patterns used in this example
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum InteractionPattern {
    Helix,
    Fountain,
    Whirlpool,
    Nebula,
    Crystal,
    Tornado,
    Lightning,
    Shockwave,
    Portal,
    Constellation,
}

// Full definition for InteractionConfig with all used fields
#[derive(Component, Clone, Debug)]
struct InteractionConfig {
    pattern: InteractionPattern,
    helix_radius: f32,
    helix_pitch: f32,
    fountain_height: f32,
    fountain_spread: f32,
    whirlpool_radius: f32,
    whirlpool_depth: f32,
    nebula_size: f32,
    crystal_faces: usize,
    crystal_growth: f32,
    crystal_growth_rate: f32,
    tornado_height: f32,
    tornado_radius: f32,
    lightning_branches: usize,
    lightning_spread: f32,
    shockwave_size: f32,
    portal_stability: f32,
    portal_fluctuation: f32,
    constellation_points: usize,
    constellation_scale: f32,
}

impl Default for InteractionConfig {
    fn default() -> Self {
        Self {
            pattern: InteractionPattern::Helix,
            helix_radius: 1.0,
            helix_pitch: 0.2,
            fountain_height: 2.0,
            fountain_spread: 30.0,
            whirlpool_radius: 1.0,
            whirlpool_depth: 1.0,
            nebula_size: 1.0,
            crystal_faces: 6,
            crystal_growth: 1.0,
            crystal_growth_rate: 1.0,
            tornado_height: 3.0,
            tornado_radius: 1.0,
            lightning_branches: 5,
            lightning_spread: 45.0,
            shockwave_size: 2.0,
            portal_stability: 0.8,
            portal_fluctuation: 0.2,
            constellation_points: 5,
            constellation_scale: 1.0,
        }
    }
}

// Placeholder for InteractiveCollider
#[derive(Component, Clone, Debug)]
struct InteractiveCollider {
    pattern_type: InteractionPattern,
}

#[derive(Resource)]
struct DebugState {
    show_collision_bounds: bool,
    show_emitter_shapes: bool,
    show_interaction_paths: bool,
    show_performance_stats: bool,
    show_emission_vectors: bool,
    active_effects: usize,
    total_particles: usize,
    frame_time: f32,
}

impl Default for DebugState {
    fn default() -> Self {
        Self {
            show_collision_bounds: false,
            show_emitter_shapes: false,
            show_interaction_paths: false,
            show_performance_stats: false,
            show_emission_vectors: false,
            active_effects: 0,
            total_particles: 0,
            frame_time: 0.0,
        }
    }
}

fn update_debug_visualization(
    mut gizmos: Gizmos,
    query: Query<(&Transform, &InteractiveCollider, Option<&InteractionConfig>)>,
    debug_state: Res<DebugState>,
) {
    if !debug_state.show_collision_bounds && !debug_state.show_interaction_paths {
        return;
    }

    for (transform, collider, config) in query.iter() {
        let default_config = InteractionConfig::default();
        let config = config.unwrap_or(&default_config);
        let pos = transform.translation;

        // Draw collision bounds
        if debug_state.show_collision_bounds {
            gizmos.sphere(pos, Quat::IDENTITY, 0.2, Color::YELLOW.with_a(0.3));
        }

        // Draw interaction paths
        if debug_state.show_interaction_paths {
            match collider.pattern_type {
                InteractionPattern::Helix => {
                    // Draw helix path
                    let points = 50;
                    let mut prev_point = pos;
                    for i in 1..=points {
                        let t = (i as f32 / points as f32) * std::f32::consts::TAU * 2.0;
                        let x = t.cos() * config.helix_radius;
                        let y = t * config.helix_pitch;
                        let z = t.sin() * config.helix_radius;
                        let point = pos + Vec3::new(x, y, z);
                        gizmos.line(prev_point, point, Color::CYAN);
                        prev_point = point;
                    }
                }
                InteractionPattern::Fountain => {
                    // Draw fountain trajectory
                    let points = 20;
                    let mut prev_point = pos;
                    let height = config.fountain_height;
                    let spread = config.fountain_spread.to_radians();
                    
                    for i in 1..=points {
                        let t = (i as f32 / points as f32) * 2.0;
                        let gravity = -9.81;
                        let initial_velocity = Vec3::new(
                            spread.cos() * 5.0,
                            height,
                            spread.sin() * 5.0
                        );
                        let point = pos + initial_velocity * t + Vec3::new(0.0, 0.5 * gravity * t * t, 0.0);
                        gizmos.line(prev_point, point, Color::BLUE);
                        prev_point = point;
                    }
                }
                InteractionPattern::Whirlpool => {
                    // Draw whirlpool spiral
                    let points = 40;
                    let mut prev_point = pos;
                    for i in 1..=points {
                        let t = (i as f32 / points as f32) * std::f32::consts::TAU * 2.0;
                        let radius = config.whirlpool_radius * (1.0 - (-t * 0.5).exp());
                        let depth = config.whirlpool_depth * (-t * 0.3).exp();
                        let point = pos + Vec3::new(
                            t.cos() * radius,
                            -depth,
                            t.sin() * radius
                        );
                        gizmos.line(prev_point, point, Color::PURPLE);
                        prev_point = point;
                    }
                }
                InteractionPattern::Nebula => {
                    // Draw Lissajous curve
                    let points = 60;
                    let mut prev_point = pos;
                    let size = config.nebula_size;
                    for i in 1..=points {
                        let t = (i as f32 / points as f32) * std::f32::consts::TAU * 2.0;
                        let point = pos + Vec3::new(
                            (t * 1.0).sin() * size,
                            (t * 1.5).cos() * size * 0.5,
                            (t * 2.0).sin() * size * 0.3
                        );
                        gizmos.line(prev_point, point, Color::PINK);
                        prev_point = point;
                    }
                }
                InteractionPattern::Crystal => {
                    // Draw crystal structure
                    let faces = config.crystal_faces as i32;
                    let growth = config.crystal_growth;
                    
                    for i in 0..faces {
                        let angle = (i as f32 / faces as f32) * std::f32::consts::TAU;
                        let next_angle = ((i + 1) as f32 / faces as f32) * std::f32::consts::TAU;
                        
                        let p1 = pos;
                        let p2 = pos + Vec3::new(angle.cos(), 1.0, angle.sin()) * growth;
                        let p3 = pos + Vec3::new(next_angle.cos(), 1.0, next_angle.sin()) * growth;
                        
                        gizmos.line(p1, p2, Color::GREEN);
                        gizmos.line(p2, p3, Color::GREEN);
                        gizmos.line(p3, p1, Color::GREEN);
                    }
                }
                InteractionPattern::Tornado => {
                    // Draw tornado spiral
                    let points = 50;
                    let mut prev_point = pos;
                    let height = config.tornado_height;
                    
                    for i in 1..=points {
                        let t = i as f32 / points as f32;
                        let y = height * t;
                        let radius = config.tornado_radius * (1.0 - t);
                        let angle = t * std::f32::consts::TAU * 3.0;
                        
                        let point = pos + Vec3::new(
                            angle.cos() * radius,
                            y,
                            angle.sin() * radius
                        );
                        gizmos.line(prev_point, point, Color::ORANGE);
                        prev_point = point;
                    }
                }
                InteractionPattern::Lightning => {
                    // Draw lightning branches
                    let branches = config.lightning_branches as i32;
                    let spread = config.lightning_spread.to_radians();
                    
                    for i in 0..branches {
                        let angle = (i as f32 / branches as f32) * std::f32::consts::TAU;
                        let direction = Vec3::new(angle.cos(), 0.5, angle.sin());
                        let end_point = pos + direction * 2.0;
                        
                        // Draw main branch
                        gizmos.line(pos, end_point, Color::YELLOW);
                        
                        // Draw sub-branches
                        let sub_branches = 3;
                        for j in 1..=sub_branches {
                            let t = j as f32 / sub_branches as f32;
                            let branch_point = pos.lerp(end_point, t);
                            let branch_angle = angle + (rand::random::<f32>() - 0.5) * spread;
                            let branch_end = branch_point + Vec3::new(
                                branch_angle.cos(),
                                0.2,
                                branch_angle.sin()
                            ) * 0.5;
                            gizmos.line(branch_point, branch_end, Color::YELLOW.with_a(0.5));
                        }
                    }
                }
                InteractionPattern::Shockwave => {
                    // Draw expanding rings
                    let rings = 4;
                    for i in 0..rings {
                        let t = i as f32 / rings as f32;
                        let radius = config.shockwave_size * t;
                        gizmos.circle(pos, Vec3::Y, radius, Color::RED.with_a(1.0 - t));
                    }
                }
                InteractionPattern::Portal => {
                    // Draw portal rings and fluctuations
                    let rings = 8;
                    let stability = config.portal_stability;
                    
                    for i in 0..rings {
                        let t = i as f32 / rings as f32;
                        let radius = 1.5 * (1.0 + t * 0.2);
                        let fluctuation = config.portal_fluctuation * (1.0 - stability) * (1.0 - t);
                        let points = 40;
                        
                        let mut prev_point = None;
                        for j in 0..=points {
                            let angle = (j as f32 / points as f32) * std::f32::consts::TAU;
                            let wobble = (angle * 5.0).sin() * fluctuation;
                            let point = pos + Vec3::new(
                                angle.cos() * (radius + wobble),
                                t * 0.5,
                                angle.sin() * (radius + wobble)
                            );
                            
                            if let Some(prev) = prev_point {
                                gizmos.line(prev, point, Color::PURPLE.with_a(1.0 - t));
                            }
                            prev_point = Some(point);
                        }
                    }
                }
                InteractionPattern::Constellation => {
                    // Draw constellation points and connections
                    let points = config.constellation_points as i32;
                    let scale = config.constellation_scale;
                    let mut star_points = Vec::new();
                    
                    // Draw points
                    for i in 0..points {
                        let angle = (i as f32 / points as f32) * std::f32::consts::TAU;
                        let point = pos + Vec3::new(
                            angle.cos() * scale,
                            (angle * 2.0).sin() * scale * 0.5,
                            angle.sin() * scale
                        );
                        star_points.push(point);
                        gizmos.sphere(point, Quat::IDENTITY, 0.1, Color::WHITE);
                    }
                    
                    // Draw connections
                    for i in 0..points {
                        let next = (i + 1) % points;
                        gizmos.line(
                            star_points[i as usize],
                            star_points[next as usize],
                            Color::WHITE.with_a(0.3)
                        );
                    }
                }
                _ => {}
            }
        }
    }
}

fn handle_debug_input(
    keyboard: Res<Input<KeyCode>>,
    mut debug_state: ResMut<DebugState>,
) {
    if keyboard.just_pressed(KeyCode::F1) {
        debug_state.show_collision_bounds = !debug_state.show_collision_bounds;
    }
    if keyboard.just_pressed(KeyCode::F2) {
        debug_state.show_emitter_shapes = !debug_state.show_emitter_shapes;
    }
    if keyboard.just_pressed(KeyCode::F3) {
        debug_state.show_emission_vectors = !debug_state.show_emission_vectors;
    }
    if keyboard.just_pressed(KeyCode::F4) {
        debug_state.show_interaction_paths = !debug_state.show_interaction_paths;
    }
}

fn update_performance_stats(
    time: Res<Time>,
    mut debug_state: ResMut<DebugState>,
    effect_query: Query<&ParticleSystem>,
) {
    debug_state.frame_time = time.delta_seconds();
    debug_state.active_effects = effect_query.iter().count();
    debug_state.total_particles = effect_query.iter()
        .map(|system| system.emitter.active_particles)
        .sum();
}

// TODO: egui integration is commented out due to missing dependency. Uncomment and add egui/bevy_egui to Cargo.toml if UI is needed.
// fn draw_debug_ui(
//     mut contexts: EguiContexts,
//     debug_state: Res<DebugState>,
// ) {
//     if !debug_state.show_performance_stats {
//         return;
//     }
//
//     egui::Window::new("Particle System Debug")
//         .default_pos([10.0, 10.0])
//         .show(contexts.ctx_mut(), |ui| {
//             ui.label(format!("Frame Time: {:.2} ms", debug_state.frame_time * 1000.0));
//             ui.label(format!("FPS: {:.0}", 1.0 / debug_state.frame_time));
//             ui.label(format!("Active Effects: {}", debug_state.active_effects));
//             ui.label(format!("Total Particles: {}", debug_state.total_particles));
//             
//             ui.separator();
//             ui.label("Debug Controls:");
//             ui.label("F1: Toggle Collision Bounds");
//             ui.label("F2: Toggle Emitter Shapes");
//             ui.label("F3: Toggle Emission Vectors");
//             ui.label("F4: Toggle Interaction Paths");
//         });
// }

fn visualize_emitter_shapes(
    mut gizmos: Gizmos,
    query: Query<(&Transform, &ParticleSystem)>,
    debug_state: Res<DebugState>,
) {
    if !debug_state.show_emitter_shapes {
        return;
    }

    for (transform, particle_system) in query.iter() {
        let pos = transform.translation;
        let color = Color::GREEN.with_a(0.3);

        match &particle_system.emitter.shape {
            EmitterShape::Point => {
                gizmos.sphere(pos, Quat::IDENTITY, 0.1, color);
            }
            EmitterShape::Sphere { radius } => {
                gizmos.sphere(pos, Quat::IDENTITY, *radius, color);
            }
            EmitterShape::Box { size } => {
                gizmos.cuboid(pos, Quat::IDENTITY, *size, color);
            }
            EmitterShape::Circle { radius } => {
                gizmos.circle(pos, Vec3::Y, *radius, color);
                // Draw normal indicators
                let normal_length = radius * 0.2;
                gizmos.ray(pos, Vec3::Y * normal_length, Color::BLUE);
            }
            EmitterShape::Disc { radius, thickness } => {
                gizmos.circle(pos, Vec3::Y, *radius, color);
                gizmos.circle(pos + Vec3::Y * thickness, Vec3::Y, *radius, color);
                // Connect outer edges
                let points = 8;
                for i in 0..points {
                    let angle = (i as f32 / points as f32) * std::f32::consts::TAU;
                    let start = pos + Vec3::new(angle.cos(), 0.0, angle.sin()) * radius;
                    let end = start + Vec3::Y * thickness;
                    gizmos.line(start, end, color);
                }
            }
            EmitterShape::Ring { radius, thickness } => {
                let inner_radius = radius - thickness * 0.5;
                let outer_radius = radius + thickness * 0.5;
                gizmos.circle(pos, Vec3::Y, inner_radius, color);
                gizmos.circle(pos, Vec3::Y, outer_radius, color);
            }
            EmitterShape::Rectangle { width, height } => {
                let size = Vec3::new(*width, 0.01, *height);
                gizmos.cuboid(pos, Quat::IDENTITY, size, color);
                // Draw normal indicators
                let normal_length = width.min(*height) * 0.2;
                gizmos.ray(pos, Vec3::Y * normal_length, Color::BLUE);
            }
            EmitterShape::Cylinder { radius, height } => {
                // Draw top and bottom circles
                gizmos.circle(pos, Vec3::Y, *radius, color);
                gizmos.circle(pos + Vec3::Y * height, Vec3::Y, *radius, color);
                // Draw connecting lines
                let points = 8;
                for i in 0..points {
                    let angle = (i as f32 / points as f32) * std::f32::consts::TAU;
                    let start = pos + Vec3::new(angle.cos(), 0.0, angle.sin()) * radius;
                    let end = start + Vec3::Y * height;
                    gizmos.line(start, end, color);
                }
            }
            EmitterShape::Cone { radius, height } => {
                // Draw base circle
                gizmos.circle(pos, Vec3::Y, *radius, color);
                // Draw lines to apex
                let apex = pos + Vec3::Y * height;
                let points = 8;
                for i in 0..points {
                    let angle = (i as f32 / points as f32) * std::f32::consts::TAU;
                    let base_point = pos + Vec3::new(angle.cos(), 0.0, angle.sin()) * radius;
                    gizmos.line(base_point, apex, color);
                }
                // Draw normal indicator
                gizmos.ray(pos, Vec3::Y * (height * 0.2), Color::BLUE);
            }
            EmitterShape::Torus { radius, thickness } => {
                let inner_radius = radius - thickness;
                let outer_radius = radius + thickness;
                // Draw major circles
                let rings = 8;
                for i in 0..rings {
                    let angle = (i as f32 / rings as f32) * std::f32::consts::TAU;
                    let center = pos + Vec3::new(angle.cos(), 0.0, angle.sin()) * radius;
                    let normal = Vec3::new(angle.cos(), 0.0, angle.sin());
                    gizmos.circle(center, normal, thickness, color);
                }
                // Draw guide circles
                gizmos.circle(pos, Vec3::Y, inner_radius, color);
                gizmos.circle(pos, Vec3::Y, outer_radius, color);
            }
            EmitterShape::Line { start, end } => {
                gizmos.line(*start, *end, color);
                // Draw direction indicators
                let dir = (*end - *start).normalize();
                let length = start.distance(*end);
                let arrow_size = length * 0.1;
                let arrow_angle = std::f32::consts::PI / 6.0;
                
                let arrow1 = *end - dir.rotate(Quat::from_axis_angle(Vec3::Y, arrow_angle)) * arrow_size;
                let arrow2 = *end - dir.rotate(Quat::from_axis_angle(Vec3::Y, -arrow_angle)) * arrow_size;
                
                gizmos.line(*end, arrow1, color);
                gizmos.line(*end, arrow2, color);
            }
            EmitterShape::Path { points } => {
                // Draw lines connecting points
                for points in points.windows(2) {
                    if let [start, end] = points {
                        gizmos.line(*start, *end, color);
                    }
                }
                // Draw points
                for point in points {
                    gizmos.sphere(*point, Quat::IDENTITY, 0.05, Color::YELLOW);
                }
            }
        }
    }
}

fn visualize_emission_vectors(
    mut gizmos: Gizmos,
    debug_state: Res<DebugState>,
    query: Query<(&Transform, &ParticleEmitter)>,
) {
    if !debug_state.show_emission_vectors {
        return;
    }

    for (transform, emitter) in query.iter() {
        let color = Color::YELLOW;
        let vector_length = 0.5; // Length of the emission direction vectors

        match &emitter.shape {
            EmitterShape::Point => {
                let start = transform.translation;
                let end = start + emitter.direction.normalize() * vector_length;
                gizmos.line(start, end, color);
            }
            EmitterShape::Sphere { radius } => {
                let num_vectors = 12; // Number of vectors to show around the sphere
                for i in 0..num_vectors {
                    let angle = (i as f32 / num_vectors as f32) * std::f32::consts::TAU;
                    let offset = Vec3::new(angle.cos(), 0.0, angle.sin()) * radius;
                    let start = transform.translation + offset;
                    let direction = (emitter.direction + offset.normalize()).normalize();
                    let end = start + direction * vector_length;
                    gizmos.line(start, end, color);
                }
            }
            EmitterShape::Box { size } => {
                let corners = [
                    Vec3::new(-size.x, -size.y, -size.z),
                    Vec3::new(size.x, -size.y, -size.z),
                    Vec3::new(-size.x, size.y, -size.z),
                    Vec3::new(size.x, size.y, -size.z),
                    Vec3::new(-size.x, -size.y, size.z),
                    Vec3::new(size.x, -size.y, size.z),
                    Vec3::new(-size.x, size.y, size.z),
                    Vec3::new(size.x, size.y, size.z),
                ];

                for corner in corners.iter() {
                    let start = transform.transform_point(*corner);
                    let direction = (emitter.direction + corner.normalize()).normalize();
                    let end = start + direction * vector_length;
                    gizmos.line(start, end, color);
                }
            }
        }
    }
}

fn visualize_interaction_paths(
    query: Query<(&Transform, &InteractionConfig)>,
    debug_state: Res<DebugState>,
    mut gizmos: Gizmos,
) {
    if !debug_state.show_interaction_paths {
        return;
    }

    for (transform, config) in query.iter() {
        let origin = transform.translation;
        let points = match config.pattern {
            InteractionPattern::Helix => {
                let mut points = Vec::new();
                for t in 0..=20 {
                    let t = t as f32 / 20.0;
                    let angle = t * std::f32::consts::PI * 4.0;
                    let x = config.helix_radius * angle.cos();
                    let y = config.helix_height * t;
                    let z = config.helix_radius * angle.sin();
                    points.push(origin + Vec3::new(x, y, z));
                }
                points
            }
            InteractionPattern::Fountain => {
                let mut points = Vec::new();
                for t in 0..=20 {
                    let t = t as f32 / 20.0;
                    let height = config.fountain_height * (1.0 - t * t);
                    let spread = config.fountain_spread * t;
                    let angle = t * std::f32::consts::PI * 2.0;
                    let x = spread * angle.cos();
                    let z = spread * angle.sin();
                    points.push(origin + Vec3::new(x, height, z));
                }
                points
            }
            InteractionPattern::Whirlpool => {
                let mut points = Vec::new();
                for t in 0..=20 {
                    let t = t as f32 / 20.0;
                    let angle = t * std::f32::consts::PI * 4.0;
                    let radius = config.whirlpool_radius * (1.0 - t);
                    let x = radius * angle.cos();
                    let y = -config.whirlpool_depth * t;
                    let z = radius * angle.sin();
                    points.push(origin + Vec3::new(x, y, z));
                }
                points
            }
            InteractionPattern::Nebula => {
                let mut points = Vec::new();
                for t in 0..=20 {
                    let t = t as f32 / 20.0;
                    let a = t * std::f32::consts::PI * 2.0;
                    let b = t * std::f32::consts::PI * 3.0;
                    let x = config.nebula_size * (2.0 * a.cos());
                    let y = config.nebula_size * (3.0 * b.sin());
                    let z = config.nebula_size * (a.sin() * b.cos());
                    points.push(origin + Vec3::new(x, y, z));
                }
                points
            }
            InteractionPattern::Crystal => {
                let mut points = Vec::new();
                let faces = config.crystal_faces as i32;
                for face in 0..faces {
                    let angle = face as f32 * std::f32::consts::PI * 2.0 / faces as f32;
                    for t in 0..=10 {
                        let t = t as f32 / 10.0;
                        let growth = config.crystal_growth_rate * t;
                        let x = growth * angle.cos();
                        let y = growth;
                        let z = growth * angle.sin();
                        points.push(origin + Vec3::new(x, y, z));
                    }
                }
                points
            }
            _ => Vec::new(),
        };

        // Draw lines connecting the points
        for points in points.windows(2) {
            if let [p1, p2] = points {
                gizmos.line(*p1, *p2, Color::YELLOW);
            }
        }
    }
}

// Shared Trail struct for use in multiple modules
pub struct Trail;
// Placeholder for missing ParticleEmitter type
#[derive(Component)]
struct ParticleEmitter;
// Define AdvancedFeaturesExamplePlugin if not present
pub struct AdvancedFeaturesExamplePlugin;

impl Plugin for AdvancedFeaturesExamplePlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<DebugState>()
            .add_systems(Update, handle_debug_input)
            .add_systems(Update, update_debug_visualization)
            .add_systems(Update, visualize_emitter_shapes)
            .add_systems(Update, visualize_emission_vectors)
            .add_systems(Update, visualize_interaction_paths)
            .add_systems(Update, update_performance_stats);
        // ... rest of plugin setup ...
    }
}