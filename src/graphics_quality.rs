// Runtime graphics quality tier — Low / Medium / High.
//
// Sprint 41 introduces texture-backed PBR, triplanar terrain, splat blending,
// post-FX, and other expensive niceties. This module is the single source of
// truth that gates every one of those features so the game can scale down to
// older hardware (Low) and up to a beefy GPU (High).
//
// Selection order at startup (first match wins):
//   1. CLI flag:          `--quality=low|medium|high`
//   2. Persisted config:  ~/.skoffroad/config.json -> "graphics_quality"
//   3. Default:           High
//
// Subsequent commits in this sprint read capability accessors on the resource
// (e.g. `q.triplanar_terrain()`, `q.ssao()`) rather than matching on the enum
// directly, so adding a new tier later doesn't ripple through every plugin.

use bevy::prelude::*;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

pub struct GraphicsQualityPlugin;

impl Plugin for GraphicsQualityPlugin {
    fn build(&self, app: &mut App) {
        // Read CLI flag here so the resource is correct from the very first
        // frame — every other plugin's Startup system will see the right tier.
        let quality = parse_cli_quality().unwrap_or_default();
        info!("graphics_quality: active tier = {}", quality.as_str());
        app.insert_resource(quality);
    }
}

#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GraphicsQuality {
    Low,
    Medium,
    High,
}

impl Default for GraphicsQuality {
    fn default() -> Self {
        Self::High
    }
}

impl GraphicsQuality {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "low" | "l" => Some(Self::Low),
            "medium" | "med" | "m" => Some(Self::Medium),
            "high" | "h" => Some(Self::High),
            _ => None,
        }
    }

    pub fn cycle_next(self) -> Self {
        match self {
            Self::Low => Self::Medium,
            Self::Medium => Self::High,
            Self::High => Self::Low,
        }
    }

    // ---- capability accessors (the rest of the codebase reads these) -----

    /// Texture resolution cap for runtime-loaded materials.
    pub fn texture_size_px(self) -> u32 {
        match self {
            Self::Low => 512,
            Self::Medium => 1024,
            Self::High => 2048,
        }
    }

    /// Use the triplanar terrain shader (no UV stretch on cliffs).
    /// Low keeps the existing vertex-color terrain to stay light on GPU.
    pub fn triplanar_terrain(self) -> bool {
        !matches!(self, Self::Low)
    }

    /// Number of splat blend layers (1 = single material, no blend).
    pub fn splat_layers(self) -> u32 {
        match self {
            Self::Low => 1,
            Self::Medium => 2,
            Self::High => 4,
        }
    }

    /// Enable the wet-surface uniform driven by storm/weather plugins.
    pub fn wet_shader(self) -> bool {
        !matches!(self, Self::Low)
    }

    /// Screen-space ambient occlusion (Bevy 0.18 `ScreenSpaceAmbientOcclusion`).
    pub fn ssao(self) -> bool {
        matches!(self, Self::High)
    }

    /// Bloom post-process. Already used by `bloom_pp` — Low disables it.
    pub fn bloom(self) -> bool {
        !matches!(self, Self::Low)
    }

    /// Filmic tonemap (TonyMcMapface) + ColorGrading.
    pub fn filmic_tonemap(self) -> bool {
        !matches!(self, Self::Low)
    }

    /// Photoreal grass billboards instead of simple cross quads.
    pub fn grass_billboards(self) -> bool {
        !matches!(self, Self::Low)
    }

    /// Photoreal scanned rocks (PBR-textured) instead of procedural mats.
    pub fn photoreal_rocks(self) -> bool {
        !matches!(self, Self::Low)
    }

    /// Glossy car-paint material on the chassis (metallic + low roughness).
    /// Sprint 43.
    pub fn vehicle_clearcoat(self) -> bool {
        !matches!(self, Self::Low)
    }

    /// Render distance multiplier for scatter / LOD plugins.
    pub fn scatter_distance_mul(self) -> f32 {
        match self {
            Self::Low => 0.6,
            Self::Medium => 1.0,
            Self::High => 1.4,
        }
    }
}

// ---------------------------------------------------------------------------
// CLI parsing
// ---------------------------------------------------------------------------

fn parse_cli_quality() -> Option<GraphicsQuality> {
    // Accept either `--quality=low` or `--quality low`.
    let mut args = std::env::args().peekable();
    while let Some(a) = args.next() {
        if let Some(rest) = a.strip_prefix("--quality=") {
            return GraphicsQuality::from_str(rest);
        }
        if a == "--quality" {
            if let Some(v) = args.next() {
                return GraphicsQuality::from_str(&v);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_strings() {
        assert_eq!(GraphicsQuality::from_str("low"), Some(GraphicsQuality::Low));
        assert_eq!(GraphicsQuality::from_str("MEDIUM"), Some(GraphicsQuality::Medium));
        assert_eq!(GraphicsQuality::from_str("h"), Some(GraphicsQuality::High));
        assert_eq!(GraphicsQuality::from_str("ultra"), None);
    }

    #[test]
    fn cycle_wraps() {
        assert_eq!(GraphicsQuality::Low.cycle_next(), GraphicsQuality::Medium);
        assert_eq!(GraphicsQuality::Medium.cycle_next(), GraphicsQuality::High);
        assert_eq!(GraphicsQuality::High.cycle_next(), GraphicsQuality::Low);
    }

    #[test]
    fn capabilities_scale_monotonically() {
        let l = GraphicsQuality::Low;
        let m = GraphicsQuality::Medium;
        let h = GraphicsQuality::High;
        assert!(l.texture_size_px() < m.texture_size_px());
        assert!(m.texture_size_px() < h.texture_size_px());
        assert!(l.splat_layers() <= m.splat_layers());
        assert!(m.splat_layers() <= h.splat_layers());
        assert!(!l.ssao() && !m.ssao() && h.ssao());
        assert!(!l.bloom() && m.bloom() && h.bloom());
    }
}
