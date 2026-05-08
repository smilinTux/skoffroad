use bevy::prelude::*;
use super::GameMode;

/// Events that can occur during gameplay
#[derive(Event, Debug, Clone)]
pub enum GameEvent {
    /// Game started
    Start,
    /// Game paused
    Pause,
    /// Game resumed
    Resume,
    /// Game ended
    End { score: f32 },
    /// Mode changed
    ModeChange { new_mode: GameMode },
    /// Score updated
    ScoreUpdate { new_score: f32 },
}

pub struct GameStatus;

impl GameStatus {
    /// Create a new game state with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Start the game
    pub fn start(&mut self) {
        self.paused = false;
        self.elapsed_time = 0.0;
        self.score = 0.0;
    }

    /// Pause the game
    pub fn pause(&mut self) {
        self.paused = true;
    }

    /// Resume the game
    pub fn resume(&mut self) {
        self.paused = false;
    }

    /// Update the game state
    pub fn update(&mut self, delta_time: f32) {
        if !self.paused {
            self.elapsed_time += delta_time;
        }
    }

    /// Change the game mode
    pub fn set_mode(&mut self, mode: GameMode) {
        self.mode = mode;
    }

    /// Update the score
    pub fn update_score(&mut self, points: f32) {
        self.score += points;
    }

    /// Reset the game state
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Check if the game is active (not paused)
    pub fn is_active(&self) -> bool {
        !self.paused
    }

    /// Get the current elapsed time
    pub fn get_elapsed_time(&self) -> f32 {
        self.elapsed_time
    }

    /// Get the current score
    pub fn get_score(&self) -> f32 {
        self.score
    }

    /// Get the current game mode
    pub fn get_mode(&self) -> GameMode {
        self.mode
    }
} 