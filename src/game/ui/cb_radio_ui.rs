use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use super::super::audio::cb_radio::{CBRadio, CBRadioState, CBRadioVolume, CBRadioChannel, EMERGENCY_CHANNEL, TRUCKER_CHANNEL};

/// Component for CB radio UI
#[derive(Component)]
pub struct CBRadioUI {
    channel_input: Entity,
    volume_slider: Entity,
    signal_meter: Entity,
    squelch_toggle: Entity,
    emergency_toggle: Entity,
}

/// Plugin for CB radio UI
pub struct CBRadioUIPlugin;

impl Plugin for CBRadioUIPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_cb_radio_ui)
           .add_systems(Update, (
               update_cb_radio_ui,
               handle_channel_input,
               handle_volume_slider,
               handle_squelch_toggle,
               handle_emergency_toggle,
               draw_cb_radio_ui,
           ).chain());
    }
}

/// Set up the CB radio UI
fn setup_cb_radio_ui(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font = asset_server.load("fonts/FiraSans-Bold.ttf");

    // Main container
    let container = commands.spawn(NodeBundle {
        style: Style {
            position_type: PositionType::Absolute,
            right: Val::Px(20.0),
            top: Val::Px(20.0),
            flex_direction: FlexDirection::Column,
            padding: UiRect::all(Val::Px(10.0)),
            gap: Val::Px(10.0),
            ..default()
        },
        background_color: Color::rgba(0.1, 0.1, 0.1, 0.8).into(),
        ..default()
    }).id();

    // Channel selector
    let channel_input = commands.spawn(TextBundle {
        text: Text::from_section(
            "Channel: 19",
            TextStyle {
                font: font.clone(),
                font_size: 20.0,
                color: Color::WHITE,
            },
        ),
        ..default()
    }).id();

    // Volume slider
    let volume_slider = commands.spawn(NodeBundle {
        style: Style {
            width: Val::Px(150.0),
            height: Val::Px(20.0),
            ..default()
        },
        background_color: Color::GRAY.into(),
        ..default()
    }).with_children(|parent| {
        parent.spawn(NodeBundle {
            style: Style {
                width: Val::Percent(80.0),
                height: Val::Percent(100.0),
                ..default()
            },
            background_color: Color::GREEN.into(),
            ..default()
        });
    }).id();

    // Signal strength meter
    let signal_meter = commands.spawn(NodeBundle {
        style: Style {
            width: Val::Px(150.0),
            height: Val::Px(20.0),
            ..default()
        },
        background_color: Color::GRAY.into(),
        ..default()
    }).with_children(|parent| {
        parent.spawn(NodeBundle {
            style: Style {
                width: Val::Percent(0.0),
                height: Val::Percent(100.0),
                ..default()
            },
            background_color: Color::YELLOW.into(),
            ..default()
        });
    }).id();

    // Squelch toggle
    let squelch_toggle = commands.spawn(ButtonBundle {
        style: Style {
            padding: UiRect::all(Val::Px(5.0)),
            ..default()
        },
        background_color: Color::DARK_GREEN.into(),
        ..default()
    }).with_children(|parent| {
        parent.spawn(TextBundle::from_section(
            "Squelch: ON",
            TextStyle {
                font: font.clone(),
                font_size: 16.0,
                color: Color::WHITE,
            },
        ));
    }).id();

    // Emergency channel toggle
    let emergency_toggle = commands.spawn(ButtonBundle {
        style: Style {
            padding: UiRect::all(Val::Px(5.0)),
            ..default()
        },
        background_color: Color::DARK_RED.into(),
        ..default()
    }).with_children(|parent| {
        parent.spawn(TextBundle::from_section(
            "Monitor Emergency: OFF",
            TextStyle {
                font: font.clone(),
                font_size: 16.0,
                color: Color::WHITE,
            },
        ));
    }).id();

    // Add all elements to container
    commands.entity(container).push_children(&[
        channel_input,
        volume_slider,
        signal_meter,
        squelch_toggle,
        emergency_toggle,
    ]);

    // Create UI component
    commands.spawn(CBRadioUI {
        channel_input,
        volume_slider,
        signal_meter,
        squelch_toggle,
        emergency_toggle,
    });
}

/// Update the CB radio UI based on radio state
fn update_cb_radio_ui(
    mut ui_query: Query<&CBRadioUI>,
    radio_query: Query<(&CBRadio, &SignalQuality)>,
    mut text_query: Query<&mut Text>,
    mut style_query: Query<&mut Style>,
    mut bg_color_query: Query<&mut BackgroundColor>,
) {
    if let Ok(ui) = ui_query.get_single() {
        if let Ok((radio, signal_quality)) = radio_query.get_single() {
            // Update channel text
            if let Ok(mut text) = text_query.get_mut(ui.channel_input) {
                text.sections[0].value = format!("Channel: {}", radio.channel);
            }

            // Update volume slider
            if let Ok(mut style) = style_query.get_mut(ui.volume_slider) {
                style.width = Val::Percent(radio.volume * 100.0);
            }

            // Update signal meter
            if let Ok(mut style) = style_query.get_mut(ui.signal_meter) {
                style.width = Val::Percent(signal_quality.strength * 100.0);
            }
            if let Ok(mut color) = bg_color_query.get_mut(ui.signal_meter) {
                color.0 = if signal_quality.strength > 0.7 {
                    Color::GREEN
                } else if signal_quality.strength > 0.3 {
                    Color::YELLOW
                } else {
                    Color::RED
                };
            }

            // Update squelch button
            if let Ok(mut text) = text_query.get_mut(ui.squelch_toggle) {
                text.sections[0].value = format!("Squelch: {}", if radio.squelch_enabled { "ON" } else { "OFF" });
            }
            if let Ok(mut color) = bg_color_query.get_mut(ui.squelch_toggle) {
                color.0 = if radio.squelch_enabled {
                    Color::DARK_GREEN
                } else {
                    Color::DARK_GRAY
                }.into();
            }

            // Update emergency monitor button
            if let Ok(mut text) = text_query.get_mut(ui.emergency_toggle) {
                text.sections[0].value = format!("Monitor Emergency: {}", if radio.monitor_emergency { "ON" } else { "OFF" });
            }
            if let Ok(mut color) = bg_color_query.get_mut(ui.emergency_toggle) {
                color.0 = if radio.monitor_emergency {
                    Color::RED
                } else {
                    Color::DARK_RED
                }.into();
            }
        }
    }
}

/// Handle channel input changes
fn handle_channel_input(
    mut radio_query: Query<&mut CBRadio>,
    keyboard_input: Res<Input<KeyCode>>,
) {
    if let Ok(mut radio) = radio_query.get_single_mut() {
        // Channel up/down with arrow keys
        if keyboard_input.just_pressed(KeyCode::Up) {
            radio.channel = (radio.channel % 40 + 1).clamp(1, 40);
        }
        if keyboard_input.just_pressed(KeyCode::Down) {
            radio.channel = if radio.channel > 1 { radio.channel - 1 } else { 40 };
        }
    }
}

/// Handle volume slider interaction
fn handle_volume_slider(
    mut radio_query: Query<&mut CBRadio>,
    mut interaction_query: Query<(&Interaction, &mut BackgroundColor), Changed<Interaction>>,
) {
    if let Ok(mut radio) = radio_query.get_single_mut() {
        for (interaction, mut color) in interaction_query.iter_mut() {
            match *interaction {
                Interaction::Pressed => {
                    // TODO: Implement volume adjustment based on click position
                    color.0 = Color::rgb(0.3, 0.3, 0.3);
                }
                Interaction::Hovered => {
                    color.0 = Color::rgb(0.2, 0.2, 0.2);
                }
                Interaction::None => {
                    color.0 = Color::rgb(0.1, 0.1, 0.1);
                }
            }
        }
    }
}

/// Handle squelch toggle button
fn handle_squelch_toggle(
    mut radio_query: Query<&mut CBRadio>,
    mut interaction_query: Query<(&Interaction, &mut BackgroundColor), (Changed<Interaction>, With<Button>)>,
) {
    if let Ok(mut radio) = radio_query.get_single_mut() {
        for (interaction, mut color) in interaction_query.iter_mut() {
            if let Interaction::Pressed = *interaction {
                radio.squelch_enabled = !radio.squelch_enabled;
                color.0 = if radio.squelch_enabled {
                    Color::DARK_GREEN
                } else {
                    Color::DARK_GRAY
                }.into();
            }
        }
    }
}

/// Handle emergency channel monitoring toggle
fn handle_emergency_toggle(
    mut radio_query: Query<&mut CBRadio>,
    mut interaction_query: Query<(&Interaction, &mut BackgroundColor), (Changed<Interaction>, With<Button>)>,
) {
    if let Ok(mut radio) = radio_query.get_single_mut() {
        for (interaction, mut color) in interaction_query.iter_mut() {
            if let Interaction::Pressed = *interaction {
                radio.monitor_emergency = !radio.monitor_emergency;
                color.0 = if radio.monitor_emergency {
                    Color::RED
                } else {
                    Color::DARK_RED
                }.into();
            }
        }
    }
}

fn draw_cb_radio_ui(
    mut contexts: EguiContexts,
    mut radio_state: ResMut<CBRadioState>,
    mut radio_volume: ResMut<CBRadioVolume>,
    radio_query: Query<&CBRadio>,
) {
    egui::Window::new("CB Radio")
        .resizable(false)
        .show(contexts.ctx_mut(), |ui| {
            ui.horizontal(|ui| {
                // Channel selection
                ui.label("Channel:");
                if ui.button("◀").clicked() {
                    radio_state.channel = radio_state.channel.prev();
                }
                ui.label(format!("{:02}", radio_state.channel.0));
                if ui.button("▶").clicked() {
                    radio_state.channel = radio_state.channel.next();
                }

                // Quick access buttons
                ui.separator();
                if ui.button("🚨 CH9").clicked() {
                    radio_state.channel = CBRadioChannel(EMERGENCY_CHANNEL);
                }
                if ui.button("🚛 CH19").clicked() {
                    radio_state.channel = CBRadioChannel(TRUCKER_CHANNEL);
                }
            });

            ui.separator();

            // Volume control
            ui.horizontal(|ui| {
                ui.label("Volume:");
                ui.add(egui::Slider::new(&mut radio_volume.0, 0.0..=1.0));
            });

            // Signal strength indicator
            ui.horizontal(|ui| {
                ui.label("Signal:");
                let signal_rect = ui.available_rect_before_wrap();
                let signal_strength = radio_state.signal_strength;
                let (rect, _) = ui.allocate_exact_size(signal_rect.size(), egui::Sense::hover());
                
                // Draw signal bars
                let bar_count = 5;
                let bar_width = rect.width() / (bar_count as f32 * 2.0);
                for i in 0..bar_count {
                    let threshold = (i as f32 + 1.0) / (bar_count as f32);
                    let color = if signal_strength >= threshold {
                        egui::Color32::GREEN
                    } else {
                        egui::Color32::from_gray(64)
                    };
                    
                    let height = rect.height() * ((i as f32 + 1.0) / bar_count as f32);
                    let bar_rect = egui::Rect::from_min_size(
                        egui::pos2(
                            rect.min.x + (i as f32 * bar_width * 2.0),
                            rect.max.y - height
                        ),
                        egui::vec2(bar_width, height)
                    );
                    ui.painter().rect_filled(bar_rect, 0.0, color);
                }
            });

            // Transmission status
            ui.horizontal(|ui| {
                ui.label("Status:");
                let status_text = if radio_state.is_transmitting {
                    "📤 Transmitting"
                } else if radio_state.is_receiving {
                    "📥 Receiving"
                } else {
                    "⚪ Standby"
                };
                ui.label(status_text);
            });

            // Push-to-talk button
            let ptt_response = ui.add_sized(
                [ui.available_width(), 40.0],
                egui::Button::new(
                    if radio_state.is_transmitting {
                        "🎙️ Transmitting..."
                    } else {
                        "🎙️ Push to Talk"
                    }
                ).fill(if radio_state.is_transmitting {
                    egui::Color32::from_rgb(200, 50, 50)
                } else {
                    egui::Color32::from_rgb(50, 150, 50)
                })
            );

            if ptt_response.is_pointer_button_down_on() {
                radio_state.is_transmitting = true;
            } else {
                radio_state.is_transmitting = false;
            }

            // Display squelch status if available
            if let Ok(radio) = radio_query.get_single() {
                ui.horizontal(|ui| {
                    ui.label("Squelch:");
                    let squelch_text = if radio.squelch_enabled {
                        "✅ Enabled"
                    } else {
                        "❌ Disabled"
                    };
                    ui.label(squelch_text);
                });
            }
        });
} 