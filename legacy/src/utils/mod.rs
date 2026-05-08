use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use crate::physics::Terrain;
use rand::Rng;

pub fn create_terrain_mesh(
    width: usize,
    depth: usize,
    height_map: &[f32],
) -> Mesh {
    let mut vertices = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut indices = Vec::new();

    for z in 0..depth {
        for x in 0..width {
            let height = height_map[z * width + x];
            vertices.push([x as f32, height, z as f32]);
            normals.push([0.0, 1.0, 0.0]);
            uvs.push([x as f32 / width as f32, z as f32 / depth as f32]);
        }
    }

    for z in 0..depth - 1 {
        for x in 0..width - 1 {
            let top_left = z * width + x;
            let top_right = top_left + 1;
            let bottom_left = (z + 1) * width + x;
            let bottom_right = bottom_left + 1;

            indices.push(top_left as u32);
            indices.push(bottom_left as u32);
            indices.push(top_right as u32);
            indices.push(top_right as u32);
            indices.push(bottom_left as u32);
            indices.push(bottom_right as u32);
        }
    }

    let mut mesh = Mesh::new(bevy::render::mesh::PrimitiveTopology::TriangleList);
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.set_indices(Some(bevy::render::mesh::Indices::U32(indices)));

    mesh
}

pub fn generate_height_map(width: usize, depth: usize, _seed: u32) -> Vec<f32> {
    let mut heights = vec![0.0; width * depth];
    let mut rng = rand::thread_rng();

    // Simple random terrain generation
    for x in 0..width {
        for z in 0..depth {
            let height = rng.gen_range(-1.0..1.0);
            heights[x + z * width] = height;
        }
    }

    heights
}

pub fn get_terrain_height(terrain: &Terrain, _position: Vec3) -> f32 {
    // For now, just return a flat terrain at the specified height
    terrain.height
}

pub fn create_vehicle_collider(vehicle_type: &str) -> Collider {
    match vehicle_type {
        "truck" => Collider::cuboid(1.0, 1.0, 2.0),
        "buggy" => Collider::cuboid(0.8, 0.8, 1.5),
        _ => Collider::cuboid(0.5, 0.5, 1.0),
    }
}

pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

pub fn smooth_damp(
    current: f32,
    target: f32,
    current_velocity: &mut f32,
    smooth_time: f32,
    delta_time: f32,
) -> f32 {
    let smooth_time = smooth_time.max(0.0001);
    let omega = 2.0 / smooth_time;
    let x = omega * delta_time;
    let exp = 1.0 / (1.0 + x + 0.48 * x * x + 0.235 * x * x * x);
    let change = current - target;
    let temp = (*current_velocity + omega * change) * delta_time;
    *current_velocity = (*current_velocity - omega * temp) * exp;
    target + (change + temp) * exp
} 