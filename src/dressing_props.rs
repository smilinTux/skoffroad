// dressing_props.rs -- Sprint 87
//
// Extra off-road-park prop variety: tire stacks, hay bales, traffic cones,
// jersey barriers, oil drums, and wooden pallets.  Scattered across the map
// at sensible positions, each with a PropLod component for distance culling.
//
// Prop counts are tier-scaled:
//   Low    -- 50% of Medium counts (fewest props)
//   Medium -- baseline counts
//   High   -- 150% of Medium counts (most variety)
//
// All props get plain-colored materials on Low (PropTextures absent).
// On Medium/High the relevant surface textures from PropTextures are applied
// via base_color_texture on the material.
//
// Subtle scale+color jitter is applied so repeated props don't look cloned.
//
// Public API
//   DressingPropsPlugin

use bevy::prelude::*;
use avian3d::prelude::*;

use crate::terrain::terrain_height_at;
use crate::prop_lod::PropLod;
use crate::graphics_quality::GraphicsQuality;
use crate::prop_textures::PropTextures;
use crate::map_dressing::MapProp;

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct DressingPropsPlugin;

impl Plugin for DressingPropsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_dressing_props);
    }
}

// ---------------------------------------------------------------------------
// Colour constants
// ---------------------------------------------------------------------------

const RUBBER_DARK:  Color = Color::srgb(0.07, 0.06, 0.05);
const HAY_COLOR:    Color = Color::srgb(0.72, 0.60, 0.28);
const CONE_ORANGE:  Color = Color::srgb(0.90, 0.38, 0.05);
const CONE_WHITE:   Color = Color::srgb(0.92, 0.90, 0.88);
const JERSEY_GREY:  Color = Color::srgb(0.60, 0.58, 0.56);
const DRUM_RED:     Color = Color::srgb(0.62, 0.12, 0.08);
const DRUM_METAL:   Color = Color::srgb(0.48, 0.48, 0.52);
const PALLET_WOOD:  Color = Color::srgb(0.52, 0.38, 0.22);

// ---------------------------------------------------------------------------
// Prop placement tables
//
// (x, z, yaw_degrees)   -- Y is always terrain-snapped
// ---------------------------------------------------------------------------

// Tire stacks: scattered around hillclimb approach and obstacle course.
const TIRE_STACK_POSITIONS: [(f32, f32, f32); 8] = [
    (-145.0, -170.0,  0.0),
    (-140.0, -175.0, 45.0),
    (-55.0,  195.0,  15.0),
    (-50.0,  205.0, -20.0),
    (  5.0,  -50.0,  30.0),
    ( 10.0,   60.0,   0.0),
    ( 95.0,   -5.0,  10.0),
    ( 85.0,   10.0, -15.0),
];

// Hay bales: near trail markers and spawn area.
const HAY_BALE_POSITIONS: [(f32, f32, f32); 6] = [
    ( 20.0,  12.0,  0.0),
    ( 55.0,   6.0, 20.0),
    ( 80.0,  10.0,  0.0),
    (-25.0,  -5.0, 45.0),
    (-35.0,  -8.0,  0.0),
    ( 30.0, -25.0, 15.0),
];

// Traffic cones: lining approach to obstacle course and hillclimb start.
const CONE_POSITIONS: [(f32, f32, f32); 10] = [
    (-62.0, 200.0,  0.0),
    (-62.0, 215.0,  0.0),
    (-62.0, 230.0,  0.0),
    (-62.0, 245.0,  0.0),
    (-62.0, 260.0,  0.0),
    (-148.0, -182.0, 0.0),
    (-148.0, -195.0, 0.0),
    (-148.0, -210.0, 0.0),
    (-148.0, -225.0, 0.0),
    (-148.0, -240.0, 0.0),
];

// Jersey barriers: at the rock crawl entry and obstacle course ends.
const JERSEY_BARRIER_POSITIONS: [(f32, f32, f32); 4] = [
    (118.0,  -3.0,  0.0),
    (118.0,   3.0,  0.0),
    ( 58.0, -123.0, 0.0),
    ( 58.0, -117.0, 0.0),
];

// Oil drums: near the spawn area ranger hut and gas stations.
const OIL_DRUM_POSITIONS: [(f32, f32, f32); 6] = [
    (-18.0, -18.0, 0.0),
    (-15.0, -18.0, 0.0),
    (-12.0, -18.0, 0.0),
    (  2.0,   5.0, 30.0),
    (  5.0,   5.0,  0.0),
    (-28.0,  -3.0, 15.0),
];

// Wooden pallets: stacked near spawn area and trailhead markers.
const PALLET_POSITIONS: [(f32, f32, f32); 5] = [
    (-22.0, -20.0,  0.0),
    ( 18.0,  16.0, 45.0),
    ( 50.0,  12.0,  0.0),
    ( -8.0,  -5.0, 90.0),
    (-30.0,  12.0,  0.0),
];

// ---------------------------------------------------------------------------
// Spawn system
// ---------------------------------------------------------------------------

fn spawn_dressing_props(
    mut commands:  Commands,
    mut meshes:    ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    quality:       Res<GraphicsQuality>,
    prop_textures: Option<Res<PropTextures>>,
) {
    let is_low = matches!(*quality, GraphicsQuality::Low);

    // Scale factor for prop counts: Low=0.5, Medium=1.0, High=1.5 (rounded).
    let count_mul: f32 = match *quality {
        GraphicsQuality::Low    => 0.5,
        GraphicsQuality::Medium => 1.0,
        GraphicsQuality::High   => 1.5,
    };

    // Helper: how many props of a set to spawn.
    let take_n = |full: usize| -> usize {
        let n = (full as f32 * count_mul).round() as usize;
        n.max(1)
    };

    // ---- Materials -----------------------------------------------------------

    // Wood grain texture handle (Medium+).
    let wood_tex  = prop_textures.as_ref().map(|t| t.wood_grain.clone());
    let metal_tex = prop_textures.as_ref().map(|t| t.weathered_metal.clone());
    let rock_tex  = prop_textures.as_ref().map(|t| t.rough_rock.clone());

    // Rubber mat (no texture -- plain dark rubber).
    let rubber_mat = materials.add(StandardMaterial {
        base_color: RUBBER_DARK,
        perceptual_roughness: 0.98,
        metallic: 0.0,
        ..default()
    });

    // Hay bale: each bale gets its own per-instance material with colour jitter.
    // (No shared hay_mat -- see spawn loop below.)

    // Cone mat: bright orange body.
    let cone_body_mat = materials.add(StandardMaterial {
        base_color: CONE_ORANGE,
        perceptual_roughness: 0.70,
        metallic: 0.0,
        emissive: LinearRgba::rgb(0.10, 0.03, 0.0),
        ..default()
    });
    let cone_stripe_mat = materials.add(StandardMaterial {
        base_color: CONE_WHITE,
        perceptual_roughness: 0.70,
        metallic: 0.0,
        ..default()
    });

    // Jersey barrier mat: concrete grey; rock texture on Medium+.
    let jersey_mat = {
        let mut m = StandardMaterial {
            base_color: JERSEY_GREY,
            perceptual_roughness: 0.96,
            metallic: 0.0,
            ..default()
        };
        if !is_low {
            m.base_color_texture = rock_tex.clone();
        }
        materials.add(m)
    };

    // Oil drum: red steel body; metal texture on Medium+.
    let drum_body_mat = {
        let mut m = StandardMaterial {
            base_color: DRUM_RED,
            perceptual_roughness: 0.55,
            metallic: 0.30,
            ..default()
        };
        if !is_low {
            m.base_color_texture = metal_tex.clone();
        }
        materials.add(m)
    };
    let drum_lid_mat = {
        let mut m = StandardMaterial {
            base_color: DRUM_METAL,
            perceptual_roughness: 0.50,
            metallic: 0.60,
            ..default()
        };
        if !is_low {
            m.base_color_texture = metal_tex.clone();
        }
        materials.add(m)
    };

    // Pallet: rough wood; wood texture on Medium+.
    let pallet_mat = {
        let mut m = StandardMaterial {
            base_color: PALLET_WOOD,
            perceptual_roughness: 0.92,
            metallic: 0.0,
            ..default()
        };
        if !is_low {
            m.base_color_texture = wood_tex.clone();
        }
        materials.add(m)
    };

    // ---- Meshes (shared) ----------------------------------------------------

    // Tire: flattened sphere.
    let tire_mesh = meshes.add(Sphere::new(0.38));

    // Hay bale: rectangular cuboid (1.2 x 0.8 x 0.8).
    let bale_mesh = meshes.add(Cuboid::new(1.2, 0.8, 0.8));

    // Traffic cone: simple cone shape approximated by a cylinder tapering --
    // Bevy has Cone primitive.
    let cone_body_mesh   = meshes.add(Cone { radius: 0.18, height: 0.65 });
    let cone_stripe_mesh = meshes.add(Cylinder::new(0.19, 0.08));

    // Jersey barrier: K-rail shape approximated as a cuboid.
    let jersey_mesh = meshes.add(Cuboid::new(2.4, 0.9, 0.65));

    // Oil drum: cylinder (diameter 0.6, height 0.9).
    let drum_body_mesh = meshes.add(Cylinder::new(0.30, 0.82));
    let drum_lid_mesh  = meshes.add(Cylinder::new(0.31, 0.04));

    // Wooden pallet: flat cuboid with two runner beams.
    let pallet_deck_mesh   = meshes.add(Cuboid::new(1.2, 0.08, 0.90));
    let pallet_runner_mesh = meshes.add(Cuboid::new(1.2, 0.12, 0.12));

    let mut total = 0usize;

    // ---- TIRE STACKS --------------------------------------------------------
    // Each stack: 2 tires on ground side-by-side + 1 on top (pyramid).
    let tire_count = take_n(TIRE_STACK_POSITIONS.len());
    for (i, &(bx, bz, yaw_deg)) in TIRE_STACK_POSITIONS.iter().enumerate().take(tire_count) {
        let jitter = scale_jitter(i, 0.10);
        let ground  = terrain_height_at(bx, bz);
        let r       = 0.38 * (1.0 + jitter * 0.5);
        let offsets: [(f32, f32, f32); 3] = [
            (-r * 1.05, ground + r, 0.0),
            ( r * 1.05, ground + r, 0.0),
            ( 0.0,      ground + r * 3.0, 0.0),
        ];
        let base_rot = Quat::from_rotation_y(yaw_deg.to_radians());
        for (dx, ey, dz) in offsets {
            let offset = base_rot.mul_vec3(Vec3::new(dx, 0.0, dz));
            commands.spawn((
                MapProp,
                PropLod { half_height: r },
                Mesh3d(tire_mesh.clone()),
                MeshMaterial3d(rubber_mat.clone()),
                Transform::from_xyz(bx + offset.x, ey, bz + offset.z),
                RigidBody::Static,
                Collider::sphere(r),
            ));
            total += 1;
        }
    }

    // ---- HAY BALES ----------------------------------------------------------
    let bale_count = take_n(HAY_BALE_POSITIONS.len());
    for (i, &(bx, bz, yaw_deg)) in HAY_BALE_POSITIONS.iter().enumerate().take(bale_count) {
        let jitter   = scale_jitter(i, 0.08);
        let scale_x  = 1.0 + jitter;
        let scale_z  = 1.0 - jitter * 0.5;
        let half_h   = 0.40 * scale_z;
        let bale_y   = terrain_height_at(bx, bz) + half_h;
        let color_jitter = color_brightness_jitter(i, 0.12);
        let bale_color = Color::srgb(
            (HAY_COLOR.to_srgba().red   * color_jitter).clamp(0.0, 1.0),
            (HAY_COLOR.to_srgba().green * color_jitter).clamp(0.0, 1.0),
            (HAY_COLOR.to_srgba().blue  * color_jitter).clamp(0.0, 1.0),
        );
        let this_bale_mat = if !is_low {
            materials.add(StandardMaterial {
                base_color: bale_color,
                perceptual_roughness: 0.90,
                metallic: 0.0,
                base_color_texture: wood_tex.clone(),
                ..default()
            })
        } else {
            materials.add(StandardMaterial {
                base_color: bale_color,
                perceptual_roughness: 0.90,
                metallic: 0.0,
                ..default()
            })
        };
        commands.spawn((
            MapProp,
            PropLod { half_height: half_h },
            Mesh3d(bale_mesh.clone()),
            MeshMaterial3d(this_bale_mat),
            Transform {
                translation: Vec3::new(bx, bale_y, bz),
                rotation:    Quat::from_rotation_y(yaw_deg.to_radians()),
                scale:       Vec3::new(scale_x, 1.0, scale_z),
            },
            RigidBody::Static,
            Collider::cuboid(0.6 * scale_x, half_h, 0.4 * scale_z),
        ));
        total += 1;
    }

    // ---- TRAFFIC CONES ------------------------------------------------------
    let cone_count = take_n(CONE_POSITIONS.len());
    for (i, &(cx, cz, _)) in CONE_POSITIONS.iter().enumerate().take(cone_count) {
        let jitter  = scale_jitter(i, 0.06);
        let scale_s = 1.0 + jitter;
        let cone_h  = 0.65 * scale_s;
        let ground  = terrain_height_at(cx, cz);
        // Cone body: Cone primitive origin is at its centre; half-height = cone_h/2.
        let body_y  = ground + cone_h * 0.5;
        // White stripe disk sits 40% up from the base.
        let stripe_y = ground + cone_h * 0.40;
        commands.spawn((
            MapProp,
            PropLod { half_height: cone_h * 0.5 },
            Mesh3d(cone_body_mesh.clone()),
            MeshMaterial3d(cone_body_mat.clone()),
            Transform {
                translation: Vec3::new(cx, body_y, cz),
                scale:       Vec3::splat(scale_s),
                ..default()
            },
            RigidBody::Static,
            // Approximate cone with a cylinder for the collider.
            Collider::cylinder(0.12 * scale_s, cone_h * 0.5),
        ));
        // White reflective stripe.
        commands.spawn((
            MapProp,
            PropLod { half_height: 0.04 },
            Mesh3d(cone_stripe_mesh.clone()),
            MeshMaterial3d(cone_stripe_mat.clone()),
            Transform::from_xyz(cx, stripe_y, cz),
            RigidBody::Static,
            Collider::cylinder(0.19 * scale_s, 0.04),
        ));
        total += 2;
    }

    // ---- JERSEY BARRIERS ----------------------------------------------------
    let barrier_count = take_n(JERSEY_BARRIER_POSITIONS.len());
    for (i, &(jx, jz, yaw_deg)) in JERSEY_BARRIER_POSITIONS.iter().enumerate().take(barrier_count) {
        let jitter = scale_jitter(i, 0.04);
        let half_h = 0.45;
        let bary   = terrain_height_at(jx, jz) + half_h;
        commands.spawn((
            MapProp,
            PropLod { half_height: half_h },
            Mesh3d(jersey_mesh.clone()),
            MeshMaterial3d(jersey_mat.clone()),
            Transform {
                translation: Vec3::new(jx, bary, jz),
                rotation:    Quat::from_rotation_y(yaw_deg.to_radians()),
                scale:       Vec3::new(1.0 + jitter * 0.3, 1.0, 1.0),
            },
            RigidBody::Static,
            Collider::cuboid(1.2, half_h, 0.325),
        ));
        total += 1;
    }

    // ---- OIL DRUMS ----------------------------------------------------------
    let drum_count = take_n(OIL_DRUM_POSITIONS.len());
    for (i, &(dx, dz, yaw_deg)) in OIL_DRUM_POSITIONS.iter().enumerate().take(drum_count) {
        let jitter  = scale_jitter(i, 0.05);
        let scale_s = 1.0 + jitter * 0.3;
        let half_h  = 0.45 * scale_s;
        let ground  = terrain_height_at(dx, dz);
        let body_y  = ground + half_h;
        let lid_y   = ground + half_h * 2.0 + 0.02;
        let rot     = Quat::from_rotation_y(yaw_deg.to_radians());
        // Body.
        commands.spawn((
            MapProp,
            PropLod { half_height: half_h },
            Mesh3d(drum_body_mesh.clone()),
            MeshMaterial3d(drum_body_mat.clone()),
            Transform {
                translation: Vec3::new(dx, body_y, dz),
                rotation:    rot,
                scale:       Vec3::splat(scale_s),
            },
            RigidBody::Static,
            Collider::cylinder(0.30 * scale_s, half_h),
        ));
        // Lid.
        commands.spawn((
            MapProp,
            PropLod { half_height: 0.02 },
            Mesh3d(drum_lid_mesh.clone()),
            MeshMaterial3d(drum_lid_mat.clone()),
            Transform::from_xyz(dx, lid_y, dz),
            RigidBody::Static,
            Collider::cylinder(0.31 * scale_s, 0.02),
        ));
        total += 2;
    }

    // ---- WOODEN PALLETS -----------------------------------------------------
    let pallet_count = take_n(PALLET_POSITIONS.len());
    for (i, &(px, pz, yaw_deg)) in PALLET_POSITIONS.iter().enumerate().take(pallet_count) {
        let jitter    = scale_jitter(i, 0.06);
        let scale_xz  = 1.0 + jitter * 0.4;
        let ground    = terrain_height_at(px, pz);
        let deck_y    = ground + 0.12 + 0.04;  // runner height + half deck
        let runner_y  = ground + 0.06;
        let rot       = Quat::from_rotation_y(yaw_deg.to_radians());
        // Deck.
        commands.spawn((
            MapProp,
            PropLod { half_height: 0.04 },
            Mesh3d(pallet_deck_mesh.clone()),
            MeshMaterial3d(pallet_mat.clone()),
            Transform {
                translation: Vec3::new(px, deck_y, pz),
                rotation:    rot,
                scale:       Vec3::new(scale_xz, 1.0, scale_xz),
            },
            RigidBody::Static,
            Collider::cuboid(0.6 * scale_xz, 0.04, 0.45 * scale_xz),
        ));
        // Two longitudinal runner beams.
        let runner_offsets = [0.30_f32, -0.30_f32];
        for &roff in &runner_offsets {
            let roff_v = rot.mul_vec3(Vec3::new(0.0, 0.0, roff * scale_xz));
            commands.spawn((
                MapProp,
                PropLod { half_height: 0.06 },
                Mesh3d(pallet_runner_mesh.clone()),
                MeshMaterial3d(pallet_mat.clone()),
                Transform {
                    translation: Vec3::new(px + roff_v.x, runner_y, pz + roff_v.z),
                    rotation:    rot,
                    scale:       Vec3::new(scale_xz, 1.0, 1.0),
                },
                RigidBody::Static,
                Collider::cuboid(0.6 * scale_xz, 0.06, 0.06),
            ));
            total += 1;
        }
        total += 1; // deck
    }

    info!(
        "dressing_props: spawned {} props (quality={}, mul={:.1}x)",
        total,
        quality.as_str(),
        count_mul
    );
}

// ---------------------------------------------------------------------------
// Jitter helpers
// ---------------------------------------------------------------------------

/// Returns a small deterministic scale jitter in [-max, +max] for index i.
#[inline]
fn scale_jitter(i: usize, max: f32) -> f32 {
    let mut v = (i as u32).wrapping_mul(2654435761).wrapping_add(0xDEAD);
    v ^= v >> 16;
    v = v.wrapping_mul(0x45d9f3b);
    v ^= v >> 16;
    (v as f32 / u32::MAX as f32) * 2.0 * max - max
}

/// Returns a brightness multiplier in [1-max, 1+max] for colour jitter.
#[inline]
fn color_brightness_jitter(i: usize, max: f32) -> f32 {
    1.0 + scale_jitter(i.wrapping_add(77), max)
}
