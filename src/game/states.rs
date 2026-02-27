use bevy::prelude::*;
use crate::game::state::GameState;

#[derive(States, Default, Debug, Clone, Eq, PartialEq, Hash)]
pub enum GameMode {
    #[default]
    TimeAttack,
    Race,
    FreeRoam,
}

#[derive(Resource, Default)]
pub struct GameTimer {
    pub elapsed: f32,
    pub best_time: Option<f32>,
}

#[derive(Resource)]
pub struct GameProgress {
    pub current_level: u32,
    pub unlocked_levels: u32,
    pub best_times: Vec<f32>,
    pub total_score: u32,
}

impl Default for GameProgress {
    fn default() -> Self {
        Self {
            current_level: 1,
            unlocked_levels: 1,
            best_times: vec![0.0; 10], // Assuming 10 levels
            total_score: 0,
        }
    }
}

pub struct GameStatePlugin;

impl Plugin for GameStatePlugin {
    fn build(&self, app: &mut App) {
        app.add_state::<GameState>()
            .add_state::<GameMode>()
            .init_resource::<GameTimer>()
            .init_resource::<GameProgress>()
            .add_systems(Update, update_game_timer.run_if(in_state(GameState::Playing)));
    }
}

fn update_game_timer(time: Res<Time>, mut game_timer: ResMut<GameTimer>) {
    game_timer.elapsed += time.delta_seconds();
} 