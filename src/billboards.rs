// Billboards: 4 procedural roadside billboards placed at the four quadrant
// corners of the map.  Each billboard is assembled from plain cuboid primitives
// (post + panel + frame strips + decorative colour stripes) — no text rendering
// or external assets required.
//
// Public API:
//   BillboardsPlugin

use bevy::prelude::*;
use avian3d::prelude::*;

use crate::terrain::terrain_height_at;

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct BillboardsPlugin;

impl Plugin for BillboardsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_billboards);
    }
}

// ---------------------------------------------------------------------------
// Marker component
// ---------------------------------------------------------------------------

/// Marks the root entity of every spawned billboard.
#[derive(Component)]
pub struct Billboard;

// ---------------------------------------------------------------------------
// Geometry constants
// ---------------------------------------------------------------------------

// Post
const POST_W: f32 = 0.4;
const POST_H: f32 = 6.0;
const POST_D: f32 = 0.4;

// Panel
const PANEL_W: f32 = 6.0;
const PANEL_H: f32 = 3.0;
const PANEL_D: f32 = 0.2;

// Frame strips (4 thin cuboids around panel edges)
const FRAME_T: f32 = 0.15; // thickness / width of each frame strip
const FRAME_D: f32 = PANEL_D + 0.02; // slightly proud of panel face

// Decorative stripes inside the panel
const STRIPE_W: f32 = 5.6;
const STRIPE_H: f32 = 0.3;
const STRIPE_D: f32 = 0.21; // slightly proud of panel face

const STRIPE_COUNT: usize = 5;

// The bottom of the panel sits at local Y = 3.0 (same as POST_H / 2.0
// because we treat local Y=0 as ground level and place the panel centre at
// POST_H / 2.0 + PANEL_H / 2.0).
const PANEL_CENTER_Y: f32 = POST_H / 2.0 + PANEL_H / 2.0;

// Collider half-extents for the whole billboard (panel bounding box)
const COLLIDER_HX: f32 = PANEL_W / 2.0;
const COLLIDER_HY: f32 = PANEL_H / 2.0;
const COLLIDER_HZ: f32 = 0.4;

// ---------------------------------------------------------------------------
// Stripe colour palettes (alternating pairs per billboard index)
// ---------------------------------------------------------------------------

/// Returns the two alternating accent colours for the given billboard index.
fn stripe_colors(index: usize) -> (Color, Color) {
    match index {
        0 => (
            Color::srgb(0.85, 0.20, 0.20), // red
            Color::srgb(0.95, 0.95, 0.92), // white
        ),
        1 => (
            Color::srgb(0.20, 0.45, 0.85), // blue
            Color::srgb(1.0,  0.90, 0.30), // yellow
        ),
        2 => (
            Color::srgb(0.20, 0.65, 0.30), // green
            Color::srgb(0.10, 0.10, 0.10), // black
        ),
        _ => (
            Color::srgb(1.0,  0.55, 0.20), // orange
            Color::srgb(0.10, 0.10, 0.10), // black
        ),
    }
}

// ---------------------------------------------------------------------------
// Fixed quadrant positions
// ---------------------------------------------------------------------------

const POSITIONS: [(f32, f32); 4] = [
    ( 50.0,  50.0), // Q1
    (-50.0,  50.0), // Q2
    (-50.0, -50.0), // Q3
    ( 50.0, -50.0), // Q4
];

// ---------------------------------------------------------------------------
// Spawn system
// ---------------------------------------------------------------------------

fn spawn_billboards(
    mut commands: Commands,
    mut meshes:   ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // --- shared meshes ---
    let post_mesh   = meshes.add(Cuboid::new(POST_W, POST_H, POST_D));
    let panel_mesh  = meshes.add(Cuboid::new(PANEL_W, PANEL_H, PANEL_D));

    // Frame strips
    // Top/bottom: full width, narrow height, shallow depth
    let frame_horiz_mesh = meshes.add(Cuboid::new(PANEL_W + FRAME_T * 2.0, FRAME_T, FRAME_D));
    // Left/right: narrow width, panel height (no overlap with horiz), shallow depth
    let frame_vert_mesh  = meshes.add(Cuboid::new(FRAME_T, PANEL_H, FRAME_D));

    // Stripe mesh (one size fits all)
    let stripe_mesh = meshes.add(Cuboid::new(STRIPE_W, STRIPE_H, STRIPE_D));

    // --- shared materials ---
    let post_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.30, 0.20, 0.15),
        perceptual_roughness: 0.90,
        ..default()
    });
    let panel_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.95, 0.95, 0.92),
        perceptual_roughness: 0.80,
        ..default()
    });
    let frame_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.20, 0.15, 0.10),
        perceptual_roughness: 0.88,
        ..default()
    });

    for (idx, &(bx, bz)) in POSITIONS.iter().enumerate() {
        let ground_y = terrain_height_at(bx, bz);

        // Parent sits so that local Y=0 == ground level.
        // Billboard faces origin: looking_at target is origin, up is +Y.
        let parent_translation = Vec3::new(bx, ground_y, bz);
        let look_target = Vec3::new(0.0, ground_y, 0.0);
        let parent_transform = Transform::from_translation(parent_translation)
            .looking_at(look_target, Vec3::Y);

        // Collider is centred at panel height.
        let collider_offset = Vec3::new(0.0, PANEL_CENTER_Y, 0.0);

        let parent = commands
            .spawn((
                Billboard,
                Transform {
                    translation: parent_translation,
                    rotation: parent_transform.rotation,
                    scale: Vec3::ONE,
                },
                Visibility::default(),
                RigidBody::Static,
                Collider::cuboid(COLLIDER_HX, COLLIDER_HY, COLLIDER_HZ),
                ColliderTransform {
                    translation: collider_offset,
                    ..default()
                },
            ))
            .id();

        // --- Post ---
        // Post centre at local (0, POST_H/2, 0).
        let post = commands
            .spawn((
                Mesh3d(post_mesh.clone()),
                MeshMaterial3d(post_mat.clone()),
                Transform::from_translation(Vec3::new(0.0, POST_H / 2.0, 0.0)),
            ))
            .id();

        // --- Panel ---
        // Panel centre at local (0, PANEL_CENTER_Y, 0).
        let panel = commands
            .spawn((
                Mesh3d(panel_mesh.clone()),
                MeshMaterial3d(panel_mat.clone()),
                Transform::from_translation(Vec3::new(0.0, PANEL_CENTER_Y, 0.0)),
            ))
            .id();

        // --- Frame strips ---
        // Top frame: above panel by half frame thickness
        let frame_top_y = PANEL_CENTER_Y + PANEL_H / 2.0 + FRAME_T / 2.0;
        let frame_top = commands
            .spawn((
                Mesh3d(frame_horiz_mesh.clone()),
                MeshMaterial3d(frame_mat.clone()),
                Transform::from_translation(Vec3::new(0.0, frame_top_y, 0.0)),
            ))
            .id();

        // Bottom frame
        let frame_bot_y = PANEL_CENTER_Y - PANEL_H / 2.0 - FRAME_T / 2.0;
        let frame_bot = commands
            .spawn((
                Mesh3d(frame_horiz_mesh.clone()),
                MeshMaterial3d(frame_mat.clone()),
                Transform::from_translation(Vec3::new(0.0, frame_bot_y, 0.0)),
            ))
            .id();

        // Left frame
        let frame_left_x = -(PANEL_W / 2.0 + FRAME_T / 2.0);
        let frame_left = commands
            .spawn((
                Mesh3d(frame_vert_mesh.clone()),
                MeshMaterial3d(frame_mat.clone()),
                Transform::from_translation(Vec3::new(frame_left_x, PANEL_CENTER_Y, 0.0)),
            ))
            .id();

        // Right frame
        let frame_right_x = PANEL_W / 2.0 + FRAME_T / 2.0;
        let frame_right = commands
            .spawn((
                Mesh3d(frame_vert_mesh.clone()),
                MeshMaterial3d(frame_mat.clone()),
                Transform::from_translation(Vec3::new(frame_right_x, PANEL_CENTER_Y, 0.0)),
            ))
            .id();

        // --- Decorative stripes ---
        let (color_a, color_b) = stripe_colors(idx);
        let mat_a = materials.add(StandardMaterial {
            base_color: color_a,
            perceptual_roughness: 0.75,
            ..default()
        });
        let mat_b = materials.add(StandardMaterial {
            base_color: color_b,
            perceptual_roughness: 0.75,
            ..default()
        });

        // Distribute 5 stripes evenly within the panel height.
        // Usable interior height = PANEL_H - 2 * FRAME_T (inside the frame).
        // We place 5 stripes with equal gaps.
        let interior_h  = PANEL_H - 2.0 * FRAME_T;
        let total_stripe = STRIPE_COUNT as f32 * STRIPE_H;
        let gap          = (interior_h - total_stripe) / (STRIPE_COUNT as f32 + 1.0);
        let stripe_start = PANEL_CENTER_Y - PANEL_H / 2.0 + FRAME_T + gap + STRIPE_H / 2.0;

        let mut stripe_children: Vec<Entity> = Vec::with_capacity(STRIPE_COUNT);
        for s in 0..STRIPE_COUNT {
            let stripe_y  = stripe_start + s as f32 * (STRIPE_H + gap);
            let stripe_mat = if s % 2 == 0 { mat_a.clone() } else { mat_b.clone() };
            let stripe = commands
                .spawn((
                    Mesh3d(stripe_mesh.clone()),
                    MeshMaterial3d(stripe_mat),
                    Transform::from_translation(Vec3::new(0.0, stripe_y, 0.0)),
                ))
                .id();
            stripe_children.push(stripe);
        }

        // Attach all children to the parent.
        let mut fixed_children = vec![post, panel, frame_top, frame_bot, frame_left, frame_right];
        fixed_children.extend(stripe_children);
        commands.entity(parent).add_children(&fixed_children);
    }

    bevy::log::info!("billboards: 4 procedural roadside billboards spawned");
}
