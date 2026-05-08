use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use crate::game::assets::loading::AssetLoadingState;

pub fn render_loading_screen(
    mut contexts: EguiContexts,
    loading_state: Res<AssetLoadingState>,
) {
    egui::CentralPanel::default().show(contexts.ctx_mut(), |ui| {
        ui.vertical_centered(|ui| {
            ui.add_space(200.0);
            ui.heading("Loading Game Assets");
            ui.add_space(20.0);

            // Overall progress bar
            let total_progress = loading_state.loaded_count as f32 / loading_state.total_assets.max(1) as f32;
            ui.add(egui::ProgressBar::new(total_progress)
                .text(format!("Overall Progress: {:.1}%", total_progress * 100.0))
                .animate(true));
            ui.add_space(10.0);

            // Visualization assets progress
            ui.label("Visualization Assets");
            ui.add(egui::ProgressBar::new(loading_state.visualization_loading_progress)
                .text(format!("{:.1}%", loading_state.visualization_loading_progress * 100.0))
                .animate(true));
            ui.add_space(5.0);

            // Metrics assets progress
            ui.label("Metrics Assets");
            ui.add(egui::ProgressBar::new(loading_state.metrics_loading_progress)
                .text(format!("{:.1}%", loading_state.metrics_loading_progress * 100.0))
                .animate(true));
            ui.add_space(20.0);

            // Loading status text
            ui.label(format!(
                "Loaded {} of {} assets",
                loading_state.loaded_count,
                loading_state.total_assets
            ));

            // Loading tips
            ui.add_space(40.0);
            ui.label("Tips:");
            ui.label("• Use the debug menu (F3) to access visualization options");
            ui.label("• Press ESC to open the settings menu");
            ui.label("• Hold right-click to rotate the camera");
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::app::App;
    use bevy_egui::EguiPlugin;

    #[test]
    fn test_loading_screen_initialization() {
        let mut app = App::new();
        app.add_plugins(EguiPlugin)
            .init_resource::<AssetLoadingState>();

        let mut loading_state = app.world.resource_mut::<AssetLoadingState>();
        loading_state.total_assets = 10;
        loading_state.loaded_count = 5;
        loading_state.visualization_loading_progress = 0.5;
        loading_state.metrics_loading_progress = 0.7;

        assert_eq!(loading_state.total_assets, 10);
        assert_eq!(loading_state.loaded_count, 5);
        assert_eq!(loading_state.visualization_loading_progress, 0.5);
        assert_eq!(loading_state.metrics_loading_progress, 0.7);
    }
}