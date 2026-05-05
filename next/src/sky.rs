// Sky, lighting, fog, and atmosphere for sandk-offroad.
// Strategy: Option A — StandardMaterial base_color lerped each frame between
// day/night palettes. Vertex colors baked at spawn carry the zenith→horizon
// gradient; the uniform tint multiplies on top (one material write per frame).

use std::f32::consts::PI;

use bevy::{
    asset::RenderAssetUsages,
    mesh::{Indices, PrimitiveTopology},
    pbr::{DistanceFog, FogFalloff},
    prelude::*,
};

pub struct SkyPlugin;

impl Plugin for SkyPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(TimeOfDay::default())
           .add_systems(Startup, (setup_sun, spawn_sky_dome))
           .add_systems(Update, (
               time_of_day_input,
               tick_time_of_day,
               update_sun,
               update_sky_tint,
               update_fog_color,
               update_ambient,
               attach_fog_to_camera,
           ).chain());
    }
}

/// Tracks the current time of day.
///
/// `t` runs 0.0 → 1.0 over one full day:
///   0.00 = midnight, 0.25 = sunrise, 0.50 = noon, 0.75 = sunset, 1.00 = midnight
#[derive(Resource)]
pub struct TimeOfDay {
    pub t:            f32,
    /// Real seconds for one full cycle.
    pub day_length_s: f32,
    pub paused:       bool,
}

impl Default for TimeOfDay {
    fn default() -> Self {
        // Start at noon so the world is fully lit at startup. Player can press T
        // to pause the cycle or [/] to scrub to dawn / dusk / night manually.
        Self { t: 0.5, day_length_s: 600.0, paused: false }
    }
}

#[derive(Component)] struct SkyDome;
#[derive(Component)] struct FogAttached;


const DAY_ZENITH:    [f32; 4] = [0.10, 0.20, 0.55, 1.0];
const NIGHT_ZENITH:  [f32; 4] = [0.02, 0.03, 0.08, 1.0];
const DAY_HORIZON:   [f32; 4] = [0.62, 0.76, 0.90, 1.0];
const NIGHT_HORIZON: [f32; 4] = [0.08, 0.10, 0.18, 1.0];
/// Orange tint injected at the horizon during golden-hour (±15° of horizon).
const GOLDEN_HORIZON:[f32; 4] = [1.00, 0.50, 0.30, 1.0];

fn setup_sun(mut commands: Commands, mut ambient: ResMut<GlobalAmbientLight>) {
    commands.spawn((
        DirectionalLight {
            illuminance: 10_000.0,
            shadows_enabled: true,
            shadow_depth_bias: 0.04,
            shadow_normal_bias: 2.0,
            color: Color::srgb(1.0, 0.97, 0.88),
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.9, 0.5, 0.0)),
    ));

    ambient.color      = Color::srgb(0.5, 0.55, 0.65);
    ambient.brightness = 200.0;
}

fn spawn_sky_dome(
    mut commands:  Commands,
    mut meshes:    ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mesh = build_sky_dome(32, 16, 900.0);

    let mat = materials.add(StandardMaterial {
        base_color: Color::WHITE, // vertex colors + per-frame tint
        unlit: true,
        cull_mode: None,          // render from inside the dome
        ..default()
    });

    commands.spawn((
        Mesh3d(meshes.add(mesh)),
        MeshMaterial3d(mat),
        Transform::default(),
        SkyDome,
    ));
}

/// T = toggle pause; [ / ] scrub time.
fn time_of_day_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut tod: ResMut<TimeOfDay>,
) {
    if keys.just_pressed(KeyCode::KeyT) {
        tod.paused = !tod.paused;
    }
    if keys.just_pressed(KeyCode::BracketLeft) {
        tod.t = (tod.t - 0.05).rem_euclid(1.0);
    }
    if keys.just_pressed(KeyCode::BracketRight) {
        tod.t = (tod.t + 0.05).rem_euclid(1.0);
    }
}

fn tick_time_of_day(time: Res<Time>, mut tod: ResMut<TimeOfDay>) {
    if !tod.paused {
        tod.t = (tod.t + time.delta_secs() / tod.day_length_s).rem_euclid(1.0);
    }
}

/// Rotate the directional light and update its illuminance + colour.
///
/// elevation_rad sweeps a full circle over one day:
///   t=0.25 → 0 rad (rising), t=0.50 → π/2 (zenith), t=0.75 → π (setting).
/// A 30° Y-axis tilt gives the sun an east-biased arc so it's never dead-overhead.
fn update_sun(
    tod: Res<TimeOfDay>,
    mut lights: Query<(&mut DirectionalLight, &mut Transform)>,
) {
    let elevation_rad = (tod.t - 0.25) * 2.0 * PI;
    let sin_el = elevation_rad.sin(); // 1 at noon, −1 at midnight

    // Negative X pitch: light shines downward when elevation is positive.
    let pitch    = Quat::from_rotation_x(-elevation_rad);
    let yaw      = Quat::from_rotation_y(0.52); // ≈ 30° east offset
    let rotation = yaw * pitch;

    let above  = sin_el.max(0.0);
    // golden: 1.0 when sun is near the horizon (sin_el ≈ 0).
    let golden = smooth_step((1.0 - (sin_el.abs() / 0.18).min(1.0)).max(0.0));

    let illuminance = {
        let base = lerp(50.0, 10_000.0, smooth_step(above));
        lerp(base, 3_000.0, golden) // pull toward 3 000 lx at golden hour
    };

    let night_c:  [f32; 3] = [0.40, 0.50, 0.70];
    let day_c:    [f32; 3] = [1.00, 0.97, 0.88];
    let golden_c: [f32; 3] = [1.00, 0.60, 0.40];
    let base_c = lerp_color3(&night_c, &day_c, smooth_step(above));
    let color  = lerp_color3(&base_c, &golden_c, golden);

    for (mut light, mut transform) in &mut lights {
        transform.rotation = rotation;
        light.illuminance  = illuminance;
        light.color        = Color::srgb(color[0], color[1], color[2]);
    }
}

fn update_sky_tint(
    tod: Res<TimeOfDay>,
    sky_query: Query<&MeshMaterial3d<StandardMaterial>, With<SkyDome>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let elevation_rad = (tod.t - 0.25) * 2.0 * PI;
    let sin_el = elevation_rad.sin();
    let above  = sin_el.max(0.0);

    let golden = smooth_step((1.0 - sin_el.abs().min(0.18) / 0.18).max(0.0));
    let horizon = {
        let base = lerp_color(&NIGHT_HORIZON, &DAY_HORIZON, smooth_step(above));
        lerp_color(&base, &GOLDEN_HORIZON, golden)
    };
    let zenith = lerp_color(&NIGHT_ZENITH, &DAY_ZENITH, smooth_step(above));
    // Midpoint between horizon and zenith as a single-colour dome tint (Option A).
    let tint = lerp_color(&zenith, &horizon, 0.5);

    for mat_handle in &sky_query {
        if let Some(mat) = materials.get_mut(mat_handle) {
            mat.base_color = Color::srgba(tint[0], tint[1], tint[2], tint[3]);
        }
    }
}

fn update_fog_color(
    tod: Res<TimeOfDay>,
    mut fogs: Query<&mut DistanceFog>,
) {
    let elevation_rad = (tod.t - 0.25) * 2.0 * PI;
    let sin_el = elevation_rad.sin();
    let above  = sin_el.max(0.0);
    let golden = smooth_step((1.0 - sin_el.abs().min(0.18) / 0.18).max(0.0));

    let horizon = {
        let base = lerp_color(&NIGHT_HORIZON, &DAY_HORIZON, smooth_step(above));
        lerp_color(&base, &GOLDEN_HORIZON, golden)
    };

    for mut fog in &mut fogs {
        fog.color = Color::srgba(horizon[0], horizon[1], horizon[2], 1.0);
    }
}

fn update_ambient(tod: Res<TimeOfDay>, mut ambient: ResMut<GlobalAmbientLight>) {
    let elevation_rad = (tod.t - 0.25) * 2.0 * PI;
    let sin_el = elevation_rad.sin();
    let above  = smooth_step(sin_el.max(0.0));

    // Bumped night floor 20→200 and day cap 200→500 — the original values
    // produced a dim mid-morning (t=0.4) that made it hard to see the world.
    ambient.brightness = lerp(200.0, 500.0, above);
    let night: [f32; 3] = [0.35, 0.40, 0.60];
    let day:   [f32; 3] = [0.50, 0.55, 0.65];
    let c = lerp_color3(&night, &day, above);
    ambient.color = Color::srgb(c[0], c[1], c[2]);
}

fn attach_fog_to_camera(
    mut commands: Commands,
    cameras: Query<Entity, (Added<Camera3d>, Without<FogAttached>)>,
) {
    for entity in &cameras {
        commands.entity(entity).insert((
            DistanceFog {
                color: Color::srgb(DAY_HORIZON[0], DAY_HORIZON[1], DAY_HORIZON[2]),
                falloff: FogFalloff::Linear { start: 80.0, end: 250.0 },
                directional_light_color: Color::srgb(1.0, 0.95, 0.75),
                directional_light_exponent: 16.0,
                ..default()
            },
            FogAttached,
        ));
    }
}

/// Inverted UV-sphere. Vertex colors bake the night zenith→horizon gradient;
/// the per-frame material tint blends it toward the day palette.
fn build_sky_dome(slices: u32, stacks: u32, radius: f32) -> Mesh {
    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut normals:   Vec<[f32; 3]> = Vec::new();
    let mut uvs:       Vec<[f32; 2]> = Vec::new();
    let mut colors:    Vec<[f32; 4]> = Vec::new();
    let mut indices:   Vec<u32>      = Vec::new();

    for stack in 0..=stacks {
        let phi = PI * stack as f32 / stacks as f32;
        let y   = phi.cos();
        let r   = phi.sin();

        let t     = ((y + 1.0) * 0.5).clamp(0.0, 1.0); // 1=zenith, 0=nadir
        let color = lerp_color(&NIGHT_HORIZON, &NIGHT_ZENITH, smooth_step(t));

        for slice in 0..=slices {
            let theta = 2.0 * PI * slice as f32 / slices as f32;
            let px = r * theta.cos() * radius;
            let py = y * radius;
            let pz = r * theta.sin() * radius;

            positions.push([px, py, pz]);
            let n = Vec3::new(px, py, pz).normalize();
            normals.push([-n.x, -n.y, -n.z]); // inward normals
            uvs.push([slice as f32 / slices as f32, stack as f32 / stacks as f32]);
            colors.push(color);
        }
    }

    let ring = slices + 1;
    for stack in 0..stacks {
        for slice in 0..slices {
            let tl = stack * ring + slice;
            let tr = tl + 1;
            let bl = tl + ring;
            let br = bl + 1;
            indices.extend_from_slice(&[tl, tr, bl, tr, br, bl]);
        }
    }

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL,   normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0,     uvs);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR,    colors);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

#[inline]
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

#[inline]
fn lerp_color(a: &[f32; 4], b: &[f32; 4], t: f32) -> [f32; 4] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
        1.0,
    ]
}

#[inline]
fn lerp_color3(a: &[f32; 3], b: &[f32; 3], t: f32) -> [f32; 3] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

/// Smooth cubic ease-in/ease-out (Ken Perlin's smoothstep).
#[inline]
fn smooth_step(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}
