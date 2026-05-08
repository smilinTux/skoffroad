// Landmarks: 3 tall procedural structures visible from anywhere on the 200 m map.
//   • Water tower  at (-80, _, -80) — ~16 m tall, rusted-steel cylinders
//   • Lighthouse   at ( 90, _,  95) — ~18 m tall, red/white striped tower + glowing cap
//   • Radio tower  at (-95, _,  75) — ~25 m tall, lattice legs + blinking red beacon
//
// Each landmark is a static collider (bounding-box cuboid on the parent).
// The radio-tower beacon pulses via an Update system.
//
// Public API:
//   LandmarksPlugin

use bevy::prelude::*;
use avian3d::prelude::*;

use crate::terrain::terrain_height_at;

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct LandmarksPlugin;

impl Plugin for LandmarksPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_landmarks)
           .add_systems(Update, pulse_beacon);
    }
}

// ---------------------------------------------------------------------------
// Marker components
// ---------------------------------------------------------------------------

/// Root entity of each landmark.
#[derive(Component)]
pub struct Landmark;

/// Marks the blinking light on the radio tower.
#[derive(Component)]
pub struct LandmarkBeacon;

// ---------------------------------------------------------------------------
// Spawn system
// ---------------------------------------------------------------------------

fn spawn_landmarks(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // ---- Water tower -------------------------------------------------------
    let wt_ground = terrain_height_at(-80.0, -80.0);
    spawn_water_tower(
        &mut commands,
        &mut meshes,
        &mut materials,
        Vec3::new(-80.0, wt_ground, -80.0),
    );

    // ---- Lighthouse --------------------------------------------------------
    let lh_ground = terrain_height_at(90.0, 95.0);
    spawn_lighthouse(
        &mut commands,
        &mut meshes,
        &mut materials,
        Vec3::new(90.0, lh_ground, 95.0),
    );

    // ---- Radio tower -------------------------------------------------------
    let rt_ground = terrain_height_at(-95.0, 75.0);
    spawn_radio_tower(
        &mut commands,
        &mut meshes,
        &mut materials,
        Vec3::new(-95.0, rt_ground, 75.0),
    );

    bevy::log::info!("landmarks: water tower, lighthouse, radio tower spawned");
}

// ---------------------------------------------------------------------------
// Water tower (~16 m tall)
// ---------------------------------------------------------------------------
//
// Layout (Y measured from ground level = 0):
//   4 legs: Cylinder(r=0.4, h=8)  centres at (±2, 4, ±2)
//   Tank:   Cylinder(r=4, h=4)    centre at (0, 10, 0)  [rests on leg tops at 8 + half 4 = 10]
//   Roof:   Cylinder(r=0.5, h=1)  centre at (0, 12.5, 0)
// Total height ≈ 13 m (roof top); bounding box half-extents: (4, 6.5, 4)

fn spawn_water_tower(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    origin: Vec3,
) {
    // Material — rusted steel
    let mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.55, 0.45, 0.35),
        perceptual_roughness: 0.95,
        metallic: 0.15,
        ..default()
    });

    // Meshes
    let leg_mesh  = meshes.add(Cylinder::new(0.4, 8.0));
    let tank_mesh = meshes.add(Cylinder::new(4.0, 4.0));
    let roof_mesh = meshes.add(Cylinder::new(0.5, 1.0));

    // Bounding box: wide as tank (r=4 → half=4), height 13 m (half 6.5),
    // offset the collider centre up by 6.5 from origin so its bottom is at ground.
    let bbox_half_w = 4.0_f32;
    let total_h     = 13.0_f32;
    let half_h      = total_h * 0.5;

    let parent = commands
        .spawn((
            Landmark,
            Transform::from_translation(origin),
            Visibility::default(),
            RigidBody::Static,
            Collider::cuboid(bbox_half_w, half_h, bbox_half_w),
            // Shift collider centre to the geometric middle of the tower
            ColliderTransform {
                translation: Vec3::new(0.0, half_h, 0.0),
                ..default()
            },
        ))
        .id();

    // 4 legs at corners of a 4×4 square, centre at Y=4 (half of 8 m)
    let leg_offsets = [
        Vec3::new( 2.0, 4.0,  2.0),
        Vec3::new(-2.0, 4.0,  2.0),
        Vec3::new( 2.0, 4.0, -2.0),
        Vec3::new(-2.0, 4.0, -2.0),
    ];

    let mut children: Vec<Entity> = Vec::new();

    for off in &leg_offsets {
        let e = commands
            .spawn((
                Mesh3d(leg_mesh.clone()),
                MeshMaterial3d(mat.clone()),
                Transform::from_translation(*off),
            ))
            .id();
        children.push(e);
    }

    // Tank top: sits on legs at Y=8, half-height 2, so centre at Y=10
    let tank = commands
        .spawn((
            Mesh3d(tank_mesh.clone()),
            MeshMaterial3d(mat.clone()),
            Transform::from_translation(Vec3::new(0.0, 10.0, 0.0)),
        ))
        .id();
    children.push(tank);

    // Roof: small cylinder on top of tank (tank top = Y=12, half-roof = 0.5 → centre Y=12.5)
    let roof = commands
        .spawn((
            Mesh3d(roof_mesh.clone()),
            MeshMaterial3d(mat.clone()),
            Transform::from_translation(Vec3::new(0.0, 12.5, 0.0)),
        ))
        .id();
    children.push(roof);

    commands.entity(parent).add_children(&children);
}

// ---------------------------------------------------------------------------
// Lighthouse (~18 m tall)
// ---------------------------------------------------------------------------
//
// Layout (Y from ground):
//   Base:     Cylinder(r=3, h=4)    centre at (0, 2,    0)  cream
//   Section0: Cylinder(r=2, h=4)    centre at (0, 6,    0)  red
//   Section1: Cylinder(r=1.7, h=4)  centre at (0, 10,   0)  white
//   Section2: Cylinder(r=1.4, h=4)  centre at (0, 14,   0)  red
//   Section3: Cylinder(r=1.0, h=4)  centre at (0, 18,   0)  white
//   Glass:    Cylinder(r=0.9, h=1)  centre at (0, 20.5, 0)  yellow emissive
//   Cap:      Cylinder(r=0.6, h=0.5)centre at (0, 21.25,0)  black
// Total ≈ 21.5 m; bounding box: (3, 10.75, 3)

fn spawn_lighthouse(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    origin: Vec3,
) {
    let cream_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.90, 0.85, 0.75),
        perceptual_roughness: 0.88,
        ..default()
    });
    let red_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.85, 0.15, 0.10),
        perceptual_roughness: 0.85,
        ..default()
    });
    let white_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.95, 0.95, 0.90),
        perceptual_roughness: 0.85,
        ..default()
    });
    let glass_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.95, 0.6),
        emissive: LinearRgba::rgb(2.0, 1.9, 1.0),
        perceptual_roughness: 0.1,
        ..default()
    });
    let cap_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.05, 0.05, 0.05),
        perceptual_roughness: 0.90,
        ..default()
    });

    let base_mesh  = meshes.add(Cylinder::new(3.0,  4.0));
    let sec0_mesh  = meshes.add(Cylinder::new(2.0,  4.0));
    let sec1_mesh  = meshes.add(Cylinder::new(1.7,  4.0));
    let sec2_mesh  = meshes.add(Cylinder::new(1.4,  4.0));
    let sec3_mesh  = meshes.add(Cylinder::new(1.0,  4.0));
    let glass_mesh = meshes.add(Cylinder::new(0.9,  1.0));
    let cap_mesh   = meshes.add(Cylinder::new(0.6,  0.5));

    let total_h = 21.5_f32;
    let half_h  = total_h * 0.5;

    let parent = commands
        .spawn((
            Landmark,
            Transform::from_translation(origin),
            Visibility::default(),
            RigidBody::Static,
            Collider::cuboid(3.0, half_h, 3.0),
            ColliderTransform {
                translation: Vec3::new(0.0, half_h, 0.0),
                ..default()
            },
        ))
        .id();

    let base = commands.spawn((
        Mesh3d(base_mesh),
        MeshMaterial3d(cream_mat),
        Transform::from_translation(Vec3::new(0.0, 2.0, 0.0)),
    )).id();

    let sec0 = commands.spawn((
        Mesh3d(sec0_mesh),
        MeshMaterial3d(red_mat.clone()),
        Transform::from_translation(Vec3::new(0.0, 6.0, 0.0)),
    )).id();

    let sec1 = commands.spawn((
        Mesh3d(sec1_mesh),
        MeshMaterial3d(white_mat.clone()),
        Transform::from_translation(Vec3::new(0.0, 10.0, 0.0)),
    )).id();

    let sec2 = commands.spawn((
        Mesh3d(sec2_mesh),
        MeshMaterial3d(red_mat),
        Transform::from_translation(Vec3::new(0.0, 14.0, 0.0)),
    )).id();

    let sec3 = commands.spawn((
        Mesh3d(sec3_mesh),
        MeshMaterial3d(white_mat),
        Transform::from_translation(Vec3::new(0.0, 18.0, 0.0)),
    )).id();

    let glass = commands.spawn((
        Mesh3d(glass_mesh),
        MeshMaterial3d(glass_mat),
        Transform::from_translation(Vec3::new(0.0, 20.5, 0.0)),
    )).id();

    let cap = commands.spawn((
        Mesh3d(cap_mesh),
        MeshMaterial3d(cap_mat),
        Transform::from_translation(Vec3::new(0.0, 21.25, 0.0)),
    )).id();

    commands.entity(parent).add_children(&[base, sec0, sec1, sec2, sec3, glass, cap]);
}

// ---------------------------------------------------------------------------
// Radio tower (~25 m tall)
// ---------------------------------------------------------------------------
//
// Layout (Y from ground):
//   4 vertical legs: Cylinder(r=0.2, h=25) at corners of a 3×3 sq, centre Y=12.5
//   5 horizontal trusses: Cuboid(3.0, 0.2, 0.2) at Y = 5, 10, 15, 20, 24
//   Beacon: Cylinder(r=0.4, h=0.4) at Y=25.2, emissive red — has LandmarkBeacon marker
// Total ≈ 25.4 m; bounding box: (1.5, 12.7, 1.5)

fn spawn_radio_tower(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    origin: Vec3,
) {
    let frame_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.45, 0.45, 0.50),
        perceptual_roughness: 0.80,
        metallic: 0.40,
        ..default()
    });
    let beacon_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.1, 0.1),
        emissive: LinearRgba::rgb(4.0, 0.2, 0.2),
        perceptual_roughness: 0.1,
        ..default()
    });

    let leg_mesh    = meshes.add(Cylinder::new(0.2, 25.0));
    let truss_mesh  = meshes.add(Cuboid::new(3.0, 0.2, 0.2));
    let beacon_mesh = meshes.add(Cylinder::new(0.4, 0.4));

    let total_h = 25.4_f32;
    let half_h  = total_h * 0.5;

    let parent = commands
        .spawn((
            Landmark,
            Transform::from_translation(origin),
            Visibility::default(),
            RigidBody::Static,
            Collider::cuboid(1.5, half_h, 1.5),
            ColliderTransform {
                translation: Vec3::new(0.0, half_h, 0.0),
                ..default()
            },
        ))
        .id();

    let mut children: Vec<Entity> = Vec::new();

    // 4 legs at corners of 3×3 square, centre Y=12.5
    let leg_corners = [
        Vec3::new( 1.5, 12.5,  1.5),
        Vec3::new(-1.5, 12.5,  1.5),
        Vec3::new( 1.5, 12.5, -1.5),
        Vec3::new(-1.5, 12.5, -1.5),
    ];
    for off in &leg_corners {
        let e = commands
            .spawn((
                Mesh3d(leg_mesh.clone()),
                MeshMaterial3d(frame_mat.clone()),
                Transform::from_translation(*off),
            ))
            .id();
        children.push(e);
    }

    // 5 horizontal trusses
    for &h in &[5.0_f32, 10.0, 15.0, 20.0, 24.0] {
        let e = commands
            .spawn((
                Mesh3d(truss_mesh.clone()),
                MeshMaterial3d(frame_mat.clone()),
                Transform::from_translation(Vec3::new(0.0, h, 0.0)),
            ))
            .id();
        children.push(e);
    }

    // Beacon on top (LandmarkBeacon marker for pulsing system)
    let beacon = commands
        .spawn((
            LandmarkBeacon,
            Mesh3d(beacon_mesh),
            MeshMaterial3d(beacon_mat),
            Transform::from_translation(Vec3::new(0.0, 25.2, 0.0)),
        ))
        .id();
    children.push(beacon);

    commands.entity(parent).add_children(&children);
}

// ---------------------------------------------------------------------------
// Pulsing beacon system (Update)
// ---------------------------------------------------------------------------

fn pulse_beacon(
    time: Res<Time>,
    mut query: Query<&mut Transform, With<LandmarkBeacon>>,
) {
    let t = time.elapsed_secs();
    // Gentle sin pulse: scale Y between 0.85 and 1.15 at ~1 Hz
    let scale = 1.0 + 0.15 * (t * std::f32::consts::TAU).sin();
    for mut transform in query.iter_mut() {
        transform.scale = Vec3::new(1.0, scale, 1.0);
    }
}
