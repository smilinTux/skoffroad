#[cfg(test)]
mod tests {
    use bevy::prelude::*;
    use crate::game::{GamePlugin};
    use crate::game::state::GameState;

    fn setup_test_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugin(GamePlugin);
        app
    }

    #[test]
    fn test_game_plugin_initialization() {
        let mut app = setup_test_app();
        
        // Test initial game state
        let game_state = app.world.resource::<GameState>();
        assert!(matches!(game_state.current_state, GameState::Loading));
        
        // Run startup systems
        app.update();
        
        // Verify resources are properly initialized
        assert!(app.world.contains_resource::<Time>());
        assert!(app.world.contains_resource::<GameState>());
    }

    #[test]
    fn test_state_transitions() {
        let mut app = setup_test_app();
        
        // Test state transition to MainMenu
        app.world.resource_mut::<GameState>().transition_to(GameState::MainMenu);
        app.update();
        assert!(matches!(app.world.resource::<GameState>().current_state, GameState::MainMenu));
        
        // Test state transition to InGame
        app.world.resource_mut::<GameState>().transition_to(GameState::InGame);
        app.update();
        assert!(matches!(app.world.resource::<GameState>().current_state, GameState::InGame));
    }

    #[test]
    fn test_resource_initialization() {
        let app = setup_test_app();
        
        // Verify all required resources are present
        assert!(app.world.contains_resource::<GameState>());
        // Add more resource checks as needed
    }

    #[test]
    fn test_plugin_dependencies() {
        let mut app = App::new();
        
        // Verify GamePlugin can be added with minimal dependencies
        app.add_plugins(MinimalPlugins)
            .add_plugin(GamePlugin);
            
        // This should not panic
        app.update();
    }

    #[test]
    fn test_game_setup() {
        let mut app = App::new();
        
        // Add only the essential plugins for testing
        app.add_plugins(MinimalPlugins)
            .add_plugin(GamePlugin);

        // Verify initial game state
        let game_state = app.world.resource::<GameState>();
        assert!(!game_state.paused);
        assert_eq!(game_state.score, 0);
        assert_eq!(game_state.time_elapsed, 0.0);

        // Run a few update cycles
        for _ in 0..10 {
            app.update();
        }

        // Verify game state updates
        let game_state = app.world.resource::<GameState>();
        assert!(game_state.time_elapsed > 0.0);
    }

    #[test]
    fn test_pause_functionality() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugin(GamePlugin)
            .add_systems(Update, test_input);

        // Initial state
        let game_state = app.world.resource::<GameState>();
        assert!(!game_state.paused);

        // Simulate pressing escape
        let mut input = app.world.resource_mut::<Input<KeyCode>>();
        input.press(KeyCode::Escape);
        
        // Run one frame
        app.update();

        // Verify game is paused
        let game_state = app.world.resource::<GameState>();
        assert!(game_state.paused);
    }

    fn test_input(mut input: ResMut<Input<KeyCode>>) {
        input.press(KeyCode::Escape);
    }
} 