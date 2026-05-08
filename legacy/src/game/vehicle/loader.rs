use bevy::{
    asset::{AssetLoader, LoadContext, LoadedAsset},
    prelude::*,
    reflect::TypeUuid,
    utils::BoxedFuture,
};
use serde_json::from_slice;
use crate::game::vehicle::config::VehicleConfig;

/// Custom asset type for vehicle configurations
#[derive(TypeUuid)]
#[uuid = "817c9021-9d09-4438-a8c8-5c4a6f725c4a"]
pub struct VehicleConfigAsset(pub VehicleConfig);

/// Asset loader for vehicle configuration files
#[derive(Default)]
pub struct VehicleConfigLoader;

impl AssetLoader for VehicleConfigLoader {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<(), bevy::asset::Error>> {
        Box::pin(async move {
            let config: VehicleConfig = from_slice(bytes)?;
            load_context.set_default_asset(LoadedAsset::new(VehicleConfigAsset(config)));
            Ok(())
        })
    }

    fn extensions(&self) -> &[&str] {
        &["vehicle.json"]
    }
}

/// Plugin to register the vehicle config asset loader
pub struct VehicleLoaderPlugin;

impl Plugin for VehicleLoaderPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<VehicleConfigAsset>()
            .init_asset_loader::<VehicleConfigLoader>();
    }
}

/// System to spawn a vehicle from a config asset
pub fn spawn_vehicle_from_config(
    mut commands: Commands,
    vehicle_configs: Res<Assets<VehicleConfigAsset>>,
    config_handles: Query<(Entity, &Handle<VehicleConfigAsset>)>,
) {
    for (entity, config_handle) in config_handles.iter() {
        if let Some(config_asset) = vehicle_configs.get(config_handle) {
            let config = &config_asset.0;
            
            // Create vehicle entity with configuration
            commands.entity(entity)
                .insert(config.clone())
                .insert(Name::new(config.name.clone()));

            // TODO: Add additional components based on configuration
            // This will be expanded as we implement more vehicle systems
        }
    }
} 