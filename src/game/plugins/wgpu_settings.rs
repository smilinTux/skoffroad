use bevy::prelude::Resource;
use bevy::render::render_resource::WgpuFeatures;

#[derive(Resource, Debug, Clone)]
pub struct WgpuSettings {
    pub features: WgpuFeatures,
}

impl Default for WgpuSettings {
    fn default() -> Self {
        Self {
            features: WgpuFeatures::empty(),
        }
    }
}
