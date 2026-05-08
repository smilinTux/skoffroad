// Hillclimb track: a procedural steep slope course with rock walls forcing
// the line. ~150 m long, 30°–55° grade, 12 m wide channel. Spawned at fixed
// world position (-150, 0, -150) so it's always findable via fast-travel.
//
// Public API:
//   HillclimbTrackPlugin
//   HillclimbStartGate  (component)
//   HillclimbFinishGate (component)

use bevy::prelude::*;
use avian3d::prelude::*;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

pub struct HillclimbTrackPlugin;

impl Plugin for HillclimbTrackPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_hillclimb_track);
    }
}

// HillclimbStartGate / HillclimbFinishGate live in crate::hillclimb to avoid
// a duplicate-definition conflict; re-exported here for backwards compat.
pub use crate::hillclimb::{HillclimbFinishGate, HillclimbStartGate};

// ---------------------------------------------------------------------------
// Track layout constants
// ---------------------------------------------------------------------------

/// Start position (low end, base of the slope).
const TRACK_START: Vec3 = Vec3::new(-150.0, 0.0, -150.0);

/// Number of slope segments.
const NUM_SEGMENTS: usize = 8;

/// Length of each slope segment (m).
const SEG_LENGTH: f32 = 18.0;    // 8 × 18 = 144 m run (≈150 m total)

/// Width of the slope (m).
const SLOPE_WIDTH: f32 = 12.0;

/// Thickness of each slope slab (m).
const SLOPE_THICKNESS: f32 = 1.5;

/// Yaw of the track direction (radians). We run along +X so yaw = 0.
/// The slope rises along +X from the start position.
const TRACK_YAW: f32 = 0.0; // track runs in the +X direction

/// Half-width used for wall offset.
const HALF_SLOPE_W: f32 = SLOPE_WIDTH * 0.5;

/// Wall dimensions: width (thickness) × height × length.
const WALL_THICKNESS: f32 = 0.8;
const WALL_HEIGHT: f32 = 12.0;

/// Gate post dimensions.
const POST_W: f32 = 0.4;
const POST_H: f32 = 4.0;
const POST_D: f32 = 0.4;

/// Gate beam dimensions.
const BEAM_W: f32 = SLOPE_WIDTH; // spans the full channel width
const BEAM_H: f32 = 0.4;
const BEAM_D: f32 = 0.4;

/// Marker post dimensions.
const MARKER_POST_W: f32 = 0.3;
const MARKER_POST_H: f32 = 6.0;
const MARKER_POST_D: f32 = 0.3;
const MARKER_BOARD_W: f32 = 3.0;
const MARKER_BOARD_H: f32 = 0.8;
const MARKER_BOARD_D: f32 = 0.1;

// ---------------------------------------------------------------------------
// Colour palette
// ---------------------------------------------------------------------------

const SLOPE_COLOR: Color  = Color::srgb(0.45, 0.42, 0.38);
const WALL_COLOR:  Color  = Color::srgb(0.30, 0.25, 0.22);
const GATE_START_COLOR: Color = Color::srgb(1.0,  0.85, 0.0);   // bright yellow
const GATE_FINISH_COLOR: Color = Color::srgb(0.1,  0.85, 0.2);  // bright green
const MARKER_COLOR: Color = Color::srgb(1.0, 0.95, 0.0);        // vivid yellow

// ---------------------------------------------------------------------------
// Spawn system
// ---------------------------------------------------------------------------

fn spawn_hillclimb_track(
    mut commands: Commands,
    mut meshes:   ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Pre-build shared materials.
    let slope_mat = materials.add(StandardMaterial {
        base_color: SLOPE_COLOR,
        perceptual_roughness: 0.95,
        ..default()
    });
    let wall_mat = materials.add(StandardMaterial {
        base_color: WALL_COLOR,
        perceptual_roughness: 0.97,
        ..default()
    });

    // ---------------------------------------------------------------------------
    // 1. Slope segments
    // ---------------------------------------------------------------------------
    //
    // Track direction vector along the ground (unit).
    let dir_xz = Vec3::new(TRACK_YAW.cos(), 0.0, TRACK_YAW.sin()); // (1, 0, 0)
    // Per-segment tilt angle (pitch) around the local right axis.
    // rise/run per segment: RISE_PER_SEG / SEG_LENGTH ≈ 0.521 → ~27.5°.
    // We distribute grades non-uniformly across segments to create 30°–55° range:
    // early segments shallower, mid-segments steep, last segment moderate.
    // Grades in degrees for each segment index:
    let segment_grades_deg: [f32; NUM_SEGMENTS] = [30.0, 33.0, 38.0, 45.0, 50.0, 55.0, 42.0, 35.0];

    // We track the cumulative translation of the segment centres.
    // Each segment is a tilted slab. The centre of segment i sits at the midpoint
    // between its lower and upper edges.
    //
    // For a segment tilted at angle θ (pitch), the slab length projected onto the
    // ground is SEG_LENGTH * cos(θ) and the height rise is SEG_LENGTH * sin(θ).
    // The segment centre is advanced horizontally by SEG_LENGTH * cos(θ) / 2 from
    // the previous upper edge, and vertically by SEG_LENGTH * sin(θ) / 2.

    // Current "foot" position — the leading (lower) edge of the next segment.
    let mut foot = TRACK_START; // (-150, 0, -150)

    // Per-segment data we collect to position walls and gates.
    struct SegData {
        centre: Vec3,
        pitch:  f32,  // radians, positive = nose-up
    }
    let mut seg_data: Vec<SegData> = Vec::with_capacity(NUM_SEGMENTS);

    for i in 0..NUM_SEGMENTS {
        let pitch_rad = segment_grades_deg[i].to_radians();
        let run   = SEG_LENGTH * pitch_rad.cos(); // horizontal extent
        let rise  = SEG_LENGTH * pitch_rad.sin(); // vertical extent

        // Centre of this slab is halfway along the run+rise vector.
        let centre = Vec3::new(
            foot.x + dir_xz.x * run  * 0.5,
            foot.y +             rise * 0.5,
            foot.z + dir_xz.z * run  * 0.5,
        );

        // Rotation: yaw then pitch (pitch about local right = -Z for YAW=0 is +X dir)
        // The slab runs along +Z in mesh space, so we pitch about -X (local right).
        let rotation = Quat::from_rotation_y(TRACK_YAW)
                     * Quat::from_rotation_x(-pitch_rad);

        // Mesh: width × thickness × length  (local X × Y × Z).
        let mesh   = meshes.add(Cuboid::new(SLOPE_WIDTH, SLOPE_THICKNESS, SEG_LENGTH));
        let half_w = SLOPE_WIDTH     * 0.5;
        let half_h = SLOPE_THICKNESS * 0.5;
        let half_l = SEG_LENGTH      * 0.5;

        commands.spawn((
            Mesh3d(mesh),
            MeshMaterial3d(slope_mat.clone()),
            Transform {
                translation: centre,
                rotation,
                scale: Vec3::ONE,
            },
            RigidBody::Static,
            Collider::cuboid(half_w, half_h, half_l),
        ));

        seg_data.push(SegData { centre, pitch: pitch_rad });

        // Advance foot to the upper edge of this segment.
        foot = Vec3::new(
            foot.x + dir_xz.x * run,
            foot.y + rise,
            foot.z + dir_xz.z * run,
        );
    }

    let finish_foot = foot; // top of the track

    // ---------------------------------------------------------------------------
    // 2. Side walls (8 per side = 16 total)
    // ---------------------------------------------------------------------------

    let wall_mesh = meshes.add(Cuboid::new(WALL_THICKNESS, WALL_HEIGHT, SEG_LENGTH));
    let hw = WALL_THICKNESS * 0.5;
    let hh = WALL_HEIGHT    * 0.5;
    let hl = SEG_LENGTH     * 0.5;

    // Right-hand normal of the track (perpendicular, local +X for YAW=0 → actually ±X
    // but since we go in +X direction the sides are along ±Z).
    // Track direction is +X (yaw=0), so lateral offsets are along ±Z.
    let lateral = Vec3::new(0.0, 0.0, 1.0); // right side
    let lateral_neg = Vec3::new(0.0, 0.0, -1.0); // left side

    for seg in &seg_data {
        // Wall centre is at the slope segment centre, shifted laterally and raised
        // to sit alongside the slope (wall foot at slope surface level).
        // Slope segment centre Y already accounts for the midpoint height.
        // We lift the wall so its base aligns with the slope's lateral surface:
        // wall base = slope_centre.y + SLOPE_THICKNESS * 0.5 (approx flat)
        let wall_base_y = seg.centre.y + SLOPE_THICKNESS * 0.5;
        let wall_centre_y = wall_base_y + WALL_HEIGHT * 0.5;

        let rotation = Quat::from_rotation_y(TRACK_YAW)
                     * Quat::from_rotation_x(-seg.pitch);

        for side in [lateral, lateral_neg] {
            let centre = Vec3::new(
                seg.centre.x + side.x * HALF_SLOPE_W,
                wall_centre_y,
                seg.centre.z + side.z * HALF_SLOPE_W,
            );

            commands.spawn((
                Mesh3d(wall_mesh.clone()),
                MeshMaterial3d(wall_mat.clone()),
                Transform {
                    translation: centre,
                    rotation,
                    scale: Vec3::ONE,
                },
                RigidBody::Static,
                Collider::cuboid(hw, hh, hl),
            ));
        }
    }

    // ---------------------------------------------------------------------------
    // 3. Start gate
    // ---------------------------------------------------------------------------

    let start_y = TRACK_START.y;
    // Gate sits just in front of the track start.
    let start_gate_pos = Vec3::new(TRACK_START.x, start_y, TRACK_START.z);

    let start_mat = materials.add(StandardMaterial {
        base_color: GATE_START_COLOR,
        perceptual_roughness: 0.5,
        emissive: LinearRgba::rgb(0.3, 0.2, 0.0),
        ..default()
    });

    let post_mesh  = meshes.add(Cuboid::new(POST_W, POST_H, POST_D));
    let beam_mesh  = meshes.add(Cuboid::new(BEAM_W, BEAM_H, BEAM_D));

    // Gate straddles the channel: posts at ±6 m on the lateral axis (Z in our case).
    let post_offset_z = HALF_SLOPE_W - POST_W * 0.5;

    commands
        .spawn((
            Transform::from_translation(start_gate_pos),
            Visibility::default(),
            HillclimbStartGate,
        ))
        .with_children(|parent| {
            // Left post (−Z side)
            parent.spawn((
                Mesh3d(post_mesh.clone()),
                MeshMaterial3d(start_mat.clone()),
                Transform::from_xyz(0.0, POST_H * 0.5, -post_offset_z),
                RigidBody::Static,
                Collider::cuboid(POST_W * 0.5, POST_H * 0.5, POST_D * 0.5),
            ));
            // Right post (+Z side)
            parent.spawn((
                Mesh3d(post_mesh.clone()),
                MeshMaterial3d(start_mat.clone()),
                Transform::from_xyz(0.0, POST_H * 0.5, post_offset_z),
                RigidBody::Static,
                Collider::cuboid(POST_W * 0.5, POST_H * 0.5, POST_D * 0.5),
            ));
            // Horizontal beam across the top
            parent.spawn((
                Mesh3d(beam_mesh.clone()),
                MeshMaterial3d(start_mat.clone()),
                Transform::from_xyz(0.0, POST_H + BEAM_H * 0.5, 0.0),
                RigidBody::Static,
                Collider::cuboid(BEAM_W * 0.5, BEAM_H * 0.5, BEAM_D * 0.5),
            ));
        });

    // ---------------------------------------------------------------------------
    // 4. Finish gate
    // ---------------------------------------------------------------------------

    let finish_gate_pos = finish_foot;

    let finish_mat = materials.add(StandardMaterial {
        base_color: GATE_FINISH_COLOR,
        perceptual_roughness: 0.5,
        emissive: LinearRgba::rgb(0.0, 0.25, 0.05),
        ..default()
    });

    commands
        .spawn((
            Transform::from_translation(finish_gate_pos),
            Visibility::default(),
            HillclimbFinishGate,
        ))
        .with_children(|parent| {
            // Left post
            parent.spawn((
                Mesh3d(post_mesh.clone()),
                MeshMaterial3d(finish_mat.clone()),
                Transform::from_xyz(0.0, POST_H * 0.5, -post_offset_z),
                RigidBody::Static,
                Collider::cuboid(POST_W * 0.5, POST_H * 0.5, POST_D * 0.5),
            ));
            // Right post
            parent.spawn((
                Mesh3d(post_mesh.clone()),
                MeshMaterial3d(finish_mat.clone()),
                Transform::from_xyz(0.0, POST_H * 0.5, post_offset_z),
                RigidBody::Static,
                Collider::cuboid(POST_W * 0.5, POST_H * 0.5, POST_D * 0.5),
            ));
            // Horizontal beam
            parent.spawn((
                Mesh3d(beam_mesh.clone()),
                MeshMaterial3d(finish_mat.clone()),
                Transform::from_xyz(0.0, POST_H + BEAM_H * 0.5, 0.0),
                RigidBody::Static,
                Collider::cuboid(BEAM_W * 0.5, BEAM_H * 0.5, BEAM_D * 0.5),
            ));
        });

    // ---------------------------------------------------------------------------
    // 5. Marker sign-post at the track entrance
    // ---------------------------------------------------------------------------
    //
    // Tall yellow post at (-160, terrain_y + 5, -150).  No actual text; the
    // vivid yellow colour and sign board shape imply "HILL CLIMB".

    let marker_x = -160.0_f32;
    let marker_z = -150.0_f32;
    // terrain_height_at would cost an FBM evaluation; for this out-of-play-area
    // feature we use a fixed ground baseline (0.0) and add the 5 m lift from the
    // PRD spec directly.
    let marker_terrain_y = 0.0_f32;
    let marker_base_y    = marker_terrain_y + 5.0 + MARKER_POST_H * 0.5;

    let marker_mat = materials.add(StandardMaterial {
        base_color: MARKER_COLOR,
        perceptual_roughness: 0.4,
        emissive: LinearRgba::rgb(0.15, 0.12, 0.0),
        ..default()
    });

    let marker_post_mesh  = meshes.add(Cuboid::new(MARKER_POST_W, MARKER_POST_H, MARKER_POST_D));
    let marker_board_mesh = meshes.add(Cuboid::new(MARKER_BOARD_W, MARKER_BOARD_H, MARKER_BOARD_D));

    commands
        .spawn((
            Transform::from_xyz(marker_x, marker_base_y, marker_z),
            Visibility::default(),
        ))
        .with_children(|parent| {
            // Vertical post.
            parent.spawn((
                Mesh3d(marker_post_mesh),
                MeshMaterial3d(marker_mat.clone()),
                Transform::default(),
            ));
            // Sign board near the top.
            parent.spawn((
                Mesh3d(marker_board_mesh),
                MeshMaterial3d(marker_mat),
                Transform::from_xyz(0.0, MARKER_POST_H * 0.4, 0.0),
            ));
        });

    bevy::log::info!(
        "hillclimb_track: spawned {} slope segments + {} walls, start=({:.0},{:.0},{:.0}), finish=({:.0},{:.0},{:.0})",
        NUM_SEGMENTS,
        NUM_SEGMENTS * 2,
        TRACK_START.x, TRACK_START.y, TRACK_START.z,
        finish_foot.x, finish_foot.y, finish_foot.z,
    );
}
