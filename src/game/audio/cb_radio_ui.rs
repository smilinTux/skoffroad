use bevy::prelude::*;
use super::cb_radio::{CBRadio, CBRadioConfig};

/// Component marking the CB radio UI root entity
#[derive(Component)]
pub struct CBRadioUI;

/// Component for the channel display
#[derive(Component)]
pub struct ChannelDisplay;

/// Component for the signal strength indicator
#[derive(Component)]
pub struct SignalIndicator;

/// Component for volume control
#[derive(Component)]
pub struct VolumeControl;

/// Component for the power button
#[derive(Component)]
pub struct PowerButton;

/// Plugin for CB radio UI
pub struct CBRadioUIPlugin;

impl Plugin for CBRadioUIPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_cb_radio_ui)
           .add_systems(Update, (
               update_channel_display,
               update_signal_indicator,
               update_volume_display,
               handle_radio_input,
           ));
    }
}

/// Colors for UI elements
const PANEL_COLOR: Color = Color::rgb(0.13, 0.13, 0.13);
const TEXT_COLOR: Color = Color::rgb(0.9, 0.9, 0.9);
const ACTIVE_COLOR: Color = Color::rgb(0.3, 0.8, 0.3);
const INACTIVE_COLOR: Color = Color::rgb(0.3, 0.3, 0.3);

/// System to set up the CB radio UI
fn setup_cb_radio_ui(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    let font = asset_server.load("fonts/FiraSans-Bold.ttf");

    // Main radio panel
    commands.spawn((
        CBRadioUI,
        NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                right: Val::Px(20.0),
                bottom: Val::Px(20.0),
                width: Val::Px(200.0),
                height: Val::Px(120.0),
                padding: UiRect::all(Val::Px(10.0)),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                ..default()
            },
            background_color: PANEL_COLOR.into(),
            ..default()
        },
    )).with_children(|parent| {
        // Channel display
        parent.spawn((
            ChannelDisplay,
            TextBundle::from_section(
                "CH 19",
                TextStyle {
                    font: font.clone(),
                    font_size: 24.0,
                    color: TEXT_COLOR,
                },
            ),
        ));

        // Signal strength indicator
        parent.spawn((
            SignalIndicator,
            TextBundle::from_section(
                "Signal: ▁▂▃▄▅",
                TextStyle {
                    font: font.clone(),
                    font_size: 16.0,
                    color: ACTIVE_COLOR,
                },
            ),
        ));

        // Volume control
        parent.spawn((
            VolumeControl,
            TextBundle::from_section(
                "Vol: ▂▃▄▅▆",
                TextStyle {
                    font: font.clone(),
                    font_size: 16.0,
                    color: TEXT_COLOR,
                },
            ),
        ));

        // Power button
        parent.spawn((
            PowerButton,
            ButtonBundle {
                style: Style {
                    width: Val::Px(80.0),
                    height: Val::Px(30.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                background_color: INACTIVE_COLOR.into(),
                ..default()
            },
        )).with_children(|parent| {
            parent.spawn(TextBundle::from_section(
                "POWER",
                TextStyle {
                    font: font.clone(),
                    font_size: 16.0,
                    color: TEXT_COLOR,
                },
            ));
        });
    });
}

/// System to update the channel display
fn update_channel_display(
    radios: Query<&CBRadio>,
    mut displays: Query<&mut Text, With<ChannelDisplay>>,
) {
    // For now, just use the first radio
    if let Some(radio) = radios.iter().next() {
        for mut text in displays.iter_mut() {
            text.sections[0].value = format!("CH {:02}", radio.channel);
        }
    }
}

/// System to update the signal strength indicator
fn update_signal_indicator(
    radios: Query<&CBRadio>,
    mut indicators: Query<&mut Text, With<SignalIndicator>>,
) {
    if let Some(radio) = radios.iter().next() {
        for mut text in indicators.iter_mut() {
            let bars = (radio.signal_strength * 5.0).round() as usize;
            let indicator = "▁▂▃▄▅"[..bars.min(5)].to_string();
            text.sections[0].value = format!("Signal: {}", indicator);
            text.sections[0].style.color = if radio.powered {
                ACTIVE_COLOR
            } else {
                INACTIVE_COLOR
            };
        }
    }
}

/// System to update the volume display
fn update_volume_display(
    radios: Query<&CBRadio>,
    mut displays: Query<&mut Text, With<VolumeControl>>,
) {
    if let Some(radio) = radios.iter().next() {
        for mut text in displays.iter_mut() {
            let bars = (radio.volume * 5.0).round() as usize;
            let indicator = "▂▃▄▅▆"[..bars.min(5)].to_string();
            text.sections[0].value = format!("Vol: {}", indicator);
            text.sections[0].style.color = if radio.powered {
                TEXT_COLOR
            } else {
                INACTIVE_COLOR
            };
        }
    }
}

/// System to handle radio input
fn handle_radio_input(
    mut radios: Query<&mut CBRadio>,
    mut power_buttons: Query<(&Interaction, &mut BackgroundColor), (Changed<Interaction>, With<PowerButton>)>,
    keyboard: Res<Input<KeyCode>>,
    time: Res<Time>,
) {
    if let Some(mut radio) = radios.iter_mut().next() {
        // Handle power button clicks
        for (interaction, mut color) in power_buttons.iter_mut() {
            match *interaction {
                Interaction::Pressed => {
                    radio.powered = !radio.powered;
                    *color = if radio.powered {
                        ACTIVE_COLOR
                    } else {
                        INACTIVE_COLOR
                    }.into();
                }
                Interaction::Hovered => {
                    *color = Color::rgb(0.4, 0.4, 0.4).into();
                }
                Interaction::None => {
                    *color = if radio.powered {
                        ACTIVE_COLOR
                    } else {
                        INACTIVE_COLOR
                    }.into();
                }
            }
        }

        // Handle keyboard input
        if radio.powered {
            // Channel controls
            if keyboard.just_pressed(KeyCode::Up) {
                radio.channel = (radio.channel % 40 + 1).max(1);
            }
            if keyboard.just_pressed(KeyCode::Down) {
                radio.channel = if radio.channel <= 1 { 40 } else { radio.channel - 1 };
            }

            // Volume controls
            if keyboard.just_pressed(KeyCode::Right) {
                radio.volume = (radio.volume + 0.1).min(1.0);
            }
            if keyboard.just_pressed(KeyCode::Left) {
                radio.volume = (radio.volume - 0.1).max(0.0);
            }

            // Push-to-talk
            radio.transmitting = keyboard.pressed(KeyCode::T);

            // Emergency channel monitoring
            if keyboard.just_pressed(KeyCode::Key9) {
                radio.monitor_emergency = !radio.monitor_emergency;
            }

            // Squelch toggle
            if keyboard.just_pressed(KeyCode::S) {
                radio.squelch_enabled = !radio.squelch_enabled;
            }
        }
    }
} 