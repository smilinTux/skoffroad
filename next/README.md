# SandK Offroad - Next

Clean-slate Bevy 0.18 rewrite. A TJ-ish vehicle on procedural terrain.

## How to run

```sh
cd next
cargo run --features dev   # fast iteration (dynamic linking)
cargo run --release        # optimised build
```

## Controls

| Key | Action |
|-----|--------|
| W / Arrow Up | Accelerate forward |
| S / Arrow Down | Reverse |
| A / Arrow Left | Steer left |
| D / Arrow Right | Steer right |
| Space | Brake |
| Right Mouse + drag | Orbit camera |

## Stack

- Bevy 0.18.1
- Avian3D 0.6.1 (physics)
- bevy_hanabi 0.18.0 (GPU particles — dependency compiled, hookup is a TODO)
- noise 0.9.0 (heightmap generation)
- bevy-inspector-egui 0.36.0 (dev-dependency; F3 inspector is a TODO)

## Status

Scaffold: compiles, launches a window, terrain + vehicle + chase camera wired up.

TODO:
- Wire bevy_hanabi dust particles on wheel contact
- Wire bevy-inspector-egui F3 toggle
- Tune suspension / joint constraints
- Add GLTF body mesh swap
