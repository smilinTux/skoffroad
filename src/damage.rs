// Damage accumulation, smoke effect, and damage HUD bar.
// Reads EventLog (cap-8 ring buffer) each frame; tracks last-processed timestamp
// to avoid double-counting across ring wraps.
// Regeneration: REGEN_PER_SEC = 0.001 (comment-out for "scars are forever").
// Smoke: Hanabi emitter parented to chassis, active when damage > 0.4.
// Wreck: red-flash border at damage > 0.95; press R (wired elsewhere) to reset.

use bevy::prelude::*;
use bevy_hanabi::{
    prelude::*,
    Gradient as HanabiGradient,
};

use crate::events::{EventLog, GameEvent};
use crate::vehicle::{Chassis, VehicleRoot};

// ---- Public types -----------------------------------------------------------

pub struct DamagePlugin;

impl Plugin for DamagePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DamageState>()
            .add_systems(Startup, spawn_damage_hud);

        // Smoke uses bevy_hanabi (compute storage buffers); browser WebGL2
        // can't run those. Skip the smoke pipeline on wasm32 so the rest
        // of the damage system (HUD, wreck flash, accumulator) still runs.
        #[cfg(not(target_arch = "wasm32"))]
        app.add_systems(Startup, setup_smoke_effect)
            .add_systems(
                Update,
                (
                    attach_smoke_emitter.run_if(resource_exists::<SmokeEffect>),
                    update_smoke.run_if(resource_exists::<SmokeEffect>),
                )
                    .run_if(resource_exists::<VehicleRoot>),
            );

        app.add_systems(
            Update,
            (
                accumulate_damage,
                update_damage_hud,
                update_wreck_flash,
            )
                .run_if(resource_exists::<VehicleRoot>),
        );
    }
}

/// Overall vehicle condition. 0 = pristine, 1 = wrecked.
#[derive(Resource, Default)]
pub struct DamageState {
    /// 0.0..=1.0. 0 = pristine, 1 = wrecked.
    pub damage: f32,
    /// Cumulative impact count.
    pub impact_count: u32,
    /// Cumulative tilt-warning count.
    pub tilt_count: u32,
}

// ---- Constants ---------------------------------------------------------------

// Regeneration rate (damage/sec).  Full recovery from 1.0 → 0.0 takes ~1000 s
// (~16 min) with no new incidents.  Set to 0.0 to disable.
const REGEN_PER_SEC: f32 = 0.001;

// Smoke begins at this damage level.
const SMOKE_THRESHOLD: f32 = 0.4;
// Maximum spawn rate (particles/sec) reached at damage = 1.0.
const SMOKE_RATE_MAX: f32 = 30.0;

// Red-flash warning threshold.
const WRECK_THRESHOLD: f32 = 0.95;
// Flash period (seconds per full on-off cycle).
const FLASH_PERIOD: f32 = 0.6;

// ---- Private types ----------------------------------------------------------

#[derive(Component)] struct SmokeEmitter;
#[derive(Component)] struct DamageBarFill;
#[derive(Component)] struct DamageBarText;
#[derive(Component)] struct DamageHudRoot;
#[derive(Component)] struct WreckBorder;
#[derive(Resource)]  struct SmokeEffect(Handle<EffectAsset>);

// ---- Damage accumulation ----------------------------------------------------

fn accumulate_damage(
    log: Res<EventLog>,
    time: Res<Time>,
    mut state: ResMut<DamageState>,
    // Timestamp of the last event we processed; -1.0 sentinel on first frame.
    mut last_ts: Local<f32>,
) {
    if *last_ts == 0.0 && state.damage == 0.0 && state.impact_count == 0 {
        *last_ts = -1.0;
    }

    for (ts, ev) in &log.events {
        if *ts <= *last_ts {
            continue;
        }
        match ev {
            GameEvent::HardImpact { v } => {
                // v is negative (downward).  Faster landings hurt more.
                let delta = (v.abs() / 10.0).min(0.3);
                state.damage += delta;
                state.impact_count += 1;
            }
            GameEvent::BigTilt { .. } => {
                state.damage += 0.05;
                state.tilt_count += 1;
            }
            GameEvent::Airtime { duration_s } => {
                // Only penalise hard landings after meaningful air (> 1 s).
                if *duration_s > 1.0 {
                    state.damage += 0.04;
                }
            }
            _ => {}
        }
    }

    // Advance timestamp to the newest event we've now seen.
    if let Some((ts, _)) = log.events.back() {
        if *ts > *last_ts {
            *last_ts = *ts;
        }
    }

    // Slow regeneration — comment REGEN_PER_SEC to zero to disable.
    let dt = time.delta_secs();
    state.damage -= REGEN_PER_SEC * dt;

    state.damage = state.damage.clamp(0.0, 1.0);
}

// ---- Smoke effect -----------------------------------------------------------

fn setup_smoke_effect(mut commands: Commands, mut effects: ResMut<Assets<EffectAsset>>) {
    commands.insert_resource(SmokeEffect(effects.add(build_smoke_asset())));
}

fn build_smoke_asset() -> EffectAsset {
    let mut module = Module::default();

    let init_pos = SetAttributeModifier::new(Attribute::POSITION, module.lit(Vec3::ZERO));
    // Drift upward and slightly back (positive Z = rearward in chassis space).
    let init_vel = SetAttributeModifier::new(
        Attribute::VELOCITY, module.lit(Vec3::new(0.0, 1.2, 0.5)));
    let init_lifetime = SetAttributeModifier::new(Attribute::LIFETIME, module.lit(2.0_f32));
    let update_gravity = AccelModifier::new(module.lit(Vec3::new(0.0, -2.0, 0.0)));

    // Grey at 0.4 damage → near-black at 1.0, fading to transparent.
    let mut color_grad = HanabiGradient::<Vec4>::new();
    color_grad.add_key(0.0, Vec4::new(0.2, 0.2, 0.2, 0.8));
    color_grad.add_key(1.0, Vec4::new(0.05, 0.05, 0.05, 0.0));
    let color_mod = ColorOverLifetimeModifier::new(color_grad);

    let mut size_grad = HanabiGradient::<Vec3>::new();
    size_grad.add_key(0.0, Vec3::splat(0.25));
    size_grad.add_key(1.0, Vec3::splat(0.60));
    let size_mod = SizeOverLifetimeModifier {
        gradient: size_grad,
        screen_space_size: false,
    };

    // Capacity 512 handles up to 30/s × 2 s lifetime comfortably.
    EffectAsset::new(512, SpawnerSettings::rate(1.0.into()), module)
        .with_name("ChassisDamageSmoke")
        .init(init_pos)
        .init(init_vel)
        .init(init_lifetime)
        .update(update_gravity)
        .render(color_mod)
        .render(size_mod)
}

// ---- Smoke emitter attachment -----------------------------------------------

fn attach_smoke_emitter(
    mut commands: Commands,
    smoke: Res<SmokeEffect>,
    vehicle: Res<VehicleRoot>,
    // Without<SmokeEmitter> on chassis: run only once.
    chassis_q: Query<Entity, (With<Chassis>, Without<SmokeEmitter>)>,
) {
    let Ok(chassis_entity) = chassis_q.get(vehicle.chassis) else { return };
    let emitter = commands.spawn((
        SmokeEmitter,
        ParticleEffect::new(smoke.0.clone()),
        Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
    )).id();
    commands.entity(chassis_entity).add_child(emitter);
    commands.entity(chassis_entity).insert(SmokeEmitter);
}

// ---- Smoke rate modulation --------------------------------------------------

fn update_smoke(
    damage: Res<DamageState>,
    mut spawner_q: Query<&mut EffectSpawner, With<SmokeEmitter>>,
) {
    let excess = (damage.damage - SMOKE_THRESHOLD).max(0.0);
    // Scale: 0 at threshold, SMOKE_RATE_MAX at damage = 1.0.
    // Range of excess: 0.0 .. 0.6 → divide by 0.6 to normalise to 0..1.
    let rate = (excess / (1.0 - SMOKE_THRESHOLD)) * SMOKE_RATE_MAX;

    for mut spawner in &mut spawner_q {
        if rate <= 0.0 {
            spawner.active = false;
            spawner.spawn_count = 0;
        } else {
            spawner.active = true;
            // Convert particles/sec to particles/frame at ~60 Hz.
            let count = (rate / 60.0).round() as u32;
            spawner.spawn_count = count.max(1);
        }
    }
}

// ---- Damage HUD bar ---------------------------------------------------------

fn spawn_damage_hud(mut commands: Commands) {
    const BG: Color = Color::srgba(0.05, 0.05, 0.07, 0.75);
    const PANEL_W: f32 = 150.0;
    const PANEL_H: f32 = 30.0;
    const BAR_H: f32 = 8.0;

    // Root: top-centre, hidden when damage is clean.
    let root = commands.spawn((
        DamageHudRoot,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Percent(50.0),
            top: Val::Px(12.0),
            // Negative margin centres a fixed-width panel.
            margin: UiRect {
                left: Val::Px(-(PANEL_W / 2.0)),
                ..default()
            },
            width: Val::Px(PANEL_W),
            height: Val::Px(PANEL_H),
            flex_direction: FlexDirection::Column,
            padding: UiRect::all(Val::Px(4.0)),
            row_gap: Val::Px(3.0),
            display: Display::None,
            ..default()
        },
        BackgroundColor(BG),
        Outline {
            width: Val::Px(1.0),
            offset: Val::Px(0.0),
            color: Color::WHITE,
        },
    )).id();

    // Bar background (outline track).
    let bar_bg = commands.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Px(BAR_H),
            ..default()
        },
        BackgroundColor(Color::srgba(0.15, 0.15, 0.15, 0.8)),
    )).id();

    // Bar fill — width driven each frame.
    let bar_fill = commands.spawn((
        DamageBarFill,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            top: Val::Px(0.0),
            width: Val::Percent(0.0),
            height: Val::Percent(100.0),
            ..default()
        },
        BackgroundColor(Color::srgb(0.1, 0.8, 0.1)),
    )).id();

    commands.entity(bar_bg).add_child(bar_fill);

    // Label text.
    let label = commands.spawn((
        DamageBarText,
        Text::new("DMG: 0%"),
        TextFont { font_size: 11.0, ..default() },
        TextColor(Color::WHITE),
    )).id();

    commands.entity(root).add_children(&[bar_bg, label]);
}

fn update_damage_hud(
    damage: Res<DamageState>,
    mut root_q: Query<&mut Node, With<DamageHudRoot>>,
    mut fill_q: Query<(&mut Node, &mut BackgroundColor), (With<DamageBarFill>, Without<DamageHudRoot>)>,
    mut text_q: Query<(&mut Text, &mut TextColor), With<DamageBarText>>,
) {
    let d = damage.damage;

    // Hide panel when chassis is clean.
    for mut node in &mut root_q {
        node.display = if d < 0.05 { Display::None } else { Display::Flex };
    }

    // Bar fill width and colour.
    let bar_color = if d < 0.3 {
        Color::srgb(0.1, 0.8, 0.1)
    } else if d < 0.6 {
        let t = (d - 0.3) / 0.3;
        Color::srgb(0.1 + 0.85 * t, 0.8 - 0.35 * t, 0.1)
    } else {
        Color::srgb(0.95, 0.2, 0.1)
    };

    for (mut node, mut bg) in &mut fill_q {
        node.width = Val::Percent(d * 100.0);
        bg.0 = bar_color;
    }

    // Text label.
    let pct = (d * 100.0).round() as u32;
    for (mut text, _) in &mut text_q {
        text.0 = format!("DMG: {}%", pct);
    }
}

// ---- Wreck flash border -----------------------------------------------------

// Pulsing red screen-edge border at damage >= WRECK_THRESHOLD.
// No auto-teleport: press R (wired elsewhere) to respawn.

fn update_wreck_flash(
    damage: Res<DamageState>,
    time: Res<Time>,
    mut commands: Commands,
    border_q: Query<Entity, With<WreckBorder>>,
) {
    let is_wrecked = damage.damage >= WRECK_THRESHOLD;

    if !is_wrecked {
        // Remove the border entity if it exists.
        for entity in &border_q {
            commands.entity(entity).despawn();
        }
        return;
    }

    // Flash: alpha oscillates between 0 and 0.6.
    let phase = (time.elapsed_secs() / FLASH_PERIOD * std::f32::consts::TAU).sin();
    let alpha = (phase * 0.5 + 0.5) * 0.6;
    let flash_color = Color::srgba(1.0, 0.0, 0.0, alpha);

    if border_q.is_empty() {
        // Spawn a full-screen overlay with a thick red border.
        commands.spawn((
            WreckBorder,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            Outline {
                width: Val::Px(6.0),
                offset: Val::Px(0.0),
                color: flash_color,
            },
            BackgroundColor(Color::NONE),
        ));
    } else {
        // Update the existing border colour each frame.
        for entity in &border_q {
            commands.entity(entity).insert(Outline {
                width: Val::Px(6.0),
                offset: Val::Px(0.0),
                color: flash_color,
            });
        }
    }
}
