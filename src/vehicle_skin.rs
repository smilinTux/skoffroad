//! Drivable glTF vehicle skin — feature `vehicle_skin`.
//!
//! Parents a loaded GLB scene (from `glb_loader.rs` / `assets/manifest.json`) to
//! the physics `Chassis` and hides the procedural body (`DefaultSkin`), so a
//! dropped-in glTF truck becomes the vehicle you actually drive. PHYSICS IS
//! UNCHANGED — the collider, mass, and wheels are still the procedural rig in
//! `vehicle.rs`; this only swaps the visible body shell.
//!
//! Off by default (no regression). Enable with:
//!   cargo run --features vehicle_skin
//!   cargo run --features "vehicle_skin engine_samples"   # both
//!
//! The model is fitted with the SKIN_* consts below. I can't see the render, so
//! these are best-guess defaults — tune SKIN_SCALE / SKIN_YAW_DEG / SKIN_OFFSET
//! to your model (most glTF cars need a yaw of 0 or 180 and a small Y offset so
//! the wheels meet the ground). See docs/ASSETS.md.

use bevy::prelude::*;

pub struct VehicleSkinPlugin;

impl Plugin for VehicleSkinPlugin {
    fn build(&self, app: &mut App) {
        #[cfg(feature = "vehicle_skin")]
        imp::register(app);
        #[cfg(not(feature = "vehicle_skin"))]
        {
            let _ = app; // no-op: procedural body in vehicle.rs stays in use.
        }
    }
}

#[cfg(feature = "vehicle_skin")]
mod imp {
    use bevy::prelude::*;

    use crate::glb_loader::LoadedVehicleGlbs;
    use crate::vehicle::{Chassis, DefaultSkin, VehicleRoot};

    /// Manifest vehicle to use as the drivable body, by glb file stem.
    const SKIN_STEM: &str = "pickup_truck";
    /// Uniform scale applied to the glTF body.
    const SKIN_SCALE: f32 = 1.0;
    /// Chassis-local offset (Y lowers/raises the body onto the wheels).
    const SKIN_OFFSET: Vec3 = Vec3::new(0.0, -0.45, 0.0);
    /// Yaw in degrees — set to 180 if the model faces the wrong way (game fwd = -Z).
    const SKIN_YAW_DEG: f32 = 0.0;

    #[derive(Component)]
    struct GlbSkinRoot;

    /// Register the skin system (kept here so `apply_vehicle_skin` and the
    /// private `GlbSkinRoot` marker never need to be exposed outside this module).
    pub fn register(app: &mut App) {
        app.add_systems(Update, apply_vehicle_skin);
    }

    /// Attach the GLB body to the current chassis and hide the procedural shell.
    /// Re-runs after a respawn (chassis entity changes) so the skin persists
    /// across vehicle-mod rebuilds.
    fn apply_vehicle_skin(
        mut commands: Commands,
        vehicle: Option<Res<VehicleRoot>>,
        glbs: Option<Res<LoadedVehicleGlbs>>,
        mut skinned: Local<Option<Entity>>,
        chassis_q: Query<Entity, With<Chassis>>,
        existing_skins: Query<Entity, With<GlbSkinRoot>>,
        mut default_skin: Query<&mut Visibility, With<DefaultSkin>>,
    ) {
        let (Some(vehicle), Some(glbs)) = (vehicle, glbs) else { return };
        let chassis = vehicle.chassis;

        // Already skinned this exact chassis and it still exists → nothing to do.
        if *skinned == Some(chassis) && chassis_q.get(chassis).is_ok() {
            return;
        }
        // Chassis not spawned yet → wait.
        if chassis_q.get(chassis).is_err() {
            return;
        }
        // GLB scene must be registered by glb_loader (PostStartup, async load).
        let Some(scene) = glbs.by_name.get(SKIN_STEM) else { return };

        // Drop any skin left over from a previous (despawned) chassis.
        for e in &existing_skins {
            commands.entity(e).despawn();
        }

        // Hide the procedural body shell (re-created visible on each respawn).
        for mut vis in &mut default_skin {
            *vis = Visibility::Hidden;
        }

        // Parent the glTF body to the chassis.
        let tf = Transform::from_translation(SKIN_OFFSET)
            .with_rotation(Quat::from_rotation_y(SKIN_YAW_DEG.to_radians()))
            .with_scale(Vec3::splat(SKIN_SCALE));
        let skin = commands.spawn((GlbSkinRoot, SceneRoot(scene.clone()), tf)).id();
        commands.entity(chassis).add_child(skin);

        *skinned = Some(chassis);
        info!("vehicle_skin: attached '{}' glTF body to chassis", SKIN_STEM);
    }
}
