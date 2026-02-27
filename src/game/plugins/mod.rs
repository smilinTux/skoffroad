use bevy::prelude::*;
use bevy::app::PluginGroupBuilder;
use bevy::input::mouse::MouseMotion;
use crate::game::state::GameState;
use crate::game::state::StatePlugin;
// use crate::game::plugins::ui::UiPlugin;

mod camera;
mod debug;
mod input;
// pub use lighting::LightingPlugin; // TODO: Fix or implement LightingPlugin
mod particle_system;
// pub use physics::PhysicsPlugin; // TODO: Fix or implement PhysicsPlugin
mod post_process;
mod state;
// pub use state::StatePlugin; // TODO: Fix or implement StatePlugin
mod ui;
mod vehicle;
mod terrain;
mod weather;

pub use camera::CameraPlugin;
pub use debug::DebugPlugin;
pub use input::InputPlugin;
// pub use lighting::LightingPlugin; // TODO: Fix or implement LightingPlugin
pub use particle_system::ParticleSystemPlugin;
// pub use physics::PhysicsPlugin; // TODO: Fix or implement PhysicsPlugin
pub use post_process::PostProcessPlugin;
// pub use state::StatePlugin; // TODO: Fix or implement StatePlugin
pub use ui::UiPlugin;
pub use vehicle::VehiclePlugin;
pub use terrain::TerrainPlugin;
pub use weather::WeatherPlugin;

/// Main plugin group that initializes all core game systems
pub struct GamePluginGroup;

impl PluginGroup for GamePluginGroup {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(StatePlugin)
            .add(InputPlugin)
            .add(VehiclePlugin)
            .add(CameraPlugin)
            .add(UiPlugin)
            // .add(LightingPlugin)
            // .add(PhysicsPlugin)
            .add(ParticleSystemPlugin)
            .add(PostProcessPlugin)
            .add(DebugPlugin)
            .add(TerrainPlugin)
            .add(WeatherPlugin)
    }
}

/// Core game plugin that sets up shared resources and systems
pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(GamePluginGroup)
            .init_resource::<Time>()
            .init_resource::<InputState>()
            .init_resource::<DebugInfo>()
            .add_systems(Startup, setup_game)
            .add_systems(Update, update_game_state)
            .add_systems(Update, handle_input)
            .add_systems(Update, update_physics)
            .add_systems(Update, update_vehicles)
            .add_systems(Update, update_camera)
            .add_systems(Update, update_ui);
    }
}

// Resources
#[derive(Resource, Default)]
pub struct InputState {
    pub forward: bool,
    pub backward: bool,
    pub left: bool,
    pub right: bool,
    pub brake: bool,
    pub handbrake: bool,
    pub camera_rotate: Vec2,
    pub camera_zoom: f32,
}

#[derive(Resource, Default)]
pub struct DebugInfo;

// Systems
fn setup_game(mut commands: Commands, asset_server: Res<AssetServer>) {
    // Load initial assets
    asset_server.load_folder("textures");
    asset_server.load_folder("models");
    asset_server.load_folder("audio");
    // No need to insert GameState as a resource
}

fn update_game_state(
    state: Res<State<GameState>>,
    mut next_state: ResMut<NextState<GameState>>,
    time: Res<Time>,
    input: Res<InputState>,
) {
    // Handle game state transitions based on input and conditions
    match state.get() {
        GameState::Loading => {
            // Transition to menu when assets are loaded
            next_state.set(GameState::MainMenu);
        }
        GameState::MainMenu => {
            // Handle menu state logic
        }
        GameState::InGame => {
            // Handle gameplay state logic
        }
        GameState::Paused => {
            // Handle paused state logic
        }
    }
}

fn handle_input(
    mut input_state: ResMut<InputState>,
    keyboard: Res<Input<KeyCode>>,
    mut mouse_motion: EventReader<MouseMotion>,
) {
    // Update input state based on keyboard and mouse input
    input_state.forward = keyboard.pressed(KeyCode::W);
    input_state.backward = keyboard.pressed(KeyCode::S);
    input_state.left = keyboard.pressed(KeyCode::A);
    input_state.right = keyboard.pressed(KeyCode::D);
    input_state.brake = keyboard.pressed(KeyCode::Space);
    input_state.handbrake = keyboard.pressed(KeyCode::ShiftLeft);

    // Handle mouse input for camera control
    for motion in mouse_motion.read() {
        input_state.camera_rotate = motion.delta;
    }
}

fn update_physics(/* physics parameters */) {
    // Update physics simulation
}

fn update_vehicles(/* vehicle parameters */) {
    // Update vehicle states and physics
}

fn update_camera(/* camera parameters */) {
    // Update camera position and rotation
}

fn update_ui(/* ui parameters */) {
    // Update UI elements
}

/// Collection of core game plugins
pub struct CorePlugins;

impl PluginGroup for CorePlugins {
    fn build(self) -> bevy::app::PluginGroupBuilder {
        bevy::app::PluginGroupBuilder::start::<Self>()
            .add(vehicle::VehiclePlugin)
            .add(terrain::TerrainPlugin)
            .add(camera::CameraPlugin)
            .add(ui::UiPlugin)
            .add(weather::WeatherPlugin)
            .add(particle_system::ParticleSystemPlugin)
    }
} 