use bevy::prelude::*;
use bevy::window::WindowMode;
use crate::game::state::GameState;
use bevy_egui::EguiPlugin;

mod game;
mod core;
mod physics;
mod rendering;
mod audio;
mod assets;
mod utils;
mod terrain;

fn main() {
    App::new()
        .add_state::<GameState>()
        .insert_resource(ClearColor(Color::rgb(0.5, 0.7, 1.0))) // Sky blue
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "SandK Offroad".into(),
                mode: WindowMode::Windowed,
                resolution: (800., 600.).into(),
                present_mode: bevy::window::PresentMode::Immediate, // Use immediate mode for testing
                ..default()
            }),
            ..default()
        }))
        .add_plugins((
            EguiPlugin,
            game::GamePlugin,
            core::CorePlugin,
            physics::PhysicsPlugin,
            rendering::RenderingPlugin,
            audio::AudioPlugin,
        ))
        .add_systems(Startup, setup_initial_state)
        .run();
}

fn setup_initial_state(mut next_state: ResMut<NextState<GameState>>) {
    next_state.set(GameState::InGame);
} 