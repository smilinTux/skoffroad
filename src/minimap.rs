// Mini-map: bottom-left corner, 200x200 px UI panel.
//
// Approach: Fallback A (pre-baked image).
//   A 128x128 RGBA Image is generated once at startup by sampling terrain_height_at
//   over the full 200x200 m world extents. This avoids 16 384 dynamic nodes and is
//   effectively free at runtime. The map shows the whole terrain in greyscale.
//   The chassis dot is a child Node with PositionType::Absolute; its left/top are
//   updated each frame to track the chassis world-space position.
//   A heading indicator (thin rectangle, rotated via transform) sits on top of the dot.
//
// Toggle: press M to show/hide.

use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};

use crate::terrain::terrain_height_at;
use crate::vehicle::{Chassis, VehicleRoot};

// ---- Plugin -----------------------------------------------------------------

pub struct MinimapPlugin;

impl Plugin for MinimapPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MinimapVisible>()
            .add_systems(Startup, spawn_minimap)
            .add_systems(
                Update,
                (update_minimap, toggle_minimap).run_if(resource_exists::<VehicleRoot>),
            );
    }
}

// ---- Resources & components -------------------------------------------------

#[derive(Resource)]
struct MinimapVisible(bool);

impl Default for MinimapVisible {
    fn default() -> Self {
        Self(true)
    }
}

/// Marks the outer border panel — used for M-key toggle.
#[derive(Component)]
struct MinimapRoot;

/// Marks the chassis dot so update_minimap can query it.
#[derive(Component)]
struct ChassisDot;

/// Marks the heading indicator rectangle.
#[derive(Component)]
struct HeadingIndicator;

// ---- Constants --------------------------------------------------------------

const MAP_PX: f32 = 200.0;      // rendered size in screen pixels
const WORLD_EXTENT: f32 = 200.0; // terrain spans [-100, +100] on X and Z
const TEX_N: usize = 128;        // texture resolution (each axis)
const DOT_PX: f32 = 8.0;        // chassis dot size in pixels
const HDG_W: f32 = 3.0;         // heading line width in pixels
const HDG_H: f32 = 14.0;        // heading line height in pixels
// Match HUD background colour from hud.rs
const BG: Color = Color::srgba(0.05, 0.05, 0.07, 0.75);

// ---- Startup ----------------------------------------------------------------

fn spawn_minimap(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let img_handle = images.add(build_heightmap_image());

    // Outer border panel — bottom-left corner.
    let root = commands
        .spawn((
            MinimapRoot,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(12.0),
                bottom: Val::Px(12.0),
                width: Val::Px(MAP_PX + 8.0),  // 4 px padding each side
                height: Val::Px(MAP_PX + 8.0),
                padding: UiRect::all(Val::Px(4.0)),
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(BG),
            ZIndex(10),
        ))
        .id();

    // Heightmap image fills the inner area.
    let map_img = commands
        .spawn((
            ImageNode::new(img_handle),
            Node {
                width: Val::Px(MAP_PX),
                height: Val::Px(MAP_PX),
                position_type: PositionType::Relative,
                overflow: Overflow::clip(),
                ..default()
            },
        ))
        .id();

    // Chassis dot — positioned absolutely relative to the map image container.
    // Initial position: center of map.
    let dot = commands
        .spawn((
            ChassisDot,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Px(DOT_PX),
                height: Val::Px(DOT_PX),
                left: Val::Px(MAP_PX / 2.0 - DOT_PX / 2.0),
                top: Val::Px(MAP_PX / 2.0 - DOT_PX / 2.0),
                ..default()
            },
            BackgroundColor(Color::srgb(0.95, 0.15, 0.10)),
            ZIndex(11),
        ))
        .id();

    // Heading indicator — thin rectangle centered on the dot, pointing up (forward = -Z).
    // Rotation is applied via Transform each frame.
    let heading = commands
        .spawn((
            HeadingIndicator,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Px(HDG_W),
                height: Val::Px(HDG_H),
                // Center it horizontally on the dot; extend upward from dot center.
                left: Val::Px(DOT_PX / 2.0 - HDG_W / 2.0),
                top: Val::Px(-HDG_H + DOT_PX / 2.0),
                ..default()
            },
            BackgroundColor(Color::srgb(1.0, 0.90, 0.10)),
            ZIndex(12),
        ))
        .id();

    // Wire up hierarchy: root -> map_img -> dot -> heading
    commands.entity(dot).add_child(heading);
    commands.entity(map_img).add_child(dot);
    commands.entity(root).add_child(map_img);
}

// ---- Heightmap image builder ------------------------------------------------

fn build_heightmap_image() -> Image {
    let n = TEX_N;
    let mut data: Vec<u8> = Vec::with_capacity(n * n * 4);

    // Pre-sample min/max to normalise the greyscale range.
    // One forward pass collects raw heights; a second pass encodes them.
    let mut raw: Vec<f32> = Vec::with_capacity(n * n);
    let (mut h_min, mut h_max) = (f32::MAX, f32::MIN);

    for y in 0..n {
        for x in 0..n {
            let wx = (x as f32 / n as f32 - 0.5) * WORLD_EXTENT;
            let wz = (y as f32 / n as f32 - 0.5) * WORLD_EXTENT;
            let h = terrain_height_at(wx, wz);
            if h < h_min { h_min = h; }
            if h > h_max { h_max = h; }
            raw.push(h);
        }
    }

    let h_range = (h_max - h_min).max(0.001);

    for h in &raw {
        let v = (((h - h_min) / h_range) * 255.0).clamp(0.0, 255.0) as u8;
        data.extend_from_slice(&[v, v, v, 255]);
    }

    Image::new(
        Extent3d {
            width: n as u32,
            height: n as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    )
}

// ---- Per-frame update -------------------------------------------------------

fn update_minimap(
    vehicle: Res<VehicleRoot>,
    // Disjoint from hdg_q (Without<HeadingIndicator>) — both touch Transform,
    // one immutable on Chassis and one mutable on HeadingIndicator. Bevy
    // requires the filters to make these provably non-overlapping.
    chassis_q: Query<&Transform, (With<Chassis>, Without<HeadingIndicator>)>,
    mut dot_q: Query<&mut Node, (With<ChassisDot>, Without<HeadingIndicator>)>,
    mut hdg_q: Query<&mut Transform, (With<HeadingIndicator>, Without<Chassis>)>,
) {
    let Ok(chassis_tf) = chassis_q.get(vehicle.chassis) else {
        return;
    };

    let pos = chassis_tf.translation;

    // Map world XZ to pixel coordinates within [0, MAP_PX].
    let px = (pos.x / WORLD_EXTENT + 0.5) * MAP_PX;
    let pz = (pos.z / WORLD_EXTENT + 0.5) * MAP_PX;

    // Update dot Node position (left/top in the image container's local space).
    if let Ok(mut node) = dot_q.single_mut() {
        node.left = Val::Px((px - DOT_PX / 2.0).clamp(0.0, MAP_PX - DOT_PX));
        node.top  = Val::Px((pz - DOT_PX / 2.0).clamp(0.0, MAP_PX - DOT_PX));
    }

    // Heading indicator: rotate around Z to match chassis yaw projected onto XZ.
    // chassis forward is -Z; atan2 gives angle from +X axis.
    if let Ok(mut hdg_tf) = hdg_q.single_mut() {
        let fwd = *chassis_tf.forward(); // Vec3 pointing forward
        // Yaw angle: 0 = up (+Z screen = world +Z), positive = clockwise.
        // In map space +X is right, +Z is down.
        let angle = fwd.x.atan2(-fwd.z); // negative because screen Y grows downward
        hdg_tf.rotation = Quat::from_rotation_z(angle);
    }
}

// ---- Toggle -----------------------------------------------------------------

fn toggle_minimap(
    keys: Res<ButtonInput<KeyCode>>,
    mut visible: ResMut<MinimapVisible>,
    mut root_q: Query<&mut Node, With<MinimapRoot>>,
) {
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if !shift && keys.just_pressed(KeyCode::KeyM) {
        visible.0 = !visible.0;
        let disp = if visible.0 { Display::Flex } else { Display::None };
        for mut node in &mut root_q {
            node.display = disp;
        }
    }
}
