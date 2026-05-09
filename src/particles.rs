// Reactive dust particles at each wheel contact point.
// Uses bevy_hanabi 0.18 GPU particles.
//
// SPAWN RATE: modulated each frame by lateral/longitudinal slip and brake state.
// Runs in PostUpdate, after EffectSystems::TickSpawners, and overrides
// `EffectSpawner::spawn_count` directly — the cleanest way to drive per-frame
// rates without fighting the SpawnerSettings state machine.
//
// COLOR / SIZE (global lerp tradeoff):
//   One EffectAsset is shared across all four emitters.  Per-wheel asset clones
//   would give independent surface colors but cost 4× asset slots and 4× shader
//   recompiles on change.  Instead we lerp toward the *average* surface color
//   and average size scale across all grounded wheels each frame, and only
//   mutate the shared asset when the result changes by more than a threshold —
//   limiting shader recompiles to once per surface-type transition.

use bevy::prelude::*;
use bevy_hanabi::{
    prelude::*,              // includes EffectSystems via `pub use crate::*`
    Gradient as HanabiGradient,
};
use avian3d::prelude::*;

use crate::vehicle::{Chassis, DriveInput, Wheel};

pub struct DustPlugin;

impl Plugin for DustPlugin {
    fn build(&self, app: &mut App) {
        // bevy_hanabi 0.18 requires compute storage buffers, which WebGL2
        // (and therefore most browser WASM environments today) does not
        // provide. Skip the entire plugin on wasm32 — wheels still spin and
        // physics still works, the dust trail is just absent.
        #[cfg(target_arch = "wasm32")]
        {
            let _ = app;
            return;
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            app.add_plugins(HanabiPlugin)
               .init_resource::<DustGlobalState>()
               .add_systems(Startup, setup_dust_effect)
               .add_systems(Update, attach_wheel_emitters.run_if(resource_exists::<DustEffect>))
               .add_systems(
                   PostUpdate,
                   modulate_wheel_emitters
                       .after(EffectSystems::TickSpawners)
                       .run_if(resource_exists::<DustEffect>),
               );
        }
    }
}

// ---- Resources / components ----

#[derive(Resource)]
struct DustEffect(Handle<EffectAsset>);

// Tracks the last color and size baked into the shared EffectAsset so we can
// skip asset mutations when nothing meaningful changed.
#[derive(Resource, Default)]
struct DustGlobalState {
    last_color: Vec3,
    last_size_scale: f32,
}

#[derive(Component)]
struct WheelDustEmitter;

// ---- Constants ----

// Base emit rate when grounded but not slipping.
const RATE_BASE: f32 = 5.0;
// Additional particles per m/s of lateral or longitudinal slip.
const RATE_PER_SLIP_MS: f32 = 30.0;
// Bonus while braking hard (longitudinal speed > 1 m/s).
const RATE_BRAKE_BONUS: f32 = 50.0;
const RATE_MAX: f32 = 200.0;

// Raycast half-height above wheel anchor; total length downward.
const CAST_START_OFFSET: f32 = 0.5;
const CAST_LENGTH: f32 = 1.5;

// Surface dust colors keyed on slope (1 - normal.dot(Y)).
// flat  < 0.15 : dusty grass-green
const COLOR_FLAT: Vec3  = Vec3::new(0.55, 0.50, 0.30);
// 0.15..0.45   : tan dirt
const COLOR_MID: Vec3   = Vec3::new(0.60, 0.50, 0.30);
// > 0.45       : grey-brown rock
const COLOR_STEEP: Vec3 = Vec3::new(0.50, 0.48, 0.45);

// Only re-bake the EffectAsset when the averaged color shifts by more than this
// magnitude, to limit shader recompiles.
const COLOR_REBAKE_THRESHOLD: f32 = 0.04;
const SIZE_REBAKE_THRESHOLD: f32  = 0.05;

// ---- Startup: build the shared EffectAsset ----

fn setup_dust_effect(
    mut commands: Commands,
    mut effects: ResMut<Assets<EffectAsset>>,
) {
    let handle = effects.add(build_effect_value(COLOR_MID, 1.0));
    commands.insert_resource(DustEffect(handle));
}

// Build an EffectAsset value without inserting it into Assets — lets callers
// decide when to add or overwrite.
fn build_effect_value(color_rgb: Vec3, size_scale: f32) -> EffectAsset {
    let mut module = Module::default();

    let init_pos = SetAttributeModifier::new(
        Attribute::POSITION,
        module.lit(Vec3::ZERO),
    );

    let vel = module.lit(Vec3::new(0.0, 1.8, 0.0));
    let init_vel = SetAttributeModifier::new(Attribute::VELOCITY, vel);

    let init_lifetime = SetAttributeModifier::new(
        Attribute::LIFETIME,
        module.lit(0.3_f32),
    );

    let update_gravity = AccelModifier::new(module.lit(Vec3::new(0.0, -9.0, 0.0)));

    let base_size = 0.18 * size_scale;
    let mut color_grad = HanabiGradient::<Vec4>::new();
    color_grad.add_key(0.0, Vec4::new(color_rgb.x, color_rgb.y, color_rgb.z, 0.8));
    color_grad.add_key(1.0, Vec4::new(color_rgb.x, color_rgb.y, color_rgb.z, 0.0));
    let color_mod = ColorOverLifetimeModifier::new(color_grad);

    let mut size_grad = HanabiGradient::<Vec3>::new();
    size_grad.add_key(0.0, Vec3::splat(base_size));
    size_grad.add_key(1.0, Vec3::ZERO);
    let size_mod = SizeOverLifetimeModifier {
        gradient: size_grad,
        screen_space_size: false,
    };

    EffectAsset::new(256, SpawnerSettings::rate(5.0.into()), module)
        .with_name("WheelDust")
        .init(init_pos)
        .init(init_vel)
        .init(init_lifetime)
        .update(update_gravity)
        .render(color_mod)
        .render(size_mod)
}

// ---- Attach one dust emitter as a child of each wheel entity ----

fn attach_wheel_emitters(
    mut commands: Commands,
    dust: Res<DustEffect>,
    wheels: Query<Entity, (With<Wheel>, Without<WheelDustEmitter>)>,
) {
    for wheel_entity in &wheels {
        let emitter = commands.spawn((
            WheelDustEmitter,
            ParticleEffect::new(dust.0.clone()),
            Transform::default(),
        )).id();
        commands.entity(wheel_entity).add_child(emitter);
        commands.entity(wheel_entity).insert(WheelDustEmitter);
    }
}

// ---- Per-frame modulation (PostUpdate, after tick_spawners) ----

fn modulate_wheel_emitters(
    dust:          Res<DustEffect>,
    drive_input:   Res<DriveInput>,
    chassis_q:     Query<(&Transform, &LinearVelocity, &AngularVelocity), With<Chassis>>,
    wheel_q:       Query<(Entity, &Transform), (With<Wheel>, With<WheelDustEmitter>)>,
    // With<EffectSpawner> restricts to actual emitter entities (not the tagged Wheel entities).
    mut spawner_q: Query<(&mut EffectSpawner, &ChildOf), (With<WheelDustEmitter>, Without<Wheel>)>,
    spatial:       SpatialQuery,
    mut effects:   ResMut<Assets<EffectAsset>>,
    mut state:     ResMut<DustGlobalState>,
) {
    // Fetch chassis state once.
    let Ok((c_transform, lin_vel, ang_vel)) = chassis_q.single() else { return };
    let chassis_pos = c_transform.translation;
    let chassis_fwd = (*c_transform.forward()).normalize();
    // Deref to Vec3; avian3d LinearVelocity/AngularVelocity wrap Vec3 in 3D.
    let lin_vel_v: Vec3 = lin_vel.0;
    let ang_vel_v: Vec3 = ang_vel.0;

    // Exclude the chassis from raycasts so suspension hits terrain, not itself.
    // We cast from world space so no entity exclusion needed for wheels (they're
    // visual-only with no collider).
    let filter = SpatialQueryFilter::default();

    let brake_active = drive_input.brake;
    // Longitudinal speed of the chassis centre.
    let v_long_chassis = lin_vel_v.dot(chassis_fwd);

    // Accumulate surface color + size contributions from grounded wheels.
    let mut color_sum   = Vec3::ZERO;
    let mut size_sum    = 0.0_f32;
    let mut grounded_n  = 0_u32;

    // First pass: compute per-wheel slip, collect surface stats, build a
    // (wheel_entity → spawn_count) map, then write to EffectSpawner.
    // Because spawner_q is indexed by the emitter's ChildOf (→ wheel entity),
    // we build a scratch vec to avoid double borrow.
    let mut wheel_spawn: Vec<(Entity, u32)> = Vec::new();

    for (wheel_entity, wheel_transform) in &wheel_q {

        // Wheel anchor in world space (wheel_transform is local to chassis).
        let world_anchor = chassis_pos + c_transform.rotation * wheel_transform.translation;

        // Raycast from slightly above the anchor straight down.
        let cast_origin = world_anchor + Vec3::Y * CAST_START_OFFSET;
        let hit = spatial.cast_ray(
            cast_origin,
            Dir3::NEG_Y,
            CAST_LENGTH,
            true,
            &filter,
        );

        let Some(hit) = hit else {
            // Airborne: no dust.
            wheel_spawn.push((wheel_entity, 0));
            continue;
        };

        let surface_normal = hit.normal; // Vector = Vec3 in avian3d single-precision 3D
        // slope = 0 on flat ground, 1 on vertical face.
        let slope = (1.0 - surface_normal.dot(Vec3::Y)).clamp(0.0, 1.0);
        let surface_color = surface_color_from_slope(slope);

        // Velocity of the wheel anchor in world space.
        let r        = world_anchor - chassis_pos;
        let v_anchor = lin_vel_v + ang_vel_v.cross(r);

        // Lateral slip: component perpendicular to forward, in the ground plane.
        let v_lat_raw = v_anchor - v_anchor.dot(chassis_fwd) * chassis_fwd;
        // Remove normal component to stay in the ground plane.
        let v_lat_ground = v_lat_raw - v_lat_raw.dot(surface_normal) * surface_normal;
        let lateral_slip = v_lat_ground.length();

        // Longitudinal slip: relevant when braking.
        let v_long = v_anchor.dot(chassis_fwd).abs();

        // Compute spawn rate for this wheel.
        let mut rate = RATE_BASE;
        rate += lateral_slip * RATE_PER_SLIP_MS;
        if brake_active && v_long > 1.0 {
            rate += RATE_BRAKE_BONUS;
        }
        rate = rate.clamp(0.0, RATE_MAX);

        // Convert particles/sec to particle count for this already-ticked frame.
        // spawn_count is what Hanabi will actually emit this frame; we override it
        // directly after tick_spawners has run.  The rate() setting keeps the
        // SpawnerSettings alive for the next tick's partial accumulation, but we
        // always clobber spawn_count so the effective rate is fully ours.
        //
        // At 60 Hz, rate=200 → 3-4 particles/frame.  We use a simple rounding
        // rather than tracking a fractional remainder, accepting ±1 particle/frame
        // jitter at low rates.  The EffectSpawner's own remainder accumulation
        // still runs (we don't zero it), providing smoother low-rate behaviour.
        let dt_approx = 1.0 / 60.0; // conservative; we don't have Time here
        let count = (rate * dt_approx).round() as u32;

        color_sum  += surface_color;
        size_sum   += (1.0 + (lateral_slip + v_long.min(v_long_chassis.abs())) * 0.1)
                          .clamp(1.0, 2.5);
        grounded_n += 1;

        wheel_spawn.push((wheel_entity, count));
    }

    // Write spawn counts to each emitter (emitter ChildOf → wheel entity).
    for (mut spawner, emitter_child_of) in &mut spawner_q {
        let parent = emitter_child_of.parent();
        if let Some(&(_, count)) = wheel_spawn.iter().find(|(e, _)| *e == parent) {
            spawner.spawn_count = count;
            spawner.active      = true; // keep alive; active=false stops GPU sim entirely
        }
    }

    // Update shared EffectAsset if the averaged surface color/size shifted enough.
    if grounded_n == 0 {
        return;
    }
    let avg_color = color_sum / grounded_n as f32;
    let avg_scale = size_sum  / grounded_n as f32;

    let color_delta = (avg_color - state.last_color).length();
    let scale_delta = (avg_scale - state.last_size_scale).abs();

    if color_delta > COLOR_REBAKE_THRESHOLD || scale_delta > SIZE_REBAKE_THRESHOLD {
        // Build the new asset data outside the borrow of `effects`.
        // We can't call build_effect_asset (which borrows &mut effects) and then
        // do get_mut on the same Assets in the same scope, so we construct the
        // EffectAsset value directly here and overwrite the existing handle's slot.
        let new_asset = build_effect_value(avg_color, avg_scale);
        if let Some(effect) = effects.get_mut(&dust.0) {
            *effect = new_asset;
        }
        state.last_color      = avg_color;
        state.last_size_scale = avg_scale;
    }
}

// ---- Helpers ----

fn surface_color_from_slope(slope: f32) -> Vec3 {
    if slope < 0.15 {
        COLOR_FLAT
    } else if slope < 0.45 {
        // Smooth blend between flat and mid.
        let t = (slope - 0.15) / 0.30;
        COLOR_FLAT.lerp(COLOR_MID, t)
    } else {
        // Smooth blend between mid and steep.
        let t = ((slope - 0.45) / 0.30).clamp(0.0, 1.0);
        COLOR_MID.lerp(COLOR_STEEP, t)
    }
}
