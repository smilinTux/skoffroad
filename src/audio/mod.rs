use bevy::prelude::*;
use bevy::audio::*;
use bevy::math::Vec3;
// use crate::physics::vehicle::Vehicle;
use bevy_rapier3d::prelude::CollisionEvent;
use bevy_rapier3d::prelude::Velocity;
use std::collections::HashMap;

pub struct AudioPlugin;

impl Plugin for AudioPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AudioAssets>()
           .init_resource::<AudioSettings>()
           .init_resource::<SoundEffectPool>()
           .add_systems(Update, (
                update_vehicle_sounds,
                handle_environment_sounds,
                update_spatial_audio,
                cleanup_finished_sounds,
            ));
    }
}

#[derive(Resource)]
pub struct AudioAssets {
    pub engine_sound: Handle<AudioSource>,
    pub crash_sound: Handle<AudioSource>,
    pub ambient_sound: Handle<AudioSource>,
    pub tire_squeal: Handle<AudioSource>,
    pub wind: Handle<AudioSource>,
    pub suspension: Handle<AudioSource>,
}

impl FromWorld for AudioAssets {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.resource::<AssetServer>();
        Self {
            engine_sound: asset_server.load("sounds/engine.ogg"),
            crash_sound: asset_server.load("sounds/crash.ogg"),
            ambient_sound: asset_server.load("sounds/ambient.ogg"),
            tire_squeal: asset_server.load("sounds/tire_squeal.ogg"),
            wind: asset_server.load("sounds/wind.ogg"),
            suspension: asset_server.load("sounds/suspension.ogg"),
        }
    }
}

#[derive(Resource)]
pub struct AudioSettings {
    master_volume: f32,
    engine_volume: f32,
    effects_volume: f32,
    ambient_volume: f32,
    spatial_scale: f32,
    doppler_effect: bool,
}

impl Default for AudioSettings {
    fn default() -> Self {
        Self {
            master_volume: 1.0,
            engine_volume: 0.8,
            effects_volume: 0.7,
            ambient_volume: 0.5,
            spatial_scale: 1.0,
            doppler_effect: true,
        }
    }
}

#[derive(Resource)]
struct SoundEffectPool {
    active_sounds: HashMap<Entity, ActiveSound>,
    available_entities: Vec<Entity>,
}

impl Default for SoundEffectPool {
    fn default() -> Self {
        Self {
            active_sounds: HashMap::new(),
            available_entities: Vec::new(),
        }
    }
}

struct ActiveSound {
    entity: Entity,
    duration: f32,
    elapsed: f32,
    category: SoundCategory,
}

#[derive(PartialEq)]
enum SoundCategory {
    Engine,
    Effect,
    Ambient,
}

fn update_vehicle_sounds(
    mut commands: Commands,
    // vehicle_query: Query<(&Vehicle, &Transform, &Velocity)>,
    audio_assets: Res<AudioAssets>,
    settings: Res<AudioSettings>,
    mut sound_pool: ResMut<SoundEffectPool>,
    time: Res<Time>,
) {
    // Vehicle sound logic removed: No vehicle data available
    // for (vehicle, transform, velocity) in vehicle_query.iter() {
    //     let speed = velocity.linvel.length();
    //     let rpm_factor = vehicle.engine.current_rpm / vehicle.engine.max_rpm;
    //     // Engine sound modulation
    //     let volume = (rpm_factor * 0.8 + 0.2) * settings.engine_volume * settings.master_volume;
    //     let base_pitch = rpm_factor * 0.5 + 0.75;
    //     let load_pitch = if vehicle.engine.throttle > 0.1 { 1.1 } else { 1.0 };
    //     let final_pitch = base_pitch * load_pitch;
    //     spawn_or_update_sound(
    //         &mut commands,
    //         &mut sound_pool,
    //         audio_assets.engine_sound.clone(),
    //         transform.translation,
    //         volume,
    //         final_pitch,
    //         SoundCategory::Engine,
    //         true,
    //         None,
    //     );
    //     // Tire squeal based on lateral force
    //     if vehicle.wheels.iter().any(|w| w.slip_ratio.abs() > 0.2) {
    //         spawn_or_update_sound(
    //             &mut commands,
    //             &mut sound_pool,
    //             audio_assets.tire_squeal.clone(),
    //             transform.translation,
    //             0.4 * settings.effects_volume * settings.master_volume,
    //             1.0,
    //             SoundCategory::Effect,
    //             true,
    //             None,
    //         );
    //     }
    //     // Wind sound based on speed
    //     if speed > 10.0 {
    //         let wind_volume = (speed / 100.0).min(1.0) * 0.3;
    //         spawn_or_update_sound(
    //             &mut commands,
    //             &mut sound_pool,
    //             audio_assets.wind.clone(),
    //             transform.translation,
    //             wind_volume * settings.effects_volume * settings.master_volume,
    //             1.0,
    //             SoundCategory::Ambient,
    //             true,
    //             None,
    //         );
    //     }
    // }
}

fn handle_environment_sounds(
    mut commands: Commands,
    mut collision_events: EventReader<CollisionEvent>,
    audio_assets: Res<AudioAssets>,
    settings: Res<AudioSettings>,
    mut sound_pool: ResMut<SoundEffectPool>,
    query: Query<&Transform>,
) {
    for event in collision_events.read() {
        if let CollisionEvent::Started(entity1, entity2, _) = event {
            // Get collision position from either entity
            if let Ok(transform) = query.get(*entity1) {
                let impact_velocity = 10.0; // TODO: Calculate from actual collision
                let volume = f32::min(impact_velocity / 20.0, 1.0) * 0.5;
                
                spawn_or_update_sound(
                    &mut commands,
                    &mut sound_pool,
                    audio_assets.crash_sound.clone(),
                    transform.translation,
                    volume * settings.effects_volume * settings.master_volume,
                    1.0,
                    SoundCategory::Effect,
                    false,
                    Some(0.5),
                );
            }
        }
    }
}

fn update_spatial_audio(
    mut audio_query: Query<(&mut Transform, &AudioSink)>,
    camera_query: Query<&Transform, With<Camera>>,
    settings: Res<AudioSettings>,
) {
    if let Ok(camera_transform) = camera_query.get_single() {
        for (mut transform, _sink) in audio_query.iter_mut() {
            if settings.spatial_scale > 0.0 {
                let distance = transform.translation.distance(camera_transform.translation);
                let attenuation = 1.0 / (1.0 + distance * settings.spatial_scale);
                // Update spatial audio parameters
                transform.translation = transform.translation * attenuation;
            }
        }
    }
}

fn cleanup_finished_sounds(
    mut commands: Commands,
    mut sound_pool: ResMut<SoundEffectPool>,
    time: Res<Time>,
) {
    let mut to_remove = Vec::new();
    for (entity, sound) in sound_pool.active_sounds.iter_mut() {
        sound.elapsed += time.delta_seconds();
        let duration = sound.duration;
        if sound.elapsed >= duration {
            to_remove.push(*entity);
        }
    }
    for entity in &to_remove {
        commands.entity(*entity).despawn();
        sound_pool.available_entities.push(*entity);
        sound_pool.active_sounds.remove(entity);
    }
}

fn spawn_or_update_sound(
    commands: &mut Commands,
    sound_pool: &mut SoundEffectPool,
    source: Handle<AudioSource>,
    position: Vec3,
    volume: f32,
    pitch: f32,
    category: SoundCategory,
    looped: bool,
    duration: Option<f32>,
) {
    let settings = if looped {
        PlaybackSettings::LOOP
    } else {
        PlaybackSettings::ONCE
    }
    .with_volume(Volume::new_relative(volume))
    .with_speed(pitch);

    let entity = if let Some(entity) = sound_pool.available_entities.pop() {
        entity
    } else {
        commands.spawn_empty().id()
    };

    commands.entity(entity).insert(AudioBundle {
        source,
        settings,
        ..default()
    });

    sound_pool.active_sounds.insert(entity, ActiveSound {
        entity,
        duration: duration.unwrap_or(f32::INFINITY),
        elapsed: 0.0,
        category,
    });
} 