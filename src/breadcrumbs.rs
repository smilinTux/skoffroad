// Breadcrumb trail: drop bright orange sphere markers every DROP_INTERVAL_M metres.
// B       — toggle dropping on/off (leaves existing crumbs in place when turned off)
// Shift+B — clear all alive crumbs immediately
// HUD panel top-right at 408 px shows BREADCRUMBS: ON/OFF (count).

use bevy::prelude::*;
use std::collections::VecDeque;

use crate::terrain::terrain_height_at;
use crate::vehicle::{Chassis, VehicleRoot};

// ---- Constants ----------------------------------------------------------------

const DROP_INTERVAL_M: f32 = 50.0;
const MAX_BREADCRUMBS: usize = 50;
const CRUMB_RADIUS: f32 = 0.3;
const CRUMB_Y_OFFSET: f32 = 0.3;

// ---- Resources ----------------------------------------------------------------

#[derive(Resource)]
pub struct BreadcrumbState {
    /// Whether dropping is active (toggled by B).
    pub dropping: bool,
    /// Last drop position (XZ).
    pub last_drop_xz: Option<Vec2>,
    /// Total breadcrumbs alive.
    pub count: u32,
}

impl Default for BreadcrumbState {
    fn default() -> Self {
        Self {
            dropping: false,
            last_drop_xz: None,
            count: 0,
        }
    }
}

#[derive(Resource, Default)]
pub struct BreadcrumbQueue {
    pub entities: VecDeque<Entity>,
}

// ---- Components ---------------------------------------------------------------

#[derive(Component)]
struct Breadcrumb;

#[derive(Component)]
struct BreadcrumbHudText;

// ---- Plugin -------------------------------------------------------------------

pub struct BreadcrumbsPlugin;

impl Plugin for BreadcrumbsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BreadcrumbState>()
            .init_resource::<BreadcrumbQueue>()
            .add_systems(Startup, spawn_breadcrumb_hud)
            .add_systems(
                Update,
                (
                    toggle_breadcrumb_drop,
                    drop_breadcrumb.run_if(resource_exists::<crate::vehicle::VehicleRoot>),
                    clear_breadcrumbs,
                    update_breadcrumb_hud,
                ),
            );
    }
}

// ---- Startup ------------------------------------------------------------------

fn spawn_breadcrumb_hud(mut commands: Commands) {
    let bg = Color::srgba(0.05, 0.05, 0.07, 0.75);

    let panel = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(12.0),
                top: Val::Px(408.0),
                width: Val::Px(140.0),
                padding: UiRect::all(Val::Px(8.0)),
                flex_direction: FlexDirection::Column,
                ..default()
            },
            BackgroundColor(bg),
            ZIndex(20),
        ))
        .id();

    let text = commands
        .spawn((
            BreadcrumbHudText,
            Text::new("BREADCRUMBS: OFF (0)"),
            TextFont {
                font_size: 12.0,
                ..default()
            },
            TextColor(Color::srgb(0.55, 0.55, 0.55)),
        ))
        .id();

    commands.entity(panel).add_child(text);
}

// ---- Update systems -----------------------------------------------------------

fn toggle_breadcrumb_drop(
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<BreadcrumbState>,
) {
    let shift_held = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);

    // Shift+B is handled by clear_breadcrumbs; skip toggle when Shift is held.
    if !shift_held && keys.just_pressed(KeyCode::KeyB) {
        state.dropping = !state.dropping;
        if state.dropping {
            info!("breadcrumbs: dropping enabled");
        }
    }
}

fn drop_breadcrumb(
    vehicle: Res<VehicleRoot>,
    chassis_q: Query<&Transform, With<Chassis>>,
    mut state: ResMut<BreadcrumbState>,
    mut queue: ResMut<BreadcrumbQueue>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if !state.dropping {
        return;
    }

    let Ok(transform) = chassis_q.get(vehicle.chassis) else {
        return;
    };

    let pos = transform.translation;
    let current_xz = Vec2::new(pos.x, pos.z);

    let should_drop = match state.last_drop_xz {
        None => true,
        Some(last) => current_xz.distance(last) >= DROP_INTERVAL_M,
    };

    if !should_drop {
        return;
    }

    // Enforce cap: despawn oldest if at limit.
    if queue.entities.len() >= MAX_BREADCRUMBS {
        if let Some(oldest) = queue.entities.pop_front() {
            commands.entity(oldest).despawn();
            state.count = state.count.saturating_sub(1);
        }
    }

    let y = terrain_height_at(pos.x, pos.z) + CRUMB_Y_OFFSET;

    let mesh = meshes.add(Sphere::new(CRUMB_RADIUS).mesh().ico(1).unwrap());
    let material = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.5, 0.0),
        emissive: LinearRgba::rgb(2.0, 1.0, 0.0),
        ..default()
    });

    let entity = commands
        .spawn((
            Breadcrumb,
            Mesh3d(mesh),
            MeshMaterial3d(material),
            Transform::from_translation(Vec3::new(pos.x, y, pos.z)),
        ))
        .id();

    queue.entities.push_back(entity);
    state.last_drop_xz = Some(current_xz);
    state.count += 1;
}

fn clear_breadcrumbs(
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<BreadcrumbState>,
    mut queue: ResMut<BreadcrumbQueue>,
    mut commands: Commands,
) {
    let shift_held = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if shift_held && keys.just_pressed(KeyCode::KeyB) {
        for entity in queue.entities.drain(..) {
            commands.entity(entity).despawn();
        }
        state.count = 0;
        state.last_drop_xz = None;
    }
}

fn update_breadcrumb_hud(
    state: Res<BreadcrumbState>,
    mut text_q: Query<(&mut Text, &mut TextColor), With<BreadcrumbHudText>>,
) {
    for (mut text, mut color) in &mut text_q {
        if state.dropping {
            text.0 = format!("BREADCRUMBS: ON ({})", state.count);
            color.0 = Color::srgb(0.3, 0.95, 0.3);
        } else {
            text.0 = format!("BREADCRUMBS: OFF ({})", state.count);
            color.0 = Color::srgb(0.55, 0.55, 0.55);
        }
    }
}
