// Asset browser: F11 modal listing every loaded asset (vehicles, maps,
// mods) with name + license tag. Visible-license requirement helps
// keep the project clean and players know what's user-content vs built-in.
//
// Public API:
//   AssetBrowserPlugin
//   AssetBrowserState

use bevy::prelude::*;

use crate::asset_manifest::AssetManifest;
use crate::glb_loader::LoadedVehicleGlbs;
use crate::heightmap_loader::LoadedHeightmaps;

// ---- Plugin -----------------------------------------------------------------

pub struct AssetBrowserPlugin;

impl Plugin for AssetBrowserPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AssetBrowserState>()
            .add_systems(Startup, spawn_browser_panel)
            .add_systems(Update, (toggle_with_f11, update_panel_view).chain());
    }
}

// ---- Resources & Components -------------------------------------------------

/// Tracks whether the asset browser modal is open.
#[derive(Resource, Default)]
pub struct AssetBrowserState {
    pub open: bool,
}

/// Marks the root overlay node (for show/hide).
#[derive(Component)]
struct BrowserRoot;

/// Marks the scrollable content container that gets rebuilt on open.
#[derive(Component)]
struct BrowserContent;

// ---- Colors -----------------------------------------------------------------

const OVERLAY_BG: Color = Color::srgba(0.0, 0.0, 0.0, 0.80);
const PANEL_BG: Color = Color::srgba(0.06, 0.07, 0.10, 0.97);

const TITLE_COL: Color = Color::srgb(1.0, 0.92, 0.2); // yellow — 22pt title
const SECTION_COL: Color = Color::srgb(0.4, 0.88, 0.95); // cyan — section headers
const FOOTER_COL: Color = Color::srgb(0.40, 0.40, 0.45); // dim — "F11 close"
const NONE_COL: Color = Color::srgb(0.42, 0.42, 0.46); // dim gray — "(none loaded)"

// License-coded row colors.
const COL_CC0: Color = Color::srgb(0.3, 0.92, 0.3); // green  — CC0 / Public Domain
const COL_CCBY: Color = Color::srgb(0.95, 0.88, 0.2); // yellow — CC-BY-*
const COL_OSS: Color = Color::srgb(0.4, 0.88, 0.95); // cyan   — MIT / Apache
const COL_UNTAGGED: Color = Color::srgb(0.95, 0.28, 0.28); // red    — UNTAGGED

// ---- Startup: build skeletal UI tree ----------------------------------------

fn spawn_browser_panel(mut commands: Commands) {
    // Full-screen dark overlay — hidden by default.
    let root = commands
        .spawn((
            BrowserRoot,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                display: Display::None,
                ..default()
            },
            BackgroundColor(OVERLAY_BG),
            ZIndex(210),
        ))
        .id();

    // Centered 600×460 modal panel.
    let panel = commands
        .spawn((
            Node {
                width: Val::Px(600.0),
                height: Val::Px(460.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(20.0)),
                row_gap: Val::Px(4.0),
                overflow: Overflow::clip_y(),
                ..default()
            },
            BackgroundColor(PANEL_BG),
        ))
        .id();
    commands.entity(root).add_child(panel);

    // Title: "ASSET BROWSER" 22pt yellow.
    let title = commands
        .spawn((
            Text::new("ASSET BROWSER"),
            TextFont {
                font_size: 22.0,
                ..default()
            },
            TextColor(TITLE_COL),
        ))
        .id();
    commands.entity(panel).add_child(title);

    // Thin separator gap.
    let sep = commands
        .spawn(Node {
            height: Val::Px(6.0),
            ..default()
        })
        .id();
    commands.entity(panel).add_child(sep);

    // Scrollable content area — rebuilt each time the panel opens.
    let content = commands
        .spawn((
            BrowserContent,
            Node {
                flex_direction: FlexDirection::Column,
                flex_grow: 1.0,
                row_gap: Val::Px(3.0),
                overflow: Overflow::clip_y(),
                ..default()
            },
        ))
        .id();
    commands.entity(panel).add_child(content);

    // Footer gap + hint.
    let footer_gap = commands
        .spawn(Node {
            height: Val::Px(6.0),
            ..default()
        })
        .id();
    commands.entity(panel).add_child(footer_gap);

    let footer = commands
        .spawn((
            Text::new("F11  close"),
            TextFont {
                font_size: 11.0,
                ..default()
            },
            TextColor(FOOTER_COL),
        ))
        .id();
    commands.entity(panel).add_child(footer);
}

// ---- Toggle: F11 flips open state ------------------------------------------

fn toggle_with_f11(
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<AssetBrowserState>,
    mut root_q: Query<&mut Node, With<BrowserRoot>>,
) {
    if keys.just_pressed(KeyCode::F11) {
        state.open = !state.open;
        let display = if state.open {
            Display::Flex
        } else {
            Display::None
        };
        for mut node in &mut root_q {
            node.display = display;
        }
    }
}

// ---- Update: rebuild content rows on open -----------------------------------

fn update_panel_view(
    state: Res<AssetBrowserState>,
    manifest: Option<Res<AssetManifest>>,
    glbs: Option<Res<LoadedVehicleGlbs>>,
    heightmaps: Option<Res<LoadedHeightmaps>>,
    content_q: Query<Entity, With<BrowserContent>>,
    mut commands: Commands,
) {
    // Only rebuild when the open flag just became true.
    if !state.is_changed() || !state.open {
        return;
    }

    let Ok(content_entity) = content_q.single() else {
        return;
    };

    // Despawn all existing children of the content node.
    commands.entity(content_entity).despawn_related::<Children>();

    // ---- Gather manifest data (fall back to empty defaults) ----

    let empty_manifest = AssetManifest::default();
    let manifest = manifest
        .as_deref()
        .unwrap_or(&empty_manifest);

    let empty_glbs = LoadedVehicleGlbs::default();
    let glbs = glbs.as_deref().unwrap_or(&empty_glbs);

    let empty_hm = LoadedHeightmaps::default();
    let heightmaps = heightmaps.as_deref().unwrap_or(&empty_hm);

    // Collect manifest vehicle names for untagged-GLB detection.
    let manifest_vehicle_names: std::collections::HashSet<&str> = manifest
        .vehicles
        .iter()
        .map(|v| v.name.as_str())
        .collect();

    // Collect untagged GLBs (loaded but not in manifest).
    let untagged_vehicles: Vec<&String> = glbs
        .by_name
        .keys()
        .filter(|k| !manifest_vehicle_names.contains(k.as_str()))
        .collect();

    // Collect manifest map names for untagged-heightmap detection.
    let manifest_map_names: std::collections::HashSet<&str> = manifest
        .maps
        .iter()
        .map(|m| m.name.as_str())
        .collect();

    // Collect untagged heightmaps (loaded but not in manifest).
    let untagged_maps: Vec<&String> = heightmaps
        .by_name
        .keys()
        .filter(|k| !manifest_map_names.contains(k.as_str()))
        .collect();

    let vehicle_count = manifest.vehicles.len() + untagged_vehicles.len();
    let map_count = manifest.maps.len() + untagged_maps.len();
    let mod_count = manifest.mods.len();

    let mut children: Vec<Entity> = Vec::new();

    // ---- VEHICLES SECTION ----
    children.push(section_header(
        &mut commands,
        &format!("Vehicles ({})", vehicle_count),
    ));

    if vehicle_count == 0 {
        children.push(none_loaded_row(&mut commands));
    } else {
        for v in &manifest.vehicles {
            let label = format!("{} — {} — {}", v.name, v.author, v.license);
            let color = license_color(&v.license);
            children.push(row_text(&mut commands, &label, color));
        }
        let mut untagged_sorted = untagged_vehicles;
        untagged_sorted.sort();
        for name in untagged_sorted {
            let label = format!("{} — UNTAGGED", name);
            children.push(row_text(&mut commands, &label, COL_UNTAGGED));
        }
    }

    // Section gap.
    children.push(gap_node(&mut commands, 8.0));

    // ---- MAPS SECTION ----
    children.push(section_header(
        &mut commands,
        &format!("Maps ({})", map_count),
    ));

    if map_count == 0 {
        children.push(none_loaded_row(&mut commands));
    } else {
        for m in &manifest.maps {
            let label = format!("{} — {}", m.name, m.license);
            let color = license_color(&m.license);
            children.push(row_text(&mut commands, &label, color));
        }
        let mut untagged_sorted = untagged_maps;
        untagged_sorted.sort();
        for name in untagged_sorted {
            let label = format!("{} — UNTAGGED", name);
            children.push(row_text(&mut commands, &label, COL_UNTAGGED));
        }
    }

    // Section gap.
    children.push(gap_node(&mut commands, 8.0));

    // ---- MODS SECTION ----
    children.push(section_header(
        &mut commands,
        &format!("Mods ({})", mod_count),
    ));

    if mod_count == 0 {
        children.push(none_loaded_row(&mut commands));
    } else {
        for mo in &manifest.mods {
            let label = format!("{} v{} — {} — {}", mo.name, mo.version, mo.author, mo.license);
            let color = license_color(&mo.license);
            children.push(row_text(&mut commands, &label, color));
        }
    }

    // Attach all new children to the content node.
    commands.entity(content_entity).add_children(&children);
}

// ---- Color coding by license tag --------------------------------------------

fn license_color(license: &str) -> Color {
    let lower = license.to_lowercase();
    if lower.contains("cc0") || lower.contains("public domain") {
        COL_CC0
    } else if lower.starts_with("cc-by") || lower.starts_with("cc by") {
        COL_CCBY
    } else if lower.contains("mit") || lower.contains("apache") {
        COL_OSS
    } else {
        COL_UNTAGGED
    }
}

// ---- Small UI helpers -------------------------------------------------------

fn section_header(commands: &mut Commands, text: &str) -> Entity {
    commands
        .spawn((
            Text::new(text.to_owned()),
            TextFont {
                font_size: 14.0,
                ..default()
            },
            TextColor(SECTION_COL),
        ))
        .id()
}

fn row_text(commands: &mut Commands, text: &str, color: Color) -> Entity {
    commands
        .spawn((
            Text::new(text.to_owned()),
            TextFont {
                font_size: 12.0,
                ..default()
            },
            TextColor(color),
        ))
        .id()
}

fn none_loaded_row(commands: &mut Commands) -> Entity {
    commands
        .spawn((
            Text::new("(none loaded)"),
            TextFont {
                font_size: 12.0,
                ..default()
            },
            TextColor(NONE_COL),
        ))
        .id()
}

fn gap_node(commands: &mut Commands, px: f32) -> Entity {
    commands
        .spawn(Node {
            height: Val::Px(px),
            ..default()
        })
        .id()
}
