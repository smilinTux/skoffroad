// Sprint 71 — Ad signage placement.
// Sprint 85 — Brand logo textures applied to sign panels (Medium+).
//
// Places five categories of in-world advertising signs, all drawing from
// the ParodyBrands catalog.  Placement is fully deterministic (fixed seeds)
// so the demo is reproducible.
//
// Sign categories and approximate counts
// ─────────────────────────────────────────────────────────────────────────
//  Roadside billboards   8  (elevated 2-post, near mode corridors)
//  Gate arch banners     4  (sponsor strips on the existing arch beams)
//  Trailside small signs 24 (staked panels along all 3 fence rows + trails)
//  Fence-line banners     8 (vinyl panels on fence-row posts)
//  Stadium expo boards    6 (cluster near spawn ranger area)
//                       ──
//  Total                50  (within the 60-100 target)
//
// Every sign gets:
//   • AdSign { brand_id }      — analytics hook
//   • RigidBody::Static        — crashable
//   • Collider                 — physics presence
//
// Panel geometry:
//   Post  = Cylinder (r=0.25, h=POST_H)
//   Panel = Cuboid (PANEL_W x PANEL_H x 0.18)  — colored with brand primary
//   Logo quad = Rectangle (PANEL_W x PANEL_H) 1 mm in front of panel face,
//               carrying the brand logo texture (Medium+).
//   Accent stripe = thin Cuboid (full width x 0.22 x 0.22) in secondary color
//
// On Low quality the logo quad is omitted; the plain colored panel remains.
//
// Public API:
//   AdSignagePlugin

use bevy::prelude::*;
use avian3d::prelude::*;

use crate::parody_brands::{AdSign, ParodyBrands, brand_hash, pick_brand};
use crate::terrain::terrain_height_at;
use crate::sponsor_scatter::SponsorAnalytics;
use crate::brand_logo_tex::{BrandLogoTextures, brand_logo_texture};

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct AdSignagePlugin;

impl Plugin for AdSignagePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (
            spawn_roadside_billboards,
            spawn_gate_sponsor_banners,
            spawn_trailside_signs,
            spawn_fence_line_banners,
            spawn_stadium_expo_boards,
        ));
    }
}

// ---------------------------------------------------------------------------
// Geometry constants
// ---------------------------------------------------------------------------

// Large roadside billboard
const BB_POST_R:    f32 = 0.25;
const BB_POST_H:    f32 = 7.0;
const BB_PANEL_W:   f32 = 7.0;
const BB_PANEL_H:   f32 = 3.5;
const BB_PANEL_D:   f32 = 0.18;
const BB_PANEL_Y:   f32 = BB_POST_H + BB_PANEL_H * 0.5; // top of post + half panel

// Gate sponsor banner (hung on existing arch beam — thin wide strip)
const GS_BANNER_W:  f32 = 7.6;
const GS_BANNER_H:  f32 = 1.2;
const GS_BANNER_D:  f32 = 0.12;

// Trailside small sign
const TS_POST_R:    f32 = 0.05;
const TS_POST_H:    f32 = 1.8;
const TS_PANEL_W:   f32 = 1.4;
const TS_PANEL_H:   f32 = 0.75;
const TS_PANEL_D:   f32 = 0.08;
const TS_PANEL_Y:   f32 = TS_POST_H + TS_PANEL_H * 0.5;

// Fence-line banner (tall narrow vinyl on fence-row posts)
const FL_BANNER_W:  f32 = 0.9;
const FL_BANNER_H:  f32 = 1.2;
const FL_BANNER_D:  f32 = 0.06;

// Stadium expo board (wider short sign, no post — mounted low on a slab)
const SE_PANEL_W:   f32 = 5.0;
const SE_PANEL_H:   f32 = 2.2;
const SE_PANEL_D:   f32 = 0.20;
const SE_SLAB_H:    f32 = 0.4;

// Accent stripe (shared)
const ACCENT_H:     f32 = 0.22;
const ACCENT_D:     f32 = 0.22;

// ---------------------------------------------------------------------------
// Logo quad helper
// ---------------------------------------------------------------------------
//
// Spawns a thin Rectangle mesh just in front of a panel face.
// The Rectangle UV layout maps [0,1]x[0,1] onto the full quad, so the brand
// logo texture fills it cleanly right-side-up.
// Returns None on Low quality (BrandLogoTextures has no handles).

fn spawn_logo_quad(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    logo_textures: &BrandLogoTextures,
    brand_id: usize,
    width: f32,
    height: f32,
    local_y: f32,
    local_z: f32,
) -> Option<Entity> {
    let tex_handle = brand_logo_texture(logo_textures, brand_id)?;

    let quad_mesh = meshes.add(Rectangle::new(width, height));
    let quad_mat  = materials.add(StandardMaterial {
        base_color_texture: Some(tex_handle),
        base_color: Color::WHITE,
        unlit: false,
        alpha_mode: AlphaMode::Opaque,
        perceptual_roughness: 0.65,
        ..default()
    });

    // Rectangle faces +Z by default. local_z should be panel_depth/2 + ~0.002
    // so the quad sits proud of the Cuboid front face with no Z-fight.
    Some(commands.spawn((
        Mesh3d(quad_mesh),
        MeshMaterial3d(quad_mat),
        Transform::from_xyz(0.0, local_y, local_z),
    )).id())
}

// Deterministic salts (each category gets its own so signs don't cluster)
const SALT_ROAD:    u32 = 0xAD51_6001;
const SALT_GATE:    u32 = 0xAD51_6002;
const SALT_TRAIL:   u32 = 0xAD51_6003;
const SALT_FENCE:   u32 = 0xAD51_6004;
const SALT_STADIUM: u32 = 0xAD51_6005;

// ---------------------------------------------------------------------------
// 1. Roadside billboards (8 signs along major driving corridors)
// ---------------------------------------------------------------------------
//
// Positions hand-placed near mode area access roads, referencing
// map_dressing.rs coordinate grid:
//   Hillclimb access   ≈ X=-150 Z=[-180…-240]
//   Rock crawl access  ≈ X=[60…120] Z=[0…-120]
//   Obstacle course    ≈ X=-60 Z=[200…260]
//   Spawn corridor     ≈ X=[-30…30] Z=[-30…30]

const ROAD_BB_POSITIONS: [(f32, f32, f32); 8] = [
    // Near hillclimb start (Z = -180)
    (-135.0, 0.0, -165.0),
    (-165.0, 0.0, -195.0),
    // Near rock crawl access
    ( 100.0, 0.0,  -15.0),
    (  65.0, 0.0, -105.0),
    // Near obstacle course
    ( -45.0, 0.0,  185.0),
    ( -75.0, 0.0,  215.0),
    // Spawn approach corridors
    (  25.0, 0.0,  -25.0),
    ( -35.0, 0.0,   30.0),
];

fn spawn_roadside_billboards(
    mut commands: Commands,
    mut meshes:   ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    brands: Res<ParodyBrands>,
    mut analytics: ResMut<SponsorAnalytics>,
    logo_textures: Option<Res<BrandLogoTextures>>,
) {
    let post_mesh   = meshes.add(Cylinder::new(BB_POST_R, BB_POST_H));
    let panel_mesh  = meshes.add(Cuboid::new(BB_PANEL_W, BB_PANEL_H, BB_PANEL_D));
    let accent_mesh = meshes.add(Cuboid::new(BB_PANEL_W, ACCENT_H, ACCENT_D));

    let post_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.22, 0.18, 0.14),
        perceptual_roughness: 0.92,
        ..default()
    });

    for (idx, &(bx, _, bz)) in ROAD_BB_POSITIONS.iter().enumerate() {
        let ground_y = terrain_height_at(bx, bz);
        let brand_r  = brand_hash(idx as i32, 7, SALT_ROAD);
        let brand_id = pick_brand(&brands, brand_r);
        let brand    = &brands.brands[brand_id];

        // Face away from origin
        let yaw = (-bx).atan2(-bz);

        let panel_mat = materials.add(StandardMaterial {
            base_color: Color::srgb(brand.primary[0], brand.primary[1], brand.primary[2]),
            perceptual_roughness: 0.80,
            ..default()
        });
        let accent_mat = materials.add(StandardMaterial {
            base_color: Color::srgb(brand.secondary[0], brand.secondary[1], brand.secondary[2]),
            emissive: LinearRgba::new(
                brand.secondary[0] * 1.2,
                brand.secondary[1] * 1.2,
                brand.secondary[2] * 1.2,
                1.0,
            ),
            perceptual_roughness: 0.55,
            ..default()
        });

        let root_y = ground_y;
        let root = commands.spawn((
            AdSign { brand_id },
            Transform::from_xyz(bx, root_y, bz)
                .with_rotation(Quat::from_rotation_y(yaw)),
            Visibility::default(),
            RigidBody::Static,
            Collider::cuboid(BB_PANEL_W * 0.5, BB_PANEL_H * 0.5, BB_PANEL_D * 0.5),
            ColliderTransform {
                translation: Vec3::new(0.0, BB_PANEL_Y, 0.0),
                ..default()
            },
            Name::new(format!("AdBillboard[{}]", brand.id)),
        )).id();

        let post = commands.spawn((
            Mesh3d(post_mesh.clone()),
            MeshMaterial3d(post_mat.clone()),
            Transform::from_xyz(0.0, BB_POST_H * 0.5, 0.0),
        )).id();

        let panel = commands.spawn((
            Mesh3d(panel_mesh.clone()),
            MeshMaterial3d(panel_mat),
            Transform::from_xyz(0.0, BB_PANEL_Y, 0.0),
        )).id();

        // Accent stripe along top edge of panel
        let accent_y = BB_PANEL_Y + BB_PANEL_H * 0.5 - ACCENT_H * 0.5;
        let accent = commands.spawn((
            Mesh3d(accent_mesh.clone()),
            MeshMaterial3d(accent_mat),
            Transform::from_xyz(0.0, accent_y, BB_PANEL_D * 0.5 + ACCENT_D * 0.5),
        )).id();

        let mut children = vec![post, panel, accent];

        // Logo quad: a Rectangle sitting just in front of the panel face (+Z).
        // The panel root is at (0, BB_PANEL_Y) in root space; the quad is at
        // the same Y offset relative to root, with Z just past the front face.
        if let Some(ref logos) = logo_textures {
            let logo_z = BB_PANEL_D * 0.5 + 0.002;
            if let Some(logo) = spawn_logo_quad(
                &mut commands,
                &mut meshes,
                &mut materials,
                logos,
                brand_id,
                BB_PANEL_W,
                BB_PANEL_H,
                BB_PANEL_Y,
                logo_z,
            ) {
                children.push(logo);
            }
        }

        commands.entity(root).add_children(&children);

        // Increment impressions counter
        *analytics.impressions.entry(brand.id.to_string()).or_insert(0) += 1;
    }

    info!("ad_signage: {} roadside billboards spawned", ROAD_BB_POSITIONS.len());
}

// ---------------------------------------------------------------------------
// 2. Gate sponsor banners (4 — hung at arch locations from banners.rs)
// ---------------------------------------------------------------------------
//
// The existing arch beam sits at Y = PILLAR_HEIGHT + BEAM_HEIGHT/2 = 8.4 m
// relative to ground.  We place a thin sponsor strip just below the beam.

const ARCH_POSITIONS: [(f32, f32); 4] = [
    (  5.0,  -5.0),   // START arch
    ( 40.0,  30.0),   // CHECKPOINT 1
    (-40.0,  50.0),   // CHECKPOINT 2
    ( 60.0, -40.0),   // FINISH arch
];

// Banner hangs a bit below the top beam of the arch
const ARCH_BANNER_Y_OFFSET: f32 = 7.2; // above terrain_height_at

fn spawn_gate_sponsor_banners(
    mut commands: Commands,
    mut meshes:   ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    brands: Res<ParodyBrands>,
    mut analytics: ResMut<SponsorAnalytics>,
    logo_textures: Option<Res<BrandLogoTextures>>,
) {
    let banner_mesh = meshes.add(Cuboid::new(GS_BANNER_W, GS_BANNER_H, GS_BANNER_D));
    let accent_mesh = meshes.add(Cuboid::new(GS_BANNER_W, ACCENT_H, ACCENT_D));

    for (idx, &(ax, az)) in ARCH_POSITIONS.iter().enumerate() {
        let ground_y = terrain_height_at(ax, az);
        let brand_r  = brand_hash(idx as i32, 3, SALT_GATE);
        let brand_id = pick_brand(&brands, brand_r);
        let brand    = &brands.brands[brand_id];

        // Same yaw computation used in banners.rs
        let next_i = (idx + 1) % ARCH_POSITIONS.len();
        let (nx, nz) = ARCH_POSITIONS[next_i];
        let dir = Vec2::new(nx - ax, nz - az);
        let yaw = dir.x.atan2(dir.y);

        let banner_mat = materials.add(StandardMaterial {
            base_color: Color::srgb(brand.primary[0], brand.primary[1], brand.primary[2]),
            perceptual_roughness: 0.65,
            ..default()
        });
        let accent_mat = materials.add(StandardMaterial {
            base_color: Color::srgb(brand.secondary[0], brand.secondary[1], brand.secondary[2]),
            emissive: LinearRgba::new(
                brand.secondary[0] * 1.5,
                brand.secondary[1] * 1.5,
                brand.secondary[2] * 1.5,
                1.0,
            ),
            perceptual_roughness: 0.40,
            ..default()
        });

        let banner_y = ground_y + ARCH_BANNER_Y_OFFSET;

        let root = commands.spawn((
            AdSign { brand_id },
            Transform::from_xyz(ax, banner_y, az)
                .with_rotation(Quat::from_rotation_y(yaw)),
            Visibility::default(),
            RigidBody::Static,
            Collider::cuboid(GS_BANNER_W * 0.5, GS_BANNER_H * 0.5, GS_BANNER_D * 0.5),
            Name::new(format!("AdGateBanner[{}]", brand.id)),
        )).id();

        let banner = commands.spawn((
            Mesh3d(banner_mesh.clone()),
            MeshMaterial3d(banner_mat),
            Transform::default(),
        )).id();

        let accent_y_local = GS_BANNER_H * 0.5 - ACCENT_H * 0.5;
        let accent = commands.spawn((
            Mesh3d(accent_mesh.clone()),
            MeshMaterial3d(accent_mat),
            Transform::from_xyz(0.0, accent_y_local, GS_BANNER_D * 0.5 + ACCENT_D * 0.5),
        )).id();

        let mut children = vec![banner, accent];

        if let Some(ref logos) = logo_textures {
            let logo_z = GS_BANNER_D * 0.5 + 0.002;
            if let Some(logo) = spawn_logo_quad(
                &mut commands,
                &mut meshes,
                &mut materials,
                logos,
                brand_id,
                GS_BANNER_W,
                GS_BANNER_H,
                0.0,
                logo_z,
            ) {
                children.push(logo);
            }
        }

        commands.entity(root).add_children(&children);

        *analytics.impressions.entry(brand.id.to_string()).or_insert(0) += 1;
    }

    info!("ad_signage: {} gate sponsor banners spawned", ARCH_POSITIONS.len());
}

// ---------------------------------------------------------------------------
// 3. Trailside small signs (24 — along fence rows + hillclimb approach)
// ---------------------------------------------------------------------------
//
// Two signs per fence segment gap (evenly spaced), plus 6 extra along the
// hillclimb / rock-crawl approach for a total of 24.

struct SignLine {
    start: Vec2,
    end:   Vec2,
    count: usize,
    side_offset: f32,  // lateral offset so signs don't sit on the road centre
    salt_offset: i32,
}

const SIGN_LINES: [SignLine; 5] = [
    // Along fence row 1 (toward rock garden, X=40..110 Z=0)
    SignLine { start: Vec2::new( 42.0,  3.5), end: Vec2::new(108.0,  3.5), count: 5, side_offset:  4.0, salt_offset: 0 },
    // Along fence row 2 (toward lighthouse, X=35..80 Z=60..90)
    SignLine { start: Vec2::new( 38.0, 62.0), end: Vec2::new( 78.0, 88.0), count: 5, side_offset: -4.0, salt_offset: 10 },
    // Along fence row 3 (toward hillclimb, X=-40..-130 Z=-50..-130)
    SignLine { start: Vec2::new(-42.0,-52.0), end: Vec2::new(-128.0,-128.0), count: 5, side_offset:  4.0, salt_offset: 20 },
    // Hillclimb approach corridor (Z=-160..-240)
    SignLine { start: Vec2::new(-148.0,-162.0), end: Vec2::new(-148.0,-238.0), count: 5, side_offset: -6.0, salt_offset: 30 },
    // Obstacle course approach (Z=185..260)
    SignLine { start: Vec2::new(-58.0, 188.0), end: Vec2::new(-58.0, 258.0), count: 4, side_offset:  6.0, salt_offset: 40 },
];

fn spawn_trailside_signs(
    mut commands: Commands,
    mut meshes:   ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    brands: Res<ParodyBrands>,
    mut analytics: ResMut<SponsorAnalytics>,
    logo_textures: Option<Res<BrandLogoTextures>>,
) {
    let post_mesh   = meshes.add(Cylinder::new(TS_POST_R, TS_POST_H));
    let panel_mesh  = meshes.add(Cuboid::new(TS_PANEL_W, TS_PANEL_H, TS_PANEL_D));
    let accent_mesh = meshes.add(Cuboid::new(TS_PANEL_W, ACCENT_H * 0.6, TS_PANEL_D + 0.02));

    let post_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.35, 0.25, 0.15),
        perceptual_roughness: 0.95,
        ..default()
    });

    let mut total = 0usize;

    for line in &SIGN_LINES {
        let n = line.count;
        for i in 0..n {
            let t = if n == 1 { 0.5 } else { i as f32 / (n - 1) as f32 };
            let pos = line.start.lerp(line.end, t);
            // perpendicular offset in XZ
            let dir = (line.end - line.start).normalize_or_zero();
            let perp = Vec2::new(-dir.y, dir.x); // rotate 90°
            let final_xz = pos + perp * line.side_offset;

            let sx = final_xz.x;
            let sz = final_xz.y;
            let ground_y = terrain_height_at(sx, sz);

            let brand_r  = brand_hash(i as i32 + line.salt_offset, 11, SALT_TRAIL);
            let brand_id = pick_brand(&brands, brand_r);
            let brand    = &brands.brands[brand_id];

            // Face toward the driving corridor (face inward toward origin-ish)
            let yaw = (-sx).atan2(-sz);

            let panel_mat = materials.add(StandardMaterial {
                base_color: Color::srgb(brand.primary[0], brand.primary[1], brand.primary[2]),
                perceptual_roughness: 0.78,
                ..default()
            });
            let accent_mat = materials.add(StandardMaterial {
                base_color: Color::srgb(brand.secondary[0], brand.secondary[1], brand.secondary[2]),
                emissive: LinearRgba::new(
                    brand.secondary[0] * 0.8,
                    brand.secondary[1] * 0.8,
                    brand.secondary[2] * 0.8,
                    1.0,
                ),
                perceptual_roughness: 0.55,
                ..default()
            });

            let root = commands.spawn((
                AdSign { brand_id },
                Transform::from_xyz(sx, ground_y, sz)
                    .with_rotation(Quat::from_rotation_y(yaw)),
                Visibility::default(),
                RigidBody::Static,
                Collider::cuboid(TS_PANEL_W * 0.5, TS_PANEL_H * 0.5, TS_PANEL_D * 0.5),
                ColliderTransform {
                    translation: Vec3::new(0.0, TS_PANEL_Y, 0.0),
                    ..default()
                },
                Name::new(format!("AdTrailSign[{}]", brand.id)),
            )).id();

            let post = commands.spawn((
                Mesh3d(post_mesh.clone()),
                MeshMaterial3d(post_mat.clone()),
                Transform::from_xyz(0.0, TS_POST_H * 0.5, 0.0),
            )).id();

            let panel = commands.spawn((
                Mesh3d(panel_mesh.clone()),
                MeshMaterial3d(panel_mat),
                Transform::from_xyz(0.0, TS_PANEL_Y, 0.0),
            )).id();

            let acc_y = TS_PANEL_Y + TS_PANEL_H * 0.5 - ACCENT_H * 0.3;
            let accent = commands.spawn((
                Mesh3d(accent_mesh.clone()),
                MeshMaterial3d(accent_mat),
                Transform::from_xyz(0.0, acc_y, TS_PANEL_D * 0.5 + 0.01),
            )).id();

            let mut children = vec![post, panel, accent];

            if let Some(ref logos) = logo_textures {
                let logo_z = TS_PANEL_D * 0.5 + 0.002;
                if let Some(logo) = spawn_logo_quad(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    logos,
                    brand_id,
                    TS_PANEL_W,
                    TS_PANEL_H,
                    TS_PANEL_Y,
                    logo_z,
                ) {
                    children.push(logo);
                }
            }

            commands.entity(root).add_children(&children);
            *analytics.impressions.entry(brand.id.to_string()).or_insert(0) += 1;
            total += 1;
        }
    }

    info!("ad_signage: {} trailside signs spawned", total);
}

// ---------------------------------------------------------------------------
// 4. Fence-line banners (8 — vinyl panels on fence post positions)
// ---------------------------------------------------------------------------
//
// Pick representative posts from the 3 fence rows (defined in fence_posts.rs):
//   Row 1: X=40..110 Z=0      — 3 banner positions
//   Row 2: X=35..80 Z=60..90  — 2 banner positions
//   Row 3: X=-40..-130 Z=-50..-130 — 3 banner positions

const FENCE_BANNER_POSITIONS: [(f32, f32); 8] = [
    // Row 1
    ( 52.0,  0.0),
    ( 75.0,  0.0),
    ( 98.0,  0.0),
    // Row 2
    ( 48.0, 69.0),
    ( 67.0, 81.0),
    // Row 3
    (-58.0, -68.0),
    (-85.0, -90.0),
    (-115.0,-115.0),
];

fn spawn_fence_line_banners(
    mut commands: Commands,
    mut meshes:   ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    brands: Res<ParodyBrands>,
    mut analytics: ResMut<SponsorAnalytics>,
    logo_textures: Option<Res<BrandLogoTextures>>,
) {
    let banner_mesh = meshes.add(Cuboid::new(FL_BANNER_W, FL_BANNER_H, FL_BANNER_D));

    for (idx, &(bx, bz)) in FENCE_BANNER_POSITIONS.iter().enumerate() {
        let ground_y = terrain_height_at(bx, bz);
        let brand_r  = brand_hash(idx as i32, 5, SALT_FENCE);
        let brand_id = pick_brand(&brands, brand_r);
        let brand    = &brands.brands[brand_id];

        let yaw = (-bx).atan2(-bz);

        let banner_mat = materials.add(StandardMaterial {
            base_color: Color::srgb(brand.primary[0], brand.primary[1], brand.primary[2]),
            perceptual_roughness: 0.70,
            ..default()
        });

        // Place the banner at a height that fits over the existing fence post (h=1.5)
        let banner_y = ground_y + 1.5 + FL_BANNER_H * 0.5 + 0.1;

        // Fence banners are spawned as a single entity (no root/child split).
        // For the logo quad we need a parent; convert to root + child pattern.
        let root = commands.spawn((
            AdSign { brand_id },
            Transform::from_xyz(bx, banner_y, bz)
                .with_rotation(Quat::from_rotation_y(yaw)),
            Visibility::default(),
            RigidBody::Static,
            Collider::cuboid(FL_BANNER_W * 0.5, FL_BANNER_H * 0.5, FL_BANNER_D * 0.5),
            Name::new(format!("AdFenceBanner[{}]", brand.id)),
        )).id();

        let banner = commands.spawn((
            Mesh3d(banner_mesh.clone()),
            MeshMaterial3d(banner_mat),
            Transform::default(),
        )).id();

        let mut children = vec![banner];

        if let Some(ref logos) = logo_textures {
            let logo_z = FL_BANNER_D * 0.5 + 0.002;
            if let Some(logo) = spawn_logo_quad(
                &mut commands,
                &mut meshes,
                &mut materials,
                logos,
                brand_id,
                FL_BANNER_W,
                FL_BANNER_H,
                0.0,
                logo_z,
            ) {
                children.push(logo);
            }
        }

        commands.entity(root).add_children(&children);

        *analytics.impressions.entry(brand.id.to_string()).or_insert(0) += 1;
    }

    info!("ad_signage: {} fence-line banners spawned", FENCE_BANNER_POSITIONS.len());
}

// ---------------------------------------------------------------------------
// 5. Stadium expo boards (6 — near spawn ranger area at (-20, _, -15))
// ---------------------------------------------------------------------------
//
// Arranged in a loose arc around the front of the ranger hut.

const STADIUM_POSITIONS: [(f32, f32, f32); 6] = [
    // front arc facing outward from the hut
    ( -5.0, 0.0, -28.0),
    (-14.0, 0.0, -30.0),
    (-24.0, 0.0, -30.0),
    (-33.0, 0.0, -28.0),
    ( -8.0, 0.0,  12.0),
    (-30.0, 0.0,  10.0),
];

fn spawn_stadium_expo_boards(
    mut commands: Commands,
    mut meshes:   ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    brands: Res<ParodyBrands>,
    mut analytics: ResMut<SponsorAnalytics>,
    logo_textures: Option<Res<BrandLogoTextures>>,
) {
    let slab_mesh  = meshes.add(Cuboid::new(SE_PANEL_W + 0.4, SE_SLAB_H, 0.35));
    let panel_mesh = meshes.add(Cuboid::new(SE_PANEL_W, SE_PANEL_H, SE_PANEL_D));
    let accent_mesh = meshes.add(Cuboid::new(SE_PANEL_W, ACCENT_H, SE_PANEL_D + 0.02));

    let slab_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.30, 0.28, 0.26),
        perceptual_roughness: 0.95,
        ..default()
    });

    for (idx, &(sx, _, sz)) in STADIUM_POSITIONS.iter().enumerate() {
        let ground_y = terrain_height_at(sx, sz);
        let brand_r  = brand_hash(idx as i32, 13, SALT_STADIUM);
        let brand_id = pick_brand(&brands, brand_r);
        let brand    = &brands.brands[brand_id];

        let yaw = (-sx).atan2(-sz);

        let panel_mat = materials.add(StandardMaterial {
            base_color: Color::srgb(brand.primary[0], brand.primary[1], brand.primary[2]),
            perceptual_roughness: 0.72,
            ..default()
        });
        let accent_mat = materials.add(StandardMaterial {
            base_color: Color::srgb(brand.secondary[0], brand.secondary[1], brand.secondary[2]),
            emissive: LinearRgba::new(
                brand.secondary[0] * 1.8,
                brand.secondary[1] * 1.8,
                brand.secondary[2] * 1.8,
                1.0,
            ),
            perceptual_roughness: 0.35,
            ..default()
        });

        // Slab sits on the ground; panel rises above it
        let slab_y  = ground_y + SE_SLAB_H * 0.5;
        let panel_y = ground_y + SE_SLAB_H + SE_PANEL_H * 0.5;
        let acc_y   = panel_y + SE_PANEL_H * 0.5 - ACCENT_H * 0.5;

        let panel_y_local = panel_y - ground_y;

        let root = commands.spawn((
            AdSign { brand_id },
            Transform::from_xyz(sx, ground_y, sz)
                .with_rotation(Quat::from_rotation_y(yaw)),
            Visibility::default(),
            RigidBody::Static,
            Collider::cuboid(SE_PANEL_W * 0.5, SE_PANEL_H * 0.5, SE_PANEL_D * 0.5),
            ColliderTransform {
                translation: Vec3::new(0.0, panel_y_local, 0.0),
                ..default()
            },
            Name::new(format!("AdStadiumBoard[{}]", brand.id)),
        )).id();

        let slab = commands.spawn((
            Mesh3d(slab_mesh.clone()),
            MeshMaterial3d(slab_mat.clone()),
            Transform::from_xyz(0.0, slab_y - ground_y, 0.0),
        )).id();

        let panel = commands.spawn((
            Mesh3d(panel_mesh.clone()),
            MeshMaterial3d(panel_mat),
            Transform::from_xyz(0.0, panel_y_local, 0.0),
        )).id();

        let accent = commands.spawn((
            Mesh3d(accent_mesh.clone()),
            MeshMaterial3d(accent_mat),
            Transform::from_xyz(0.0, acc_y - ground_y, SE_PANEL_D * 0.5 + 0.01),
        )).id();

        let mut children = vec![slab, panel, accent];

        if let Some(ref logos) = logo_textures {
            let logo_z = SE_PANEL_D * 0.5 + 0.002;
            if let Some(logo) = spawn_logo_quad(
                &mut commands,
                &mut meshes,
                &mut materials,
                logos,
                brand_id,
                SE_PANEL_W,
                SE_PANEL_H,
                panel_y_local,
                logo_z,
            ) {
                children.push(logo);
            }
        }

        commands.entity(root).add_children(&children);
        *analytics.impressions.entry(brand.id.to_string()).or_insert(0) += 1;
    }

    info!("ad_signage: {} stadium expo boards spawned", STADIUM_POSITIONS.len());
}
