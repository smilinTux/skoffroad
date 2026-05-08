use bevy::prelude::*;
use bevy::app::App;
use bevy::asset::{AssetPlugin, Handle};
use bevy::log::LogPlugin;
use tempfile::TempDir;
use std::path::PathBuf;

/// Test fixture for setting up a minimal Bevy app with required plugins
pub struct TestApp {
    pub app: App,
    pub temp_dir: TempDir,
    pub asset_path: PathBuf,
}

impl Default for TestApp {
    fn default() -> Self {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let asset_path = temp_dir.path().join("assets");
        std::fs::create_dir_all(&asset_path).expect("Failed to create asset directory");

        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            AssetPlugin::default(),
            LogPlugin::default(),
        ));

        Self {
            app,
            temp_dir,
            asset_path,
        }
    }
}

impl TestApp {
    /// Add a plugin to the test app
    pub fn add_plugin<T: Plugin>(&mut self, plugin: T) -> &mut Self {
        self.app.add_plugin(plugin);
        self
    }

    /// Add a resource to the test app
    pub fn add_resource<T: Resource>(&mut self, resource: T) -> &mut Self {
        self.app.insert_resource(resource);
        self
    }

    /// Add a system to the test app
    pub fn add_system<Params>(&mut self, system: impl IntoScheduleConfig<Params>) -> &mut Self {
        self.app.add_systems(Update, system);
        self
    }

    /// Run the app for a specified number of frames
    pub fn run_frames(&mut self, frames: usize) -> &mut Self {
        for _ in 0..frames {
            self.app.update();
        }
        self
    }

    /// Get a resource from the app
    pub fn get_resource<T: Resource>(&self) -> Option<&T> {
        self.app.world.get_resource::<T>()
    }

    /// Get a resource mutably from the app
    pub fn get_resource_mut<T: Resource>(&mut self) -> Option<&mut T> {
        self.app.world.get_resource_mut::<T>()
    }

    /// Create a test asset file in the temp directory
    pub fn create_test_asset(&self, filename: &str, content: &[u8]) -> PathBuf {
        let path = self.asset_path.join(filename);
        std::fs::write(&path, content).expect("Failed to write test asset");
        path
    }
}

/// Helper function to create a test image asset
pub fn create_test_image(width: u32, height: u32) -> Vec<u8> {
    let size = (width * height * 4) as usize;
    vec![255; size] // Create a white RGBA image
}

/// Helper function to create a test audio asset
pub fn create_test_audio() -> Vec<u8> {
    // Create a minimal valid WAV file
    vec![
        b'R', b'I', b'F', b'F', // "RIFF"
        44, 0, 0, 0,            // File size - 8
        b'W', b'A', b'V', b'E', // "WAVE"
        b'f', b'm', b't', b' ', // "fmt "
        16, 0, 0, 0,            // Subchunk1Size
        1, 0,                   // AudioFormat (PCM)
        1, 0,                   // NumChannels (Mono)
        68, 172, 0, 0,         // SampleRate (44100)
        68, 172, 0, 0,         // ByteRate
        1, 0,                   // BlockAlign
        8, 0,                   // BitsPerSample
        b'd', b'a', b't', b'a', // "data"
        4, 0, 0, 0,            // Subchunk2Size
        0, 0, 0, 0             // Sample data
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_initialization() {
        let test_app = TestApp::default();
        assert!(test_app.temp_dir.path().exists());
        assert!(test_app.asset_path.exists());
    }

    #[test]
    fn test_resource_management() {
        let mut test_app = TestApp::default();
        
        #[derive(Resource)]
        struct TestResource(i32);
        
        test_app.add_resource(TestResource(42));
        assert_eq!(test_app.get_resource::<TestResource>().unwrap().0, 42);
    }

    #[test]
    fn test_system_execution() {
        let mut test_app = TestApp::default();
        
        #[derive(Resource, Default)]
        struct Counter(usize);
        
        fn increment_counter(mut counter: ResMut<Counter>) {
            counter.0 += 1;
        }
        
        test_app
            .add_resource(Counter::default())
            .add_system(increment_counter)
            .run_frames(3);
            
        assert_eq!(test_app.get_resource::<Counter>().unwrap().0, 3);
    }

    #[test]
    fn test_asset_creation() {
        let test_app = TestApp::default();
        let test_data = b"test data";
        let path = test_app.create_test_asset("test.txt", test_data);
        
        assert!(path.exists());
        assert_eq!(std::fs::read(path).unwrap(), test_data);
    }
} 