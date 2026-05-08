// Central font asset loader: avoids each HUD module loading its own font.
// Uses bevy's default font for now; this module is the seam where a custom
// TTF/OTF would be loaded later.
//
// Public API:
//   FontAssetsPlugin
//   FontAssets (resource exposing handles)

use bevy::prelude::*;

pub struct FontAssetsPlugin;

impl Plugin for FontAssetsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FontAssets>()
            .add_systems(Startup, load_fonts);
    }
}

/// Attempts to load font files from `assets/fonts/`.  If the files are absent
/// Bevy will log a soft asset-server warning but the default (built-in) font
/// is used automatically because `Handle::default()` is the zero handle.
fn load_fonts(asset_server: Res<AssetServer>, mut fonts: ResMut<FontAssets>) {
    fonts.primary = asset_server.load("fonts/primary.ttf");
    fonts.display = asset_server.load("fonts/display.ttf");
}

/// Resource that centralises font handles for the whole app.
/// Both fields default to `Handle::default()`, which Bevy treats as the
/// built-in font, so callers don't need to check whether the files exist.
#[derive(Resource, Default, Clone)]
pub struct FontAssets {
    pub primary: Handle<Font>,
    pub display: Handle<Font>,
}

impl FontAssets {
    /// Returns a `TextFont` suitable for small labels (14 px, primary face).
    pub fn label_font(&self) -> TextFont {
        TextFont {
            font: self.primary.clone(),
            font_size: 14.0,
            ..default()
        }
    }

    /// Returns a `TextFont` suitable for headers / titles (display face, caller
    /// chooses the size).
    pub fn header_font(&self, size: f32) -> TextFont {
        TextFont {
            font: self.display.clone(),
            font_size: size,
            ..default()
        }
    }
}
