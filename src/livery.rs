// Paint-color (livery) selection for the player vehicle.
//
// Number keys 1–5 cycle through five preset body colors.
// The body material is identified at PostStartup by scanning every
// MeshMaterial3d<StandardMaterial> component in the world and comparing
// base_color to the known body-red (within float tolerance 0.02 per channel).
// The matching handle is stored in BodyLivery and mutated in place on key press.

use bevy::prelude::*;

// ---- Plugin -----------------------------------------------------------------

pub struct LiveryPlugin;

impl Plugin for LiveryPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LiveryState>()
           .init_resource::<BodyLivery>()
           .init_resource::<LiveryHudTimer>()
           .add_systems(Startup, spawn_livery_hud)
           .add_systems(PostStartup, find_body_material)
           .add_systems(Update, (cycle_livery, update_livery_hud));
    }
}

// ---- Public resources -------------------------------------------------------

#[derive(Resource, Default)]
pub struct LiveryState {
    pub current: u8, // 0..5 — 0 = Trail Red (default)
}

// ---- Internal resources & components ----------------------------------------

/// Handle to the body StandardMaterial once found at PostStartup.
#[derive(Resource, Default)]
struct BodyLivery {
    handle: Option<Handle<StandardMaterial>>,
}

/// Seconds remaining for the livery popup to stay visible.
#[derive(Resource, Default)]
struct LiveryHudTimer(f32);

#[derive(Component)]
struct LiveryHud;

#[derive(Component)]
struct LiveryHudText;

// ---- Livery presets ----------------------------------------------------------

struct Preset {
    name: &'static str,
    color: Color,
}

const PRESETS: [Preset; 5] = [
    Preset { name: "Trail Red",    color: Color::srgb(0.80, 0.20, 0.10) },
    Preset { name: "Highway Blue", color: Color::srgb(0.20, 0.40, 0.85) },
    Preset { name: "Desert Tan",   color: Color::srgb(0.95, 0.85, 0.20) },
    Preset { name: "Forest Green", color: Color::srgb(0.10, 0.30, 0.15) },
    Preset { name: "Arctic White", color: Color::srgb(0.92, 0.92, 0.95) },
];

// Expected base_color of the body material (Trail Red).
const BODY_RED: [f32; 3] = [0.80, 0.20, 0.10];
const COLOR_TOL: f32     = 0.02;

// ---- Systems ----------------------------------------------------------------

/// Spawn the top-right popup for livery feedback (hidden until a key is pressed).
fn spawn_livery_hud(mut commands: Commands) {
    commands.spawn((
        LiveryHud,
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(12.0),
            top: Val::Px(360.0),
            width: Val::Px(200.0),
            padding: UiRect::all(Val::Px(8.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.05, 0.05, 0.07, 0.80)),
        Visibility::Hidden,
    )).with_children(|parent| {
        parent.spawn((
            LiveryHudText,
            Text::new("LIVERY: Trail Red"),
            TextFont { font_size: 13.0, ..default() },
            TextColor(Color::srgb(0.90, 0.85, 0.60)),
        ));
    });
}

/// At PostStartup, scan ALL MeshMaterial3d<StandardMaterial> entities for the body red.
/// Stores the handle in BodyLivery, or logs a warning and leaves it as None on failure.
fn find_body_material(
    mat_q: Query<&MeshMaterial3d<StandardMaterial>>,
    materials: Res<Assets<StandardMaterial>>,
    mut body_livery: ResMut<BodyLivery>,
) {
    for mat_handle in mat_q.iter() {
        let Some(mat) = materials.get(mat_handle.id()) else { continue };
        let Srgba { red, green, blue, .. } = mat.base_color.to_srgba();
        if (red   - BODY_RED[0]).abs() < COLOR_TOL
        && (green - BODY_RED[1]).abs() < COLOR_TOL
        && (blue  - BODY_RED[2]).abs() < COLOR_TOL
        {
            body_livery.handle = Some(mat_handle.0.clone());
            return;
        }
    }
    warn!("livery: body material not found at PostStartup — color cycling disabled");
}

/// On number key press: mutate the body material color, update HUD text, start timer.
fn cycle_livery(
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<LiveryState>,
    body_livery: Res<BodyLivery>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut hud_text_q: Query<&mut Text, With<LiveryHudText>>,
    mut timer: ResMut<LiveryHudTimer>,
) {
    let key_map: [(KeyCode, u8); 5] = [
        (KeyCode::Digit1, 0),
        (KeyCode::Digit2, 1),
        (KeyCode::Digit3, 2),
        (KeyCode::Digit4, 3),
        (KeyCode::Digit5, 4),
    ];

    let mut pressed_idx: Option<u8> = None;
    for (key, idx) in key_map {
        if keys.just_pressed(key) {
            pressed_idx = Some(idx);
            break;
        }
    }

    let Some(idx) = pressed_idx else { return };

    state.current = idx;

    // Mutate the body material in place — no-op if PostStartup scan failed.
    if let Some(ref handle) = body_livery.handle {
        if let Some(mat) = materials.get_mut(handle) {
            mat.base_color = PRESETS[idx as usize].color;
        }
    }

    // Update HUD text and arm the 2-second display timer.
    let name = PRESETS[idx as usize].name;
    for mut text in &mut hud_text_q {
        text.0 = format!("LIVERY: {}", name);
    }
    timer.0 = 2.0;
}

/// Tick the popup timer and show/hide the HUD node accordingly.
fn update_livery_hud(
    time: Res<Time>,
    mut timer: ResMut<LiveryHudTimer>,
    mut hud_q: Query<&mut Visibility, With<LiveryHud>>,
) {
    timer.0 = (timer.0 - time.delta_secs()).max(0.0);
    let target = if timer.0 > 0.0 { Visibility::Inherited } else { Visibility::Hidden };
    for mut vis in &mut hud_q {
        *vis = target;
    }
}
