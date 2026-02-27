mod spatial_audio;
mod engine_audio;
mod environment_audio;

pub use spatial_audio::*;
pub use engine_audio::*;
pub use environment_audio::*;

use bevy::prelude::*;
use bevy::audio::*;
use bevy_kira_audio::{AudioPlugin, AudioSource};

pub mod voice_chat;
pub mod cb_radio;
pub mod cb_chatter;
pub mod ui;

use voice_chat::VoiceChatPlugin;
use cb_radio::{CBRadioPlugin, CBRadio, SignalQuality};
use cb_chatter::CBChatterPlugin;
use ui::cb_radio_ui::CBRadioUIPlugin;

pub use cb_radio::{CBRadio, SignalQuality};

/// Resource for managing audio assets
#[derive(Resource)]
pub struct AudioAssets {
    /// Engine sounds (idle, rev, high rpm)
    pub engine_sounds: Vec<Handle<AudioSource>>,
    /// Environmental ambient sounds
    pub environment_sounds: Vec<Handle<AudioSource>>,
    /// Radio station music tracks
    pub radio_tracks: Vec<Handle<AudioSource>>,
    /// CB radio effects (static, chatter)
    pub cb_radio_effects: Vec<Handle<AudioSource>>,
}

/// Resource for audio settings
#[derive(Resource)]
pub struct AudioSettings {
    /// Master volume (0.0 - 1.0)
    pub master_volume: f32,
    /// Engine sounds volume
    pub engine_volume: f32,
    /// Environment sounds volume
    pub environment_volume: f32,
    /// Radio volume
    pub radio_volume: f32,
    /// CB radio volume
    pub cb_radio_volume: f32,
    /// Toggle flags for different audio categories
    pub flags: AudioFlags,
}

/// Flags for toggling different audio categories
#[derive(Default)]
pub struct AudioFlags {
    pub engine_enabled: bool,
    pub environment_enabled: bool,
    pub radio_enabled: bool,
    pub cb_radio_enabled: bool,
}

impl Default for AudioSettings {
    fn default() -> Self {
        Self {
            master_volume: 0.8,
            engine_volume: 0.7,
            environment_volume: 0.5,
            radio_volume: 0.4,
            cb_radio_volume: 0.6,
            flags: AudioFlags {
                engine_enabled: true,
                environment_enabled: true,
                radio_enabled: true,
                cb_radio_enabled: true,
            },
        }
    }
}

/// Main audio plugin that sets up all audio systems
pub struct AudioPlugin;

impl Plugin for AudioPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AudioSettings>()
           .add_plugins((
               SpatialAudioPlugin,
               EngineAudioPlugin,
               EnvironmentAudioPlugin,
               VoiceChatPlugin,
               CBRadioPlugin,
               CBChatterPlugin,
               CBRadioUIPlugin,
           ))
           .add_systems(Startup, load_audio_assets)
           .add_systems(Update, (
               update_audio_settings,
               handle_audio_keyboard_input,
           ));
    }
}

/// System to load all audio assets
fn load_audio_assets(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    let audio_assets = AudioAssets {
        engine_sounds: vec![
            asset_server.load("audio/engine/idle.ogg"),
            asset_server.load("audio/engine/rev.ogg"),
            asset_server.load("audio/engine/high_rpm.ogg"),
        ],
        environment_sounds: vec![
            asset_server.load("audio/environment/city_ambient.ogg"),
            asset_server.load("audio/environment/highway_ambient.ogg"),
            asset_server.load("audio/environment/forest_ambient.ogg"),
            asset_server.load("audio/environment/desert_ambient.ogg"),
            asset_server.load("audio/environment/mountain_ambient.ogg"),
            asset_server.load("audio/environment/beach_ambient.ogg"),
            asset_server.load("audio/environment/swamp_ambient.ogg"),
            // Weather sounds
            asset_server.load("audio/environment/rain.ogg"),
            asset_server.load("audio/environment/storm.ogg"),
            asset_server.load("audio/environment/wind.ogg"),
            asset_server.load("audio/environment/snow.ogg"),
        ],
        radio_tracks: vec![
            // Will be populated when implementing radio system
        ],
        cb_radio_effects: vec![
            // Will be populated when implementing CB radio system
        ],
    };

    commands.insert_resource(audio_assets);
}

/// System to update audio settings
fn update_audio_settings(
    settings: Res<AudioSettings>,
    mut audio_sinks: Query<(&mut AudioSink, Option<&EngineAudioSource>, Option<&EnvironmentAudioSource>)>,
) {
    for (mut sink, engine_source, env_source) in audio_sinks.iter_mut() {
        let base_volume = if let Some(_) = engine_source {
            settings.engine_volume * settings.flags.engine_enabled as u8 as f32
        } else if let Some(_) = env_source {
            settings.environment_volume * settings.flags.environment_enabled as u8 as f32
        } else {
            1.0
        };

        sink.set_volume(base_volume * settings.master_volume);
    }
}

/// System to handle keyboard input for audio settings
fn handle_audio_keyboard_input(
    mut settings: ResMut<AudioSettings>,
    keyboard: Res<Input<KeyCode>>,
) {
    // Toggle flags
    if keyboard.just_pressed(KeyCode::Key1) {
        settings.flags.engine_enabled = !settings.flags.engine_enabled;
    }
    if keyboard.just_pressed(KeyCode::Key2) {
        settings.flags.environment_enabled = !settings.flags.environment_enabled;
    }
    if keyboard.just_pressed(KeyCode::Key3) {
        settings.flags.radio_enabled = !settings.flags.radio_enabled;
    }
    if keyboard.just_pressed(KeyCode::Key4) {
        settings.flags.cb_radio_enabled = !settings.flags.cb_radio_enabled;
    }

    // Volume controls
    let volume_change = if keyboard.pressed(KeyCode::LShift) { 0.1 } else { 0.05 };
    
    if keyboard.just_pressed(KeyCode::Equals) {
        settings.master_volume = (settings.master_volume + volume_change).min(1.0);
    }
    if keyboard.just_pressed(KeyCode::Minus) {
        settings.master_volume = (settings.master_volume - volume_change).max(0.0);
    }
}

// Re-export important types
pub use spatial_audio::{SpatialAudioSource, AudioPriority};
pub use engine_audio::EngineAudioConfig; 