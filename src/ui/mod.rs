use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiPlugin};
use crate::game::state::GameState;
// use crate::physics::vehicle::Vehicle;
// use crate::game::state::GameState;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(EguiPlugin)
            .init_resource::<UiState>()
            .add_systems(Update, (
                update_hud,
                handle_menu_interactions,
            ));
    }
}

#[derive(Resource, Default)]
pub struct UiState {
    pub show_menu: bool,
}

fn update_hud(
    mut contexts: EguiContexts,
    // vehicle_query: Query<&Vehicle>,
    state: Res<State<GameState>>,
    ui_state: Res<UiState>,
) {
    if state.get() != &GameState::InGame || ui_state.show_menu {
        return;
    }

    egui::Window::new("HUD")
        .fixed_pos((10.0, 10.0))
        .show(contexts.ctx_mut(), |ui| {
            // Vehicle HUD removed: No vehicle data available
            // if let Ok(vehicle) = vehicle_query.get_single() {
            //     let speed_percentage = (vehicle.speed / vehicle.max_speed).min(1.0);
            //     ui.add(egui::ProgressBar::new(speed_percentage)
            //         .text(format!("Speed: {:.0} km/h", vehicle.speed * 3.6)));
            // }
        });
}

fn handle_menu_interactions(
    mut contexts: EguiContexts,
    mut next_state: ResMut<NextState<GameState>>,
    mut ui_state: ResMut<UiState>,
    keyboard: Res<Input<KeyCode>>,
) {
    if keyboard.just_pressed(KeyCode::Escape) {
        ui_state.show_menu = !ui_state.show_menu;
    }

    if !ui_state.show_menu {
        return;
    }

    egui::Window::new("Menu")
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(contexts.ctx_mut(), |ui| {
            if ui.button("Resume").clicked() {
                ui_state.show_menu = false;
            }
            if ui.button("Restart").clicked() {
                next_state.set(GameState::Loading);
                ui_state.show_menu = false;
            }
            if ui.button("Quit").clicked() {
                next_state.set(GameState::MainMenu);
                ui_state.show_menu = false;
            }
        });
} 