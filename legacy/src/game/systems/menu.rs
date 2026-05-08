use bevy::prelude::*;
use bevy::app::AppExit;
use bevy::window::PrimaryWindow;
use std::time::Duration;

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
pub enum MenuState {
    #[default]
    Main,
    Settings,
    Garage,
    VehicleCustomization,
    Race,
    RaceSetup,
    Paused,
    Credits,
    Tutorial,
    Profile,
}

#[derive(Resource)]
pub struct GameSettings {
    pub graphics: GraphicsSettings,
    pub audio: AudioSettings,
    pub controls: ControlSettings,
}

#[derive(Resource)]
pub struct GraphicsSettings {
    pub resolution: (u32, u32),
    pub fullscreen: bool,
    pub vsync: bool,
    pub shadow_quality: ShadowQuality,
    pub particle_quality: ParticleQuality,
    pub texture_quality: TextureQuality,
    pub antialiasing: AntiAliasing,
    pub view_distance: f32,
    pub foliage_density: f32,
    pub motion_blur: bool,
    pub ambient_occlusion: bool,
}

#[derive(Resource)]
pub struct AudioSettings {
    pub master_volume: f32,
    pub music_volume: f32,
    pub sfx_volume: f32,
}

#[derive(Resource)]
pub struct ControlSettings {
    pub mouse_sensitivity: f32,
    pub invert_y: bool,
    pub controller_vibration: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShadowQuality {
    Low,
    Medium,
    High,
    Ultra,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParticleQuality {
    Low,
    Medium,
    High,
    Ultra,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextureQuality {
    Low,
    Medium,
    High,
    Ultra,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AntiAliasing {
    None,
    FXAA,
    MSAA2x,
    MSAA4x,
    MSAA8x,
}

#[derive(Component)]
pub struct MenuButton;

#[derive(Component)]
pub struct SelectedOption;

#[derive(Component)]
pub struct MenuPanel;

#[derive(Component)]
pub struct MenuTransition {
    pub timer: Timer,
    pub transition_type: TransitionType,
    pub initial_style: Option<Style>,
    pub initial_color: Option<Color>,
    pub target_style: Option<Style>,
    pub target_color: Option<Color>,
}

#[derive(Debug, Clone, Copy)]
pub enum TransitionType {
    FadeIn,
    FadeOut,
    SlideIn(Direction),
    SlideOut(Direction),
    ScaleIn,
    ScaleOut,
}

#[derive(Clone, Copy)]
pub enum Direction {
    Left,
    Right,
    Up,
    Down,
}

#[derive(Resource)]
pub struct VehicleSelection {
    pub available_vehicles: Vec<Vehicle>,
    pub selected_vehicle: usize,
}

#[derive(Clone)]
pub struct Vehicle {
    pub name: String,
    pub description: String,
    pub stats: VehicleStats,
    pub preview_image: String,
    pub price: u32,
    pub unlocked: bool,
    pub customization: VehicleCustomizationOptions,
}

#[derive(Clone)]
pub struct VehicleStats {
    pub speed: f32,
    pub acceleration: f32,
    pub handling: f32,
    pub offroad: f32,
}

#[derive(Clone)]
pub struct VehicleCustomizationOptions {
    pub colors: Vec<Color>,
    pub selected_color: usize,
    pub upgrades: VehicleUpgrades,
    pub decals: Vec<String>,
    pub selected_decal: Option<usize>,
}

#[derive(Clone)]
pub struct VehicleUpgrades {
    pub engine: UpgradeLevel,
    pub suspension: UpgradeLevel,
    pub tires: UpgradeLevel,
    pub armor: UpgradeLevel,
}

#[derive(Clone, Copy)]
pub enum UpgradeLevel {
    Stock,
    Level1,
    Level2,
    Level3,
}

#[derive(Resource)]
pub struct RaceSetup {
    pub track: Track,
    pub time_of_day: TimeOfDay,
    pub weather: Weather,
    pub difficulty: Difficulty,
    pub laps: u32,
    pub opponents: u32,
}

#[derive(Debug, Clone)]
pub struct Track {
    pub name: String,
    pub description: String,
    pub preview_image: String,
    pub difficulty_rating: u32,
    pub length_km: f32,
    pub weather_options: Vec<Weather>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeOfDay {
    Dawn,
    Morning,
    Noon,
    Afternoon,
    Sunset,
    Night,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Weather {
    Clear,
    Cloudy,
    Rain,
    Storm,
    Fog,
    Snow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Difficulty {
    Easy,
    Medium,
    Hard,
    Expert,
}

pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_state::<MenuState>()
            .init_resource::<GameSettings>()
            .init_resource::<RaceSetup>()
            .add_systems(Startup, setup_menu)
            .add_systems(Update, (
                update_menu_transitions,
                setup_menu_transitions,
                animate_button_hover,
                button_interaction_system,
                handle_menu_navigation.run_if(in_state(MenuState::Main)),
                handle_settings_menu.run_if(in_state(MenuState::Settings)),
                handle_garage_menu.run_if(in_state(MenuState::Garage)),
                handle_vehicle_customization.run_if(in_state(MenuState::VehicleCustomization)),
                handle_race_setup.run_if(in_state(MenuState::RaceSetup)),
                handle_race_menu.run_if(in_state(MenuState::Race)),
                handle_pause_menu.run_if(in_state(MenuState::Paused)),
                handle_credits_menu.run_if(in_state(MenuState::Credits)),
                handle_tutorial_menu.run_if(in_state(MenuState::Tutorial)),
                handle_profile_menu.run_if(in_state(MenuState::Profile)),
            ));
    }
}

impl Default for GameSettings {
    fn default() -> Self {
        Self {
            graphics: GraphicsSettings {
                resolution: (1920, 1080),
                fullscreen: false,
                vsync: true,
                shadow_quality: ShadowQuality::High,
                particle_quality: ParticleQuality::High,
                texture_quality: TextureQuality::High,
                antialiasing: AntiAliasing::FXAA,
                view_distance: 1000.0,
                foliage_density: 0.5,
                motion_blur: true,
                ambient_occlusion: true,
            },
            audio: AudioSettings {
                master_volume: 0.8,
                music_volume: 0.7,
                sfx_volume: 0.9,
            },
            controls: ControlSettings {
                mouse_sensitivity: 1.0,
                invert_y: false,
                controller_vibration: true,
            },
        }
    }
}

impl Default for VehicleSelection {
    fn default() -> Self {
        Self {
            available_vehicles: vec![
                Vehicle {
                    name: "Jeep TJ".to_string(),
                    description: "Classic off-road vehicle with excellent handling and reliability.".to_string(),
                    stats: VehicleStats {
                        speed: 0.7,
                        acceleration: 0.6,
                        handling: 0.8,
                        offroad: 0.9,
                    },
                    preview_image: "vehicles/jeep_tj.png".to_string(),
                    price: 0,
                    unlocked: true,
                    customization: VehicleCustomizationOptions::default(),
                },
                Vehicle {
                    name: "Land Rover Defender".to_string(),
                    description: "Rugged and powerful, perfect for challenging terrain.".to_string(),
                    stats: VehicleStats {
                        speed: 0.6,
                        acceleration: 0.5,
                        handling: 0.7,
                        offroad: 1.0,
                    },
                    preview_image: "vehicles/defender.png".to_string(),
                    price: 25000,
                    unlocked: false,
                    customization: VehicleCustomizationOptions::default(),
                },
                Vehicle {
                    name: "Ford Raptor".to_string(),
                    description: "High-performance pickup with exceptional speed and power.".to_string(),
                    stats: VehicleStats {
                        speed: 0.9,
                        acceleration: 0.8,
                        handling: 0.7,
                        offroad: 0.8,
                    },
                    preview_image: "vehicles/raptor.png".to_string(),
                    price: 35000,
                    unlocked: false,
                    customization: VehicleCustomizationOptions::default(),
                },
                Vehicle {
                    name: "Toyota 4Runner".to_string(),
                    description: "Reliable SUV with balanced performance and comfort.".to_string(),
                    stats: VehicleStats {
                        speed: 0.75,
                        acceleration: 0.7,
                        handling: 0.75,
                        offroad: 0.8,
                    },
                    preview_image: "vehicles/4runner.png".to_string(),
                    price: 28000,
                    unlocked: false,
                    customization: VehicleCustomizationOptions::default(),
                },
                Vehicle {
                    name: "Chevrolet Silverado ZR2".to_string(),
                    description: "Heavy-duty pickup with impressive towing capacity and off-road capability.".to_string(),
                    stats: VehicleStats {
                        speed: 0.8,
                        acceleration: 0.7,
                        handling: 0.6,
                        offroad: 0.85,
                    },
                    preview_image: "vehicles/silverado.png".to_string(),
                    price: 40000,
                    unlocked: false,
                    customization: VehicleCustomizationOptions::default(),
                },
                Vehicle {
                    name: "Mercedes G-Wagon".to_string(),
                    description: "Luxury off-road vehicle with premium features and strong performance.".to_string(),
                    stats: VehicleStats {
                        speed: 0.85,
                        acceleration: 0.8,
                        handling: 0.75,
                        offroad: 0.85,
                    },
                    preview_image: "vehicles/g-wagon.png".to_string(),
                    price: 50000,
                    unlocked: false,
                    customization: VehicleCustomizationOptions::default(),
                },
            ],
            selected_vehicle: 0,
        }
    }
}

fn setup_menu(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font = asset_server.load("fonts/FiraSans-Bold.ttf");
    
    // Root node with background
    commands
        .spawn((
            NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                background_color: Color::rgba(0.1, 0.1, 0.1, 0.9).into(),
                ..default()
            },
            MenuPanel,
        ))
        .with_children(|parent| {
            // Title with animation
            parent.spawn((
                TextBundle::from_section(
                    "SandK Offroad",
                    TextStyle {
                        font: font.clone(),
                        font_size: 80.0,
                        color: Color::WHITE,
                    }
                ).with_style(Style {
                    margin: UiRect::all(Val::Px(20.0)),
                    ..default()
                }),
                MenuTransition {
                    timer: Timer::from_seconds(0.5, TimerMode::Once),
                    transition_type: TransitionType::FadeIn,
                    initial_style: None,
                    initial_color: None,
                    target_style: None,
                    target_color: None,
                },
            ));

            // Main menu buttons with hover effects
            let button_data = [
                ("Play", MenuState::RaceSetup),
                ("Garage", MenuState::Garage),
                ("Settings", MenuState::Settings),
                ("Profile", MenuState::Profile),
                ("Tutorial", MenuState::Tutorial),
                ("Credits", MenuState::Credits),
                ("Quit", MenuState::Main),
            ];

            for (idx, (text, state)) in button_data.iter().enumerate() {
                let delay = idx as f32 * 0.1;
                parent.spawn((
                    ButtonBundle {
                        style: Style {
                            width: Val::Px(300.0),
                            height: Val::Px(65.0),
                            margin: UiRect::all(Val::Px(10.0)),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        background_color: Color::rgb(0.15, 0.15, 0.15).into(),
                        ..default()
                    },
                    MenuButton,
                    MenuTransition {
                        timer: Timer::from_seconds(0.3 + delay, TimerMode::Once),
                        transition_type: TransitionType::SlideIn(Direction::Left),
                        initial_style: None,
                        initial_color: None,
                        target_style: None,
                        target_color: None,
                    },
                ))
                .with_children(|parent| {
                    parent.spawn(TextBundle::from_section(
                        *text,
                        TextStyle {
                            font: font.clone(),
                            font_size: 40.0,
                            color: Color::rgb(0.9, 0.9, 0.9),
                        },
                    ));
                });
            }
        });
}

fn update_menu_transitions(
    time: Res<Time>,
    mut commands: Commands,
    mut query: Query<(Entity, &mut MenuTransition, &mut Style, &mut BackgroundColor)>,
) {
    for (entity, mut transition, mut style, mut bg_color) in query.iter_mut() {
        transition.timer.tick(time.delta());
        
        let progress = transition.timer.percent();
        
        match transition.transition_type {
            TransitionType::FadeIn => {
                if let Some(target_color) = transition.target_color {
                    bg_color.0.set_a(progress * target_color.a());
                }
            }
            TransitionType::FadeOut => {
                if let Some(initial_color) = transition.initial_color {
                    bg_color.0.set_a((1.0 - progress) * initial_color.a());
                }
            }
            TransitionType::SlideIn(direction) => {
                if let Some(target_style) = transition.target_style {
                    match direction {
                        Direction::Left => {
                            style.left = Val::Px(lerp(-100.0, 0.0, progress));
                        }
                        Direction::Right => {
                            style.right = Val::Px(lerp(-100.0, 0.0, progress));
                        }
                        Direction::Up => {
                            style.top = Val::Px(lerp(-100.0, 0.0, progress));
                        }
                        Direction::Down => {
                            style.bottom = Val::Px(lerp(-100.0, 0.0, progress));
                        }
                    }
                }
            }
            TransitionType::SlideOut(direction) => {
                if let Some(initial_style) = transition.initial_style {
                    match direction {
                        Direction::Left => {
                            style.left = Val::Px(lerp(0.0, 100.0, progress));
                        }
                        Direction::Right => {
                            style.right = Val::Px(lerp(0.0, 100.0, progress));
                        }
                        Direction::Up => {
                            style.top = Val::Px(lerp(0.0, 100.0, progress));
                        }
                        Direction::Down => {
                            style.bottom = Val::Px(lerp(0.0, 100.0, progress));
                        }
                    }
                }
            }
            TransitionType::ScaleIn => {
                style.scale = Vec2::splat(progress).into();
            }
            TransitionType::ScaleOut => {
                style.scale = Vec2::splat(1.0 - progress).into();
            }
        }
        
        if transition.timer.finished() {
            commands.entity(entity).remove::<MenuTransition>();
        }
    }
}

fn lerp(start: f32, end: f32, t: f32) -> f32 {
    start + (end - start) * t
}

fn setup_menu_transitions(mut commands: Commands, query: Query<Entity, Added<MenuPanel>>) {
    for entity in query.iter() {
        commands.entity(entity).insert(MenuTransition {
            timer: Timer::from_seconds(0.5, TimerMode::Once),
            transition_type: TransitionType::FadeIn,
            initial_color: Some(Color::NONE),
            target_color: Some(Color::rgba(0.1, 0.1, 0.1, 0.95)),
            ..default()
        });
    }
}

fn animate_button_hover(
    mut query: Query<(&Interaction, &mut BackgroundColor), (Changed<Interaction>, With<MenuButton>)>,
) {
    for (interaction, mut bg_color) in query.iter_mut() {
        match *interaction {
            Interaction::Hovered => {
                bg_color.0 = bg_color.0.with_lightness(bg_color.0.l() + 0.1);
            }
            Interaction::None => {
                bg_color.0 = bg_color.0.with_lightness(bg_color.0.l() - 0.1);
            }
            _ => {}
        }
    }
}

fn button_interaction_system(
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor, &Children),
        (Changed<Interaction>, With<MenuButton>),
    >,
    mut text_query: Query<&mut Text>,
) {
    for (interaction, mut color, children) in &mut interaction_query {
        let mut text = text_query.get_mut(children[0]).unwrap();
        match *interaction {
            Interaction::Pressed => {
                *color = Color::rgb(0.35, 0.35, 0.35).into();
                text.sections[0].style.color = Color::WHITE;
            }
            Interaction::Hovered => {
                *color = Color::rgb(0.25, 0.25, 0.25).into();
                text.sections[0].style.color = Color::WHITE;
            }
            Interaction::None => {
                *color = Color::rgb(0.15, 0.15, 0.15).into();
                text.sections[0].style.color = Color::rgb(0.9, 0.9, 0.9);
            }
        }
    }
}

fn handle_menu_navigation(
    mut next_state: ResMut<NextState<MenuState>>,
    interaction_query: Query<(&Interaction, &Children), (Changed<Interaction>, With<MenuButton>)>,
    text_query: Query<&Text>,
) {
    for (interaction, children) in &interaction_query {
        if *interaction == Interaction::Pressed {
            let text = text_query.get(children[0]).unwrap();
            match text.sections[0].value.as_str() {
                "Play" => next_state.set(MenuState::RaceSetup),
                "Garage" => next_state.set(MenuState::Garage),
                "Settings" => next_state.set(MenuState::Settings),
                "Profile" => next_state.set(MenuState::Profile),
                "Tutorial" => next_state.set(MenuState::Tutorial),
                "Credits" => next_state.set(MenuState::Credits),
                "Quit" => std::process::exit(0),
                _ => {}
            }
        }
    }
}

fn handle_settings_menu(
    mut commands: Commands,
    keyboard: Res<Input<KeyCode>>,
    mut next_state: ResMut<NextState<MenuState>>,
) {
    if keyboard.just_pressed(KeyCode::Escape) {
        next_state.set(MenuState::Main);
    }
}

fn handle_garage_menu(
    mut commands: Commands,
    keyboard: Res<Input<KeyCode>>,
    mut next_state: ResMut<NextState<MenuState>>,
    mut vehicle_selection: ResMut<VehicleSelection>,
    interaction_query: Query<(&Interaction, &Children), (Changed<Interaction>, With<MenuButton>)>,
    text_query: Query<&Text>,
) {
    // Handle escape to return to main menu
    if keyboard.just_pressed(KeyCode::Escape) {
        next_state.set(MenuState::Main);
        return;
    }

    // Handle button interactions
    for (interaction, children) in interaction_query.iter() {
        if *interaction == Interaction::Pressed {
            if let Ok(text) = text_query.get(children[0]) {
                match text.sections[0].value.as_str() {
                    "Select" => {
                        next_state.set(MenuState::VehicleCustomization);
                    }
                    text if text.starts_with("Purchase") => {
                        // TODO: Handle vehicle purchase
                    }
                    _ => {
                        // Handle vehicle selection from the list
                        if let Some(idx) = vehicle_selection.available_vehicles
                            .iter()
                            .position(|v| v.name == text.sections[0].value) {
                            vehicle_selection.selected_vehicle = idx;
                        }
                    }
                }
            }
        }
    }
}

fn handle_race_menu(
    mut commands: Commands,
    keyboard: Res<Input<KeyCode>>,
    mut next_state: ResMut<NextState<MenuState>>,
) {
    if keyboard.just_pressed(KeyCode::Escape) {
        next_state.set(MenuState::Paused);
    }
}

fn handle_pause_menu(
    mut commands: Commands,
    keyboard: Res<Input<KeyCode>>,
    mut next_state: ResMut<NextState<MenuState>>,
) {
    if keyboard.just_pressed(KeyCode::Escape) {
        next_state.set(MenuState::Main);
    }
}

fn setup_vehicle_customization(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    vehicle_selection: Res<VehicleSelection>,
) {
    let font = asset_server.load("fonts/FiraSans-Bold.ttf");
    let vehicle = &vehicle_selection.available_vehicles[vehicle_selection.selected_vehicle];
    
    commands.spawn((
        NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Row,
                ..default()
            },
            background_color: Color::rgba(0.1, 0.1, 0.1, 0.95).into(),
            ..default()
        },
        MenuPanel,
    ))
    .with_children(|parent| {
        // Left panel - Vehicle preview
        parent.spawn(NodeBundle {
            style: Style {
                width: Val::Percent(60.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                padding: UiRect::all(Val::Px(20.0)),
                ..default()
            },
            ..default()
        })
        .with_children(|parent| {
            // Vehicle preview image
            parent.spawn((
                ImageBundle {
                    style: Style {
                        width: Val::Percent(90.0),
                        height: Val::Percent(60.0),
                        ..default()
                    },
                    image: asset_server.load(&vehicle.preview_image).into(),
                    ..default()
                },
                MenuTransition {
                    timer: Timer::from_seconds(0.3, TimerMode::Once),
                    transition_type: TransitionType::FadeIn,
                    initial_style: None,
                    initial_color: None,
                    target_style: None,
                    target_color: None,
                },
            ));
        });

        // Right panel - Customization options
        parent.spawn(NodeBundle {
            style: Style {
                width: Val::Percent(40.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(20.0)),
                ..default()
            },
            background_color: Color::rgba(0.15, 0.15, 0.15, 0.95).into(),
            ..default()
        })
        .with_children(|parent| {
            // Title
            parent.spawn(TextBundle::from_section(
                "Customize Vehicle",
                TextStyle {
                    font: font.clone(),
                    font_size: 32.0,
                    color: Color::WHITE,
                },
            ));

            // Color selection
            spawn_customization_section(parent, "Paint Color", font.clone(), |parent| {
                parent.spawn(NodeBundle {
                    style: Style {
                        width: Val::Percent(100.0),
                        height: Val::Px(50.0),
                        flex_direction: FlexDirection::Row,
                        justify_content: JustifyContent::SpaceAround,
                        align_items: AlignItems::Center,
                        margin: UiRect::vertical(Val::Px(10.0)),
                        ..default()
                    },
                    ..default()
                })
                .with_children(|parent| {
                    for (idx, color) in vehicle.customization.colors.iter().enumerate() {
                        parent.spawn((
                            ButtonBundle {
                                style: Style {
                                    width: Val::Px(40.0),
                                    height: Val::Px(40.0),
                                    border: UiRect::all(Val::Px(2.0)),
                                    ..default()
                                },
                                background_color: (*color).into(),
                                border_color: if idx == vehicle.customization.selected_color {
                                    Color::WHITE.into()
                                } else {
                                    Color::DARK_GRAY.into()
                                },
                                ..default()
                            },
                            MenuButton,
                        ));
                    }
                });
            });

            // Upgrades
            spawn_customization_section(parent, "Upgrades", font.clone(), |parent| {
                spawn_upgrade_row(parent, "Engine", vehicle.customization.upgrades.engine, font.clone());
                spawn_upgrade_row(parent, "Suspension", vehicle.customization.upgrades.suspension, font.clone());
                spawn_upgrade_row(parent, "Tires", vehicle.customization.upgrades.tires, font.clone());
                spawn_upgrade_row(parent, "Armor", vehicle.customization.upgrades.armor, font.clone());
            });

            // Decals
            spawn_customization_section(parent, "Decals", font.clone(), |parent| {
                parent.spawn(NodeBundle {
                    style: Style {
                        width: Val::Percent(100.0),
                        height: Val::Px(100.0),
                        flex_direction: FlexDirection::Row,
                        justify_content: JustifyContent::SpaceAround,
                        align_items: AlignItems::Center,
                        margin: UiRect::vertical(Val::Px(10.0)),
                        ..default()
                    },
                    ..default()
                })
                .with_children(|parent| {
                    for (idx, decal) in vehicle.customization.decals.iter().enumerate() {
                        parent.spawn((
                            ButtonBundle {
                                style: Style {
                                    width: Val::Px(80.0),
                                    height: Val::Px(80.0),
                                    border: UiRect::all(Val::Px(2.0)),
                                    ..default()
                                },
                                border_color: if Some(idx) == vehicle.customization.selected_decal {
                                    Color::WHITE.into()
                                } else {
                                    Color::DARK_GRAY.into()
                                },
                                ..default()
                            },
                            MenuButton,
                        ))
                        .with_children(|parent| {
                            parent.spawn(ImageBundle {
                                style: Style {
                                    width: Val::Percent(100.0),
                                    height: Val::Percent(100.0),
                                    ..default()
                                },
                                image: asset_server.load(decal).into(),
                                ..default()
                            });
                        });
                    }
                });
            });

            // Action buttons
            parent.spawn(NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    height: Val::Px(50.0),
                    justify_content: JustifyContent::SpaceAround,
                    align_items: AlignItems::Center,
                    margin: UiRect::top(Val::Px(20.0)),
                    ..default()
                },
                ..default()
            })
            .with_children(|parent| {
                // Save button
                parent.spawn((
                    ButtonBundle {
                        style: Style {
                            width: Val::Px(150.0),
                            height: Val::Percent(100.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        background_color: Color::rgb(0.3, 0.8, 0.3).into(),
                        ..default()
                    },
                    MenuButton,
                ))
                .with_children(|parent| {
                    parent.spawn(TextBundle::from_section(
                        "Save & Exit",
                        TextStyle {
                            font: font.clone(),
                            font_size: 24.0,
                            color: Color::WHITE,
                        },
                    ));
                });

                // Cancel button
                parent.spawn((
                    ButtonBundle {
                        style: Style {
                            width: Val::Px(150.0),
                            height: Val::Percent(100.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        background_color: Color::rgb(0.8, 0.3, 0.3).into(),
                        ..default()
                    },
                    MenuButton,
                ))
                .with_children(|parent| {
                    parent.spawn(TextBundle::from_section(
                        "Cancel",
                        TextStyle {
                            font,
                            font_size: 24.0,
                            color: Color::WHITE,
                        },
                    ));
                });
            });
        });
    });
}

fn spawn_customization_section(parent: &mut ChildBuilder, title: &str, font: Handle<Font>, content: impl FnOnce(&mut ChildBuilder)) {
    parent.spawn(NodeBundle {
        style: Style {
            width: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            margin: UiRect::vertical(Val::Px(10.0)),
            ..default()
        },
        ..default()
    })
    .with_children(|parent| {
        parent.spawn(TextBundle::from_section(
            title,
            TextStyle {
                font: font.clone(),
                font_size: 24.0,
                color: Color::WHITE,
            },
        ));

        parent.spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(10.0)),
                margin: UiRect::top(Val::Px(5.0)),
                ..default()
            },
            background_color: Color::rgba(0.2, 0.2, 0.2, 0.5).into(),
            ..default()
        })
        .with_children(content);
    });
}

fn spawn_upgrade_row(parent: &mut ChildBuilder, label: &str, level: UpgradeLevel, font: Handle<Font>) {
    parent.spawn(NodeBundle {
        style: Style {
            width: Val::Percent(100.0),
            height: Val::Px(30.0),
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Center,
            margin: UiRect::vertical(Val::Px(5.0)),
            ..default()
        },
        ..default()
    })
    .with_children(|parent| {
        parent.spawn(TextBundle::from_section(
            label,
            TextStyle {
                font: font.clone(),
                font_size: 20.0,
                color: Color::WHITE,
            },
        ));

        parent.spawn(NodeBundle {
            style: Style {
                flex_direction: FlexDirection::Row,
                gap: Val::Px(5.0),
                ..default()
            },
            ..default()
        })
        .with_children(|parent| {
            for upgrade_level in [UpgradeLevel::Stock, UpgradeLevel::Level1, UpgradeLevel::Level2, UpgradeLevel::Level3] {
                parent.spawn((
                    ButtonBundle {
                        style: Style {
                            width: Val::Px(30.0),
                            height: Val::Px(30.0),
                            border: UiRect::all(Val::Px(2.0)),
                            ..default()
                        },
                        background_color: if std::mem::discriminant(&upgrade_level) <= std::mem::discriminant(&level) {
                            Color::rgb(0.3, 0.6, 0.9).into()
                        } else {
                            Color::rgb(0.2, 0.2, 0.2).into()
                        },
                        border_color: Color::DARK_GRAY.into(),
                        ..default()
                    },
                    MenuButton,
                ));
            }
        });
    });
}

fn setup_race_menu(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    race_setup: Res<RaceSetup>,
) {
    let font = asset_server.load("fonts/FiraSans-Bold.ttf");
    
    commands.spawn((
        NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Row,
                ..default()
            },
            background_color: Color::rgba(0.1, 0.1, 0.1, 0.95).into(),
            ..default()
        },
        MenuPanel,
    ))
    .with_children(|parent| {
        // Left panel - Track preview and info
        parent.spawn(NodeBundle {
            style: Style {
                width: Val::Percent(60.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(20.0)),
                ..default()
            },
            ..default()
        })
        .with_children(|parent| {
            // Track preview image
            parent.spawn((
                ImageBundle {
                    style: Style {
                        width: Val::Percent(100.0),
                        height: Val::Percent(50.0),
                        ..default()
                    },
                    image: asset_server.load(&race_setup.track.preview_image).into(),
                    ..default()
                },
                MenuTransition {
                    timer: Timer::from_seconds(0.3, TimerMode::Once),
                    transition_type: TransitionType::FadeIn,
                    initial_style: None,
                    initial_color: None,
                    target_style: None,
                    target_color: None,
                },
            ));

            // Track info
            parent.spawn(NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    margin: UiRect::top(Val::Px(20.0)),
                    ..default()
                },
                background_color: Color::rgba(0.15, 0.15, 0.15, 0.8).into(),
                ..default()
            })
            .with_children(|parent| {
                parent.spawn(TextBundle::from_section(
                    &race_setup.track.name,
                    TextStyle {
                        font: font.clone(),
                        font_size: 32.0,
                        color: Color::WHITE,
                    },
                ));

                parent.spawn(TextBundle::from_section(
                    &race_setup.track.description,
                    TextStyle {
                        font: font.clone(),
                        font_size: 18.0,
                        color: Color::GRAY,
                    },
                ));

                // Track stats
                parent.spawn(NodeBundle {
                    style: Style {
                        width: Val::Percent(100.0),
                        margin: UiRect::top(Val::Px(10.0)),
                        ..default()
                    },
                    ..default()
                })
                .with_children(|parent| {
                    spawn_race_stat(parent, "Length", &format!("{:.1} km", race_setup.track.length_km), font.clone());
                    spawn_race_stat(parent, "Difficulty", &"★".repeat(race_setup.track.difficulty_rating as usize), font.clone());
                });
            });
        });

        // Right panel - Race settings
        parent.spawn(NodeBundle {
            style: Style {
                width: Val::Percent(40.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(20.0)),
                ..default()
            },
            background_color: Color::rgba(0.15, 0.15, 0.15, 0.95).into(),
            ..default()
        })
        .with_children(|parent| {
            // Title
            parent.spawn(TextBundle::from_section(
                "Race Setup",
                TextStyle {
                    font: font.clone(),
                    font_size: 32.0,
                    color: Color::WHITE,
                },
            ));

            // Time of day selection
            spawn_race_section(parent, "Time of Day", font.clone(), |parent| {
                let times = [
                    TimeOfDay::Dawn,
                    TimeOfDay::Morning,
                    TimeOfDay::Noon,
                    TimeOfDay::Afternoon,
                    TimeOfDay::Sunset,
                    TimeOfDay::Night,
                ];
                for time in times {
                    spawn_race_option(parent, &format!("{:?}", time), race_setup.time_of_day == time, font.clone());
                }
            });

            // Weather selection
            spawn_race_section(parent, "Weather", font.clone(), |parent| {
                for weather in &race_setup.track.weather_options {
                    spawn_race_option(parent, &format!("{:?}", weather), race_setup.weather == *weather, font.clone());
                }
            });

            // Difficulty selection
            spawn_race_section(parent, "Difficulty", font.clone(), |parent| {
                let difficulties = [
                    Difficulty::Easy,
                    Difficulty::Medium,
                    Difficulty::Hard,
                    Difficulty::Expert,
                ];
                for difficulty in difficulties {
                    spawn_race_option(parent, &format!("{:?}", difficulty), race_setup.difficulty == difficulty, font.clone());
                }
            });

            // Race parameters
            spawn_race_section(parent, "Race Parameters", font.clone(), |parent| {
                // Laps slider
                spawn_number_selector(parent, "Laps", race_setup.laps, 1, 10, font.clone());
                // Opponents slider
                spawn_number_selector(parent, "Opponents", race_setup.opponents, 1, 8, font.clone());
            });

            // Start race button
            parent.spawn((
                ButtonBundle {
                    style: Style {
                        width: Val::Percent(100.0),
                        height: Val::Px(60.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        margin: UiRect::top(Val::Px(20.0)),
                        ..default()
                    },
                    background_color: Color::rgb(0.3, 0.8, 0.3).into(),
                    ..default()
                },
                MenuButton,
            ))
            .with_children(|parent| {
                parent.spawn(TextBundle::from_section(
                    "Start Race",
                    TextStyle {
                        font,
                        font_size: 28.0,
                        color: Color::WHITE,
                    },
                ));
            });
        });
    });
}

fn spawn_race_section(parent: &mut ChildBuilder, title: &str, font: Handle<Font>, content: impl FnOnce(&mut ChildBuilder)) {
    parent.spawn(NodeBundle {
        style: Style {
            width: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            margin: UiRect::vertical(Val::Px(10.0)),
            ..default()
        },
        ..default()
    })
    .with_children(|parent| {
        parent.spawn(TextBundle::from_section(
            title,
            TextStyle {
                font: font.clone(),
                font_size: 24.0,
                color: Color::WHITE,
            },
        ));

        parent.spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(10.0)),
                margin: UiRect::top(Val::Px(5.0)),
                ..default()
            },
            background_color: Color::rgba(0.2, 0.2, 0.2, 0.5).into(),
            ..default()
        })
        .with_children(content);
    });
}

fn spawn_race_option(parent: &mut ChildBuilder, label: &str, selected: bool, font: Handle<Font>) {
    parent.spawn((
        ButtonBundle {
            style: Style {
                width: Val::Percent(100.0),
                height: Val::Px(40.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                margin: UiRect::vertical(Val::Px(2.0)),
                ..default()
            },
            background_color: if selected {
                Color::rgb(0.3, 0.6, 0.9).into()
            } else {
                Color::rgb(0.2, 0.2, 0.2).into()
            },
            ..default()
        },
        MenuButton,
    ))
    .with_children(|parent| {
        parent.spawn(TextBundle::from_section(
            label,
            TextStyle {
                font,
                font_size: 20.0,
                color: if selected { Color::WHITE } else { Color::GRAY },
            },
        ));
    });
}

fn spawn_race_stat(parent: &mut ChildBuilder, label: &str, value: &str, font: Handle<Font>) {
    parent.spawn(NodeBundle {
        style: Style {
            width: Val::Percent(100.0),
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Center,
            margin: UiRect::vertical(Val::Px(5.0)),
            ..default()
        },
        ..default()
    })
    .with_children(|parent| {
        parent.spawn(TextBundle::from_section(
            label,
            TextStyle {
                font: font.clone(),
                font_size: 18.0,
                color: Color::GRAY,
            },
        ));
        parent.spawn(TextBundle::from_section(
            value,
            TextStyle {
                font,
                font_size: 18.0,
                color: Color::WHITE,
            },
        ));
    });
}

fn spawn_number_selector(parent: &mut ChildBuilder, label: &str, value: u32, min: u32, max: u32, font: Handle<Font>) {
    parent.spawn(NodeBundle {
        style: Style {
            width: Val::Percent(100.0),
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Center,
            margin: UiRect::vertical(Val::Px(5.0)),
            ..default()
        },
        ..default()
    })
    .with_children(|parent| {
        parent.spawn(TextBundle::from_section(
            label,
            TextStyle {
                font: font.clone(),
                font_size: 20.0,
                color: Color::WHITE,
            },
        ));

        parent.spawn(NodeBundle {
            style: Style {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                gap: Val::Px(10.0),
                ..default()
            },
            ..default()
        })
        .with_children(|parent| {
            // Decrease button
            parent.spawn((
                ButtonBundle {
                    style: Style {
                        width: Val::Px(30.0),
                        height: Val::Px(30.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    background_color: Color::rgb(0.2, 0.2, 0.2).into(),
                    ..default()
                },
                MenuButton,
            ))
            .with_children(|parent| {
                parent.spawn(TextBundle::from_section(
                    "-",
                    TextStyle {
                        font: font.clone(),
                        font_size: 20.0,
                        color: Color::WHITE,
                    },
                ));
            });

            // Value
            parent.spawn(TextBundle::from_section(
                value.to_string(),
                TextStyle {
                    font: font.clone(),
                    font_size: 20.0,
                    color: Color::WHITE,
                },
            ));

            // Increase button
            parent.spawn((
                ButtonBundle {
                    style: Style {
                        width: Val::Px(30.0),
                        height: Val::Px(30.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    background_color: Color::rgb(0.2, 0.2, 0.2).into(),
                    ..default()
                },
                MenuButton,
            ))
            .with_children(|parent| {
                parent.spawn(TextBundle::from_section(
                    "+",
                    TextStyle {
                        font,
                        font_size: 20.0,
                        color: Color::WHITE,
                    },
                ));
            });
        });
    });
} 