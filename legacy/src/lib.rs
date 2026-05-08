pub mod core;
pub mod game;
pub mod physics;
pub mod rendering;
pub mod audio;
pub mod ui;
pub mod utils;
pub mod assets;
pub mod terrain;
pub mod weather;

// pub use crate::core::CorePlugin;
// pub use crate::game::GamePlugin;
pub use crate::physics::PhysicsPlugin;
pub use crate::rendering::RenderingPlugin;
pub use crate::audio::AudioPlugin;
pub use crate::ui::UiPlugin;
pub use crate::assets::AssetPlugin;
pub use crate::terrain::TerrainPlugin; 