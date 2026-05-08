# skoffroad — PRD v3.1
## Asset import pipeline + Spintires-quality visuals

**Date:** 2026-05-07 (extends PRD_v3.md)
**Trigger:** User asked if we can leverage Spintires assets to bring quality "similar or better than Spintires."

## Research summary (full doc: `next/ASSET_IMPORT_RESEARCH.md`)

| Topic | Finding |
|---|---|
| Spintires asset format | `.x` (legacy DirectX) + `.xml`. Proprietary. **No public extractor.** |
| Spintires mod license | Workshop ToS does NOT grant redistribution outside Spintires. **Using = copyright infringement.** |
| MudRunner / SnowRunner | Saber's proprietary Swarm Engine. Same legal block. |
| Bevy 0.18 native | glTF 2.0 / GLB ✅. OBJ via `bevy_obj`. STL via `bevy_stl`. **No FBX, no USD.** |
| CC0 sources | Quaternius (vehicles), Kenney.nl (low-poly), Poly Haven (photogrammetry rocks/trees/HDRI), AmbientCG (PBR terrain), Sketchfab CC0 filter |
| Heightmaps | 16-bit grayscale PNG is standard. Tools: World Machine, Gaea, Houdini, Blender A.N.T. |

## Verdict (engineering)

**Do not reverse-engineer Spintires.** Modern CC0 photogrammetry (Poly Haven scans, Quaternius models) is *higher quality* than 2014-era Spintires assets, fully legal, and Bevy supports the formats natively.

**Build path:**
1. glTF/GLB import folder for vehicles + props
2. 16-bit PNG heightmap loader for terrain
3. `mod.toml` manifest for user content with license validation
4. CC0 attribution screen

## Sprint plan (37–40)

### Sprint 37 — Asset Import Pipeline
1. **glb_loader.rs** — auto-scan `assets/vehicles/*.glb`, register each as a selectable vehicle skin; hot-reload via `Res<AssetServer>`
2. **heightmap_loader.rs** — read 16-bit PNG from `assets/maps/*.png` → mesh + collider; key `M` cycles maps
3. **asset_manifest.rs** — TOML config in `assets/manifest.toml` declaring vehicle classes, masses, default skins
4. **asset_browser.rs** — F11 modal listing every loaded asset with name + license tag
5. **asset_attribution.rs** — credits roll extension for CC0 attribution per asset

### Sprint 38 — Spintires-quality terrain
1. **terrain_splatmap.rs** — vertex-color blended grass/dirt/rock textures across terrain
2. **terrain_normal_map.rs** — generated normal maps from heightmap derivatives
3. **terrain_lod.rs** — chunked LOD; full detail near camera, simplified at distance
4. **mud_depth_visual.rs** — persistent rut depth in mud zones (terrain mesh deformation)
5. **water_reflective.rs** — proper reflective water material on lakes/puddles

### Sprint 39 — Drivetrain realism
1. **transmission.rs** — actual gear shifting (5-speed manual or sequential)
2. **transfer_case.rs** — consolidated 2WD/4WD/Low-range selector with audio click
3. **winch_cable_physics.rs** — segmented cable with realistic sag (replaces straight cylinder)
4. **engine_audio_layered.rs** — 4-layer engine: idle, low-rev, mid, high-rev crossfade
5. **fuel_consumption_real.rs** — fuel burn proportional to RPM × throttle

### Sprint 40 — Photo-quality props
1. **photoreal_rocks.rs** — load Poly Haven rock GLBs from `assets/rocks/*.glb` and scatter
2. **terrain_grass_blades.rs** — actual billboarded grass blade meshes (CC0 textures)
3. **hdr_skybox.rs** — HDRI skybox support (Bevy 0.18 has IBL)
4. **terrain_decals.rs** — procedural roads/trails as decals on terrain
5. **photo_hud.rs** — F8 photo mode upgraded with framing rule-of-thirds + depth-of-field

## Non-goals reinforced

- ❌ No Spintires asset extraction (legal)
- ❌ No FBX / USD import (Bevy doesn't have it)
- ❌ No multiplayer / netcode
- ❌ No proprietary engine port

## Acceptance for v0.7.0 (Sprint 40 complete)

- Drop a `.glb` into `assets/vehicles/` and have it appear in the garage cycle
- Drop a 16-bit PNG into `assets/maps/` and have it become a selectable map
- Load Poly Haven rocks via `mod.toml` and see them in the scene
- Attribution screen lists every asset by source + license

## License posture

All shipped assets must be CC0, CC-BY (with attribution), or original procedural primitives. CI must reject mods without a license tag in their manifest.
