// Sky, lighting, fog, and atmosphere for sandk-offroad.
//
// Implementation: Option A — an inverted icosphere sky-dome whose vertex colors
// are pre-baked from zenith→horizon gradient (deep blue → hazy light-blue).
// The mesh uses `cull_mode: None` + `unlit: true` so it always renders behind
// everything else and never self-occludes.
//
// In addition this plugin owns:
//   • A well-tuned DirectionalLight (10 000 lx, shadows on, bias tuned for
//     terrain-scale scenes).
//   • GlobalAmbientLight with a subtle sky-bounce tint.
//   • Distance fog inserted onto the Camera3d entity via a one-shot system that
//     triggers on `Added<Camera3d>`.

use bevy::{
    asset::RenderAssetUsages,
    mesh::{Indices, PrimitiveTopology},
    pbr::{DistanceFog, FogFalloff},
    prelude::*,
};

pub struct SkyPlugin;

impl Plugin for SkyPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (setup_sun, spawn_sky_dome))
           .add_systems(Update, attach_fog_to_camera);
    }
}

// ---------------------------------------------------------------------------
// Colours
// ---------------------------------------------------------------------------

/// Sky zenith (top of dome) – deep blue.
const ZENITH: [f32; 4]  = [0.10, 0.20, 0.55, 1.0];
/// Sky horizon – hazy pale blue matching fog colour.
const HORIZON: [f32; 4] = [0.62, 0.76, 0.90, 1.0];

/// Fog / horizon colour (same as HORIZON so the blend is seamless).
fn fog_color() -> Color {
    Color::srgb(HORIZON[0], HORIZON[1], HORIZON[2])
}

// ---------------------------------------------------------------------------
// Sun + ambient
// ---------------------------------------------------------------------------

fn setup_sun(mut commands: Commands, mut ambient: ResMut<GlobalAmbientLight>) {
    // Midday-ish sun: 10 000 lx, shadows on, bias tuned to avoid acne on a
    // 200 m terrain grid without Peter-Panning.
    commands.spawn((
        DirectionalLight {
            illuminance: 10_000.0,
            shadows_enabled: true,
            shadow_depth_bias: 0.04,
            shadow_normal_bias: 2.0,
            color: Color::srgb(1.0, 0.97, 0.88), // warm white
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.9, 0.5, 0.0)),
    ));

    // Sky-bounce ambient – slight cool blue.
    ambient.color      = Color::srgb(0.5, 0.55, 0.65);
    ambient.brightness = 200.0;
}

// ---------------------------------------------------------------------------
// Sky dome
// ---------------------------------------------------------------------------

fn spawn_sky_dome(
    mut commands:  Commands,
    mut meshes:    ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mesh = build_sky_dome(32, 16, 900.0);
    let mesh_handle = meshes.add(mesh);

    let mat = materials.add(StandardMaterial {
        base_color: Color::WHITE, // vertex colors do the tinting
        unlit: true,
        cull_mode: None,          // render from inside the dome
        ..default()
    });

    commands.spawn((
        Mesh3d(mesh_handle),
        MeshMaterial3d(mat),
        Transform::default(),
        // No physics — it's pure visual.
    ));
}

/// Build an inverted UV-sphere whose vertices are coloured by a
/// zenith→horizon gradient keyed on the cosine of the elevation angle.
///
/// `radius`   – large enough to contain the whole scene.
/// `stacks`   – latitude subdivisions (more = smoother gradient).
/// `slices`   – longitude subdivisions.
fn build_sky_dome(slices: u32, stacks: u32, radius: f32) -> Mesh {
    use std::f32::consts::PI;

    let mut positions:  Vec<[f32; 3]> = Vec::new();
    let mut normals:    Vec<[f32; 3]> = Vec::new();
    let mut uvs:        Vec<[f32; 2]> = Vec::new();
    let mut colors:     Vec<[f32; 4]> = Vec::new();
    let mut indices:    Vec<u32>       = Vec::new();

    for stack in 0..=stacks {
        let phi = PI * stack as f32 / stacks as f32; // 0 (top) … π (bottom)
        let y   = phi.cos();  // 1 at top, -1 at bottom
        let r   = phi.sin();  // ring radius

        // Map y (-1..1) to gradient: y=1 → zenith, y=-1 → horizon.
        // We only draw the upper hemisphere + a bit below horizon.
        let t = ((y + 1.0) * 0.5).clamp(0.0, 1.0); // 1 at top, 0 at bottom
        let color = lerp_color(&HORIZON, &ZENITH, smooth_step(t));

        for slice in 0..=slices {
            let theta = 2.0 * PI * slice as f32 / slices as f32;
            let px = r * theta.cos() * radius;
            let py = y * radius;
            let pz = r * theta.sin() * radius;

            positions.push([px, py, pz]);
            // Inward normals (inverted sphere – we're inside it).
            let n = Vec3::new(px, py, pz).normalize();
            normals.push([-n.x, -n.y, -n.z]);
            uvs.push([slice as f32 / slices as f32, stack as f32 / stacks as f32]);
            colors.push(color);
        }
    }

    // Build quads, winding reversed so faces point inward.
    let ring = slices + 1;
    for stack in 0..stacks {
        for slice in 0..slices {
            let tl = stack * ring + slice;
            let tr = tl + 1;
            let bl = tl + ring;
            let br = bl + 1;
            // Reversed winding for inward-facing normals.
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

// ---------------------------------------------------------------------------
// Fog — attached to Camera3d once it appears
// ---------------------------------------------------------------------------

/// Component marker so we don't attach fog twice.
#[derive(Component)]
struct FogAttached;

fn attach_fog_to_camera(
    mut commands: Commands,
    cameras: Query<Entity, (Added<Camera3d>, Without<FogAttached>)>,
) {
    for entity in &cameras {
        commands.entity(entity).insert((
            DistanceFog {
                color: fog_color(),
                falloff: FogFalloff::Linear { start: 80.0, end: 250.0 },
                directional_light_color: Color::srgb(1.0, 0.95, 0.75),
                directional_light_exponent: 16.0,
                ..default()
            },
            FogAttached,
        ));
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn lerp_color(a: &[f32; 4], b: &[f32; 4], t: f32) -> [f32; 4] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
        1.0,
    ]
}

/// Smooth cubic ease-in/ease-out (Ken Perlin's smoothstep).
fn smooth_step(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}
