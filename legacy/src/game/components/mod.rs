use bevy::prelude::*;

mod camera;
mod player;
mod vehicle;

pub use camera::{MainCamera, CameraFollow};
pub use player::{Player, PlayerInput};
pub use vehicle::{Vehicle, Suspension, Wheel};

// Re-export commonly used components
pub use camera::MainCamera;
pub use player::Player;
pub use vehicle::{Vehicle, Suspension};

#[derive(Component)]
pub struct MainCamera {
    pub follow_distance: f32,
    pub follow_height: f32,
    pub follow_smoothness: f32,
}

impl Default for MainCamera {
    fn default() -> Self {
        Self {
            follow_distance: 15.0,
            follow_height: 8.0,
            follow_smoothness: 5.0,
        }
    }
}

#[derive(Component, Default)]
pub struct CameraFollow {
    pub target: Option<Entity>,
    pub offset: Vec3,
    pub smoothness: f32,
}

#[derive(Component)]
pub struct Vehicle {
    pub steering_angle: f32,
    pub throttle: f32,
    pub brake: f32,
    pub handbrake: bool,
    pub gear: i32,
    pub rpm: f32,
    pub speed: f32,
}

impl Default for Vehicle {
    fn default() -> Self {
        Self {
            steering_angle: 0.0,
            throttle: 0.0,
            brake: 0.0,
            handbrake: false,
            gear: 1,
            rpm: 0.0,
            speed: 0.0,
        }
    }
}

#[derive(Component)]
pub struct Player {
    pub name: String,
    pub score: i32,
    pub health: f32,
}

impl Default for Player {
    fn default() -> Self {
        Self {
            name: "Player".to_string(),
            score: 0,
            health: 100.0,
        }
    }
}

#[derive(Component)]
pub struct Suspension {
    pub spring_strength: f32,
    pub damping: f32,
    pub rest_length: f32,
    pub max_travel: f32,
    pub current_compression: f32,
}

impl Default for Suspension {
    fn default() -> Self {
        Self {
            spring_strength: 50000.0,
            damping: 4000.0,
            rest_length: 0.5,
            max_travel: 0.3,
            current_compression: 0.0,
        }
    }
}

/// Component for entities that can be interacted with
#[derive(Component)]
pub struct Interactable {
    pub interaction_type: InteractionType,
}

/// Different types of interactions possible with entities
#[derive(Debug, Clone, Copy)]
pub enum InteractionType {
    Examine,
    Use,
    Enter,
}

/// Component for entities that can be selected
#[derive(Component)]
pub struct Selectable {
    pub selected: bool,
    pub highlight_color: Color,
}

/// Component for entities that have health/damage states
#[derive(Component)]
pub struct Health {
    pub current: f32,
    pub maximum: f32,
}

/// Component for entities that can trigger events
#[derive(Component)]
pub struct EventTrigger {
    pub event_type: String,
    pub trigger_radius: f32,
    pub one_shot: bool,
    pub triggered: bool,
}

impl Default for Health {
    fn default() -> Self {
        Self {
            current: 100.0,
            maximum: 100.0,
        }
    }
} 