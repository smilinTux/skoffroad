// Contact / blob shadow under the vehicle chassis.
//
// A single flat quad projected just above the ground directly below the
// chassis.  The quad's alpha scales with the chassis ride height so it
// fades out when the truck is airborne and darkens when the suspension
// compresses hard onto a rock.
//
// Tier gate: Medium+ only (Low skips spawn entirely — cheap CPU + GPU).
//
// Respawn-safe: we do NOT use a `Local<bool>` done-guard.  Instead we
// track whether the shadow entity exists via a component query.  When the
// chassis despawns (vehicle_mods change → RespawnRequest) the shadow entity
// is automatically despawned as a child.  The attach system re-fires next
// frame because the query finds no existing ContactShadow entity.
//
// Public API:
//   ContactShadowPlugin

use bevy::prelude::*;
use crate::graphics_quality::GraphicsQuality;
use crate::vehicle::{Chassis, VehicleRoot};

// ---- Constants --------------------------------------------------------------

/// Half-extents of the shadow quad (full width × full length of the truck).
const SHADOW_W: f32 = 2.4;
const SHADOW_L: f32 = 4.8;

/// The quad hovers this many metres above the chassis local Y floor so it
/// doesn't z-fight with the terrain.
const SHADOW_Y_OFFSET: f32 = -0.44;

/// Alpha at zero ride height (pressed onto ground).
const ALPHA_GROUNDED: f32 = 0.55;

/// Alpha fade start — above this many metres the shadow begins to fade.
const FADE_START_M: f32 = 0.3;

/// Alpha fade end — completely invisible above this ride height.
const FADE_END_M: f32 = 2.5;

// ---- Component / Plugin -----------------------------------------------------

/// Marker on the contact-shadow quad entity.
#[derive(Component)]
pub struct ContactShadow;

pub struct ContactShadowPlugin;

impl Plugin for ContactShadowPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (attach_shadow, update_shadow_alpha));
    }
}

// ---- attach_shadow ----------------------------------------------------------
//
// Runs every frame.  Spawns the shadow quad as a child of the chassis the
// first frame that VehicleRoot exists and no ContactShadow entity is found.
// This fires again after a respawn because the old child was auto-despawned
// with the chassis.

fn attach_shadow(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    quality: Res<GraphicsQuality>,
    vehicle: Option<Res<VehicleRoot>>,
    existing: Query<(), With<ContactShadow>>,
) {
    // Skip on Low tier.
    if *quality == GraphicsQuality::Low {
        return;
    }

    // Only spawn if VehicleRoot exists.
    let Some(vehicle) = vehicle else { return };

    // Already attached (or chassis still alive from last spawn).
    if !existing.is_empty() {
        return;
    }

    // Flat plane quad oriented horizontally (XZ plane, facing up in Y).
    // Rectangle in Bevy 0.18: Rectangle::new(width, height) creates an
    // XY-aligned quad; we rotate 90° around X to lie in XZ (horizontal).
    let shadow_mesh = meshes.add(Rectangle::new(SHADOW_W, SHADOW_L));

    let shadow_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(0.0, 0.0, 0.0, ALPHA_GROUNDED),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        // Disable backface culling so it reads from both sides.
        double_sided: true,
        cull_mode: None,
        // No depth write — pure decal/overlay.
        depth_bias: -1.0,
        ..default()
    });

    // Rotate the XY quad 90° around X so it lies flat in the XZ plane (Y up).
    let shadow_tf = Transform::from_translation(Vec3::new(0.0, SHADOW_Y_OFFSET, 0.0))
        .with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2));

    let shadow_id = commands.spawn((
        ContactShadow,
        Mesh3d(shadow_mesh),
        MeshMaterial3d(shadow_mat),
        shadow_tf,
    )).id();

    commands.entity(vehicle.chassis).add_child(shadow_id);
}

// ---- update_shadow_alpha ----------------------------------------------------
//
// Each frame: measure the chassis world height minus the terrain-approximate
// ground (we use the chassis translation Y as a proxy — when the vehicle sits
// on the ground its chassis Y is roughly 0.6–1.0 m above the terrain surface,
// depending on suspension compression).  We compare against the natural rest
// height to derive a normalised ride height.

fn update_shadow_alpha(
    vehicle: Option<Res<VehicleRoot>>,
    chassis_q: Query<&Transform, With<Chassis>>,
    shadow_q: Query<&MeshMaterial3d<StandardMaterial>, With<ContactShadow>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let Some(vehicle) = vehicle else { return };
    let Ok(c_tf) = chassis_q.get(vehicle.chassis) else { return };

    // Approximate ride height: chassis world Y.  The truck spawns at ~1.0 m;
    // when fully compressed the body may sit ~0.5 m; in the air it climbs.
    // We bias by -0.6 (the nominal rest height) so 0 = grounded, >0 = airborne.
    let ride_h = (c_tf.translation.y - 0.6).max(0.0);

    // Map [FADE_START_M, FADE_END_M] → alpha [ALPHA_GROUNDED, 0]
    let t = ((ride_h - FADE_START_M) / (FADE_END_M - FADE_START_M)).clamp(0.0, 1.0);
    let alpha = ALPHA_GROUNDED * (1.0 - t);

    for mat_handle in &shadow_q {
        if let Some(mat) = materials.get_mut(&mat_handle.0) {
            // Preserve RGB (black), only update alpha.
            mat.base_color = Color::srgba(0.0, 0.0, 0.0, alpha);
        }
    }
}
