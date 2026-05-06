// In-game vehicle swap: V key cycles through unlocked vehicle variants.
// Reads UnlockState to determine which variants are available.
//
// How visual swap works:
//   VehicleVariant (resource in variants.rs) is updated to match GarageState.
//   variants.rs does NOT watch Changed<VehicleVariant> — its cycle_variant
//   system only fires on Backslash. So garage logs "would swap" and the
//   VehicleVariant resource is kept in sync for any future systems that read it.
//
// Public API:
//   GaragePlugin
//   GarageState (resource)

use bevy::prelude::*;

use crate::unlocks::{UnlockState, Unlockable};
use crate::variants::VehicleVariant;
use crate::vehicle::VehicleRoot;

// ---- Plugin -----------------------------------------------------------------

pub struct GaragePlugin;

impl Plugin for GaragePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GarageState>()
            .add_systems(Startup, spawn_garage_toast)
            .add_systems(
                Update,
                (cycle_with_v, apply_variant_swap, tick_toast).chain(),
            );
    }
}

// ---- Public API -------------------------------------------------------------

#[derive(Resource, Default, Clone, Copy)]
pub struct GarageState {
    pub current_idx: u32,
}

// ---- Variant index mapping --------------------------------------------------

// Index → (VehicleVariant, optional unlock gate)
const VARIANT_TABLE: [(VehicleVariant, Option<Unlockable>); 5] = [
    (VehicleVariant::JeepTJ,     None),
    (VehicleVariant::FordBronco, Some(Unlockable::VariantBronco)),
    (VehicleVariant::Pickup,     Some(Unlockable::VariantPickup)),
    (VehicleVariant::Hummer,     Some(Unlockable::VariantHummer)),
    (VehicleVariant::Buggy,      Some(Unlockable::VariantBuggy)),
];

fn variant_name(idx: u32) -> &'static str {
    match idx {
        0 => "Jeep TJ",
        1 => "Ford Bronco",
        2 => "Pickup",
        3 => "Hummer",
        4 => "Buggy",
        _ => "Unknown",
    }
}

fn is_unlocked(idx: u32, unlock: &UnlockState) -> bool {
    let Some((_, gate)) = VARIANT_TABLE.get(idx as usize) else {
        return false;
    };
    match gate {
        None      => true,
        Some(key) => unlock.unlocked.contains(key),
    }
}

// ---- HUD toast components ---------------------------------------------------

#[derive(Component)]
struct GarageToastRoot;

#[derive(Component)]
struct GarageToastText;

#[derive(Resource, Default)]
struct GarageToastTimer(f32);

// ---- Startup: spawn hidden HUD toast ----------------------------------------

fn spawn_garage_toast(mut commands: Commands) {
    commands.init_resource::<GarageToastTimer>();

    commands
        .spawn((
            GarageToastRoot,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Percent(35.0),
                bottom: Val::Px(40.0),
                width: Val::Px(220.0),
                padding: UiRect::all(Val::Px(8.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.05, 0.05, 0.07, 0.80)),
            Visibility::Hidden,
            ZIndex(300),
        ))
        .with_children(|p| {
            p.spawn((
                GarageToastText,
                Text::new("VEHICLE: "),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgb(0.85, 0.90, 0.60)),
            ));
        });
}

// ---- cycle_with_v -----------------------------------------------------------

fn cycle_with_v(
    keys:    Res<ButtonInput<KeyCode>>,
    unlock:  Res<UnlockState>,
    mut gs:  ResMut<GarageState>,
) {
    if !keys.just_pressed(KeyCode::KeyV) {
        return;
    }

    let current = gs.current_idx;
    let mut next = (current + 1) % 5;

    // Advance until we land on an unlocked variant (max 5 iterations to avoid
    // infinite loop when nothing is unlocked beyond TJ, which can't happen).
    for _ in 0..5 {
        if is_unlocked(next, &unlock) {
            break;
        }
        next = (next + 1) % 5;
    }

    if next != current {
        let name = variant_name(next);
        info!("garage: switched to {name}");
        gs.current_idx = next;
    }
}

// ---- apply_variant_swap -----------------------------------------------------

fn apply_variant_swap(
    gs:             Res<GarageState>,
    mut vv:         ResMut<VehicleVariant>,
    vehicle_root:   Option<Res<VehicleRoot>>,
    mut text_q:     Query<&mut Text, With<GarageToastText>>,
    mut root_q:     Query<&mut Visibility, With<GarageToastRoot>>,
    mut timer:      ResMut<GarageToastTimer>,
) {
    if !gs.is_changed() {
        return;
    }

    let idx = gs.current_idx;
    let Some((target_variant, _)) = VARIANT_TABLE.get(idx as usize) else {
        return;
    };

    // Update VehicleVariant resource so downstream readers stay in sync.
    *vv = *target_variant;

    // Check chassis availability and log swap intent.
    if vehicle_root.is_some() {
        // variants.rs does NOT watch Changed<VehicleVariant>; it only reacts to
        // Backslash. We keep the resource in sync but the visual won't change
        // until variants.rs is extended to watch this resource.
        info!(
            "garage: would swap chassis skin to {} (variants.rs reactivity not wired)",
            variant_name(idx)
        );
    }

    // Show toast.
    let label = format!("VEHICLE: {}", variant_name(idx));
    for mut text in &mut text_q {
        text.0 = label.clone();
    }
    for mut vis in &mut root_q {
        *vis = Visibility::Inherited;
    }
    timer.0 = 2.0;
}

// ---- tick_toast -------------------------------------------------------------

fn tick_toast(
    time:      Res<Time>,
    mut timer: ResMut<GarageToastTimer>,
    mut root_q: Query<(&mut Visibility, &Children), With<GarageToastRoot>>,
    mut text_color_q: Query<&mut TextColor, With<GarageToastText>>,
) {
    if timer.0 <= 0.0 {
        return;
    }

    timer.0 = (timer.0 - time.delta_secs()).max(0.0);

    // Compute alpha: fade from 1.0 → 0.0 over the last 0.5 s of the 2 s window.
    let alpha = if timer.0 > 0.5 {
        1.0_f32
    } else {
        (timer.0 / 0.5).max(0.0)
    };

    for mut color in &mut text_color_q {
        color.0 = Color::srgba(0.85, 0.90, 0.60, alpha);
    }

    // Hide root when fully faded.
    if alpha < 0.01 {
        for (mut vis, _) in &mut root_q {
            *vis = Visibility::Hidden;
        }
    }
}
