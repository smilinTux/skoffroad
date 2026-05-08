use bevy::prelude::*;

/// Component for marking and configuring the main game camera
#[derive(Component)]
pub struct MainCamera {
    /// Target entity to follow (if any)
    pub target: Option<Entity>,
    /// Camera follow distance
    pub follow_distance: f32,
    /// Camera height offset
    pub height_offset: f32,
    /// Camera smoothing factor (0.0 - 1.0)
    pub smoothing: f32,
}

impl Default for MainCamera {
    fn default() -> Self {
        Self {
            target: None,
            follow_distance: 10.0,
            height_offset: 5.0,
            smoothing: 0.1,
        }
    }
}

/// Component for marking entities that should be followed by the camera
#[derive(Component)]
pub struct CameraFollow {
    /// Offset from the target's position
    pub offset: Vec3,
    /// Look at offset (where the camera should look relative to target)
    pub look_at_offset: Vec3,
}

impl Default for CameraFollow {
    fn default() -> Self {
        Self {
            offset: Vec3::new(0.0, 5.0, -10.0),
            look_at_offset: Vec3::new(0.0, 0.0, 2.0),
        }
    }
} 