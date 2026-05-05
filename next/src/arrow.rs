// Floating 3-D objective arrow.
//
// Spawned once at startup as a top-level entity (not parented to the chassis).
// Each Update frame the arrow is repositioned 3 m above the chassis and aimed
// horizontally at `CourseState::current_target`.  It bobs ±0.15 m on a sin
// wave for extra visibility.  Both sub-meshes are bright cyan emissive so the
// arrow pops against any terrain colour.

use bevy::prelude::*;

use crate::course::CourseState;
use crate::vehicle::{Chassis, VehicleRoot};

// ---- Plugin ----------------------------------------------------------------

pub struct ArrowPlugin;

impl Plugin for ArrowPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_arrow)
           .add_systems(Update, update_arrow);
    }
}

// ---- Marker component ------------------------------------------------------

#[derive(Component)]
struct ObjectiveArrow;

// ---- Constants -------------------------------------------------------------

/// Height above chassis origin at which the arrow hovers.
const HOVER_HEIGHT: f32 = 3.0;

/// Bob amplitude in metres.
const BOB_AMPLITUDE: f32 = 0.15;

/// Bob frequency in radians per second.
const BOB_FREQ: f32 = 2.0;

// ---- Startup: spawn --------------------------------------------------------

fn spawn_arrow(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let cyan_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.2, 1.0, 1.0),
        emissive: LinearRgba::rgb(0.5, 4.0, 4.0),
        ..default()
    });

    // Shaft: 0.2 × 0.2 × 1.5 m, centred at origin of the arrow entity.
    // We want the cone tip to be at +Z so we shift the shaft back by half its
    // length (−0.75 m) so its +Z face aligns with the origin; the cone then
    // sits just forward of that.
    let shaft_mesh = meshes.add(Cuboid::new(0.2, 0.2, 1.5));

    // Cone: radius 0.4, height 0.6 m.  Bevy's Cone base sits at −Y and tip at
    // +Y in local space, so we rotate it 90° around X to point along +Z, then
    // place it 0.75 + 0.3 = 1.05 m forward (half-shaft + half-cone-height) so
    // it sits directly in front of the shaft end.
    let head_mesh = meshes.add(Cone::new(0.4, 0.6));

    // Arrow root — hidden until we have both a chassis and a target.
    let arrow = commands.spawn((
        ObjectiveArrow,
        Transform::IDENTITY,
        Visibility::Hidden,
    )).id();

    // Shaft child: centred behind the origin so the tip of the shaft meets Z=0.
    // Shaft runs −1.5 m to 0 m along Z, so its centre is at −0.75 m.
    let shaft = commands.spawn((
        Mesh3d(shaft_mesh),
        MeshMaterial3d(cyan_mat.clone()),
        Transform::from_translation(Vec3::new(0.0, 0.0, -0.75)),
    )).id();

    // Cone child: base at Z=0, tip at Z=+0.6 m.
    // Cone needs to be rotated so its tip points along +Z.
    // Default Cone has tip at +Y; rotate −90° around X.
    let cone_rot = Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2);
    let head = commands.spawn((
        Mesh3d(head_mesh),
        MeshMaterial3d(cyan_mat),
        Transform::from_translation(Vec3::new(0.0, 0.0, 0.3))
            .with_rotation(cone_rot),
    )).id();

    commands.entity(arrow).add_children(&[shaft, head]);
}

// ---- Update: track chassis and point at target -----------------------------

fn update_arrow(
    course: Option<Res<CourseState>>,
    vehicle: Option<Res<VehicleRoot>>,
    // Disjoint filters: both queries touch Transform (immut on Chassis,
    // mut on ObjectiveArrow). Bevy needs proof they don't overlap or panics
    // with B0001 at startup. Same fix as minimap::update_minimap.
    chassis_q: Query<&Transform, (With<Chassis>, Without<ObjectiveArrow>)>,
    mut arrow_q: Query<(&mut Transform, &mut Visibility), (With<ObjectiveArrow>, Without<Chassis>)>,
    time: Res<Time>,
) {
    // Both queries need to be resolvable; bail out silently if either resource
    // hasn't been inserted yet (other agents may be filling them in).
    let Some(vehicle) = vehicle else { return };
    let Ok(chassis_tf) = chassis_q.get(vehicle.chassis) else { return };
    let Ok((mut arrow_tf, mut arrow_vis)) = arrow_q.single_mut() else { return };

    // If no target is known, hide the arrow and stop.
    let Some(ref course_state) = course else {
        *arrow_vis = Visibility::Hidden;
        return;
    };
    let Some(target) = course_state.current_target else {
        *arrow_vis = Visibility::Hidden;
        return;
    };

    *arrow_vis = Visibility::Inherited;

    let chassis_pos = chassis_tf.translation;

    // Bob offset along world Y.
    let bob = BOB_AMPLITUDE * (time.elapsed_secs() * BOB_FREQ).sin();

    // Arrow position: 3 m above chassis + bob.
    let arrow_pos = chassis_pos + Vec3::Y * (HOVER_HEIGHT + bob);

    // Horizontal direction from chassis to target (ignore altitude difference).
    let dir = (target - chassis_pos).with_y(0.0).normalize_or_zero();

    if dir.length_squared() < 1e-6 {
        // Chassis is on top of the target; keep old rotation, just update position.
        arrow_tf.translation = arrow_pos;
        return;
    }

    // Build a transform that looks along `dir` with Y as up.
    // `looking_to` points the local −Z axis toward the direction; we set up our
    // meshes so the cone tip is at +Z, so we invert: look_to the *opposite* of
    // dir for −Z, which means we pass `dir` directly (−Z faces away from dir,
    // +Z faces toward dir).
    *arrow_tf = Transform::from_translation(arrow_pos)
        .looking_to(dir, Vec3::Y);
}
