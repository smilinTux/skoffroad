// Volumetric headlight beams — Effect 2 of Sprint 67.
//
// For each SpotLight entity that is a direct child of the chassis (placed by
// headlights.rs), spawn a sibling translucent cone mesh that makes the beam
// visible in the dark. We do NOT import headlights.rs's private Headlight
// marker: instead we detect the SpotLight children of the chassis entity.
//
// Cone visibility is tied to HeadlightState.on / auto-night detection.
//
// Public API:
//   HeadlightBeamsPlugin

use bevy::prelude::*;

use crate::headlights::HeadlightState;
use crate::sky::TimeOfDay;
use crate::vehicle::VehicleRoot;

// ---- Plugin -----------------------------------------------------------------

pub struct HeadlightBeamsPlugin;

impl Plugin for HeadlightBeamsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (
               spawn_beam_cones,
               update_beam_visibility,
           ));
    }
}

// ---- Marker component -------------------------------------------------------

#[derive(Component)]
pub struct HeadlightBeamCone;

// ---- Spawn (runs once, guarded by Local<bool>) ------------------------------

fn spawn_beam_cones(
    mut done: Local<bool>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    vehicle: Option<Res<VehicleRoot>>,
    children_q: Query<&Children>,
    spotlights: Query<(Entity, &Transform), With<SpotLight>>,
) {
    if *done {
        return;
    }
    let Some(vehicle) = vehicle else { return };

    // Collect chassis children.
    let Ok(chassis_children) = children_q.get(vehicle.chassis) else { return };
    let chassis_child_set: std::collections::HashSet<Entity> =
        chassis_children.iter().collect();

    // Find spotlight children of the chassis.
    let mut found = false;
    for (entity, transform) in spotlights.iter() {
        if !chassis_child_set.contains(&entity) {
            continue;
        }
        found = true;

        let beam_mat = materials.add(StandardMaterial {
            base_color: Color::srgba(1.0, 0.95, 0.85, 0.10),
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            cull_mode: None,
            double_sided: true,
            ..default()
        });

        // Cone primitive in Bevy 0.18: apex at +Y, base at -Y, height along Y.
        // We want the beam to open toward -Z (forward), so rotate -90° around X.
        let cone_mesh = meshes.add(Cone { radius: 0.6, height: 8.0 });
        let rot = Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2);
        let cone_tf = Transform::from_translation(transform.translation)
            .with_rotation(transform.rotation * rot);

        let cone_id = commands.spawn((
            HeadlightBeamCone,
            Mesh3d(cone_mesh),
            MeshMaterial3d(beam_mat),
            cone_tf,
            Visibility::Hidden,
        )).id();

        commands.entity(vehicle.chassis).add_child(cone_id);
    }

    // Mark done once we found and processed spotlight children.
    if found {
        *done = true;
    }
}

// ---- Update: visibility tied to headlight state -----------------------------

fn update_beam_visibility(
    state: Res<HeadlightState>,
    tod:   Res<TimeOfDay>,
    mut cones: Query<&mut Visibility, With<HeadlightBeamCone>>,
) {
    let is_night = tod.t < 0.25 || tod.t > 0.75;
    let active   = if state.auto { is_night } else { state.on };
    let vis      = if active { Visibility::Visible } else { Visibility::Hidden };

    for mut v in &mut cones {
        *v = vis;
    }
}
