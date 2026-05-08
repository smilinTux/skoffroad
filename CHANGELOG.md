# Changelog

All notable changes to the skoffroad game project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.9.0] — 2026-05-08 — Sprint 45 "Skrambler"

### Added
- **Skrambler SK** — the cuboid Jeep silhouette is now a proper open-top
  off-roader: 7-slot grille, full roll cage (4 vertical bars + front/rear/
  side crossbars), fender flares on every wheel, driver/passenger doors,
  wing mirrors, roof light bar with 4 LED spots, tailgate-mounted spare
  tire. ~30 new child entities on the chassis, all primitives — zero
  licensing risk, ships in tree.
- **TJ-style paint palette** in `livery.rs` (cycle with 1–6):
  Cherry Crawler, Forest Trail, Sahara Tan, Khaki Patrol, Midnight
  Skrambler, Glacier Blue. Names + RGB approximate real Jeep TJ era
  factory colours.
- **`docs/USER_VEHICLES.md`** — license rules, drop-in `assets/vehicles/`
  layout, `vehicle.toml` schema, Blender export tips. The infrastructure
  for user-supplied GLB vehicles will land in the next sprint.

### Notes
- A previously-existing `1999_jeep_wrangler_tj.glb` on the dev machine was
  CC-BY-NC-SA-4.0 (incompatible with the GPL-3 game) — not bundled.
  Procedural Skrambler ships in its place.

## [0.8.3] — 2026-05-08 — Sprint 44 "chrome rims"

### Added
- Chrome wheel rims on Medium+ (metallic 0.95, perceptual roughness
  0.18, reflectance 0.85, near-white base color). Replaces the matte
  aluminium 0.22-tone material with something that catches the sun at
  speed. Low keeps the legacy matte rims.

## [0.8.2] — 2026-05-08 — Sprint 43 "vehicle paint"

### Added
- Glossy car-paint material on the chassis (Medium+ via
  `GraphicsQuality::vehicle_clearcoat()`): metallic 0.55, perceptual
  roughness 0.32, reflectance 0.65. Reads as clearcoat under daylight
  even without Bevy's optional clearcoat feature flag. Low keeps the
  matte legacy material untouched.

## [0.8.1] — 2026-05-08 — Sprint 42 "foliage" + Sprint 41 hotfix

### Added
- **Cross-triangle grass blade mesh** in `grass_tufts.rs`. Replaces the
  cuboid blades with two perpendicular tris (6 verts) and tip/base vertex
  colors that fade dark → bright green. Reads as grass, not as green sticks.
- **CPU wind sway**: each tuft tilts about an axis perpendicular to
  `WindState.direction` with a sin(t·1.6 + phase) lean. Phase derives from
  world position so the field sways non-uniformly. Amplitude scales with
  `WindState.speed_mps` (cap ~8°).
- Both gated by `GraphicsQuality::grass_billboards()` so Low keeps the cuboid
  path with no per-frame work.

### Fixed (Sprint 41 hotfix)
- Triplanar `ExtendedMaterial` shipped a bind-group layout mismatch at
  runtime (`Shader global ResourceBinding { group: 2, binding: 100 } is not
  available in the pipeline layout`). For the v0.8 ship we fall back to a
  regular textured `StandardMaterial` on the terrain (dirt pack: albedo +
  normal + metallic-roughness). Loses triplanar projection and the 4-way
  splat blend, keeps the photoreal look. Investigation parked in
  `docs/PARKING_LOT.md`.

## [0.8.0] — 2026-05-08 — Sprint 41 "photoreal pass"

### Added
- **`GraphicsQuality` runtime tier** (`Low` / `Medium` / `High`) gating every
  expensive feature, with capability accessors so plugins read intent
  (`triplanar_terrain()`, `ssao()`, `wet_shader()`, etc.) instead of matching
  on the enum directly.
- `--quality=low|medium|high` CLI flag (overrides persisted value).
- Persisted as `graphics_quality` in `~/.skoffroad/config.json`.
- **CC0 PBR material packs** under `assets/materials/terrain/{dirt,grass,rock,mud}/`
  (albedo + normal-GL + roughness, 1K JPG, ~19 MB total). Sourced from
  ambientCG; see `assets/materials/MATERIALS.md` for attribution.
- **`scripts/fetch_materials.sh`** — idempotent CC0 material downloader
  (`--force` to re-pull).
- **Triplanar terrain shader** (`assets/shaders/triplanar_terrain.wgsl`) +
  `terrain_pbr.rs` `ExtendedMaterial<StandardMaterial, TriplanarTerrainExt>`.
  World-space projection on three axes prevents UV stretch on cliffs;
  two-frequency sampling per layer (close + far) breaks visible tiling.
- **4-channel splat blend** across dirt / grass / rock / mud, weighted
  procedurally from world-space slope and height.
- **Wet-surface shader hook**: `wetness` uniform driven from `StormState`
  with an exponential ease (~2 s time constant). Soaked terrain darkens
  (~30%) and roughness drops (~55%); mud is biased opposite so puddles
  read on rock/grass/dirt rather than uniformly.
- **PBR-textured rocks** in `photoreal_rocks.rs` (Medium+ uses the rock
  pack textures; Low keeps the Sprint 40 procedural material).
- **`PostFxPlugin`** attaches camera post-FX in PostStartup:
  - Medium+: `Tonemapping::AgX`, tuned `ColorGrading`
  - High: `ScreenSpaceAmbientOcclusion` (Low quality preset for perf)
- **Quality picker in pause overlay**. Backslash (`\`) cycles
  Low → Medium → High → Low. Splat blend, wet shader and bloom respond
  live; SSAO/tonemap need a restart (PostStartup-attached).

### Changed
- `terrain.rs` runs in `PostStartup` so `TerrainPbrPlugin`'s Startup load
  finishes first; spawn branches on quality tier.
- `headless.rs` pins `GraphicsQuality::Low` so drive_test never sees
  triplanar / texture loading.

### Notes
- HTTPS auto-provisioned and verified at `skoffroad.skworld.io`.

## [0.7.0] — 2026-05-08
- Project rename: `sandk-offroad` → `skoffroad`. Legacy root archived
  under `legacy/`. GPL-3.0-or-later license added; Cargo.toml declares
  the license; README has license section. Repo moved to `smilinTux` org.

## [0.6.x] — 2026-04 → 2026-05
- Sprint 31–40 (skipping the boring entries): wheel-cam, articulation,
  rock garden, hillclimb, low-range, airdown, winch, engine torque,
  4WD, diff lock, interior, V8 bay, asset import pipeline,
  Spintires-quality terrain (splatmap + LOD + normals), drivetrain
  realism (gears, transfer case, layered audio), photo-quality props
  (HDR skybox, photoreal rocks, terrain decals).

## [0.1.0] — 2024-04-17
- Initial repository setup; basic directory structure; README and
  development environment configuration; core dependencies in
  Cargo.toml; documentation framework.

[0.9.0]: https://github.com/smilinTux/skoffroad/compare/v0.8.3...v0.9.0
[0.8.3]: https://github.com/smilinTux/skoffroad/compare/v0.8.2...v0.8.3
[0.8.2]: https://github.com/smilinTux/skoffroad/compare/v0.8.1...v0.8.2
[0.8.1]: https://github.com/smilinTux/skoffroad/compare/v0.8.0...v0.8.1
[0.8.0]: https://github.com/smilinTux/skoffroad/compare/v0.7.0...v0.8.0
[0.7.0]: https://github.com/smilinTux/skoffroad/compare/v0.6.12...v0.7.0
[0.1.0]: https://github.com/smilinTux/skoffroad/releases/tag/v0.1.0
