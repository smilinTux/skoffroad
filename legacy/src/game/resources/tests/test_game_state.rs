use super::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_game_state_new() {
        let state = GameState::new();
        assert!(!state.is_active());
        assert_eq!(state.get_elapsed_time(), 0.0);
        assert_eq!(state.get_score(), 0.0);
        assert!(matches!(state.get_mode(), GameMode::FreeRoam));
    }

    #[test]
    fn test_game_state_start() {
        let mut state = GameState::new();
        state.start();
        assert!(state.is_active());
        assert_eq!(state.get_elapsed_time(), 0.0);
        assert_eq!(state.get_score(), 0.0);
    }

    #[test]
    fn test_game_state_pause_resume() {
        let mut state = GameState::new();
        state.start();
        assert!(state.is_active());

        state.pause();
        assert!(!state.is_active());

        state.resume();
        assert!(state.is_active());
    }

    #[test]
    fn test_game_state_update() {
        let mut state = GameState::new();
        state.start();

        // Test time update while active
        state.update(1.0);
        assert_eq!(state.get_elapsed_time(), 1.0);

        state.update(0.5);
        assert_eq!(state.get_elapsed_time(), 1.5);

        // Test time doesn't update while paused
        state.pause();
        state.update(1.0);
        assert_eq!(state.get_elapsed_time(), 1.5);
    }

    #[test]
    fn test_game_state_score() {
        let mut state = GameState::new();
        
        state.update_score(10.0);
        assert_eq!(state.get_score(), 10.0);

        state.update_score(5.0);
        assert_eq!(state.get_score(), 15.0);
    }

    #[test]
    fn test_game_state_mode() {
        let mut state = GameState::new();
        assert!(matches!(state.get_mode(), GameMode::FreeRoam));

        state.set_mode(GameMode::Race);
        assert!(matches!(state.get_mode(), GameMode::Race));

        state.set_mode(GameMode::Challenge);
        assert!(matches!(state.get_mode(), GameMode::Challenge));
    }

    #[test]
    fn test_game_state_reset() {
        let mut state = GameState::new();
        
        // Modify state
        state.start();
        state.update_score(100.0);
        state.update(5.0);
        state.set_mode(GameMode::Race);

        // Reset state
        state.reset();

        // Verify reset to default values
        assert!(!state.is_active());
        assert_eq!(state.get_elapsed_time(), 0.0);
        assert_eq!(state.get_score(), 0.0);
        assert!(matches!(state.get_mode(), GameMode::FreeRoam));
    }

    #[test]
    fn test_game_events() {
        // Test event creation and cloning
        let events = vec![
            GameEvent::Start,
            GameEvent::Pause,
            GameEvent::Resume,
            GameEvent::End { score: 100.0 },
            GameEvent::ModeChange { new_mode: GameMode::Race },
            GameEvent::ScoreUpdate { new_score: 50.0 },
        ];

        for event in events {
            let cloned = event.clone();
            match (event, cloned) {
                (GameEvent::Start, GameEvent::Start) => (),
                (GameEvent::Pause, GameEvent::Pause) => (),
                (GameEvent::Resume, GameEvent::Resume) => (),
                (GameEvent::End { score: s1 }, GameEvent::End { score: s2 }) => {
                    assert_eq!(s1, s2);
                },
                (GameEvent::ModeChange { new_mode: m1 }, GameEvent::ModeChange { new_mode: m2 }) => {
                    assert!(matches!(m1, m2));
                },
                (GameEvent::ScoreUpdate { new_score: s1 }, GameEvent::ScoreUpdate { new_score: s2 }) => {
                    assert_eq!(s1, s2);
                },
                _ => panic!("Event cloning failed"),
            }
        }
    }
} 