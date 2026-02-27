#[derive(Resource, Default)]
pub struct AssetLoadingState {
    pub pending_assets: HashMap<Handle<Image>, LoadPriority>,
    pub loaded_assets: HashSet<Handle<Image>>,
    pub visualization_loading_progress: f32,
    pub metrics_loading_progress: f32,
    pub total_assets: usize,
    pub loaded_count: usize,
}

impl AssetLoadingState {
    pub fn add_pending_assets(&mut self, handles: Vec<Handle<Image>>, priority: LoadPriority) {
        for handle in handles {
            self.pending_assets.insert(handle, priority);
        }
        self.total_assets = self.pending_assets.len();
    }

    pub fn mark_asset_loaded(&mut self, handle: Handle<Image>) {
        if self.pending_assets.remove(&handle).is_some() {
            self.loaded_assets.insert(handle);
            self.loaded_count += 1;
            self.update_progress();
        }
    }

    pub fn update_progress(&mut self) {
        if self.total_assets == 0 {
            self.visualization_loading_progress = 1.0;
            self.metrics_loading_progress = 1.0;
            return;
        }

        let visualization_assets = self.pending_assets
            .iter()
            .filter(|(_, priority)| matches!(priority, LoadPriority::High))
            .count();
        let metrics_assets = self.pending_assets
            .iter()
            .filter(|(_, priority)| matches!(priority, LoadPriority::Medium))
            .count();

        let total_visualization = visualization_assets + self.loaded_assets
            .iter()
            .filter(|handle| {
                self.pending_assets
                    .get(handle)
                    .map_or(false, |p| matches!(p, LoadPriority::High))
            })
            .count();

        let total_metrics = metrics_assets + self.loaded_assets
            .iter()
            .filter(|handle| {
                self.pending_assets
                    .get(handle)
                    .map_or(false, |p| matches!(p, LoadPriority::Medium))
            })
            .count();

        if total_visualization > 0 {
            self.visualization_loading_progress = 1.0 - (visualization_assets as f32 / total_visualization as f32);
        }

        if total_metrics > 0 {
            self.metrics_loading_progress = 1.0 - (metrics_assets as f32 / total_metrics as f32);
        }
    }

    pub fn is_loading_complete(&self) -> bool {
        self.pending_assets.is_empty()
    }

    pub fn get_assets_by_priority(&self, priority: LoadPriority) -> Vec<Handle<Image>> {
        self.pending_assets
            .iter()
            .filter(|(_, p)| **p == priority)
            .map(|(handle, _)| handle.clone())
            .collect()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadPriority {
    High,   // For visualization assets (textures, materials, gradients)
    Medium, // For metrics assets (icons, fonts)
    Low,    // For other assets
}

pub fn update_asset_state(
    mut loading_state: ResMut<AssetLoadingState>,
    asset_server: Res<AssetServer>,
) {
    let mut newly_loaded = Vec::new();

    for handle in loading_state.pending_assets.keys() {
        match asset_server.get_load_state(handle.clone_weak()) {
            LoadState::Loaded => {
                newly_loaded.push(handle.clone_weak());
            }
            LoadState::Failed => {
                error!("Failed to load asset: {:?}", handle);
                newly_loaded.push(handle.clone_weak());
            }
            _ => {}
        }
    }

    for handle in newly_loaded {
        loading_state.mark_asset_loaded(handle);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::prelude::*;

    #[test]
    fn test_loading_state_progress() {
        let mut app = App::new();
        app.init_resource::<AssetLoadingState>();

        let mut loading_state = app.world.resource_mut::<AssetLoadingState>();

        // Create dummy handles
        let handle1 = Handle::<Image>::default();
        let handle2 = Handle::<StandardMaterial>::default();
        let handle3 = Handle::<ColorMaterial>::default();

        // Add assets with different priorities
        loading_state.add_pending_assets(vec![handle1.clone()], LoadPriority::High);
        loading_state.add_pending_assets(vec![handle2.clone()], LoadPriority::Medium);
        loading_state.add_pending_assets(vec![handle3.clone()], LoadPriority::High);

        assert_eq!(loading_state.total_assets, 3);
        assert_eq!(loading_state.visualization_loading_progress, 0.0);
        assert_eq!(loading_state.metrics_loading_progress, 0.0);

        // Mark first visualization asset as loaded
        loading_state.mark_asset_loaded(handle1);
        assert!(loading_state.visualization_loading_progress > 0.0);
        assert_eq!(loading_state.metrics_loading_progress, 0.0);

        // Mark metrics asset as loaded
        loading_state.mark_asset_loaded(handle2);
        assert!(loading_state.visualization_loading_progress > 0.0);
        assert_eq!(loading_state.metrics_loading_progress, 1.0);

        // Mark last visualization asset as loaded
        loading_state.mark_asset_loaded(handle3);
        assert_eq!(loading_state.visualization_loading_progress, 1.0);
        assert_eq!(loading_state.metrics_loading_progress, 1.0);
        assert!(loading_state.is_loading_complete());
    }

    #[test]
    fn test_asset_priority_filtering() {
        let mut loading_state = AssetLoadingState::default();

        let handle1 = Handle::<Image>::default();
        let handle2 = Handle::<StandardMaterial>::default();

        loading_state.add_pending_assets(vec![handle1.clone()], LoadPriority::High);
        loading_state.add_pending_assets(vec![handle2.clone()], LoadPriority::Medium);

        let high_priority = loading_state.get_assets_by_priority(LoadPriority::High);
        let medium_priority = loading_state.get_assets_by_priority(LoadPriority::Medium);

        assert_eq!(high_priority.len(), 1);
        assert_eq!(medium_priority.len(), 1);
        assert!(high_priority.contains(&handle1));
        assert!(medium_priority.contains(&handle2));
    }
} 