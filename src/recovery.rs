// Auto-flip recovery system.
//
// Key binding: J — rights the chassis in-place (no teleport to spawn).
// R already resets to spawn (save.rs); this is separate.
//
// Algorithm:
//   1. Extract yaw from the chassis rotation, discard pitch/roll.
//   2. Rebuild rotation from yaw only.
//   3. Lift +1.5 m so the chassis clears terrain after re-orienting.
//   4. Zero angular velocity so stored momentum doesn't re-tip it.
//   5. Halve linear velocity to reduce the jolt.
//
// HUD feedback: "RECOVERED!" banner at bottom-centre for ~1.5 s (90 ticks at 60 Hz).

use bevy::prelude::*;
use avian3d::prelude::{AngularVelocity, LinearVelocity};

use crate::vehicle::{Chassis, VehicleRoot};

// ---- Plugin ------------------------------------------------------------------

pub struct RecoveryPlugin;

impl Plugin for RecoveryPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RecoveryFlash>()
            .add_systems(Startup, spawn_recovery_hud)
            .add_systems(
                Update,
                (recover_chassis, tick_flash)
                    .run_if(resource_exists::<VehicleRoot>),
            );
    }
}

// ---- Resources / components --------------------------------------------------

#[derive(Resource, Default)]
struct RecoveryFlash {
    ticks_remaining: u32,
}

// 90 ticks ≈ 1.5 s at 60 Hz; at lower frame rates it stays up slightly longer,
// which is acceptable for a feedback banner.
const FLASH_TICKS: u32 = 90;

#[derive(Component)]
struct RecoveryBanner;

// ---- Startup: spawn the banner node (hidden by default) ----------------------

fn spawn_recovery_hud(mut commands: Commands) {
    // Outer node: full-screen, used only to position the banner at bottom-centre.
    commands
        .spawn(Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            position_type: PositionType::Absolute,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::FlexEnd,
            ..default()
        })
        .with_children(|parent| {
            parent
                .spawn((
                    RecoveryBanner,
                    Node {
                        padding: UiRect::axes(Val::Px(24.0), Val::Px(10.0)),
                        margin: UiRect::bottom(Val::Px(32.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.05, 0.35, 0.10, 0.82)),
                    Visibility::Hidden,
                ))
                .with_children(|banner| {
                    banner.spawn((
                        Text::new("RECOVERED!"),
                        TextFont {
                            font_size: 26.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.45, 1.0, 0.55)),
                    ));
                });
        });
}

// ---- Update: J key handler ---------------------------------------------------

fn recover_chassis(
    keys: Res<ButtonInput<KeyCode>>,
    vehicle: Res<VehicleRoot>,
    mut chassis_q: Query<
        (&mut Transform, &mut LinearVelocity, &mut AngularVelocity),
        With<Chassis>,
    >,
    mut flash: ResMut<RecoveryFlash>,
) {
    if !keys.just_pressed(KeyCode::KeyJ) {
        return;
    }

    let Ok((mut transform, mut lin_vel, mut ang_vel)) = chassis_q.get_mut(vehicle.chassis) else {
        return;
    };

    // Strip pitch and roll, keep only yaw so the vehicle lands upright.
    let (yaw, _pitch, _roll) = transform.rotation.to_euler(EulerRot::YXZ);
    transform.rotation = Quat::from_rotation_y(yaw);

    // Lift above terrain to avoid immediate re-collision after re-orienting.
    transform.translation.y += 1.5;

    // Clear angular momentum so the physics engine won't rotate it further.
    ang_vel.0 = Vec3::ZERO;

    // Dampen linear velocity — don't zero it so there's no jarring stop,
    // but reduce it enough that the chassis doesn't sail off a slope.
    lin_vel.0 *= 0.5;

    flash.ticks_remaining = FLASH_TICKS;

    info!("recovery: chassis righted at {:?}", transform.translation);
}

// ---- Update: banner visibility tick ------------------------------------------

fn tick_flash(
    mut flash: ResMut<RecoveryFlash>,
    mut banner_q: Query<&mut Visibility, With<RecoveryBanner>>,
) {
    let Ok(mut vis) = banner_q.single_mut() else { return };

    if flash.ticks_remaining > 0 {
        flash.ticks_remaining -= 1;
        *vis = Visibility::Visible;
    } else {
        *vis = Visibility::Hidden;
    }
}
