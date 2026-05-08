# Changelog

All notable changes to the skoffroad game project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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

[0.8.0]: https://github.com/smilinTux/skoffroad/compare/v0.7.0...v0.8.0
[0.7.0]: https://github.com/smilinTux/skoffroad/compare/v0.6.12...v0.7.0
[0.1.0]: https://github.com/smilinTux/skoffroad/releases/tag/v0.1.0
