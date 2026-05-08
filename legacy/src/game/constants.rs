// Vehicle dimensions
pub const VEHICLE_LENGTH: f32 = 4.5; // meters
pub const VEHICLE_WIDTH: f32 = 2.0;
pub const VEHICLE_HEIGHT: f32 = 1.8;
pub const VEHICLE_WHEELBASE: f32 = 2.7;
pub const VEHICLE_TRACK_WIDTH: f32 = 1.6;

// Vehicle physics
pub const MAX_STEERING_ANGLE: f32 = 0.8;  // Radians
pub const MAX_ENGINE_FORCE: f32 = 2000.0;
pub const MAX_BRAKE_FORCE: f32 = 1000.0;
pub const DRAG_COEFFICIENT: f32 = 0.3;
pub const ROLLING_RESISTANCE: f32 = 0.015;
pub const WHEELBASE: f32 = 2.5;  // Meters

// Game settings
pub const GRAVITY: f32 = 9.81;
pub const MAX_TIMESTEP: f32 = 1.0 / 60.0;
pub const MIN_TIMESTEP: f32 = 1.0 / 240.0;
pub const INITIAL_SCORE: i32 = 0;
pub const SCORE_MULTIPLIER: f32 = 1.0;
pub const TIME_SCALE: f32 = 1.0;

// Camera settings
pub const DEFAULT_FOV: f32 = 75.0;
pub const MIN_FOV: f32 = 60.0;
pub const MAX_FOV: f32 = 90.0;
pub const CAMERA_FOLLOW_SPEED: f32 = 5.0;
pub const CAMERA_HEIGHT: f32 = 3.0;
pub const CAMERA_DISTANCE: f32 = 8.0;
pub const CAMERA_FOV: f32 = 60.0;  // Degrees

// UI settings
pub const UI_FONT_SIZE: f32 = 20.0;
pub const UI_PADDING: f32 = 10.0;

// Debug settings
#[cfg(debug_assertions)]
pub const DEBUG_DRAW_COLLIDERS: bool = true;
#[cfg(debug_assertions)]
pub const DEBUG_DRAW_FORCES: bool = true;
pub const DEBUG_TEXT_SIZE: f32 = 14.0;
pub const DEBUG_TEXT_COLOR: [f32; 4] = [1.0, 1.0, 1.0, 0.8];  // White with slight transparency
pub const DEBUG_OVERLAY_MARGIN: f32 = 10.0;

// Physics Simulation Constants
pub const PHYSICS_TIMESTEP: f32 = 1.0 / 60.0;  // 60 Hz physics update
pub const MAX_SUBSTEPS: u32 = 8;
pub const MIN_SUBSTEPS: u32 = 1;

// Input Smoothing Constants
pub const INPUT_SMOOTHING_FACTOR: f32 = 0.2;
pub const STEERING_RETURN_SPEED: f32 = 2.0; 