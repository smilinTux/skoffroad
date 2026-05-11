# Billboard / Sponsor Scatter — System Spec

## Goal

Allow a JSON-defined "brand pack" to populate the world with sponsor-textured billboards along trail edges — UTM-tagged for click-through tracking. Drop-in additive plugin; no edits to existing `scatter.rs` or `billboards.rs`.

## Constraints

- Reuse existing scatter grid math from `src/scatter.rs:23` (50×50 cells over 200m, seeded by `TERRAIN_SEED`).
- Native build must compile (no wasm-only deps in the hot path); click-through is a no-op on native.
- All sponsor data lives in `assets/brand_packs/<id>.json`. Game ships with a default `_house.json` so the world is never visually empty.
- Sponsor impression / click events route through the existing `events.rs` ring buffer so analytics can piggyback later.

## File layout

```
assets/brand_packs/
  _house.json            # default house ads (skworld self-promo)
  example_arb.json       # template for a real brand pack
  README.md              # how to author a pack

src/
  sponsor_scatter.rs     # new plugin (this spec)
  brand_pack.rs          # new: JSON loader + active pack resource
  ad_sdk_bridge.rs       # new: wasm-bindgen interop for click-through
```

## Data model

```jsonc
// assets/brand_packs/example_arb.json
{
  "id": "arb_summer_2026",
  "display_name": "ARB Summer Series",
  "version": 1,

  "splash": {
    "logo_texture": "brand_packs/arb/logo.png",
    "tagline": "Trail-tested since 1975",
    "cta_url": "https://arb.com/summer?utm_source=skoffroad&utm_campaign=summer2026"
  },

  "livery": {
    "id": "arb_trail_white",
    "name": "ARB Trail White",
    "base_color": [0.95, 0.93, 0.88],
    "decal_texture": "brand_packs/arb/door_decal.png",
    "unlock_requires_video": false
  },

  "billboards": [
    {
      "texture": "brand_packs/arb/board_winch.png",
      "weight": 1.0,
      "click_url": "https://arb.com/winch?utm_source=skoffroad&utm_medium=billboard"
    },
    {
      "texture": "brand_packs/arb/board_tent.png",
      "weight": 1.0,
      "click_url": "https://arb.com/rtt?utm_source=skoffroad&utm_medium=billboard"
    }
  ],

  "scatter": {
    "density": 0.10,          // 0..1, fraction of grid cells eligible
    "min_distance_m": 12.0,   // billboards no closer than this to each other
    "edge_only": true,        // restrict to trail edges (slope band)
    "slope_band": [0.05, 0.20]
  }
}
```

## Placement algorithm

Mirror `scatter.rs`:

1. Iterate the same 50×50 grid.
2. For each cell, compute slope via `compute_slope(x, z)`.
3. Eligibility = `slope_band.0 <= slope <= slope_band.1` (gentle slopes only — trail edges).
4. Use a fresh Perlin instance seeded with `TERRAIN_SEED + 0xB1B0` (unique salt to avoid clustering with trees/rocks).
5. If `perlin(cell) > 1.0 - density`, mark cell as a candidate.
6. Among candidates, accept in scan order if `min_distance_m` is respected against previously placed sponsors (cheap O(n²) — there's only ~20–60 sponsors per map).
7. For each placed sponsor, deterministically pick a billboard from the pack via `hash2(cell_i, cell_j, salt) → idx` weighted by `weight`.

Snap Y to `terrain_height_at(x, z)` and rotate to face the nearest trail tangent (approximated as the gradient of the height field — sponsor face perpendicular to the gradient looks "facing the road").

## Bevy components

```rust
#[derive(Component)]
pub struct SponsorBillboard {
    pub pack_id: String,
    pub billboard_idx: usize,
    pub click_url: String,
}

#[derive(Resource, Default)]
pub struct ActiveBrandPack(pub Option<BrandPack>);

#[derive(Resource, Default)]
pub struct SponsorAnalytics {
    pub impressions: HashMap<String, u32>,  // pack_id → count
    pub clicks: HashMap<String, u32>,
    pub last_flushed_at: f64,
}
```

## Plugin shape

```rust
pub struct SponsorScatterPlugin;

impl Plugin for SponsorScatterPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ActiveBrandPack>()
           .init_resource::<SponsorAnalytics>()
           .add_systems(Startup, (load_default_pack, spawn_sponsors).chain())
           .add_systems(Update, (
               track_visibility,        // increment impressions when on-camera
               handle_click_interaction, // mouse-pick raycast on click
               flush_analytics_to_localstorage.run_if(on_timer(secs(10.0))),
           ));
    }
}
```

## Events

Add to `events.rs::GameEvent`:

```rust
GameEvent::SponsorImpression { pack_id: String, billboard_idx: usize, dwell_s: f32 },
GameEvent::SponsorClick      { pack_id: String, billboard_idx: usize, click_url: String },
```

## Click flow

1. Player clicks anywhere → world raycast (existing helpers in `src/pickable.rs` if present, else add one).
2. If hit entity has `SponsorBillboard`, emit `SponsorClick` event.
3. On WASM, call `window.open(click_url, "_blank")` via `web-sys::Window::open`.
4. On native, log to console and increment counter.

## Visual integration

Two implementation paths — start with **A**, upgrade to **B** later:

**A. Reuse existing billboard geometry** — extend `billboards.rs::spawn_billboards` to accept an `Option<&BrandPack>` parameter, swap the panel `StandardMaterial.base_color_texture` to the brand texture instead of solid color. Lowest lift.

**B. Bigger, varied geometry** — leaning roadside signs, trail-marker arrow signs, fabric banners between two posts (cloth-look). Add as new spawn fn in `sponsor_scatter.rs`.

## Analytics flush

Every 10s, serialize `SponsorAnalytics` to `localStorage["skoffroad/sponsor_analytics"]`. A separate JS snippet in `index.html` can POST it to a collection endpoint:

```javascript
setInterval(() => {
  const raw = localStorage.getItem("skoffroad/sponsor_analytics");
  if (!raw) return;
  navigator.sendBeacon("https://api.skworld.io/v1/sponsor/ingest", raw);
}, 30000);
```

## Privacy

- No PII collected.
- Default `click_url` has UTM tags only — no fingerprinting.
- `assets/brand_packs/_house.json` is shipped open-source as the reference / example.

## Acceptance criteria

- [ ] `cargo build --release` succeeds on native.
- [ ] `trunk build --release` succeeds for WASM.
- [ ] Loading `_house.json` places 20–60 billboards visible from spawn within 200m radius.
- [ ] Clicking a billboard on WASM opens `click_url` in a new tab.
- [ ] Headless test (`cargo test sponsor_scatter`) verifies deterministic placement for a fixed seed.
- [ ] `assets/brand_packs/_house.json` exists and is valid.
- [ ] No regression in existing tests (`cargo test`).

## Out of scope (phase 2)

- Dynamic sponsor swap mid-session.
- Animated / video billboards.
- A/B testing different creatives per pack.
- Real-time bidding integration (Bidstack / Anzu).
