// UFO sighting: every ~5 minutes a flying saucer crosses the sky for 15s
// at high altitude. Pulsing emissive lights, slow drift, no collision.
//
// Public API:
//   UfoPlugin
//   UfoState (resource)

use bevy::prelude::*;

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct UfoPlugin;

impl Plugin for UfoPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(UfoState {
               active:            false,
               elapsed_s:         0.0,
               time_until_next_s: 60.0, // first sighting after 1 min (testing)
           })
           .add_systems(Update, (tick_ufo, pulse_ufo_lights));
    }
}

// ---------------------------------------------------------------------------
// Public resource
// ---------------------------------------------------------------------------

#[derive(Resource, Default, Clone, Copy)]
pub struct UfoState {
    pub active:            bool,
    pub elapsed_s:         f32,
    pub time_until_next_s: f32,
}

// ---------------------------------------------------------------------------
// Components
// ---------------------------------------------------------------------------

/// Root marker for a UFO entity.
#[derive(Component)]
pub struct Ufo {
    /// World-space drift velocity (m/s).
    pub vel: Vec3,
}

/// Marks a pulsing light disc underneath the UFO.
#[derive(Component)]
struct UfoLight;

/// Stores the shared handle for the three pulsing-light materials so
/// `pulse_ufo_lights` can locate and mutate it without a per-entity query.
#[derive(Resource)]
struct UfoLightMaterial(Handle<StandardMaterial>);

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// World-space Z range for randomised spawn position (deterministic via time).
const SPAWN_Z_HALF: f32 = 100.0;

/// UFO starts to the left of the visible area; velocity carries it right.
const SPAWN_X: f32      = -200.0;
const SPAWN_Y: f32      =   80.0;

/// Drift speed (m/s).  30 m/s × 15 s = 450 m horizontal travel.
const DRIFT_SPEED: f32  =   30.0;

/// Lifetime of a single sighting (seconds).
const SIGHTING_DURATION: f32 = 15.0;

/// Wait between sightings after despawn (seconds).
const COOLDOWN_S: f32 = 300.0;

// Disc body
const DISC_RADIUS: f32  = 4.0;
const DISC_HEIGHT: f32  = 0.4;

// Dome top
const DOME_RADIUS: f32  = 1.6;
const DOME_Y:      f32  = 0.4;

// Pulsing light discs
const LIGHT_RADIUS:  f32 = 0.4;
const LIGHT_HEIGHT:  f32 = 0.1;
const LIGHT_Y:       f32 = -0.3;

/// Local XZ positions of the three lights.
const LIGHT_XZ: [(f32, f32); 3] = [
    ( 1.50,  0.00),
    (-0.75,  1.30),
    (-0.75, -1.30),
];

// Pulse animation
const PULSE_SPEED: f32 = 8.0;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Cheap deterministic pseudo-random value in [-1, +1) from a seed.
#[inline]
fn lcg_signed(seed: f32) -> f32 {
    let mut v = (seed * 73_856_093.0) as u32;
    v = v.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    v ^= v >> 16;
    v = v.wrapping_mul(0x45d9f3b);
    v ^= v >> 16;
    (v as f32) / (u32::MAX as f32) * 2.0 - 1.0
}

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

/// State-machine tick: countdown → spawn, drift, timeout → despawn.
fn tick_ufo(
    mut commands:  Commands,
    mut meshes:    ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut state:     ResMut<UfoState>,
    mut ufos:      Query<(Entity, &Ufo, &mut Transform)>,
    time:          Res<Time>,
) {
    let dt = time.delta_secs();

    if !state.active {
        state.time_until_next_s -= dt;
        if state.time_until_next_s <= 0.0 {
            spawn_ufo(&mut commands, &mut meshes, &mut materials, time.elapsed_secs());
            state.active    = true;
            state.elapsed_s = 0.0;
        }
        return;
    }

    // Active: advance timer and drift.
    state.elapsed_s += dt;

    for (_entity, ufo, mut transform) in &mut ufos {
        transform.translation += ufo.vel * dt;
    }

    if state.elapsed_s >= SIGHTING_DURATION {
        // Despawn all UFO entities (children are despawned with the parent
        // via Bevy 0.18 hierarchical despawn).
        let ufo_entities: Vec<Entity> = ufos.iter().map(|(e, _, _)| e).collect();
        for e in ufo_entities {
            commands.entity(e).despawn();
        }
        // Remove the shared light material resource so pulse_ufo_lights is a no-op.
        commands.remove_resource::<UfoLightMaterial>();

        state.active            = false;
        state.time_until_next_s = COOLDOWN_S;
    }
}

/// Spawns a single UFO at the fixed entry position with a randomised Z offset.
fn spawn_ufo(
    commands:  &mut Commands,
    meshes:    &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    elapsed:   f32,
) {
    info!("👽 UFO spotted!");

    // Deterministic Z randomisation from elapsed time so no rand crate needed.
    let z = lcg_signed(elapsed) * SPAWN_Z_HALF;

    let spawn_pos = Vec3::new(SPAWN_X, SPAWN_Y, z);
    let vel       = Vec3::new(DRIFT_SPEED, 0.0, 0.0);

    // ---- shared meshes ----
    let disc_mesh  = meshes.add(Cylinder::new(DISC_RADIUS,  DISC_HEIGHT));
    let dome_mesh  = meshes.add(Sphere::new(DOME_RADIUS).mesh().ico(1).unwrap());
    let light_mesh = meshes.add(Cylinder::new(LIGHT_RADIUS, LIGHT_HEIGHT));

    // ---- materials ----
    let disc_mat = materials.add(StandardMaterial {
        base_color:           Color::srgb(0.5, 0.5, 0.55),
        metallic:             0.8,
        perceptual_roughness: 0.3,
        ..default()
    });

    let dome_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(0.4, 0.6, 1.0, 0.7),
        alpha_mode: AlphaMode::Blend,
        ..default()
    });

    // Shared material for all three pulsing lights; stored as a resource so
    // `pulse_ufo_lights` can mutate it without knowing individual handles.
    let light_mat_handle = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.2, 0.5),
        emissive:   LinearRgba::rgb(1.0, 0.2, 0.5),
        unlit:      true,
        ..default()
    });

    // ---- parent UFO entity ----
    let parent = commands.spawn((
        Ufo { vel },
        Transform::from_translation(spawn_pos),
        Visibility::default(),
    )).id();

    // ---- disc body ----
    let disc = commands.spawn((
        Mesh3d(disc_mesh),
        MeshMaterial3d(disc_mat),
        Transform::IDENTITY,
    )).id();
    commands.entity(parent).add_child(disc);

    // ---- dome top ----
    let dome = commands.spawn((
        Mesh3d(dome_mesh),
        MeshMaterial3d(dome_mat),
        Transform::from_translation(Vec3::new(0.0, DOME_Y, 0.0)),
    )).id();
    commands.entity(parent).add_child(dome);

    // ---- three pulsing lights ----
    for &(lx, lz) in &LIGHT_XZ {
        let light = commands.spawn((
            UfoLight,
            Mesh3d(light_mesh.clone()),
            MeshMaterial3d(light_mat_handle.clone()),
            Transform::from_translation(Vec3::new(lx, LIGHT_Y, lz)),
        )).id();
        commands.entity(parent).add_child(light);
    }

    // Publish the shared material handle so pulse_ufo_lights can find it.
    commands.insert_resource(UfoLightMaterial(light_mat_handle));
}

/// Each frame, modulate the shared UFO light material's base colour so the
/// three discs pulse together.
fn pulse_ufo_lights(
    light_mat: Option<Res<UfoLightMaterial>>,
    mut mats:  ResMut<Assets<StandardMaterial>>,
    time:      Res<Time>,
) {
    let Some(light_mat) = light_mat else { return };

    let t     = time.elapsed_secs();
    let alpha = 0.5 + (t * PULSE_SPEED).sin() * 0.5;
    let alpha = alpha.clamp(0.0, 1.0);

    if let Some(mat) = mats.get_mut(&light_mat.0) {
        mat.base_color = Color::srgba(1.0, 0.2, 0.5, alpha);
        // Keep emissive in sync with the pulse so the glow breathes too.
        mat.emissive   = LinearRgba::rgb(alpha, alpha * 0.2, alpha * 0.5);
    }
}
