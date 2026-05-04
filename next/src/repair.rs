// Repair zone: a glowing green disk near spawn where damage decays fast.
// Placed at world XZ (20, 20) — the player must drive there intentionally.
// Healing rate: 5% per second (vs. the 0.1%/s passive regen in damage.rs).

use bevy::prelude::*;
use crate::damage::DamageState;
use crate::terrain::terrain_height_at;
use crate::vehicle::{Chassis, VehicleRoot};

// ---- Public types -----------------------------------------------------------

pub struct RepairPlugin;

impl Plugin for RepairPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RepairActive>()
           .add_systems(Startup, (spawn_repair_zone, spawn_repair_hud))
           .add_systems(Update, (
               apply_repair.run_if(resource_exists::<VehicleRoot>),
               update_repair_hud,
               rotate_marker,
           ));
    }
}

/// Shared state the HUD (and any other system) can read.
#[derive(Resource, Default)]
pub struct RepairActive {
    pub in_zone: bool,
    /// 0..=1 — 1.0 at the center, 0.0 at the outer edge.
    pub strength: f32,
}

// ---- Constants ---------------------------------------------------------------

const ZONE_XZ: Vec2   = Vec2::new(20.0, 20.0);
const ZONE_RADIUS: f32 = 4.0;
const HEAL_PER_SEC: f32 = 0.05;

// ---- Private marker components -----------------------------------------------

#[derive(Component)] struct RepairMarker;
#[derive(Component)] struct RepairHudRoot;
#[derive(Component)] struct RepairHudText;

// ---- Spawn -------------------------------------------------------------------

fn spawn_repair_zone(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let ground_y = terrain_height_at(ZONE_XZ.x, ZONE_XZ.y);
    let disk_y   = ground_y + 0.05;

    // Emissive green shared between disk and marker.
    let glow_mat = materials.add(StandardMaterial {
        base_color:  Color::srgb(0.05, 0.25, 0.08),
        emissive:    LinearRgba::rgb(0.2, 4.0, 0.4),
        unlit:       false,
        perceptual_roughness: 0.7,
        ..default()
    });

    // Flat disk — thin cylinder lying on the terrain.
    commands.spawn((
        Mesh3d(meshes.add(Cylinder::new(ZONE_RADIUS, 0.1))),
        MeshMaterial3d(glow_mat.clone()),
        Transform::from_xyz(ZONE_XZ.x, disk_y, ZONE_XZ.y),
    ));

    // Tall thin marker — visible from a distance, rotates slowly.
    commands.spawn((
        RepairMarker,
        Mesh3d(meshes.add(Cylinder::new(0.3, 4.0))),
        MeshMaterial3d(glow_mat),
        Transform::from_xyz(ZONE_XZ.x, disk_y + 2.05, ZONE_XZ.y),
    ));
}

// ---- Marker rotation ---------------------------------------------------------

fn rotate_marker(time: Res<Time>, mut q: Query<&mut Transform, With<RepairMarker>>) {
    let dt = time.delta_secs();
    for mut t in &mut q {
        t.rotate_y(dt * 0.8); // ~one full turn per ~8 s
    }
}

// ---- Healing logic -----------------------------------------------------------

fn apply_repair(
    vehicle:    Res<VehicleRoot>,
    chassis_q:  Query<&Transform, With<Chassis>>,
    time:       Res<Time>,
    mut damage: ResMut<DamageState>,
    mut active: ResMut<RepairActive>,
) {
    let Ok(transform) = chassis_q.get(vehicle.chassis) else {
        active.in_zone = false;
        active.strength = 0.0;
        return;
    };

    let pos = transform.translation;
    let dist_xz = Vec2::new(pos.x, pos.z).distance(ZONE_XZ);

    if dist_xz < ZONE_RADIUS {
        let strength = 1.0 - dist_xz / ZONE_RADIUS;
        active.in_zone  = true;
        active.strength = strength;

        let dt = time.delta_secs();
        damage.damage = (damage.damage - HEAL_PER_SEC * dt).max(0.0);
    } else {
        active.in_zone  = false;
        active.strength = 0.0;
    }
}

// ---- HUD ---------------------------------------------------------------------

fn spawn_repair_hud(mut commands: Commands) {
    // Small banner at top-centre, below the damage HUD (which sits at top: 12px).
    // top: 90px clears the damage bar without crowding trial timers.
    const PANEL_W: f32 = 200.0;
    const PANEL_H: f32 = 30.0;

    let root = commands.spawn((
        RepairHudRoot,
        Node {
            position_type: PositionType::Absolute,
            left:   Val::Percent(50.0),
            top:    Val::Px(90.0),
            margin: UiRect {
                left: Val::Px(-(PANEL_W / 2.0)),
                ..default()
            },
            width:  Val::Px(PANEL_W),
            height: Val::Px(PANEL_H),
            justify_content: JustifyContent::Center,
            align_items:     AlignItems::Center,
            display:         Display::None,
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.15, 0.05, 0.80)),
        Outline {
            width:  Val::Px(1.0),
            offset: Val::Px(0.0),
            color:  Color::srgb(0.2, 1.0, 0.4),
        },
    )).id();

    let label = commands.spawn((
        RepairHudText,
        Text::new("REPAIRING..."),
        TextFont { font_size: 14.0, ..default() },
        TextColor(Color::srgb(0.2, 1.0, 0.4)),
    )).id();

    commands.entity(root).add_child(label);
}

fn update_repair_hud(
    active:   Res<RepairActive>,
    mut root: Query<&mut Node, With<RepairHudRoot>>,
) {
    for mut node in &mut root {
        node.display = if active.in_zone { Display::Flex } else { Display::None };
    }
}
