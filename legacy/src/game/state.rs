use bevy::prelude::*;

/// Core game states
#[derive(States, Debug, Clone, Copy, Default, Eq, PartialEq, Hash)]
pub enum GameState {
    #[default]
    Loading,
    MainMenu,
    InGame,
    Paused,
}

/// Plugin for managing game state transitions and updates
pub struct StatePlugin;

impl Plugin for StatePlugin {
    fn build(&self, app: &mut App) {
        app.add_state::<GameState>()
           .add_systems(OnEnter(GameState::Loading), setup_loading)
           .add_systems(Update, update_loading.run_if(in_state(GameState::Loading)))
           
           .add_systems(OnEnter(GameState::MainMenu), setup_main_menu)
           .add_systems(Update, update_main_menu.run_if(in_state(GameState::MainMenu)))
           
           .add_systems(OnEnter(GameState::InGame), setup_game)
           .add_systems(Update, update_game.run_if(in_state(GameState::InGame)))
           
           .add_systems(OnEnter(GameState::Paused), setup_pause_menu)
           .add_systems(Update, update_pause_menu.run_if(in_state(GameState::Paused)));
    }
}

// Loading state systems
fn setup_loading(mut commands: Commands) {
    info!("Entering loading state");
    // Initialize loading screen UI and progress tracking
}

fn update_loading(
    mut next_state: ResMut<NextState<GameState>>,
    // Add loading progress resources
) {
    // Check loading progress and transition when complete
    // next_state.set(GameState::MainMenu);
}

// Main menu state systems
fn setup_main_menu(mut commands: Commands) {
    info!("Entering main menu");
    // Setup menu UI and interactions
}

fn update_main_menu(
    mut next_state: ResMut<NextState<GameState>>,
    // Add menu interaction resources
) {
    // Handle menu selections and state transitions
}

// Game state systems
fn setup_game(mut commands: Commands) {
    info!("Starting game");
    // Initialize game world and entities
}

fn update_game(
    mut next_state: ResMut<NextState<GameState>>,
    // Add game state resources
) {
    // Handle core game loop and pause transitions
}

// Pause state systems
fn setup_pause_menu(mut commands: Commands) {
    info!("Game paused");
    // Setup pause menu UI
}

fn update_pause_menu(
    mut next_state: ResMut<NextState<GameState>>,
    // Add pause menu resources
) {
    // Handle pause menu interactions and state transitions
} 