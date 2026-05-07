# Asset Import Research — Sandk Offroad (Bevy 0.18, Rust)

Research date: 2026-05-05
Question: Can we leverage Spintires/MudRunner/SnowRunner assets, and what's the right import pipeline for a Bevy 0.18 off-road game?

---

## 1. Spintires Asset Format

- **Format**: Pairs of `.x` (DirectX retained-mode mesh, Microsoft legacy format) + `.xml` (metadata: textures, physics, joints, suspension, properties).
- **Origin**: `.x` is a documented Microsoft format from the DirectX SDK (deprecated). Spintires authors meshes via 3ds Max using the community **kW X-port** exporter.
- **Proprietary?** The `.x` container is documented; Spintires' *use* of it (XML schema, physics blob conventions, packaging into the game's archives) is undocumented and proprietary.
- **Extractors**: **No public tool exists** to extract `.x`/`.xml` from a shipped Spintires/MudRunner install. Workshop mods technically contain `.x`/`.xml` but the editor refuses to load mods downloaded from Workshop back as source. The community has repeatedly confirmed this on the Focus forums and Steam discussions.
- **Legal status of community mods**: Steam Workshop's standard agreement gives Valve and the *game publisher* (Focus/Saber/Oovee) a perpetual license. Mod authors retain copyright but uploading *implies* a license only for use **inside that game**. Re-using a Workshop mod's mesh in a different commercial product is **copyright infringement** unless the author explicitly granted CC0/CC-BY/MIT — which is essentially never the case for Spintires mods. Most mods also remix the base game's wheels/textures, which are Saber/Oovee IP.
- **Verdict**: Cannot legally use Spintires assets, mods, or derivatives. Period.

## 2. MudRunner / SnowRunner Asset Format

- **Engine**: SnowRunner uses Saber's proprietary **Swarm Engine** (which evolved from the **VeeEngine** that powered Spintires/MudRunner). The terrain mud-sim is Saber-internal.
- **Format**: Saber-proprietary binary archives. SnowRunner mods are authored via the **official Saber Mod Editor** (Expeditions/SnowRunner share a documented modding pipeline at `expeditions-guides.saber.games`), distributed through **mod.io**.
- **Extractors**: None public; mod.io's distribution explicitly forbids redistribution outside the game.
- **Same engine as Spintires?** Lineage yes (VeeEngine → Swarm), but file formats are not cross-compatible — community converters exist only for MudRunner→SnowRunner mod migration *inside the official tooling*.
- **Verdict**: Same legal blocker as Spintires. Hard pass.

## 3. Open / Industry-Standard 3D Formats in Bevy 0.18

| Format | Bevy 0.18 native? | Notes |
|---|---|---|
| **glTF 2.0 / GLB** | Yes — first-class | Primary format. New in 0.18: `GltfExtensionHandler` trait for stateful extension processing. PBR, animations, skinning, lights, cameras all supported. |
| **OBJ** | Plugin: `bevy_obj` 0.18.0 | Maintained, tracks Bevy releases. Static meshes only. |
| **STL** | Plugin: `bevy_stl` | Static meshes only. |
| **FBX** | No native, no maintained plugin | Open issue #15705. FBX is Autodesk-proprietary; recommended workflow is Blender → glTF. |
| **USD / OpenUSD** | No | Open issue #14464; not on the 0.18 roadmap. |

**Recommendation**: glTF/GLB is the only sane pipeline. Authors can export from Blender, 3ds Max, Maya, Houdini all natively.

## 4. Open-Source / CC0 Off-Road Asset Sources

| Source | License | Off-road relevance |
|---|---|---|
| **Quaternius** (quaternius.com) | CC0 | Ultimate Vehicles pack includes off-road trucks, jeeps, military vehicles, low-poly stylized. Game-ready, rigged. |
| **Kenney.nl** | CC0 | 40k+ assets. Car Kit, Vehicle Pack, Nature Pack — low-poly, blocky aesthetic. Good for prototyping. |
| **Poly Haven** (polyhaven.com) | CC0 | Photoreal: HDRIs (skies/lighting), PBR textures (mud, dirt, rock, grass), 3D-scanned rocks/branches/plants (Namaqualand library — 10 rock scans, 5 branch/debris, 5 ground materials, 10+ plant sets). **Best source for terrain dressing.** |
| **Sketchfab CC0 filter** | CC0 | Mixed quality. Some good Jeep/truck CC0 models exist; verify license per-asset. |
| **AmbientCG** | CC0 | PBR material library — mud, gravel, snow, asphalt. |
| **Free3D / TurboSquid Free** | Mixed | Avoid unless explicitly CC0/CC-BY; many "free" models are non-commercial. |

For our needs: **Quaternius (vehicles) + Poly Haven (terrain/rocks/foliage/HDRI) + AmbientCG (mud/dirt PBR materials)** covers ~90% of asset needs with zero legal risk.

## 5. Heightmap Formats for Terrain Import

- **Standard**: 16-bit grayscale PNG (or R16 raw). 8-bit produces visible terracing — avoid.
- **Generators**:
  - **World Machine** — industry standard, exports 16-bit PNG/RAW.
  - **Gaea** (QuadSpinner) — modern competitor, exports PNG 16-bit (use *resample* in build settings, e.g. 2017×2017).
  - **Houdini** — procedural terrain via heightfield nodes; export to EXR/PNG.
  - **Blender A.N.T. Landscape** — free, built-in.
- **Bevy plugins** (all on GitHub):
  - `bevy_heightmap` (Katsutoshii) — loads heightmap PNGs directly as meshes.
  - `bevy_mesh_terrain` (ethereumdegen) — ECS-native; expects R16 grayscale; uses `TerrainConfig` + `TerrainData` components.
  - `bevy_terrain` (kurtkuehnert) — virtual-textured GPU terrain, more advanced.
- **Typical Bevy pipeline**: load PNG via `AssetServer` → custom asset loader walks pixels → emits `Mesh` with subdivided plane geometry, Y = sample × height_scale → splat textures via material extension.

## 6. Engineering Verdict

**Do NOT attempt to reverse-engineer Spintires/MudRunner/SnowRunner formats.** Three independent reasons:

1. **Legal**: Saber/Oovee/Focus own the assets. Workshop and mod.io licenses do not grant redistribution rights outside the host game. Shipping a single extracted truck mesh exposes the project to DMCA takedown and potential damages.
2. **Technical**: No public extractor exists. The `.x` container is partially documented but Spintires' archive packaging, physics XML schema, and Swarm Engine binaries are not. Reverse-engineering would consume weeks for assets we cannot legally use anyway.
3. **Quality ceiling**: Even if extracted, Spintires meshes are ~2014-era poly counts and texture resolution. Modern CC0 sources (Poly Haven photogrammetry) are *higher* quality.

**Recommended path forward** — build a clean import pipeline:

```
assets/
  vehicles/         *.glb        (Quaternius, custom Blender, CC0 Sketchfab)
  terrain/
    heightmaps/     *.png        (16-bit grayscale from World Machine/Gaea)
    splatmaps/      *.png        (RGBA channel = material weight)
    materials/      *.glb / PBR  (Poly Haven, AmbientCG)
  props/            *.glb        (rocks, trees, debris from Poly Haven)
  hdri/             *.exr / *.hdr (Poly Haven skies)
mods/
  <mod_name>/
    mod.toml                     (manifest: name, version, author, license, entry points)
    vehicles/*.glb
    terrain/*.png
```

**Concrete implementation tasks** (in priority order):

1. **glTF/GLB import folder**: leverage Bevy's native loader; add a `vehicle_loader` system that consumes `assets/vehicles/*.glb` + a sibling `vehicle.toml` for physics params (mass, wheel positions, suspension, drivetrain).
2. **Heightmap PNG terrain loader**: custom `AssetLoader` for 16-bit PNG → `Mesh` with configurable resolution, height scale, and splat-map sampling. Reference `bevy_mesh_terrain` for ECS shape.
3. **Mod manifest TOML + loader**: `mods/<name>/mod.toml` declares license (must be CC0/CC-BY/MIT/Apache for redistribution-safe mods), assets, and Rust-side hooks. Validate license at load time; refuse to load unlicensed mods.
4. **License audit on import**: every asset gets a `.license` sidecar (CC0/CC-BY/etc.) checked in CI. Block PRs that import un-attributed assets.

This path delivers Spintires-comparable visuals (Poly Haven photogrammetry exceeds Spintires 2014 art), legal cleanliness, and a real modding ecosystem — without touching a single byte of proprietary code.

---

## Sources

- [Spintires modding guide PDF (Focus)](http://cdn.focus-home.com/admin/games/mudrunner/docs/ModdingGuide_Mudrunner.pdf)
- [Focus forum: extracting .x files from Spintires](https://forums.focus-entmt.com/topic/36703/is-it-possible-to-extract-x-model-files-from-xml-files-of-all-vehicles-of-both-spintires-games-and-convert-them-mods-included)
- [MudRunner mod converting (Steam discussion)](https://steamcommunity.com/app/675010/discussions/0/1482109512315690783/)
- [Saber Swarm Engine (ModDB)](https://www.moddb.com/engines/swarm-engine)
- [SnowRunner / Expeditions modding docs](https://expeditions-guides.saber.games/)
- [Bevy 0.18 release notes](https://bevy.org/news/bevy-0-18/)
- [Bevy gltf API docs](https://docs.rs/bevy/latest/bevy/gltf/index.html)
- [Bevy FBX issue #15705](https://github.com/bevyengine/bevy/issues/15705)
- [Bevy USD issue #14464](https://github.com/bevyengine/bevy/issues/14464)
- [bevy_obj on crates.io](https://crates.io/crates/bevy_obj)
- [Quaternius CC0 assets](https://quaternius.com/)
- [Kenney.nl assets](https://kenney.nl/)
- [Poly Haven license (CC0)](https://polyhaven.com/license)
- [bevy_heightmap](https://github.com/Katsutoshii/bevy_heightmap)
- [bevy_mesh_terrain](https://github.com/ethereumdegen/bevy_mesh_terrain)
- [bevy_terrain](https://github.com/kurtkuehnert/bevy_terrain)
- [awesome-cc0 list](https://github.com/madjin/awesome-cc0)
