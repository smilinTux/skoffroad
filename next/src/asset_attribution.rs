// Asset attribution: Shift+A modal that auto-generates CC0/CC-BY scrolling
// attribution text from every entry in AssetManifest.  Required by CC-BY
// licenses (author must be credited) and good practice for CC0.
//
// Public API:
//   AssetAttributionPlugin

use bevy::prelude::*;
use crate::asset_manifest::{AssetManifest, MapEntry, ModEntry, VehicleClassEntry};

// ---- Plugin -----------------------------------------------------------------

pub struct AssetAttributionPlugin;

impl Plugin for AssetAttributionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AttributionState>()
            .add_systems(Startup, spawn_attribution_overlay)
            .add_systems(Update, (toggle_with_a, auto_scroll, update_scroll).chain());
    }
}

// ---- Resource ---------------------------------------------------------------

#[derive(Resource)]
pub struct AttributionState {
    pub open:     bool,
    pub scroll_y: f32,
}

impl Default for AttributionState {
    fn default() -> Self {
        Self { open: false, scroll_y: 0.0 }
    }
}

// ---- Component markers ------------------------------------------------------

/// Root node of the attribution modal.
#[derive(Component)]
struct AttributionRoot;

/// Inner scrolling column driven by `scroll_y`.
#[derive(Component)]
struct AttributionColumn;

// ---- Colour constants -------------------------------------------------------

const OVERLAY_BG:     Color = Color::srgba(0.0, 0.0, 0.02, 0.90);
const COLOR_TITLE:    Color = Color::srgb(1.0, 0.9, 0.0);   // yellow
const COLOR_SECTION:  Color = Color::srgb(0.2, 0.95, 1.0);  // cyan
const COLOR_CC0:      Color = Color::srgb(0.35, 1.0, 0.45); // green
const COLOR_CCBY:     Color = Color::srgb(1.0, 0.88, 0.2);  // warm yellow
const COLOR_CCBYSA:   Color = Color::srgb(1.0, 0.60, 0.15); // orange
const COLOR_CCBYNC:   Color = Color::srgb(0.90, 0.45, 0.45);// muted red
const COLOR_UNKNOWN:  Color = Color::srgb(0.75, 0.75, 0.75);// grey
const COLOR_EMPTY:    Color = Color::srgb(0.50, 0.50, 0.50);// dim
const COLOR_FOOTER:   Color = Color::srgb(0.60, 0.60, 0.65);// muted

const MODAL_W:       f32 = 480.0;
const MODAL_H:       f32 = 420.0;
const ROW_HEIGHT_PX: f32 = 26.0;
const SCROLL_SPEED:  f32 = 25.0; // px/sec — slower than credits (30)

// ---- Helpers ----------------------------------------------------------------

/// Map a license string to a display colour.
fn license_color(license: &str) -> Color {
    let l = license.to_ascii_lowercase();
    if l.contains("cc0") {
        COLOR_CC0
    } else if l.contains("cc-by-sa") || l.contains("cc by-sa") || l.contains("ccbysa") {
        COLOR_CCBYSA
    } else if l.contains("cc-by-nc") || l.contains("cc by-nc") || l.contains("ccbync") {
        COLOR_CCBYNC
    } else if l.contains("cc-by") || l.contains("cc by") {
        COLOR_CCBY
    } else {
        COLOR_UNKNOWN
    }
}

// ---- Startup: build the overlay tree ----------------------------------------

fn spawn_attribution_overlay(mut commands: Commands) {
    // Semi-transparent backdrop sized to the modal, centred, hidden by default.
    let root = commands
        .spawn((
            AttributionRoot,
            Node {
                width:           Val::Percent(100.0),
                height:          Val::Percent(100.0),
                position_type:   PositionType::Absolute,
                align_items:     AlignItems::Center,
                justify_content: JustifyContent::Center,
                display:         Display::None,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.55)),
            ZIndex(210),
        ))
        .id();

    // Modal card.
    let card = commands
        .spawn((
            Node {
                width:            Val::Px(MODAL_W),
                height:           Val::Px(MODAL_H),
                flex_direction:   FlexDirection::Column,
                align_items:      AlignItems::Center,
                overflow:         Overflow::clip(),
                padding:          UiRect::axes(Val::Px(8.0), Val::Px(12.0)),
                border:           UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(OVERLAY_BG),
            BorderColor::all(Color::srgba(0.4, 0.4, 0.5, 0.7)),
        ))
        .id();

    // Title row (not part of the scrolling column — stays fixed at top).
    let title = commands
        .spawn((
            Text::new("ASSET ATTRIBUTION"),
            TextFont { font_size: 22.0, ..default() },
            TextColor(COLOR_TITLE),
        ))
        .id();

    // Divider spacer.
    let divider = commands
        .spawn(Node {
            width:         Val::Percent(90.0),
            height:        Val::Px(1.0),
            margin:        UiRect::vertical(Val::Px(6.0)),
            ..default()
        })
        .id();

    // Scrolling column — absolute so we can drive its `top`.
    let column = commands
        .spawn((
            AttributionColumn,
            Node {
                position_type:  PositionType::Absolute,
                flex_direction: FlexDirection::Column,
                align_items:    AlignItems::Center,
                width:          Val::Px(MODAL_W - 16.0),
                top:            Val::Px(MODAL_H),  // start below visible area
                row_gap:        Val::Px(4.0),
                padding:        UiRect::all(Val::Px(8.0)),
                ..default()
            },
        ))
        .id();

    // Footer (static, inside column so it scrolls with content).
    let footer_text = "All built-in procedural meshes \u{00A9} 2026 SandK Offroad";
    let footer = commands
        .spawn((
            Text::new(footer_text),
            TextFont { font_size: 11.0, ..default() },
            TextColor(COLOR_FOOTER),
        ))
        .id();

    // Static "(no assets loaded yet)" placeholder — shown when manifest is empty.
    // The real content is rebuilt dynamically in toggle_with_a.
    let placeholder = commands
        .spawn((
            Text::new("(no third-party assets loaded)"),
            TextFont { font_size: 14.0, ..default() },
            TextColor(COLOR_EMPTY),
        ))
        .id();

    commands.entity(column).add_children(&[placeholder, footer]);
    commands.entity(card).add_children(&[title, divider, column]);
    commands.entity(root).add_child(card);
}

// ---- Toggle (Shift+A) -------------------------------------------------------

fn toggle_with_a(
    keys:      Res<ButtonInput<KeyCode>>,
    mut state: ResMut<AttributionState>,
    manifest:  Res<AssetManifest>,
    mut roots: Query<&mut Node, With<AttributionRoot>>,
    cols:      Query<Entity, With<AttributionColumn>>,
    mut commands: Commands,
) {
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    let toggle = shift && keys.just_pressed(KeyCode::KeyA);
    let close  = keys.just_pressed(KeyCode::Escape);

    if !toggle && !close {
        return;
    }
    if close && !state.open {
        return;
    }

    if toggle {
        state.open = !state.open;
        if state.open {
            state.scroll_y = 0.0;
        }
    } else {
        state.open     = false;
        state.scroll_y = 0.0;
    }

    let display = if state.open { Display::Flex } else { Display::None };
    for mut node in &mut roots {
        node.display = display;
    }

    // Rebuild content whenever we open the panel.
    if state.open {
        for col_entity in &cols {
            // Despawn all existing children so we can replace them.
            // Bevy 0.18: despawn() is recursive on hierarchy by default;
            // iterate children to avoid removing the column entity itself.
            commands.entity(col_entity).despawn_related::<Children>();

            let mut children: Vec<Entity> = Vec::new();

            let total = manifest.vehicles.len() + manifest.maps.len() + manifest.mods.len();
            if total == 0 {
                let e = commands
                    .spawn((
                        Text::new("(no third-party assets loaded)"),
                        TextFont { font_size: 14.0, ..default() },
                        TextColor(COLOR_EMPTY),
                    ))
                    .id();
                children.push(e);
            } else {
                // --- Vehicles section ---
                if !manifest.vehicles.is_empty() {
                    let hdr = commands
                        .spawn((
                            Text::new("Vehicles"),
                            TextFont { font_size: 16.0, ..default() },
                            TextColor(COLOR_SECTION),
                        ))
                        .id();
                    children.push(hdr);

                    for v in &manifest.vehicles {
                        children.push(vehicle_row(&mut commands, v));
                    }

                    children.push(spacer(&mut commands));
                }

                // --- Maps section ---
                if !manifest.maps.is_empty() {
                    let hdr = commands
                        .spawn((
                            Text::new("Maps"),
                            TextFont { font_size: 16.0, ..default() },
                            TextColor(COLOR_SECTION),
                        ))
                        .id();
                    children.push(hdr);

                    for m in &manifest.maps {
                        children.push(map_row(&mut commands, m));
                    }

                    children.push(spacer(&mut commands));
                }

                // --- Mods section ---
                if !manifest.mods.is_empty() {
                    let hdr = commands
                        .spawn((
                            Text::new("Mods"),
                            TextFont { font_size: 16.0, ..default() },
                            TextColor(COLOR_SECTION),
                        ))
                        .id();
                    children.push(hdr);

                    for md in &manifest.mods {
                        children.push(mod_row(&mut commands, md));
                    }

                    children.push(spacer(&mut commands));
                }
            }

            // Footer always present.
            let footer_text = "All built-in procedural meshes \u{00A9} 2026 SandK Offroad";
            let footer = commands
                .spawn((
                    Text::new(footer_text),
                    TextFont { font_size: 11.0, ..default() },
                    TextColor(COLOR_FOOTER),
                ))
                .id();
            children.push(footer);

            let hint = commands
                .spawn((
                    Text::new("Shift+A or Esc to close"),
                    TextFont { font_size: 12.0, ..default() },
                    TextColor(COLOR_EMPTY),
                ))
                .id();
            children.push(hint);

            commands.entity(col_entity).add_children(&children);
        }
    }
}

// ---- Row builder helpers ----------------------------------------------------

fn vehicle_row(commands: &mut Commands, v: &VehicleClassEntry) -> Entity {
    let label = format!("{} \u{2014} {} \u{2014} {}", v.name, v.author, v.license);
    let color = license_color(&v.license);
    commands
        .spawn((
            Text::new(label),
            TextFont { font_size: 13.0, ..default() },
            TextColor(color),
        ))
        .id()
}

fn map_row(commands: &mut Commands, m: &MapEntry) -> Entity {
    // MapEntry has no author field; omit the author slot.
    let label = format!("{} \u{2014} {}", m.name, m.license);
    let color = license_color(&m.license);
    commands
        .spawn((
            Text::new(label),
            TextFont { font_size: 13.0, ..default() },
            TextColor(color),
        ))
        .id()
}

fn mod_row(commands: &mut Commands, md: &ModEntry) -> Entity {
    let label = format!("{} \u{2014} {} \u{2014} {}", md.name, md.author, md.license);
    let color = license_color(&md.license);
    commands
        .spawn((
            Text::new(label),
            TextFont { font_size: 13.0, ..default() },
            TextColor(color),
        ))
        .id()
}

fn spacer(commands: &mut Commands) -> Entity {
    commands
        .spawn(Node {
            height: Val::Px(ROW_HEIGHT_PX * 0.4),
            ..default()
        })
        .id()
}

// ---- Auto-scroll ------------------------------------------------------------

fn auto_scroll(
    time:      Res<Time>,
    mut state: ResMut<AttributionState>,
) {
    if !state.open {
        return;
    }

    state.scroll_y += SCROLL_SPEED * time.delta_secs();

    // Generous wrap ceiling — real content can vary; 2000 px covers worst case.
    let col_height = 2000.0_f32 + MODAL_H;
    if state.scroll_y > col_height {
        state.scroll_y = 0.0;
    }
}

// ---- Apply scroll to column node --------------------------------------------

fn update_scroll(
    state:    Res<AttributionState>,
    mut cols: Query<&mut Node, With<AttributionColumn>>,
) {
    if !state.is_changed() {
        return;
    }

    for mut node in &mut cols {
        // Column starts at bottom of card (MODAL_H) and scrolls upward.
        node.top = Val::Px(MODAL_H - state.scroll_y);
    }
}
