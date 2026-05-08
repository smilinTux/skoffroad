# Adding your own vehicle

skoffroad ships with the procedural Skrambler SK out of the box, but the engine
will load any glTF/GLB you drop into `assets/vehicles/`. Use this guide to
roll your own truck/buggy/jeep without writing any Rust.

## License rules — read this first

This is a **GPL-3 game**. Anything you bundle into the main repo must be
license-compatible with GPL-3. The safe sources are:

| License           | OK to bundle? | Notes                                       |
|-------------------|:-------------:|---------------------------------------------|
| **CC0 / Public**  | ✅            | Quaternius, Kenney.nl, ambientCG, PolyHaven |
| **CC-BY-4.0**     | ✅            | Keep attribution in `assets/CREDITS.md`     |
| CC-BY-SA-4.0      | ⚠️            | OK *only* if you're fine with SA propagation |
| CC-BY-NC          | ❌            | Non-commercial blocks GPL distribution      |
| Proprietary / GTA | ❌            | Hard no                                     |

If your model is CC-BY-NC, you can still **use it locally** for personal
play — just don't commit it.

## The shortest possible path

```
assets/vehicles/<your_vehicle_name>/
├── model.glb          # the 3D model (GLB binary, glTF 2.0)
├── paint.png          # optional — base-color overlay if model has UVs
└── vehicle.toml       # config: mass, wheel positions, name, license
```

Then add a one-liner to `assets/manifest.json`:

```json
{
  "vehicles": [
    {
      "name": "My Trail Beast",
      "glb_path": "vehicles/my_trail_beast/model.glb",
      "mass_kg": 1500.0,
      "license": "CC0",
      "author": "you"
    }
  ]
}
```

Reload the game and your vehicle shows up in the variant cycle.

## vehicle.toml schema

```toml
name        = "My Trail Beast"
mass_kg     = 1500.0          # chassis mass, drives suspension feel
chassis_half = [1.0, 0.4, 2.0] # half-extents for the physics collider
wheel_offsets = [
  [-0.95, -0.30, -1.30],      # FL
  [ 0.95, -0.30, -1.30],      # FR
  [-0.95, -0.30,  1.30],      # RL
  [ 0.95, -0.30,  1.30],      # RR
]
wheel_radius     = 0.35
suspension_len_m = 0.40
license  = "CC0"
author   = "your_name"
```

`wheel_offsets` are **chassis-local** positions (-Z is forward). The four-wheel
ray-cast suspension uses these directly, so you can simulate long-arm kits
and oversized tires by tweaking `wheel_offsets` Y / `wheel_radius`.

## Modeling tips for blender

1. **Forward axis**: -Z (Bevy convention). Set "Forward: -Z, Up: +Y" on glTF
   export.
2. **Origin at chassis centre**: vehicle's pivot should be the body centre,
   not the floor.
3. **Tris under 50k**: the camera lives close so high-poly hurts framerate.
4. **PBR materials**: bake to `baseColor`, `metallicRoughness`, `normal`. The
   Bevy importer reads them automatically.
5. **No animations needed**: wheels and suspension are driven by the engine.
   Don't include rigged armatures unless you plan to wire them yourself.

## Going further

Coming in a follow-up sprint:

- `assets/vehicles/<name>/upgrades/` — long-arm kits, oversized tire packs,
  bumper variants, winch attachments — all swappable in the garage UI.
- Per-vehicle paint variants beyond the global Skrambler palette.
- Mesh-driven physics colliders (currently a single cuboid by default).

If you ship a clean CC0 vehicle pack we'll happily review a PR — open an
issue at <https://github.com/smilinTux/skoffroad/issues> and tag it
`good-first-vehicle`.
