// minimap_zoom.rs — Sprint 23
//
// +/- keys zoom the minimap UI in the range [0.5, 2.0] in steps of 0.25.
//
// Because MinimapRoot in minimap.rs is private, this module locates the root
// node by its absolute bottom-left position (left:12 px, bottom:12 px, size
// 208×208 px) and applies Transform.scale each frame.  A small "ZOOM 1.0x"
// overlay in the bottom-right corner always reflects the current level.
//
// Public API:
//   MinimapZoomPlugin
//   MinimapZoomState (Resource)

use bevy::prelude::*;

// ---- Plugin -----------------------------------------------------------------

pub struct MinimapZoomPlugin;

impl Plugin for MinimapZoomPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MinimapZoomState>()
            .add_systems(
                Update,
                (
                    adjust_with_keys,
                    apply_scale,
                    spawn_indicator_once,
                    update_indicator_text,
                )
                    .chain(),
            );
    }
}

// ---- Resource ---------------------------------------------------------------

/// Current minimap zoom level.  Scale 1.0 = 100 %, range [0.5, 2.0].
#[derive(Resource, Clone, Copy)]
pub struct MinimapZoomState {
    pub scale: f32,
}

impl Default for MinimapZoomState {
    fn default() -> Self {
        Self { scale: 1.0 }
    }
}

// ---- Components -------------------------------------------------------------

/// Marks the zoom indicator text node (bottom-right corner).
#[derive(Component)]
struct ZoomIndicator;

/// Marks the found minimap root entity so we only search once.
#[derive(Component)]
struct MinimapZoomTarget;

// ---- Constants --------------------------------------------------------------

/// Expected pixel dimensions of the minimap root node (MAP_PX + 8 padding each side).
const MINIMAP_SIZE: f32 = 208.0; // 200 + 8
/// Expected absolute left offset of the minimap root.
const MINIMAP_LEFT: f32 = 12.0;
/// Expected absolute bottom offset of the minimap root.
const MINIMAP_BOTTOM: f32 = 12.0;
/// Zoom step per keypress.
const ZOOM_STEP: f32 = 0.25;
const ZOOM_MIN: f32 = 0.5;
const ZOOM_MAX: f32 = 2.0;

// ---- Systems ----------------------------------------------------------------

/// Reads +/= and - keys and adjusts `MinimapZoomState.scale`.
fn adjust_with_keys(
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<MinimapZoomState>,
) {
    let increase = keys.just_pressed(KeyCode::Equal)  // = / + on US layout
        || keys.just_pressed(KeyCode::NumpadAdd);
    let decrease = keys.just_pressed(KeyCode::Minus)
        || keys.just_pressed(KeyCode::NumpadSubtract);

    if !increase && !decrease {
        return;
    }

    let prev = state.scale;
    if increase {
        state.scale = (state.scale + ZOOM_STEP).min(ZOOM_MAX);
    } else {
        state.scale = (state.scale - ZOOM_STEP).max(ZOOM_MIN);
    }

    // Round to avoid floating-point drift accumulating over many presses.
    state.scale = (state.scale * 4.0).round() / 4.0;

    if (state.scale - prev).abs() > 0.001 {
        info!("minimap zoom: {:.2}x", state.scale);
    }
}

/// Applies `MinimapZoomState.scale` to the minimap root node via Transform.scale.
///
/// On first call we locate the node by matching its expected absolute position
/// and size, tag it with `MinimapZoomTarget`, then apply scale every frame.
fn apply_scale(
    mut commands: Commands,
    state: Res<MinimapZoomState>,
    // All absolute nodes — used for the one-time search
    candidate_q: Query<(Entity, &Node), (With<Node>, Without<MinimapZoomTarget>)>,
    // Already-tagged target
    mut target_q: Query<&mut Transform, With<MinimapZoomTarget>>,
    mut found: Local<bool>,
) {
    // If we haven't yet located the root, scan for it.
    if !*found {
        for (entity, node) in &candidate_q {
            if node.position_type == PositionType::Absolute
                && matches!(node.left,   Val::Px(v) if (v - MINIMAP_LEFT).abs()   < 0.5)
                && matches!(node.bottom, Val::Px(v) if (v - MINIMAP_BOTTOM).abs() < 0.5)
                && matches!(node.width,  Val::Px(v) if (v - MINIMAP_SIZE).abs()   < 0.5)
                && matches!(node.height, Val::Px(v) if (v - MINIMAP_SIZE).abs()   < 0.5)
            {
                commands.entity(entity).insert(MinimapZoomTarget);
                *found = true;
                break;
            }
        }
        // Not yet spawned — try again next frame.
        if !*found {
            return;
        }
    }

    // Apply scale to the tagged entity.
    let s = state.scale;
    for mut tf in &mut target_q {
        tf.scale = Vec3::splat(s);
    }
}

/// Spawns the "ZOOM 1.0x" indicator once; guarded by a `Local<bool>`.
fn spawn_indicator_once(mut commands: Commands, mut spawned: Local<bool>) {
    if *spawned {
        return;
    }
    *spawned = true;

    commands.spawn((
        ZoomIndicator,
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(12.0),
            bottom: Val::Px(12.0),
            padding: UiRect::axes(Val::Px(6.0), Val::Px(3.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.05, 0.05, 0.07, 0.70)),
        ZIndex(20),
        Text::new("ZOOM 1.0x"),
        TextFont {
            font_size: 14.0,
            ..default()
        },
        TextColor(Color::WHITE),
    ));
}

/// Updates the indicator text to reflect the current zoom level every frame.
fn update_indicator_text(
    state: Res<MinimapZoomState>,
    mut q: Query<&mut Text, With<ZoomIndicator>>,
) {
    for mut text in &mut q {
        let formatted = format!("ZOOM {:.1}x", state.scale);
        if text.0 != formatted {
            text.0 = formatted;
        }
    }
}
