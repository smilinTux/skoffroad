use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};

use crate::game::audio::cb_radio::{CBRadioChannel, CBRadioState, CBRadioVolume};

pub struct CBRadioUIPlugin;

impl Plugin for CBRadioUIPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, cb_radio_ui_system);
    }
}

fn cb_radio_ui_system(
    mut contexts: EguiContexts,
    mut radio_state: ResMut<CBRadioState>,
    mut radio_volume: ResMut<CBRadioVolume>,
) {
    egui::Window::new("CB Radio")
        .resizable(false)
        .show(contexts.ctx_mut(), |ui| {
            // Channel selector
            ui.horizontal(|ui| {
                ui.label("Channel:");
                egui::ComboBox::from_id_source("channel_selector")
                    .selected_text(format!("Channel {}", radio_state.current_channel.0))
                    .show_ui(ui, |ui| {
                        for channel in 1..=40 {
                            if ui.selectable_label(
                                radio_state.current_channel.0 == channel,
                                format!("Channel {}", channel),
                            ).clicked() {
                                radio_state.current_channel = CBRadioChannel(channel);
                            }
                        }
                    });
            });

            // Volume slider
            ui.horizontal(|ui| {
                ui.label("Volume:");
                ui.add(egui::Slider::new(&mut radio_volume.0, 0.0..=1.0));
            });

            // Status indicators
            ui.horizontal(|ui| {
                ui.label("Status:");
                let status_color = if radio_state.is_transmitting {
                    egui::Color32::RED
                } else if radio_state.is_receiving {
                    egui::Color32::GREEN
                } else {
                    egui::Color32::GRAY
                };
                ui.colored_label(status_color, if radio_state.is_transmitting {
                    "Transmitting"
                } else if radio_state.is_receiving {
                    "Receiving"
                } else {
                    "Standby"
                });
            });

            // Signal strength indicator
            ui.horizontal(|ui| {
                ui.label("Signal:");
                let signal_strength = radio_state.signal_strength;
                let bars = (signal_strength * 5.0).round() as i32;
                let mut bar_text = String::new();
                for i in 0..5 {
                    bar_text.push(if i < bars { '█' } else { '░' });
                }
                ui.label(bar_text);
            });
        });
} 