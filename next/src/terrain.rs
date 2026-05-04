use bevy::{
    asset::RenderAssetUsages,
    mesh::{Indices, PrimitiveTopology},
    prelude::*,
};
use avian3d::prelude::*;
use noise::{NoiseFn, Perlin, Fbm};

pub struct TerrainPlugin;

impl Plugin for TerrainPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_terrain);
    }
}

const GRID: usize = 128; // vertices per side (128x128 = ~16k tris)
const SIZE: f32 = 200.0; // world-space width/depth in metres
const HEIGHT_SCALE: f32 = 12.0;
pub const TERRAIN_SEED: u32 = 42;

// Sample the layered heightmap at normalised [0,1] coords.
fn sample_height(fbm: &Fbm<Perlin>, nx: f64, nz: f64) -> f32 {
    // Two octave frequencies for gentle rolling hills plus fine detail.
    let coarse = fbm.get([nx * 2.0, nz * 2.0]) as f32;
    let fine   = fbm.get([nx * 8.0, nz * 8.0]) as f32 * 0.25;
    (coarse + fine) * HEIGHT_SCALE
}

/// Public helper used by the headless harness to replicate the terrain height
/// at an arbitrary world-space (x, z) position without spawning any entities.
pub fn terrain_height_at(x: f32, z: f32) -> f32 {
    let fbm: Fbm<Perlin> = Fbm::<Perlin>::new(TERRAIN_SEED);
    let nx = (x / SIZE + 0.5) as f64;
    let nz = (z / SIZE + 0.5) as f64;
    sample_height(&fbm, nx, nz)
}

fn spawn_terrain(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let fbm: Fbm<Perlin> = Fbm::<Perlin>::new(42);

    let vcount = GRID + 1; // vertices per edge
    let mut positions: Vec<[f32; 3]> = Vec::with_capacity(vcount * vcount);
    let mut normals:   Vec<[f32; 3]> = Vec::with_capacity(vcount * vcount);
    let mut uvs:       Vec<[f32; 2]> = Vec::with_capacity(vcount * vcount);
    let mut colors:    Vec<[f32; 4]> = Vec::with_capacity(vcount * vcount);

    // Height values kept separately for collider construction.
    let mut heights: Vec<f32> = Vec::with_capacity(vcount * vcount);

    for z in 0..vcount {
        for x in 0..vcount {
            let nx = x as f64 / GRID as f64;
            let nz = z as f64 / GRID as f64;
            let h = sample_height(&fbm, nx, nz);

            let px = (x as f32 / GRID as f32 - 0.5) * SIZE;
            let pz = (z as f32 / GRID as f32 - 0.5) * SIZE;

            positions.push([px, h, pz]);
            normals.push([0.0, 1.0, 0.0]); // overwritten below
            uvs.push([nx as f32 * 8.0, nz as f32 * 8.0]);
            heights.push(h);
        }
    }

    // Smooth normals via finite differences.
    for z in 0..vcount {
        for x in 0..vcount {
            let h  = heights[z * vcount + x];
            let hx = if x + 1 < vcount { heights[z * vcount + x + 1] } else { h };
            let hz = if z + 1 < vcount { heights[(z + 1) * vcount + x] } else { h };
            let step = SIZE / GRID as f32;
            let nx_v = Vec3::new(step, hx - h, 0.0).normalize();
            let nz_v = Vec3::new(0.0, hz - h, step).normalize();
            let n = nx_v.cross(nz_v).normalize();
            normals[z * vcount + x] = [n.x, n.y, n.z];
        }
    }

    // Slope-based vertex colors:
    //   flat  (slope < 0.15) -> grass green  srgb(0.32, 0.50, 0.20)
    //   mid   (slope 0.15-0.45) -> dirt brown srgb(0.45, 0.38, 0.25)
    //   steep (slope > 0.45) -> rock grey    srgb(0.42, 0.42, 0.45)
    // Smooth-stepped to avoid harsh banding.
    const GRASS: [f32; 3] = [0.32, 0.50, 0.20];
    const DIRT:  [f32; 3] = [0.45, 0.38, 0.25];
    const ROCK:  [f32; 3] = [0.42, 0.42, 0.45];

    for i in 0..(vcount * vcount) {
        let [nx, ny, nz] = normals[i];
        let normal = Vec3::new(nx, ny, nz);
        // slope = 0 on flat ground, 1 on vertical face.
        let slope = 1.0 - normal.dot(Vec3::Y).clamp(0.0, 1.0);

        // Blend grass->dirt over slope range 0.10..0.25, dirt->rock over 0.30..0.55
        let t_gd = slope_smooth_step(slope, 0.10, 0.25);
        let t_dr = slope_smooth_step(slope, 0.30, 0.55);

        let c = lerp3(lerp3(GRASS, DIRT, t_gd), ROCK, t_dr);
        colors.push([c[0], c[1], c[2], 1.0]);
    }

    let mut indices: Vec<u32> = Vec::with_capacity(GRID * GRID * 6);
    for z in 0..GRID {
        for x in 0..GRID {
            let tl = (z * vcount + x) as u32;
            let tr = tl + 1;
            let bl = ((z + 1) * vcount + x) as u32;
            let br = bl + 1;
            // Two triangles per quad.
            indices.extend_from_slice(&[tl, bl, tr, tr, bl, br]);
        }
    }

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh.insert_indices(Indices::U32(indices));

    let mesh_handle = meshes.add(mesh);

    // base_color WHITE so vertex colors aren't tinted (Bevy 0.18 samples
    // ATTRIBUTE_COLOR automatically when present on the mesh).
    let material = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        perceptual_roughness: 0.9,
        ..default()
    });

    commands.spawn((
        Mesh3d(mesh_handle.clone()),
        MeshMaterial3d(material),
        Transform::default(),
        RigidBody::Static,
        ColliderConstructor::TrimeshFromMesh,
    ));
}

// ---------------------------------------------------------------------------
// Colour helpers (used only during mesh build, no runtime cost)
// ---------------------------------------------------------------------------

/// Smooth cubic ease mapping a value in [lo, hi] to [0, 1].
fn slope_smooth_step(x: f32, lo: f32, hi: f32) -> f32 {
    let t = ((x - lo) / (hi - lo)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Linear interpolate between two RGB triples.
fn lerp3(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}
