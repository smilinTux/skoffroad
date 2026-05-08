// Compass strip + randomised waypoint.
//
// Strip: 600x30 px at top-center. Cardinal/inter-cardinal labels scroll as the
// chassis rotates so the current heading is always centered. Child scroller is
// 3× period wide for seamless wrap. C key toggles visibility.
//
// Waypoint: bright yellow cylinder at a random XZ in [-90,90] m. UI text below
// the strip shows distance + bearing. Despawn + recycle when chassis within 5 m.

use bevy::prelude::*;
use std::f32::consts::TAU;

use crate::terrain::terrain_height_at;
use crate::vehicle::{Chassis, VehicleRoot};

pub struct CompassPlugin;

impl Plugin for CompassPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (spawn_compass, spawn_waypoint))
            .add_systems(
                Update,
                (update_compass, update_waypoint).run_if(resource_exists::<VehicleRoot>),
            );
    }
}

#[derive(Resource)]
pub struct Waypoint {
    pub position: Vec3,
    pub reached_count: u32,
}

#[derive(Component)] struct CompassRoot;
#[derive(Component)] struct CompassScroller;
#[derive(Component)] struct WaypointArrow;
#[derive(Component)] struct WaypointInfoText;
#[derive(Component)] struct WaypointMarker;

#[derive(Resource)]
struct CompassVisible(bool);
impl Default for CompassVisible { fn default() -> Self { Self(true) } }

const STRIP_W: f32  = 600.0;
const STRIP_H: f32  = 30.0;
const PERIOD_W: f32 = STRIP_W;       // one full 360° span in pixels
const SCROLL_W: f32 = PERIOD_W * 3.0; // three repetitions for seamless wrap

const DIRS: &[(&str, f32)] = &[
    ("N", 0.000), ("NE", 0.125), ("E", 0.250), ("SE", 0.375),
    ("S", 0.500), ("SW", 0.625), ("W", 0.750), ("NW", 0.875),
];

const BG: Color           = Color::srgba(0.05, 0.05, 0.07, 0.82);
const LABEL_COLOR: Color  = Color::srgb(0.92, 0.92, 0.92);
const ARROW_COLOR: Color  = Color::srgb(1.0, 0.85, 0.10);
const CENTER_COLOR: Color = Color::srgba(1.0, 1.0, 1.0, 0.55);
const INFO_COLOR: Color   = Color::srgb(0.85, 0.85, 0.55);

fn spawn_compass(mut commands: Commands) {
    commands.init_resource::<CompassVisible>();

    let root = commands.spawn((
        CompassRoot,
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(8.0),
            width: Val::Px(STRIP_W),
            height: Val::Px(STRIP_H),
            margin: UiRect { left: Val::Auto, right: Val::Auto, ..default() },
            overflow: Overflow::clip(),
            ..default()
        },
        BackgroundColor(BG),
        ZIndex(20),
    )).id();

    let scroller = commands.spawn((
        CompassScroller,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            top: Val::Px(0.0),
            width: Val::Px(SCROLL_W),
            height: Val::Px(STRIP_H),
            ..default()
        },
    )).id();

    for rep in 0..3i32 {
        let x_base = rep as f32 * PERIOD_W;
        for &(label, frac) in DIRS {
            let lx = x_base + frac * PERIOD_W - 8.0;
            let t = commands.spawn((
                Text::new(label),
                TextFont { font_size: 13.0, ..default() },
                TextColor(LABEL_COLOR),
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(lx),
                    top: Val::Px(6.0),
                    ..default()
                },
            )).id();
            commands.entity(scroller).add_child(t);

            let tick = commands.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(x_base + frac * PERIOD_W - 0.5),
                    top: Val::Px(STRIP_H - 6.0),
                    width: Val::Px(1.0),
                    height: Val::Px(5.0),
                    ..default()
                },
                BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.35)),
            )).id();
            commands.entity(scroller).add_child(tick);
        }
    }

    let center_mark = commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(STRIP_W / 2.0 - 1.0),
            top: Val::Px(STRIP_H - 10.0),
            width: Val::Px(2.0),
            height: Val::Px(10.0),
            ..default()
        },
        BackgroundColor(CENTER_COLOR),
        ZIndex(21),
    )).id();

    let arrow = commands.spawn((
        WaypointArrow,
        Text::new("v"),
        TextFont { font_size: 12.0, ..default() },
        TextColor(ARROW_COLOR),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(STRIP_W / 2.0 - 4.0),
            top: Val::Px(1.0),
            ..default()
        },
        ZIndex(22),
    )).id();

    commands.entity(root).add_children(&[scroller, center_mark, arrow]);

    commands.spawn((
        WaypointInfoText,
        Text::new("WPT: -- m  brg --"),
        TextFont { font_size: 13.0, ..default() },
        TextColor(INFO_COLOR),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(8.0 + STRIP_H + 4.0),
            margin: UiRect { left: Val::Auto, right: Val::Auto, ..default() },
            ..default()
        },
        ZIndex(20),
    ));
}

fn spawn_waypoint(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let pos = waypoint_pos(0);
    spawn_marker(&mut commands, &mut meshes, &mut materials, pos);
    commands.insert_resource(Waypoint { position: pos, reached_count: 0 });
}

fn update_compass(
    vehicle: Res<VehicleRoot>,
    waypoint: Option<Res<Waypoint>>,
    chassis_q: Query<&Transform, With<Chassis>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut visible: ResMut<CompassVisible>,
    mut root_q: Query<&mut Node, With<CompassRoot>>,
    mut scroller_q: Query<&mut Node, (With<CompassScroller>, Without<CompassRoot>)>,
    mut arrow_q: Query<
        (&mut Text, &mut Node),
        (With<WaypointArrow>, Without<CompassRoot>, Without<CompassScroller>),
    >,
    mut info_q: Query<
        (&mut Text, &mut Node),
        (With<WaypointInfoText>, Without<CompassRoot>, Without<CompassScroller>, Without<WaypointArrow>),
    >,
) {
    if keys.just_pressed(KeyCode::KeyC) { visible.0 = !visible.0; }

    let disp = if visible.0 { Display::Flex } else { Display::None };
    for mut n in &mut root_q { n.display = disp; }
    for (_, mut n) in &mut info_q { n.display = disp; }

    if !visible.0 { return; }
    let Ok(chassis_tf) = chassis_q.get(vehicle.chassis) else { return };

    // Yaw 0 = north (-Z), positive CW toward east (+X).
    let fwd = chassis_tf.forward();
    let yaw = fwd.x.atan2(-fwd.z);

    // Place the middle repetition's heading under the strip center:
    //   PERIOD_W + (yaw/TAU)*PERIOD_W + scroll_left == STRIP_W/2
    let scroll_left = STRIP_W / 2.0 - PERIOD_W - (yaw / TAU) * PERIOD_W;
    for mut n in &mut scroller_q { n.left = Val::Px(scroll_left); }

    if let Some(wpt) = waypoint {
        let p = chassis_tf.translation;
        let (dx, dz) = (wpt.position.x - p.x, wpt.position.z - p.z);
        let dist = (dx * dx + dz * dz).sqrt();
        let world_brg = dx.atan2(-dz);
        let rel_brg   = world_brg - yaw;
        let arrow_x   = (STRIP_W / 2.0 + (rel_brg / TAU) * PERIOD_W - 4.0)
            .clamp(2.0, STRIP_W - 14.0);
        for (_, mut n) in &mut arrow_q { n.left = Val::Px(arrow_x); }
        let brg_deg = world_brg.to_degrees().rem_euclid(360.0);
        for (mut t, _) in &mut info_q {
            t.0 = format!("WPT: {:.0} m  brg {:.0}", dist, brg_deg);
        }
    }
}

fn update_waypoint(
    mut commands: Commands,
    vehicle: Res<VehicleRoot>,
    mut waypoint: Option<ResMut<Waypoint>>,
    chassis_q: Query<&Transform, With<Chassis>>,
    marker_q: Query<Entity, With<WaypointMarker>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let Some(ref mut wpt) = waypoint else { return };
    let Ok(tf) = chassis_q.get(vehicle.chassis) else { return };
    let p = tf.translation;
    let (dx, dz) = (wpt.position.x - p.x, wpt.position.z - p.z);
    if dx * dx + dz * dz < 25.0 {
        info!("waypoint reached!");
        wpt.reached_count += 1;
        for e in marker_q.iter() { commands.entity(e).despawn(); }
        let new_pos = waypoint_pos(wpt.reached_count);
        wpt.position = new_pos;
        spawn_marker(&mut commands, &mut meshes, &mut materials, new_pos);
    }
}

fn spawn_marker(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    pos: Vec3,
) {
    commands.spawn((
        WaypointMarker,
        Mesh3d(meshes.add(Cylinder::new(0.5, 6.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(1.0, 0.9, 0.0),
            emissive: LinearRgba::rgb(1.5, 1.2, 0.0),
            ..default()
        })),
        Transform::from_translation(pos),
    ));
}

/// LCG (Numerical Recipes constants) — deterministic, no external dep.
fn lcg_f32(seed: u32) -> f32 {
    let v = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    v as f32 / u32::MAX as f32
}

fn waypoint_pos(index: u32) -> Vec3 {
    let x = lcg_f32(index.wrapping_mul(2)) * 180.0 - 90.0;
    let z = lcg_f32(index.wrapping_mul(2).wrapping_add(1)) * 180.0 - 90.0;
    Vec3::new(x, terrain_height_at(x, z) + 3.0, z)
}
