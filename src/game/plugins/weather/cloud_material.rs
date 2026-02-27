use bevy::{
    prelude::*,
    reflect::TypeUuid,
    render::render_resource::{
        AsBindGroup, ShaderRef, ShaderType,
    },
    asset::Asset,
};
use super::Weather;

#[derive(Asset, AsBindGroup, TypeUuid, TypePath, Debug, Clone)]
#[uuid = "f690fdae-d598-45ab-8225-97e2a3f056e0"]
pub struct CloudMaterial {
    #[uniform(0)]
    pub params: CloudParams,
    
    #[texture(1)]
    #[sampler(2)]
    pub base_shape_texture: Handle<Image>,
    
    #[texture(3)]
    #[sampler(4)]
    pub detail_texture: Handle<Image>,
    
    #[texture(5)]
    #[sampler(6)]
    pub weather_texture: Handle<Image>,
}

#[derive(ShaderType, Debug, Clone)]
pub struct CloudParams {
    pub density: f32,
    pub coverage: f32,
    pub altitude: f32,
    pub thickness: f32,
    pub wind_direction: Vec2,
    pub wind_speed: f32,
    pub precipitation_threshold: f32,
    pub time: f32,
}

impl Default for CloudParams {
    fn default() -> Self {
        Self {
            density: 1.0,
            coverage: 0.5,
            altitude: 1000.0,
            thickness: 500.0,
            wind_direction: Vec2::new(1.0, 0.0),
            wind_speed: 10.0,
            precipitation_threshold: 0.5,
            time: 0.0,
        }
    }
}

impl Material for CloudMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/volumetric_clouds.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }
}

// Plugin to register the cloud material
pub struct CloudMaterialPlugin;

impl Plugin for CloudMaterialPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<CloudMaterial>::default());
    }
}

// Helper functions for cloud material
impl CloudMaterial {
    pub fn update_from_weather(&mut self, weather: &Weather, time: f32) {
        match weather {
            Weather::Clear => {
                self.params.density = 0.1;
                self.params.coverage = 0.2;
            },
            Weather::Cloudy => {
                self.params.density = 0.6;
                self.params.coverage = 0.7;
            },
            Weather::Rain => {
                self.params.density = 0.8;
                self.params.coverage = 0.9;
                self.params.precipitation_threshold = 0.6;
            },
            Weather::Storm => {
                self.params.density = 1.0;
                self.params.coverage = 1.0;
                self.params.precipitation_threshold = 0.4;
                self.params.wind_speed = 5.0 + (time * 0.1).sin() * 2.0;
            },
            Weather::Fog => {
                self.params.density = 0.7;
                self.params.coverage = 0.8;
                self.params.altitude = 100.0;
            },
            Weather::Snow => {
                self.params.density = 0.9;
                self.params.coverage = 0.95;
                self.params.precipitation_threshold = 0.5;
            },
        }
        self.params.time = time;
    }
} 