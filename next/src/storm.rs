// Storm: heavy rain particles + lightning flashes. Toggleable via "9" key.
// Layers over the existing wind/weather without replacing it.
//
// Public API:
//   StormPlugin
//   StormState (resource)
//
// Rain drops are simple thin cuboid mesh entities — no GPU particle library
// required.  Each RainDrop carries its own velocity and is repositioned above
// the camera when it falls below the terrain surface.
//
// Lightning uses a Local<f32> countdown timer.  When the timer fires, the
// fullscreen white overlay's BackgroundColor alpha is set to 1.0 and allowed
// to decay at 3.0/s back to 0.

use bevy::prelude::*;

use crate::terrain::terrain_height_at;

// ---- Public API ---------------------------------------------------------------

pub struct StormPlugin;

impl Plugin for StormPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<StormState>()
           .add_systems(Startup, spawn_flash_overlay)
           .add_systems(Update, (
               toggle_with_9,
               spawn_rain_streaks,
               move_rain,
               tick_lightning,
               update_flash_overlay,
           ));
    }
}

#[derive(Resource, Default, Clone, Copy)]
pub struct StormState {
    pub active:      bool,
    pub flash_alpha: f32,
}

// ---- Internal components / markers -------------------------------------------

/// Marker for a rain-drop particle entity.
#[derive(Component)]
pub struct RainDrop {
    pub vel: Vec3,
}

/// Marker for the fullscreen lightning-flash overlay node.
#[derive(Component)]
struct LightningOverlay;

// ---- Constants ---------------------------------------------------------------

/// Thin-tall cuboid half-extents: 0.04 x 0.6 x 0.04 m.
const DROP_W: f32 = 0.04;
const DROP_H: f32 = 0.60;
const DROP_D: f32 = 0.04;

/// Desired live drop count while the storm is active.
const DROP_COUNT: usize = 200;

/// Horizontal spawn radius around the camera (half-side of the 60 m cube).
const SPAWN_RADIUS: f32 = 30.0;

/// How far above the camera to spawn / respawn drops.
const SPAWN_ABOVE: f32 = 20.0;

/// Rain velocity: slight wind angle (-2, -25, -2) m/s.
const DROP_VEL: Vec3 = Vec3::new(-2.0, -25.0, -2.0);

/// Lightning countdown: random range [3, 7] s.
const LIGHTNING_MIN_S: f32 = 3.0;
const LIGHTNING_MAX_S: f32 = 7.0;

/// How fast the flash alpha decays back to zero.
const FLASH_DECAY: f32 = 3.0;

// ---- Startup: fullscreen white overlay ---------------------------------------

fn spawn_flash_overlay(mut commands: Commands) {
    commands.spawn((
        LightningOverlay,
        Node {
            width:         Val::Percent(100.0),
            height:        Val::Percent(100.0),
            position_type: PositionType::Absolute,
            ..default()
        },
        BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.0)),
        ZIndex(800),
    ));
}

// ---- Toggle ------------------------------------------------------------------

fn toggle_with_9(
    keys:          Res<ButtonInput<KeyCode>>,
    mut state:     ResMut<StormState>,
    mut commands:  Commands,
    drops:         Query<Entity, With<RainDrop>>,
) {
    if !keys.just_pressed(KeyCode::Digit9) {
        return;
    }

    state.active = !state.active;

    if !state.active {
        // Despawn all existing rain drops.
        for entity in &drops {
            commands.entity(entity).despawn();
        }
        // Also clear any residual flash.
        state.flash_alpha = 0.0;
    }
}

// ---- Spawn rain streaks ------------------------------------------------------

fn spawn_rain_streaks(
    mut commands:  Commands,
    mut meshes:    ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    state:         Res<StormState>,
    camera_q:      Query<&Transform, With<Camera3d>>,
    drops:         Query<(), With<RainDrop>>,
) {
    if !state.active {
        return;
    }

    let Ok(cam_tf) = camera_q.single() else { return };
    let cam_pos = cam_tf.translation;

    let existing = drops.iter().count();
    if existing >= DROP_COUNT {
        return;
    }

    let to_spawn = DROP_COUNT - existing;

    // Build shared mesh + material once per frame batch.  They are cloned from
    // handles so the actual GPU resource is shared across all drops.
    let mesh = meshes.add(Cuboid::new(DROP_W, DROP_H, DROP_D));
    let mat  = materials.add(StandardMaterial {
        base_color:        Color::srgba(0.35, 0.65, 1.0, 0.55),
        alpha_mode:        AlphaMode::Blend,
        unlit:             true,
        double_sided:      true,
        cull_mode:         None,
        ..default()
    });

    // Seed from camera position for deterministic-ish distribution.
    let mut seed: u32 = (cam_pos.x.to_bits() ^ cam_pos.z.to_bits())
        .wrapping_add(existing as u32)
        .wrapping_mul(1_664_525)
        .wrapping_add(1_013_904_223);

    for _ in 0..to_spawn {
        let rx = lcg_signed(&mut seed) * SPAWN_RADIUS;
        let rz = lcg_signed(&mut seed) * SPAWN_RADIUS;
        // Scatter vertically within 0–SPAWN_ABOVE so the initial cloud looks
        // like rain that's already falling, not a flat spawning plane.
        let ry = lcg_next(&mut seed) * SPAWN_ABOVE;

        let pos = Vec3::new(
            cam_pos.x + rx,
            cam_pos.y + SPAWN_ABOVE - ry,
            cam_pos.z + rz,
        );

        commands.spawn((
            RainDrop { vel: DROP_VEL },
            Mesh3d(mesh.clone()),
            MeshMaterial3d(mat.clone()),
            Transform::from_translation(pos),
        ));
    }
}

// ---- Move rain ---------------------------------------------------------------

fn move_rain(
    mut commands: Commands,
    time:         Res<Time>,
    state:        Res<StormState>,
    camera_q:     Query<&Transform, With<Camera3d>>,
    mut drops:    Query<(Entity, &mut Transform, &RainDrop), Without<Camera3d>>,
) {
    if !state.active {
        return;
    }

    let dt = time.delta_secs();
    let Ok(cam_tf) = camera_q.single() else { return };
    let cam_pos = cam_tf.translation;

    let mut seed: u32 = (cam_pos.x.to_bits() ^ cam_pos.z.to_bits())
        .wrapping_mul(1_664_525)
        .wrapping_add(1_013_904_223);

    for (entity, mut tf, drop) in &mut drops {
        // Integrate position.
        tf.translation += drop.vel * dt;

        let x = tf.translation.x;
        let z = tf.translation.z;
        let terrain_y = terrain_height_at(x, z);

        // Respawn above camera when below terrain or far outside the spawn cube.
        let dx = (tf.translation.x - cam_pos.x).abs();
        let dz = (tf.translation.z - cam_pos.z).abs();
        let out_of_range = dx > SPAWN_RADIUS * 1.5 || dz > SPAWN_RADIUS * 1.5;

        if tf.translation.y < terrain_y || out_of_range {
            // Despawn and let spawn_rain_streaks recreate it next frame.
            // (Simpler than mutating the velocity/position back; avoids needing
            //  a &mut RainDrop here since vel is constant anyway.)
            commands.entity(entity).despawn();
            let _ = seed; // keep seed advance consistent
        }

        // Advance seed to keep the sequence moving regardless of respawn.
        lcg_next(&mut seed);
        lcg_next(&mut seed);
    }
}

// ---- Lightning tick ----------------------------------------------------------

fn tick_lightning(
    time:       Res<Time>,
    state:      Res<StormState>,
    mut storm:  ResMut<StormState>,
    mut timer:  Local<f32>,
    mut seeded: Local<bool>,
    mut lseed:  Local<u32>,
) {
    if !state.active {
        // Reset timer when inactive so the first flash after re-enable isn't
        // immediate.
        *timer = 4.0;
        return;
    }

    let dt = time.delta_secs();

    // Initialise the seed once.
    if !*seeded {
        *lseed  = 0xDEAD_BEEF;
        *seeded = true;
        *timer  = next_interval(&mut lseed);
    }

    *timer -= dt;
    if *timer <= 0.0 {
        storm.flash_alpha = 1.0;
        info!("⚡ thunder");
        *timer = next_interval(&mut lseed);
    }
}

// ---- Update flash overlay ----------------------------------------------------

fn update_flash_overlay(
    time:    Res<Time>,
    mut state: ResMut<StormState>,
    mut overlays: Query<&mut BackgroundColor, With<LightningOverlay>>,
) {
    // Decay the flash alpha regardless of active state (so a flash already in
    // progress finishes even if the storm is toggled off mid-flash).
    if state.flash_alpha > 0.0 {
        state.flash_alpha = (state.flash_alpha - FLASH_DECAY * time.delta_secs()).max(0.0);
    }

    for mut bg in &mut overlays {
        bg.0 = Color::srgba(1.0, 1.0, 1.0, state.flash_alpha);
    }
}

// ---- LCG helpers (no external deps) -----------------------------------------

#[inline]
fn lcg_next(seed: &mut u32) -> f32 {
    *seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    *seed as f32 / u32::MAX as f32
}

#[inline]
fn lcg_signed(seed: &mut u32) -> f32 {
    lcg_next(seed) * 2.0 - 1.0
}

/// Draw a random interval in [LIGHTNING_MIN_S, LIGHTNING_MAX_S].
#[inline]
fn next_interval(seed: &mut u32) -> f32 {
    LIGHTNING_MIN_S + lcg_next(seed) * (LIGHTNING_MAX_S - LIGHTNING_MIN_S)
}
