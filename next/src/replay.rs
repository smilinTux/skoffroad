// Replay system: always-on ring-buffer recording + ghost-car playback.
//
// Press `.` (Period) to replay the last 10 seconds of chassis motion as a
// translucent red ghost driving alongside.  Playback runs at half speed so
// nuances of the run are easier to appreciate.

use std::collections::VecDeque;
use bevy::prelude::*;

use crate::vehicle::{Chassis, VehicleRoot};

// ---- Constants ---------------------------------------------------------------

const BUFFER_CAPACITY: usize = 600;   // 10 s at 60 Hz
const PLAYBACK_SPEED: f32    = 0.5;   // frames advanced per engine frame

// Ghost box dimensions: roughly match the Jeep chassis silhouette.
const GHOST_X: f32 = 2.0;  // full width  (CHASSIS_HALF.x * 2)
const GHOST_Y: f32 = 0.8;  // full height (CHASSIS_HALF.y * 2)
const GHOST_Z: f32 = 4.0;  // full length (CHASSIS_HALF.z * 2)

// ---- Plugin ------------------------------------------------------------------

pub struct ReplayPlugin;

impl Plugin for ReplayPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ReplayBuffer>()
           .init_resource::<ReplayPlayback>()
           .add_systems(Startup, spawn_replay_hud)
           .add_systems(Update, (
               record_chassis.run_if(resource_exists::<VehicleRoot>),
               toggle_replay,
               update_replay_ghost,
               update_replay_hud,
           ));
    }
}

// ---- Resources ---------------------------------------------------------------

#[derive(Resource, Default)]
pub struct ReplayBuffer {
    /// Ring buffer of chassis transforms.  Newest samples at the back.
    pub samples: VecDeque<Transform>,
}

#[derive(Resource, Default)]
pub struct ReplayPlayback {
    /// True when playback is active.
    pub active: bool,
    /// Fractional index into frozen_samples.  Advances 0.5 per frame.
    pub cursor: f32,
    /// Snapshot frozen at the moment playback began.
    pub frozen_samples: Vec<Transform>,
    /// Ghost entity spawned for this playback session.
    pub ghost: Option<Entity>,
}

// ---- Marker components -------------------------------------------------------

#[derive(Component)]
struct GhostCar;

#[derive(Component)]
struct ReplayBanner;

// ---- Systems -----------------------------------------------------------------

/// Push the current chassis transform into the ring buffer every frame.
fn record_chassis(
    vehicle: Res<VehicleRoot>,
    chassis_q: Query<&Transform, With<Chassis>>,
    mut buffer: ResMut<ReplayBuffer>,
) {
    let Ok(transform) = chassis_q.get(vehicle.chassis) else { return };
    buffer.samples.push_back(*transform);
    if buffer.samples.len() > BUFFER_CAPACITY {
        buffer.samples.pop_front();
    }
}

/// On Period press, freeze the buffer and spawn the ghost car.
fn toggle_replay(
    keys: Res<ButtonInput<KeyCode>>,
    buffer: ResMut<ReplayBuffer>,
    mut playback: ResMut<ReplayPlayback>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if !keys.just_pressed(KeyCode::Period) {
        return;
    }
    if playback.active {
        return;
    }
    if buffer.samples.is_empty() {
        return;
    }

    // Freeze a snapshot; recording continues in the live buffer independently.
    playback.frozen_samples = buffer.samples.iter().copied().collect();
    playback.cursor = 0.0;
    playback.active = true;

    // Spawn ghost: a translucent red box.
    let ghost_mesh = meshes.add(Cuboid::new(GHOST_X, GHOST_Y, GHOST_Z));
    let ghost_mat  = materials.add(StandardMaterial {
        base_color: Color::srgba(0.9, 0.2, 0.2, 0.45),
        alpha_mode: AlphaMode::Blend,
        double_sided: true,
        ..default()
    });

    let start_tf = *playback.frozen_samples.first().unwrap();
    let id = commands.spawn((
        GhostCar,
        Mesh3d(ghost_mesh),
        MeshMaterial3d(ghost_mat),
        start_tf,
    )).id();

    playback.ghost = Some(id);
}

/// Advance playback each frame; despawn the ghost when the buffer is exhausted.
fn update_replay_ghost(
    mut playback: ResMut<ReplayPlayback>,
    mut transforms: Query<&mut Transform, With<GhostCar>>,
    mut commands: Commands,
) {
    if !playback.active {
        // Safety net: if somehow the ghost is around while inactive, remove it.
        if let Some(ghost) = playback.ghost.take() {
            commands.entity(ghost).despawn();
        }
        return;
    }

    playback.cursor += PLAYBACK_SPEED;
    let index = playback.cursor.floor() as usize;

    if index >= playback.frozen_samples.len() {
        // Playback finished.
        if let Some(ghost) = playback.ghost.take() {
            commands.entity(ghost).despawn();
        }
        playback.active         = false;
        playback.cursor         = 0.0;
        playback.frozen_samples.clear();
        return;
    }

    let sample = playback.frozen_samples[index];
    if let Some(ghost) = playback.ghost {
        if let Ok(mut tf) = transforms.get_mut(ghost) {
            *tf = sample;
        }
    }
}

// ---- HUD banner --------------------------------------------------------------

fn spawn_replay_hud(mut commands: Commands) {
    commands.spawn((
        ReplayBanner,
        Text::new(""),
        TextFont { font_size: 22.0, ..default() },
        TextColor(Color::srgba(1.0, 0.55, 0.55, 0.95)),
        Node {
            position_type: PositionType::Absolute,
            // Centre horizontally: use left=0, right=0, align children center.
            left: Val::Percent(0.0),
            right: Val::Percent(0.0),
            top: Val::Px(10.0),
            justify_self: JustifySelf::Center,
            align_self: AlignSelf::Start,
            ..default()
        },
    ));
}

fn update_replay_hud(
    playback: Res<ReplayPlayback>,
    mut banner_q: Query<&mut Text, With<ReplayBanner>>,
) {
    let Ok(mut text) = banner_q.single_mut() else { return };

    if playback.active && !playback.frozen_samples.is_empty() {
        let total_s   = playback.frozen_samples.len() as f32 / 60.0;
        // cursor is sample index at half-speed; convert to wall-clock seconds.
        let elapsed_s = (playback.cursor * PLAYBACK_SPEED).min(total_s);
        text.0 = format!("REPLAY: {:.1} / {:.1} s", elapsed_s, total_s);
    } else {
        text.0 = String::new();
    }
}
