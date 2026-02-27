use bevy::prelude::*;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EmitterShape {
    Box { size: Vec3 },
    Point,
    Plane { size: Vec2, subdivisions: u32 },
    // Add more variants as needed
}
