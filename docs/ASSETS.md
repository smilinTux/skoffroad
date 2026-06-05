# Asset Pipeline — audio, vehicles, maps

This game is built on **Bevy**, so assets use Bevy-native formats:

| Kind     | Format                        | Loader                         | Drop into            |
|----------|-------------------------------|--------------------------------|----------------------|
| Vehicles | glTF `.glb` / `.gltf`         | `src/glb_loader.rs` (manifest) | `assets/vehicles/`   |
| Maps     | grayscale heightmap `.png`    | `src/heightmap_loader.rs`      | `assets/maps/`       |
| Audio    | `.ogg` (default), `.wav`*     | `bevy_kira_audio` + see below  | `assets/audio/...`   |

> **Not Unreal Engine.** UE assets/blueprints cannot run in Bevy. Make models in
> **Blender** (free, open-source, glTF export built in). For terrain use Gaea /
> World Machine / TerreSculptor, or real-world DEM data — all produce heightmaps.

Starter assets are already dropped in so every loader has something to load. They
are placeholders meant to be replaced with the CC0 packs listed below — that's the
"start from something, make it our own" path.

---

## 1. Engine audio (the authentic path)

The default build uses the **synthesized** engine in `src/audio.rs` (one
band-limited DSP voice). For genuinely real engine sound, build with the
`engine_samples` feature, which switches to a **VNS-style RPM crossfade**
(`src/engine_samples.rs`): it plays recorded engine loops at several RPMs and
constant-power crossfades + pitch-shifts between the two nearest. This is how
shipping driving games sound real.

```bash
# native
cargo run --features engine_samples
# web (trunk) — add the feature to the build, e.g.
trunk serve --features engine_samples
```

When the feature is on, `audio.rs` automatically **mutes its synth** (via the
`EngineSamplesActive` resource) so there is never a double engine. With the
feature off, the build is identical to the synth-only version (zero regression).

### Files it loads (replace these)
```
assets/audio/engine/idle.wav   # ~700 RPM loop
assets/audio/engine/mid.wav    # ~2600 RPM loop
assets/audio/engine/high.wav   # ~5000 RPM loop
```
The shipped `.wav`s are **procedurally generated placeholders** (synth-quality).
Replace them with real CC0 recordings for the big quality jump. Keep the same
filenames, or switch to `.ogg` (edit `PATHS` in `engine_samples.rs`; ogg is
enabled by default, so `.ogg` needs no extra feature). RPM anchors are in
`ANCHORS` in `engine_samples.rs` — set them to match your recordings.

### Where to get CC0 engine loops
- **Sonniss GameAudioGDC** — huge pro bundle, royalty-free, no attribution:
  https://sonniss.com/gameaudiogdc/
- **Freesound** (filter to CC0) — e.g. qubodup "Car Engine Loop":
  https://freesound.org/people/qubodup/sounds/147242/
- **Pixabay SFX** — CC0, no attribution, has engine-at-RPM loops:
  https://pixabay.com/sound-effects/search/engine/
- **OpenGameArt**, **Kenney audio** — CC0 misc/UI/impacts.

Tip: trim each clip to a clean, seamless loop (a whole number of firing cycles)
so there's no click at the loop point.

---

## 2. Vehicles (glTF)

`src/glb_loader.rs` reads `assets/manifest.json` → `vehicles[]` and registers each
`.glb` as a selectable scene (browse them in the in-game Asset Browser). Add an
entry, drop the file in `assets/vehicles/`, done:

```json
{ "name": "My Truck", "glb_path": "vehicles/mytruck.glb",
  "mass_kg": 1800.0, "license": "CC0-1.0", "author": "..." }
```

Shipped starter: `assets/vehicles/toycar.glb` — Khronos **ToyCar**, CC0-1.0.

### Where to get CC0 vehicles (glTF-ready)
- **Kenney Car Kit** (45+ vehicles, CC0): https://opengameart.org/content/car-kit
- **Quaternius** (CC0, glTF/Blend): https://quaternius.com/
- **Eclair Car Kit GLB pack** (50 CC0 `.glb`):
  https://eclair-assets.itch.io/car-kit-glb-pack-50-free-cc0-3d-models
- **Khronos glTF Sample Assets** (mixed CC0/CC-BY):
  https://github.com/KhronosGroup/glTF-Sample-Assets

### Make your own
Model in **Blender** → export glTF `.glb` → drop in + manifest entry. The
**Blenvy** addon lets you define Bevy components inside Blender:
https://github.com/kaosat-dev/Blenvy

> **Note (current limitation):** loaded GLBs are currently *browsable* (Asset
> Browser) but not yet swapped in as the drivable physics chassis — the player
> vehicle is still the procedural rig in `vehicle.rs`. Wiring a GLB body onto the
> physics chassis (visual mesh parented to the chassis + a low-poly collider) is
> the next step; this scaffolding gets the assets loading and catalogued first.

---

## 3. Maps (heightmaps)

`src/heightmap_loader.rs` scans `assets/maps/*.png` (8- or 16-bit grayscale;
value → elevation 0..max_height_m). Drop a PNG in and it's available; native can
also pass `--load-heightmap path.png`, and WASM supports drag-drop onto the canvas.

Shipped starter: `assets/maps/trail1.png` (procedurally generated 256×256).

### Where to get terrain
- **Real-world DEM** (free): USGS (US, down to 1 m), ESA Copernicus (worldwide) →
  export a grayscale heightmap.
- **TerreSculptor 2.0** (free, commercial OK; imports DEM/GeoTIFF/HGT),
  **Gaea** (free tier), **World Machine** (free version), or **Blender** sculpt.

---

## Licensing

All shipped starter assets are **CC0-1.0** (public domain, no attribution
required). See `assets/ATTRIBUTION.md`. When you add new assets, record their
license + author in `manifest.json` and `ATTRIBUTION.md` — keep CC0 / CC-BY only,
and honor CC-BY attribution.
