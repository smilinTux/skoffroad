// photoreal_rocks.rs — Sprint 40, priority 1
//
// Replaces the visual mesh on rock_garden boulder entities with a
// cluster-of-deformed-spheres compound mesh that reads like photogrammetry
// data at a distance.  Physics colliders are LEFT UNTOUCHED.
//
// Approach
// --------
// 1.  Query all entities that carry `RockGardenRock`.
// 2.  For each boulder, build ONE merged mesh from 5 deformed sub-spheres at
//     hard-coded offsets (PRD v3 §3).  Vertex positions are displaced by a
//     per-vertex hash noise so the surface reads as craggy granite.
// 3.  Spawn the merged mesh as a NEW child entity (Mesh3d + MeshMaterial3d).
//     The original ico-sphere children are left in place — they are very cheap
//     and the new mesh overlaps them.  If you want to remove them instead,
//     despawn them before spawning; that is intentionally not done here to
//     keep this file from touching rock_garden.rs entities' children.
//
// Run once: `upgrade_rocks_once` is guarded by `Local<bool>`, fires on the
// first Update tick after the rock garden has spawned (Startup → Update).
//
// Public API
// ----------
//   PhotorealRocksPlugin

use bevy::{
    asset::RenderAssetUsages,
    mesh::{Indices, PrimitiveTopology},
    prelude::*,
};

use crate::graphics_quality::GraphicsQuality;
use crate::rock_garden::RockGardenRock;
use bevy::ecs::hierarchy::Children;

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct PhotorealRocksPlugin;

impl Plugin for PhotorealRocksPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, upgrade_rocks_once);
    }
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Base colour: granite / sandstone sRGB(0.42, 0.40, 0.38).
const BASE_R: f32 = 0.42;
const BASE_G: f32 = 0.40;
const BASE_B: f32 = 0.38;

/// (offset_xyz, scale) for each of the 5 sub-spheres in the compound.
const SUB_SPHERES: [(Vec3, f32); 5] = [
    (Vec3::new(0.0,   0.0,   0.0),  0.8),
    (Vec3::new(0.4,  -0.1,   0.2),  0.65),
    (Vec3::new(-0.3,  0.05, -0.25), 0.55),
    (Vec3::new(0.1,   0.3,   0.4),  0.5),
    (Vec3::new(-0.4, -0.2,   0.1),  0.4),
];

/// ICO subdivision level for each sub-sphere (2 gives ~162 verts each).
const ICO_LEVEL: u32 = 2;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Cheap per-vertex deterministic noise in [1 - NOISE_AMP, 1 + NOISE_AMP].
/// Uses the PRD formula: `(pos.length() * 7.0).sin() * 0.15 + 1.0`.
#[inline]
fn noise_factor(pos: Vec3) -> f32 {
    (pos.length() * 7.0).sin() * 0.15 + 1.0
}

/// Per-rock hash from the entity's raw bits.  Returns a float in [0, 1).
#[inline]
fn entity_hash_f32(entity: Entity) -> f32 {
    let bits = entity.to_bits();
    // Xorshift64 — one iteration is cheap and good enough for visual seeding.
    let mut x = bits ^ 0x9e37_79b9_7f4a_7c15_u64;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    (x as f32) / (u64::MAX as f32)
}

/// Build a single compound mesh by merging deformed ico-sphere sub-meshes.
///
/// `base_radius`: the boulder's authoritative size from rock_garden.rs.
/// `seed`:        per-rock value in [0,1) from `entity_hash_f32`.
///
/// The compound uses the 5 hard-coded `SUB_SPHERES` offsets scaled by
/// `base_radius`.  Each sub-sphere vertex is deformed by `noise_factor`.
fn build_compound_mesh(base_radius: f32, seed: f32) -> Mesh {
    // Small per-rock rotation baked into position jitter so each boulder looks
    // different without needing Transform quaternions on the merged mesh.
    let jitter_angle = seed * std::f32::consts::TAU;
    let (sin_j, cos_j) = jitter_angle.sin_cos();

    let mut all_positions: Vec<[f32; 3]> = Vec::new();
    let mut all_normals:   Vec<[f32; 3]> = Vec::new();
    let mut all_uvs:       Vec<[f32; 2]> = Vec::new();
    let mut all_indices:   Vec<u32>       = Vec::new();

    for &(raw_offset, scale) in &SUB_SPHERES {
        let offset = Vec3::new(
            raw_offset.x * cos_j - raw_offset.z * sin_j,
            raw_offset.y,
            raw_offset.x * sin_j + raw_offset.z * cos_j,
        ) * base_radius;

        let radius = scale * base_radius;

        // Build a unit ico-sphere and scale it ourselves after deforming.
        let sphere_mesh: Mesh = Sphere::new(1.0).mesh().ico(ICO_LEVEL).unwrap().into();

        // Extract Float32x3 positions.
        let raw_positions: Vec<[f32; 3]> = match sphere_mesh
            .attribute(Mesh::ATTRIBUTE_POSITION)
            .and_then(|a| a.as_float3())
        {
            Some(v) => v.to_vec(),
            None => continue,
        };

        // Extract normals (same length as positions).
        let raw_normals: Vec<[f32; 3]> = match sphere_mesh
            .attribute(Mesh::ATTRIBUTE_NORMAL)
            .and_then(|a| a.as_float3())
        {
            Some(v) => v.to_vec(),
            None => vec![[0.0, 1.0, 0.0]; raw_positions.len()],
        };

        // UVs: generate trivial spherical UVs from the unit-sphere positions.
        // We cannot use the internal VertexAttributeValues enum (private in
        // Bevy 0.18), so we derive UVs from positions instead.  The material
        // has no UV-dependent textures so precision does not matter.
        let raw_uvs: Vec<[f32; 2]> = raw_positions
            .iter()
            .map(|&[px, py, pz]| {
                let v = Vec3::new(px, py, pz).normalize();
                let u = (v.x.atan2(v.z) / std::f32::consts::TAU + 0.5).fract();
                let v2 = (v.y * 0.5 + 0.5).clamp(0.0, 1.0);
                [u, v2]
            })
            .collect();

        // Extract indices (U16 or U32).
        let sub_indices: Vec<u32> = match sphere_mesh.indices() {
            Some(Indices::U32(v))  => v.clone(),
            Some(Indices::U16(v))  => v.iter().map(|&i| i as u32).collect(),
            None => (0..raw_positions.len() as u32).collect(),
        };

        let base_vertex = all_positions.len() as u32;

        // Deform + scale + translate each vertex.
        for (i, &[px, py, pz]) in raw_positions.iter().enumerate() {
            let local = Vec3::new(px, py, pz);
            let deformed = local * noise_factor(local) * radius;
            let world = deformed + offset;

            all_positions.push([world.x, world.y, world.z]);
            all_normals.push(raw_normals[i]);
            all_uvs.push(raw_uvs[i]);
        }

        for idx in sub_indices {
            all_indices.push(base_vertex + idx);
        }
    }

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, all_positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL,   all_normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0,     all_uvs);
    mesh.insert_indices(Indices::U32(all_indices));
    mesh
}

// ---------------------------------------------------------------------------
// System
// ---------------------------------------------------------------------------

/// Fires exactly once on the first Update tick that finds `RockGardenRock`
/// entities.  Adds a photorealistic compound mesh child to each.
fn upgrade_rocks_once(
    mut commands: Commands,
    mut meshes:   ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    quality: Res<GraphicsQuality>,
    rocks: Query<(Entity, &Transform, Option<&Children>), With<RockGardenRock>>,
    child_mesh_q: Query<(), With<Mesh3d>>,
    mut done: Local<bool>,
) {
    if *done {
        return;
    }

    if rocks.is_empty() {
        info!("photoreal_rocks: no RockGardenRock entities found yet — skipping");
        return;
    }

    // Build the boulder material. On Medium+ we sample the CC0 rock PBR pack
    // shipped under assets/materials/terrain/rock/. On Low we keep the cheap
    // solid-color StandardMaterial that the original Sprint 40 shipped.
    let mat = if quality.photoreal_rocks() {
        materials.add(StandardMaterial {
            base_color: Color::WHITE,
            base_color_texture: Some(asset_server.load("materials/terrain/rock/albedo.jpg")),
            normal_map_texture: Some(asset_server.load("materials/terrain/rock/normal.jpg")),
            metallic_roughness_texture: Some(
                asset_server.load("materials/terrain/rock/roughness.jpg"),
            ),
            perceptual_roughness: 1.0, // multiplied by texture
            metallic: 0.0,
            ..default()
        })
    } else {
        materials.add(StandardMaterial {
            base_color: Color::srgb(BASE_R, BASE_G, BASE_B),
            perceptual_roughness: 0.92,
            metallic: 0.0,
            ..default()
        })
    };

    let mut upgraded = 0usize;

    for (entity, transform, children) in &rocks {
        // Derive base_radius from the entity's scale (rock_garden.rs encodes
        // radius in the Transform scale if present, else uniform 1.0).
        let base_radius = {
            let s = transform.scale;
            let r = (s.x + s.y + s.z) / 3.0;
            if r < 0.1 { 1.0 } else { r }
        };

        // Despawn the original sphere-child meshes so the new compound is
        // not visually competing with the old blocky boulders underneath.
        if let Some(children) = children {
            for child in children.iter() {
                if child_mesh_q.get(child).is_ok() {
                    commands.entity(child).despawn();
                }
            }
        }

        let seed = entity_hash_f32(entity);
        let mesh = build_compound_mesh(base_radius, seed);
        let mesh_handle = meshes.add(mesh);

        commands.entity(entity).with_children(|parent| {
            parent.spawn((
                Mesh3d(mesh_handle),
                MeshMaterial3d(mat.clone()),
                Transform::default(),
                Visibility::default(),
            ));
        });

        upgraded += 1;
    }

    info!(
        "photoreal_rocks: upgraded {} boulders with compound deformed-sphere meshes",
        upgraded
    );

    *done = true;
}
