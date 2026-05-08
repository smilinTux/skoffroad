use bevy::prelude::*;
use crate::game::state::GameState;
use crate::game::states::{GameTimer, GameProgress};

#[test]
fn test_game_state_transitions() {
    let mut app = App::new();
    
    app.add_plugins(MinimalPlugins)
        .add_state::<GameState>()
        .init_resource::<GameTimer>()
        .init_resource::<GameProgress>();

    // Test initial state
    assert_eq!(app.world.resource::<State<GameState>>().get(), &GameState::MainMenu);

    // Test transition to Playing state
    app.world.resource_mut::<NextState<GameState>>().set(GameState::Playing);
    app.update();
    assert_eq!(app.world.resource::<State<GameState>>().get(), &GameState::Playing);

    // Test transition to Paused state
    app.world.resource_mut::<NextState<GameState>>().set(GameState::Paused);
    app.update();
    assert_eq!(app.world.resource::<State<GameState>>().get(), &GameState::Paused);

    // Test transition back to Playing state
    app.world.resource_mut::<NextState<GameState>>().set(GameState::Playing);
    app.update();
    assert_eq!(app.world.resource::<State<GameState>>().get(), &GameState::Playing);

    // Test transition to GameOver state
    app.world.resource_mut::<NextState<GameState>>().set(GameState::GameOver);
    app.update();
    assert_eq!(app.world.resource::<State<GameState>>().get(), &GameState::GameOver);
}

#[test]
fn test_game_timer() {
    let mut app = App::new();
    
    app.add_plugins(MinimalPlugins)
        .add_state::<GameState>()
        .init_resource::<GameTimer>();

    // Set initial state to Playing
    app.world.resource_mut::<NextState<GameState>>().set(GameState::Playing);
    app.update();

    // Get initial timer value
    let initial_time = app.world.resource::<GameTimer>().elapsed;

    // Update time resource
    let mut time = app.world.resource_mut::<Time>();
    time.update();
    time.set_relative_speed(1.0);
    
    // Run one update
    app.update();

    // Check if timer updated
    let current_time = app.world.resource::<GameTimer>().elapsed;
    assert!(current_time > initial_time);
}

#[test]
fn test_game_progress() {
    let mut app = App::new();
    
    app.add_plugins(MinimalPlugins)
        .init_resource::<GameProgress>();

    // Check initial values
    let game_progress = app.world.resource::<GameProgress>();
    assert_eq!(game_progress.current_level, 1);
    assert_eq!(game_progress.unlocked_levels, 1);
    assert_eq!(game_progress.total_score, 0);

    // Update progress
    {
        let mut game_progress = app.world.resource_mut::<GameProgress>();
        game_progress.current_level = 2;
        game_progress.unlocked_levels = 2;
        game_progress.total_score = 1000;
    }

    // Verify updates
    let game_progress = app.world.resource::<GameProgress>();
    assert_eq!(game_progress.current_level, 2);
    assert_eq!(game_progress.unlocked_levels, 2);
    assert_eq!(game_progress.total_score, 1000);
} 