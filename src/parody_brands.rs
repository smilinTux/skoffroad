// Parody off-road brand catalog.
//
// Defines 12 recognisable-but-legally-distinct parody brands for in-game
// advertising signage. Each brand is deliberately distinct in name and
// never uses exact real-world logos or trademarks.
//
// Public API (Sprint 72 liveries + Sprint 73 garage will consume this):
//   ParodyBrandsPlugin
//   ParodyBrands           (resource — immutable catalog)
//   ParodyBrand            (data type)
//   BrandCategory          (enum)
//   AdSign                 (component — placed on every spawned sign entity)
//   brand_primary_color    (helper — Color from brand index)
//   brand_secondary_color  (helper — Color from brand index)

use bevy::prelude::*;

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct ParodyBrandsPlugin;

impl Plugin for ParodyBrandsPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ParodyBrands::catalog());
    }
}

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

/// Off-road product category for a parody brand.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrandCategory {
    Tires,
    Winches,
    Shocks,
    Lights,
    Wheels,
    Recovery,
    Lifestyle,
}

/// A single parody brand entry.
#[derive(Debug, Clone)]
pub struct ParodyBrand {
    /// Short identifier used in analytics / livery keys.
    pub id: &'static str,
    /// Human-readable display name shown on signs.
    pub display_name: &'static str,
    /// Category (used by Sprint 72/73 to filter brands by type).
    pub category: BrandCategory,
    /// Primary panel color (RGB, 0–1).
    pub primary: [f32; 3],
    /// Secondary / accent color (RGB, 0–1).
    pub secondary: [f32; 3],
    /// Short tagline shown below the brand name on larger signs.
    pub tagline: &'static str,
}

/// Immutable catalog resource, accessible to any system.
#[derive(Resource, Debug, Clone)]
pub struct ParodyBrands {
    pub brands: Vec<ParodyBrand>,
}

impl ParodyBrands {
    /// Build the canonical 12-brand catalog.
    pub fn catalog() -> Self {
        Self {
            brands: vec![
                // 0 — Tires — Maxxis-ish
                ParodyBrand {
                    id: "apexis",
                    display_name: "APEXIS",
                    category: BrandCategory::Tires,
                    primary:   [0.80, 0.10, 0.10], // deep red
                    secondary: [0.95, 0.92, 0.88], // off-white
                    tagline: "CONQUER EVERY CORNER",
                },
                // 1 — Tires — BFGoodrich-ish
                ParodyBrand {
                    id: "bouldergrip",
                    display_name: "BOULDERGRIP",
                    category: BrandCategory::Tires,
                    primary:   [0.10, 0.10, 0.10], // black
                    secondary: [1.00, 0.50, 0.05], // orange
                    tagline: "GRIP WHAT OTHERS CAN'T",
                },
                // 2 — Tires — Goodyear-ish
                ParodyBrand {
                    id: "greatyear",
                    display_name: "GREATYEAR",
                    category: BrandCategory::Tires,
                    primary:   [0.10, 0.20, 0.65], // royal blue
                    secondary: [1.00, 0.82, 0.10], // gold
                    tagline: "A GREAT YEAR ON ANY TERRAIN",
                },
                // 3 — Winches — Warn-ish
                ParodyBrand {
                    id: "warden",
                    display_name: "WARDEN",
                    category: BrandCategory::Winches,
                    primary:   [0.08, 0.08, 0.08], // near-black
                    secondary: [0.95, 0.15, 0.10], // warning red
                    tagline: "PULL YOUR WAY OUT",
                },
                // 4 — Winches — Smittybilt-ish
                ParodyBrand {
                    id: "smithybuilt",
                    display_name: "SMITHYBUILT",
                    category: BrandCategory::Winches,
                    primary:   [0.20, 0.45, 0.20], // army green
                    secondary: [0.88, 0.84, 0.78], // sand
                    tagline: "BUILT TO PULL HARD",
                },
                // 5 — Shocks — Fox Racing-ish
                ParodyBrand {
                    id: "fawx_racing",
                    display_name: "FAWX RACING",
                    category: BrandCategory::Shocks,
                    primary:   [0.95, 0.55, 0.05], // fox orange
                    secondary: [0.08, 0.08, 0.08], // black
                    tagline: "LIVE WILD. RIDE HARDER.",
                },
                // 6 — Shocks — King-ish
                ParodyBrand {
                    id: "crown_shox",
                    display_name: "CROWN SHOX",
                    category: BrandCategory::Shocks,
                    primary:   [0.05, 0.05, 0.45], // deep navy
                    secondary: [0.85, 0.72, 0.20], // crown gold
                    tagline: "RULE THE ROUGH STUFF",
                },
                // 7 — Shocks — Bilstein-ish
                ParodyBrand {
                    id: "bilsteen",
                    display_name: "BILSTEEN",
                    category: BrandCategory::Shocks,
                    primary:   [0.95, 0.90, 0.10], // bright yellow
                    secondary: [0.08, 0.08, 0.08], // black
                    tagline: "GERMAN PRECISION. DIRT TESTED.",
                },
                // 8 — Lights — Rigid Industries-ish
                ParodyBrand {
                    id: "ridge_led",
                    display_name: "RIDGE LED",
                    category: BrandCategory::Lights,
                    primary:   [0.08, 0.08, 0.08], // black
                    secondary: [0.35, 0.85, 1.00], // electric blue
                    tagline: "SEE FURTHER. GO FURTHER.",
                },
                // 9 — Lights — Baja Designs-ish
                ParodyBrand {
                    id: "mesa_designs",
                    display_name: "MESA DESIGNS",
                    category: BrandCategory::Lights,
                    primary:   [0.75, 0.18, 0.18], // brick red
                    secondary: [0.95, 0.95, 0.95], // white
                    tagline: "LIGHT UP THE BACKCOUNTRY",
                },
                // 10 — Wheels — Method Race Wheels-ish
                ParodyBrand {
                    id: "modus_wheels",
                    display_name: "MODUS WHEELS",
                    category: BrandCategory::Wheels,
                    primary:   [0.60, 0.60, 0.62], // machined silver
                    secondary: [0.12, 0.12, 0.12], // near-black
                    tagline: "RACE-PROVEN STRENGTH",
                },
                // 11 — Recovery/Armor — ARB-ish
                ParodyBrand {
                    id: "ark_4x4",
                    display_name: "ARK 4X4",
                    category: BrandCategory::Recovery,
                    primary:   [0.75, 0.32, 0.08], // rust orange
                    secondary: [0.12, 0.12, 0.12], // near-black
                    tagline: "ARMOUR YOUR ADVENTURE",
                },
                // (extras for variety — total kept at 12)
            ],
        }
    }

    /// Return the number of brands in the catalog.
    pub fn len(&self) -> usize {
        self.brands.len()
    }

    /// Return an iterator over all brands.
    pub fn iter(&self) -> impl Iterator<Item = &ParodyBrand> {
        self.brands.iter()
    }
}

// ---------------------------------------------------------------------------
// AdSign component
// ---------------------------------------------------------------------------

/// Marker + metadata placed on every ad sign entity.
///
/// `brand_id` indexes into `ParodyBrands::brands`.  Future analytics systems
/// can query `With<AdSign>` to count impressions or drive HUD callouts.
#[derive(Component, Debug, Clone, Copy)]
pub struct AdSign {
    /// Index into `ParodyBrands::brands`.
    pub brand_id: usize,
}

// ---------------------------------------------------------------------------
// Color helpers (useful for systems that don't hold the full catalog)
// ---------------------------------------------------------------------------

/// Bevy `Color` for the primary color of brand `idx`.
/// Falls back to a generic orange if `idx` is out of range.
pub fn brand_primary_color(brands: &ParodyBrands, idx: usize) -> Color {
    brands.brands.get(idx).map(|b| Color::srgb(b.primary[0], b.primary[1], b.primary[2]))
        .unwrap_or(Color::srgb(0.8, 0.4, 0.1))
}

/// Bevy `Color` for the secondary/accent color of brand `idx`.
pub fn brand_secondary_color(brands: &ParodyBrands, idx: usize) -> Color {
    brands.brands.get(idx).map(|b| Color::srgb(b.secondary[0], b.secondary[1], b.secondary[2]))
        .unwrap_or(Color::WHITE)
}

// ---------------------------------------------------------------------------
// Deterministic brand-picker (shared helper for placement systems)
// ---------------------------------------------------------------------------

/// Pick a brand index deterministically from a hash value `r` in [0, 1).
/// Uses uniform distribution across the catalog.
pub fn pick_brand(brands: &ParodyBrands, r: f32) -> usize {
    let n = brands.brands.len();
    if n == 0 { return 0; }
    ((r.clamp(0.0, 0.9999) * n as f32) as usize).min(n - 1)
}

// ---------------------------------------------------------------------------
// Hash helper (copied from sponsor_scatter.rs pattern — no dep on that module)
// ---------------------------------------------------------------------------

/// Cheap integer hash → [0, 1) used by placement systems for determinism.
pub fn brand_hash(a: i32, b: i32, salt: u32) -> f32 {
    let mut v = (a.wrapping_mul(374761393))
        .wrapping_add(b.wrapping_mul(668265263))
        .wrapping_add(salt as i32);
    v ^= v >> 13;
    v = v.wrapping_mul(1274126177);
    v ^= v >> 16;
    (v as u32) as f32 / u32::MAX as f32
}
