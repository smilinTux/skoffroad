use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use crate::game::state::GameState;
use crate::game::states::{GameTimer, GameProgress};

pub struct GameUIPlugin;

impl Plugin for GameUIPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (
            show_main_menu.run_if(in_state(GameState::MainMenu)),
            show_game_ui.run_if(in_state(GameState::InGame)),
            show_pause_menu.run_if(in_state(GameState::Paused)),
            show_game_over.run_if(in_state(GameState::GameOver)),
        ));
    }
}

pub fn show_main_menu(
    mut contexts: EguiContexts,
    mut commands: Commands,
) {
    egui::CentralPanel::default().show(contexts.ctx_mut(), |ui| {
        ui.vertical_centered(|ui| {
            ui.heading("Offroad Racing Game");
            ui.add_space(20.0);
            if ui.button("Start Game").clicked() {
                commands.insert_resource(NextState(Some(GameState::InGame)));
            }
            if ui.button("Quit").clicked() {
                std::process::exit(0);
            }
        });
    });
}

pub fn show_game_ui(
    mut contexts: EguiContexts,
    game_timer: Res<GameTimer>,
    game_progress: Res<GameProgress>,
) {
    egui::Window::new("Game Stats").show(contexts.ctx_mut(), |ui| {
        ui.label(format!("Time: {:.2}", game_timer.elapsed));
        ui.label(format!("Level: {}", game_progress.current_level));
        ui.label(format!("Score: {}", game_progress.total_score));
    });
}

pub fn show_pause_menu(
    mut contexts: EguiContexts,
    mut commands: Commands,
) {
    egui::CentralPanel::default().show(contexts.ctx_mut(), |ui| {
        ui.vertical_centered(|ui| {
            ui.heading("Game Paused");
            ui.add_space(20.0);
            if ui.button("Resume").clicked() {
                commands.insert_resource(NextState(Some(GameState::InGame)));
            }
            if ui.button("Main Menu").clicked() {
                commands.insert_resource(NextState(Some(GameState::MainMenu)));
            }
        });
    });
}

pub fn show_game_over(
    mut contexts: EguiContexts,
    mut commands: Commands,
    game_timer: Res<GameTimer>,
) {
    egui::CentralPanel::default().show(contexts.ctx_mut(), |ui| {
        ui.vertical_centered(|ui| {
            ui.heading("Game Over");
            ui.add_space(20.0);
            ui.label(format!("Time: {:.2}", game_timer.elapsed));
            if ui.button("Restart").clicked() {
                commands.insert_resource(NextState(Some(GameState::InGame)));
            }
            if ui.button("Main Menu").clicked() {
                commands.insert_resource(NextState(Some(GameState::MainMenu)));
            }
        });
    });
} 