// Dust particles spawned at each wheel contact point.
// Uses bevy_hanabi 0.18 GPU particles.  One EffectAsset is shared across all
// four wheel emitters; each emitter entity is a child of its wheel entity.
//
// Spawn gate: the emitter is enabled when the wheel's world-space Y is close
// to the terrain surface (rough ground check) AND the chassis is moving above
// a speed threshold.

use bevy::prelude::*;
// bevy::prelude also exports a `Gradient` enum; import hanabi types by name to avoid ambiguity.
// Bevy 0.18 renamed `Parent` to `ChildOf`; the hierarchy component now holds the parent Entity.
use bevy_hanabi::{
    prelude::*,
    Gradient as HanabiGradient,
};
use avian3d::prelude::*;

use crate::vehicle::{Chassis, Wheel};

pub struct DustPlugin;

impl Plugin for DustPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(HanabiPlugin)
           .add_systems(Startup, setup_dust_effect)
           .add_systems(Update, attach_wheel_emitters.run_if(resource_exists::<DustEffect>))
           .add_systems(Update, gate_wheel_emitters.run_if(resource_exists::<DustEffect>));
    }
}

// Shared handle to the dust EffectAsset.
#[derive(Resource)]
struct DustEffect(Handle<EffectAsset>);

// Marks the per-wheel emitter entity so we can query it alongside the wheel.
#[derive(Component)]
struct WheelDustEmitter;

// Speed (m/s) below which no dust is emitted.
const MIN_SPEED: f32 = 1.5;

// A wheel is considered "on ground" when its world Y is within this distance
// above y=0 (the approximate terrain base).  Not a raycast, but good enough.
const GROUND_Y_THRESHOLD: f32 = 1.2;

fn setup_dust_effect(
    mut commands: Commands,
    mut effects: ResMut<Assets<EffectAsset>>,
) {
    let mut module = Module::default();

    // Random scatter in a small hemisphere above the contact point.
    let init_pos = SetAttributeModifier::new(
        Attribute::POSITION,
        module.lit(Vec3::ZERO),
    );

    // Initial velocity: upward + slight random lateral spread.
    let vel = module.lit(Vec3::new(0.0, 1.8, 0.0));
    let init_vel = SetAttributeModifier::new(Attribute::VELOCITY, vel);

    // Lifetime 0.3 s.
    let lifetime = module.lit(0.3_f32);
    let init_lifetime = SetAttributeModifier::new(Attribute::LIFETIME, lifetime);

    // Gravity pulling particles back down.
    let gravity = module.lit(Vec3::new(0.0, -9.0, 0.0));
    let update_gravity = AccelModifier::new(gravity);

    // Brown/tan colour fading to fully transparent.
    let mut color_grad = HanabiGradient::<Vec4>::new();
    color_grad.add_key(0.0, Vec4::new(0.72, 0.55, 0.28, 0.8));
    color_grad.add_key(1.0, Vec4::new(0.72, 0.55, 0.28, 0.0));
    let color_mod = ColorOverLifetimeModifier::new(color_grad);

    // Size shrinks from 0.18 to 0.0 over the lifetime.
    let mut size_grad = HanabiGradient::<Vec3>::new();
    size_grad.add_key(0.0, Vec3::splat(0.18));
    size_grad.add_key(1.0, Vec3::ZERO);
    let size_mod = SizeOverLifetimeModifier {
        gradient: size_grad,
        screen_space_size: false,
    };

    let effect = EffectAsset::new(
        256,
        SpawnerSettings::rate(40.0.into()),
        module,
    )
    .with_name("WheelDust")
    .init(init_pos)
    .init(init_vel)
    .init(init_lifetime)
    .update(update_gravity)
    .render(color_mod)
    .render(size_mod);

    let handle = effects.add(effect);
    commands.insert_resource(DustEffect(handle));
}

// Run once per wheel entity that doesn't yet have a dust emitter child.
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

        // Parent the emitter to the wheel so it tracks position automatically.
        commands.entity(wheel_entity).add_child(emitter);

        // Tag the wheel itself so the Without<> filter above won't match it again.
        commands.entity(wheel_entity).insert(WheelDustEmitter);
    }
}

// Each frame: enable/disable each wheel's emitter based on speed + ground proximity.
fn gate_wheel_emitters(
    chassis_q:     Query<&LinearVelocity, With<Chassis>>,
    mut spawner_q: Query<(&mut EffectSpawner, &ChildOf), With<WheelDustEmitter>>,
    wheel_q:       Query<&Transform, With<Wheel>>,
) {
    // Use chassis linear speed as a cheap proxy for "vehicle is moving".
    let speed = chassis_q
        .iter()
        .next()
        .map(|v| v.length())
        .unwrap_or(0.0);

    let moving = speed > MIN_SPEED;

    for (mut spawner, child_of) in &mut spawner_q {
        // Check ground proximity using the parent wheel's world Y.
        let on_ground = wheel_q
            .get(child_of.parent())
            .map(|t| t.translation.y < GROUND_Y_THRESHOLD)
            .unwrap_or(false);

        spawner.active = moving && on_ground;
    }
}
