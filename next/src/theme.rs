// Centralized color palette + UI sizing constants. Other modules can
// gradually adopt these for visual consistency without forcing a refactor.
//
// Public API:
//   ThemePlugin (no-op)
//   palette::* color constants
//   sizes::* spacing constants

use bevy::prelude::*;

pub struct ThemePlugin;

impl Plugin for ThemePlugin {
    fn build(&self, _: &mut App) {}
}

pub mod palette {
    use bevy::prelude::Color;

    // Backgrounds
    pub const BG_DARK_85: Color = Color::srgba(0.04, 0.04, 0.06, 0.85);
    pub const BG_DARK_75: Color = Color::srgba(0.05, 0.05, 0.07, 0.75);
    pub const BG_DARK_95: Color = Color::srgba(0.04, 0.04, 0.08, 0.95);

    // Text
    pub const TEXT_PRIMARY: Color = Color::WHITE;
    pub const TEXT_HEADER:  Color = Color::srgb(0.75, 0.75, 0.75);
    pub const TEXT_DIM:     Color = Color::srgb(0.55, 0.55, 0.55);

    // Accents
    pub const ACCENT_YELLOW: Color = Color::srgb(1.0, 0.9, 0.3);
    pub const ACCENT_CYAN:   Color = Color::srgb(0.4, 0.95, 1.0);
    pub const ACCENT_GREEN:  Color = Color::srgb(0.4, 1.0, 0.5);
    pub const ACCENT_RED:    Color = Color::srgb(1.0, 0.4, 0.4);
    pub const ACCENT_PURPLE: Color = Color::srgb(0.85, 0.6, 1.0);

    // Medals
    pub const MEDAL_GOLD:   Color = Color::srgb(1.0, 0.85, 0.20);
    pub const MEDAL_SILVER: Color = Color::srgb(0.85, 0.85, 0.90);
    pub const MEDAL_BRONZE: Color = Color::srgb(0.85, 0.55, 0.30);
}

pub mod sizes {
    pub const PANEL_PAD_X:    f32 = 12.0;
    pub const PANEL_PAD_Y:    f32 = 8.0;
    pub const FONT_BODY:      f32 = 13.0;
    pub const FONT_HEADER:    f32 = 22.0;
    pub const FONT_HUGE:      f32 = 48.0;
    pub const HUD_TOP_MARGIN:  f32 = 14.0;
    pub const HUD_SIDE_MARGIN: f32 = 14.0;
}
