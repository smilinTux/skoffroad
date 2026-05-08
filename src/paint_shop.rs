// Paint shop: cycle chassis paint colors with [ and ] keys. 8 preset colors.
// Persists choice to ~/.skoffroad/paint.json.
//
// Interaction with vehicle_dirt / damage_visual:
//   Both sibling modules cache the chassis body color at Startup/PostStartup and
//   lerp from that cached "original" toward their respective tint target.  When
//   paint_shop updates base_color (on current_idx change), the siblings are NOT
//   notified — they keep using the color they saw at first-cache time.  That is
//   acceptable: after the next game restart (or the next time their cache-refresh
//   tick fires) they'll pick up the new base.  Within a single session the dirt /
//   damage tinting will still look correct because they work additively from
//   whatever the current base is.
//
// Public API:
//   PaintShopPlugin
//   PaintShopState (resource)

use bevy::prelude::*;

use crate::platform_storage;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

pub struct PaintShopPlugin;

impl Plugin for PaintShopPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PaintShopState>()
            .init_resource::<BodyMaterialHandle>()
            .init_resource::<PaintToastTimer>()
            .init_resource::<PaintSaveDebounce>()
            .add_systems(Startup, (load_paint_config, spawn_paint_toast))
            .add_systems(PostStartup, find_body_material)
            .add_systems(
                Update,
                (
                    cycle_with_brackets,
                    apply_paint_on_change,
                    tick_toast,
                    save_on_change,
                ),
            );
    }
}

/// Current paint selection (0..7).  Default = 0 (Red).
#[derive(Resource, Default, Clone, Copy)]
pub struct PaintShopState {
    pub current_idx: u32,
}

// ---------------------------------------------------------------------------
// Palette
// ---------------------------------------------------------------------------

struct PaintEntry {
    name:  &'static str,
    color: Color,
}

const PALETTE: [PaintEntry; 8] = [
    PaintEntry { name: "Red",    color: Color::srgb(0.85, 0.15, 0.15) },
    PaintEntry { name: "Blue",   color: Color::srgb(0.15, 0.40, 0.85) },
    PaintEntry { name: "Yellow", color: Color::srgb(0.95, 0.85, 0.15) },
    PaintEntry { name: "Green",  color: Color::srgb(0.20, 0.70, 0.30) },
    PaintEntry { name: "Black",  color: Color::srgb(0.10, 0.10, 0.10) },
    PaintEntry { name: "White",  color: Color::srgb(0.95, 0.95, 0.92) },
    PaintEntry { name: "Orange", color: Color::srgb(1.00, 0.55, 0.10) },
    PaintEntry { name: "Purple", color: Color::srgb(0.60, 0.20, 0.85) },
];

// Default body red spawned in vehicle.rs (srgb(0.8, 0.2, 0.1)).
// Used at PostStartup to identify the body material by color proximity.
const BODY_RED: [f32; 3] = [0.80, 0.20, 0.10];
const COLOR_TOL: f32     = 0.05;

// ---------------------------------------------------------------------------
// Internal resources
// ---------------------------------------------------------------------------

/// Handle to the StandardMaterial on the chassis body mesh.
/// Populated at PostStartup via color scan (mirrors livery.rs pattern).
#[derive(Resource, Default)]
struct BodyMaterialHandle {
    handle: Option<Handle<StandardMaterial>>,
}

/// Seconds remaining for the paint-name toast to stay visible.
#[derive(Resource, Default)]
struct PaintToastTimer(f32);

/// Debounce state for saving to disk.
#[derive(Resource, Default)]
struct PaintSaveDebounce {
    pending:   bool,
    elapsed_s: f32,
}

// ---------------------------------------------------------------------------
// Marker components
// ---------------------------------------------------------------------------

#[derive(Component)]
struct PaintToastRoot;

#[derive(Component)]
struct PaintToastText;

// ---------------------------------------------------------------------------
// Persistence helpers
// ---------------------------------------------------------------------------

const STORAGE_KEY: &str = "paint.json";

fn paint_label() -> String {
    platform_storage::debug_path(STORAGE_KEY)
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| format!("localStorage[{}]", STORAGE_KEY))
}

fn idx_from_json(src: &str) -> Option<u32> {
    let v: serde_json::Value = serde_json::from_str(src).ok()?;
    let n = v.as_object()?.get("current_idx")?.as_u64()?;
    Some((n as u32).min(7))
}

fn idx_to_json(idx: u32) -> String {
    format!("{{\n  \"current_idx\": {}\n}}", idx)
}

// ---------------------------------------------------------------------------
// Startup systems
// ---------------------------------------------------------------------------

fn load_paint_config(mut state: ResMut<PaintShopState>) {
    let label = paint_label();
    match platform_storage::read_string(STORAGE_KEY) {
        None => {
            info!(
                "paint_shop: no saved config at {}; defaulting to Red",
                label,
            );
        }
        Some(text) => match idx_from_json(&text) {
            None => {
                info!("paint_shop: could not parse {}; defaulting to Red", label);
            }
            Some(idx) => {
                state.current_idx = idx;
                info!(
                    "paint_shop: loaded index {} ({}) from {}",
                    idx,
                    PALETTE[idx as usize].name,
                    label,
                );
            }
        },
    }
}

/// Spawn a small toast panel (hidden by default) near bottom-center.
fn spawn_paint_toast(mut commands: Commands) {
    commands
        .spawn((
            PaintToastRoot,
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(60.0),
                left: Val::Percent(50.0),
                width: Val::Px(220.0),
                padding: UiRect::all(Val::Px(8.0)),
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.05, 0.05, 0.07, 0.85)),
            Visibility::Hidden,
        ))
        .with_children(|parent| {
            parent.spawn((
                PaintToastText,
                Text::new("PAINT: Red"),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgb(0.95, 0.85, 0.55)),
            ));
        });
}

/// PostStartup: scan all StandardMaterial entities for the body-red color.
/// Must run after vehicle.rs's spawn_vehicle (which runs in Startup).
fn find_body_material(
    mat_q: Query<&MeshMaterial3d<StandardMaterial>>,
    materials: Res<Assets<StandardMaterial>>,
    state: Res<PaintShopState>,
    mut body_handle: ResMut<BodyMaterialHandle>,
) {
    // First: scan for the default body red spawned by vehicle.rs.
    for mat_handle in mat_q.iter() {
        let Some(mat) = materials.get(mat_handle.id()) else {
            continue;
        };
        let Srgba { red, green, blue, .. } = mat.base_color.to_srgba();
        if (red   - BODY_RED[0]).abs() < COLOR_TOL
            && (green - BODY_RED[1]).abs() < COLOR_TOL
            && (blue  - BODY_RED[2]).abs() < COLOR_TOL
        {
            body_handle.handle = Some(mat_handle.0.clone());
            info!(
                "paint_shop: body material found; current paint = {} ({})",
                state.current_idx,
                PALETTE[state.current_idx as usize].name
            );
            return;
        }
    }
    warn!("paint_shop: body material not found at PostStartup — paint cycling disabled");
}

// ---------------------------------------------------------------------------
// Update systems
// ---------------------------------------------------------------------------

/// [ / ] keys cycle through the 8-entry palette.
fn cycle_with_brackets(
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<PaintShopState>,
    mut timer: ResMut<PaintToastTimer>,
    mut toast_text_q: Query<&mut Text, With<PaintToastText>>,
) {
    let mut changed = false;

    if keys.just_pressed(KeyCode::BracketRight) {
        state.current_idx = (state.current_idx + 1) % 8;
        changed = true;
    } else if keys.just_pressed(KeyCode::BracketLeft) {
        state.current_idx = (state.current_idx + 7) % 8;
        changed = true;
    }

    if changed {
        let name = PALETTE[state.current_idx as usize].name;
        info!("paint: {}", name);
        timer.0 = 2.0;
        for mut text in &mut toast_text_q {
            text.0 = format!("PAINT: {}", name);
        }
    }
}

/// When PaintShopState changes, update the chassis body material base_color.
/// vehicle_dirt and damage_visual will keep lerping from the new base — acceptable.
fn apply_paint_on_change(
    state: Res<PaintShopState>,
    body_handle: Res<BodyMaterialHandle>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if !state.is_changed() {
        return;
    }

    let Some(ref handle) = body_handle.handle else {
        return;
    };

    let Some(mat) = materials.get_mut(handle) else {
        return;
    };

    mat.base_color = PALETTE[state.current_idx as usize].color;
}

/// Show the toast for 2 s after a paint change, then hide it.
fn tick_toast(
    time: Res<Time>,
    mut timer: ResMut<PaintToastTimer>,
    mut toast_q: Query<&mut Visibility, With<PaintToastRoot>>,
) {
    if timer.0 > 0.0 {
        timer.0 = (timer.0 - time.delta_secs()).max(0.0);
    }

    let target = if timer.0 > 0.0 {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };

    for mut vis in &mut toast_q {
        *vis = target;
    }
}

/// Debounced 0.5 s write to ~/.skoffroad/paint.json.
fn save_on_change(
    state: Res<PaintShopState>,
    mut deb: ResMut<PaintSaveDebounce>,
    time: Res<Time>,
) {
    // Arm the debounce timer on any mutation of PaintShopState.
    if state.is_changed() {
        deb.pending   = true;
        deb.elapsed_s = 0.0;
        return;
    }

    if !deb.pending {
        return;
    }

    deb.elapsed_s += time.delta_secs();
    if deb.elapsed_s < 0.5 {
        return;
    }

    // Timer elapsed — write to disk.
    deb.pending   = false;
    deb.elapsed_s = 0.0;

    let json = idx_to_json(state.current_idx);
    let label = paint_label();

    match platform_storage::write_string(STORAGE_KEY, &json) {
        Err(e) => {
            warn!("paint_shop: {}", e);
        }
        Ok(()) => {
            info!(
                "paint_shop: saved index {} ({}) to {}",
                state.current_idx,
                PALETTE[state.current_idx as usize].name,
                label,
            );
        }
    }
}
