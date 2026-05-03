# SandK Offroad

A single-player, physics-driven off-road vehicle simulation in Rust and Bevy, built around
a Jeep TJ-class vehicle on procedurally generated terrain.

## Status

v0.4 restart in progress on `next/`. The original `src/` tree accumulated features that were
never shipped and a build that broke in May 2025. The restart targets a tight vertical slice:
one vehicle, one terrain, 60+ FPS, no networking, no economy. Legacy `src/` is retained for
reference until `next/` reaches feature parity, then will be removed. See `MVP_SCOPE.md` for
the full scope definition and milestone breakdown.

## Quick start

Prerequisites: latest stable Rust, a Vulkan-capable GPU, Linux / Windows / macOS.

```sh
cd next
cargo run --features dev    # development build with dynamic linking
cargo run --release         # optimised build
```

No external tools, SDKs, or setup steps are needed beyond a working Rust toolchain and
Vulkan drivers.

## Controls

| Key / Input            | Action               |
|------------------------|----------------------|
| W / Arrow Up           | Accelerate forward   |
| S / Arrow Down         | Reverse              |
| A / Arrow Left         | Steer left           |
| D / Arrow Right        | Steer right          |
| Space                  | Brake                |
| Right mouse + drag     | Orbit camera         |

## Layout

```
sandk-offroad/
├── next/        canonical codebase (Bevy 0.18 + Avian3D)
├── src/         legacy reference tree (Bevy 0.12, broken build — do not add code here)
├── assets/      shared game assets (textures, models, audio, shaders)
├── docs/        design and architecture notes
├── tests/       integration tests
├── benches/     performance benchmarks
├── examples/    standalone runnable examples
└── tasks/       leftover from a deprecated task-management tooling experiment
```

## Tech stack

- Bevy 0.18.1 — ECS engine and renderer
- Avian3D 0.6.1 — physics (rigid bodies, raycasts, collision)
- bevy_hanabi 0.18.0 — GPU-driven particle system (dust, rain)
- noise 0.9.0 — heightmap generation (FBM / Perlin)
- bevy-inspector-egui 0.36.0 — dev-only parameter inspector (`--features dev`)
- bevy_kira_audio — spatial audio (planned for Phase 4)

## Documentation

- `MVP_SCOPE.md` — scope definition, phase milestones, and acceptance criteria
- `docs/` — additional design and architecture notes

## License

MIT
