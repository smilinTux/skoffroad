# SandK Offroad

Procedural off-road sandbox built with **Rust + Bevy 0.18 + Avian3D**, generated through 22 sprints of parallel Sonnet sub-agent work.

## Quick start

```bash
cd next
cargo run --bin sandk-offroad-next --release
```

Dev mode with the inspector (F3):

```bash
cargo run --features dev
```

Headless physics tests:

```bash
cargo test --test drive_test
```

Headless scenario harness:

```bash
cargo build --bin sim
./target/debug/sim drop --json --verbose
```

## Highlights

- **5 vehicle variants** — Jeep TJ, Ford Bronco, Pickup, Hummer, Buggy. All built from cuboid+cylinder primitives with photo-referenced detailing (grilles, fender flares, roll bars, tailpipes, winches, exhaust headers, etc.). License-clean, no GLTF assets required.
- **3 maps** — VALLEY (default rolling hills), DUNES (cacti + amber fog), CANYON (red rock pillars + dusty haze). Tab to switch with a 1-second black fade.
- **AI rivals** — RED, GRN, BLU rivals with skill-based AI driving along a Catmull-Rom densified race path.
- **Career mode** — 8 sequential objectives spanning all the game systems.
- **Daily challenge** — deterministic per-day rotation across 5 challenge kinds.
- **Medals** — bronze/silver/gold awarded for course time, race position, gem count, longest jump, top speed.
- **Time trial with ghost** — record your best, race against a translucent ghost car.
- **Pursuit / demolition / explore / challenges** — alternate game modes built on the same physics base.
- **Procedural everything** — terrain via noise, vehicle audio (4-cylinder firing-pulse synth), music (state machine over chord pads + arpeggios), tire surface audio (grass/dirt/rock blend by slope), wind/birds/crickets ambient mixed by TimeOfDay.
- **Visual polish** — Bloom + ACES tonemapping, faux god rays at sunrise/sunset, persistent tire-track decals, vehicle dirt accumulation, storm rain + lightning.
- **World flavor** — 8 procedural shacks + barns, 12-bird boid flock overhead, 5 ambient NPC trucks (toggleable), startup ASCII logo + 5-second cinematic intro orbit.
- **Persistence** — config + keybindings autosaved to `~/.sandk-offroad/`. F12 benchmark mode logs frame-time stats.
- **Accessibility** — colorblind palette swap, reduce-motion toggle, HUD scale 1.0/1.25/1.5x.

## Controls

See [CONTROLS.md](CONTROLS.md) for the full reference. Highlights:

| Key | Action |
|---|---|
| `WASD` | Drive (configurable via `/`) |
| `Space` | Brake |
| `Shift` | Handbrake |
| `B` | Boost |
| `R` | Start/restart race |
| `T` | Time trial |
| `P` | Pursuit |
| `X` | Demolition |
| `C` | Random 30s challenge |
| `Tab` | Map select |
| `H` | Help overlay |
| `Esc` | Pause menu |

## Architecture

The game is decomposed into ~95 plugin modules, each owning a single file in `next/src/*.rs`. Plugins are registered as tuples in `main.rs`. Cross-module communication uses Bevy resources and events; markers + queries do the rest.

Each sprint added 5 modules concurrently via Sonnet sub-agents working on pre-staged stub files.

## Sprint history

| Sprint | Version | Focus |
|---|---|---|
| 14 | 0.4.15 | Audio polish — music, engine_pro, surfaces, world_audio, mixer + photo-referenced vehicle silhouettes |
| 15 | 0.4.16 | AI rivals & racing |
| 16 | 0.4.17 | Career & progression — XP curve, unlocks, career, daily, medals |
| 17 | 0.4.18 | World variety — multiple maps, biomes, transitions |
| 18 | 0.4.19 | Polish + persistence — config, fonts, theme, loading screen, credits |
| 19 | 0.4.20 | QoL — input remap, accessibility, benchmark, demo mode, changelog |
| 20 | 0.4.21 | Visual polish — storm, vehicle dirt, decals, bloom, god rays |
| 21 | 0.4.22 | Game modes — time trial, pursuit, demolition, explore, challenges |
| 22 | 0.4.23 | World flavor — ASCII logo, intro video, traffic, buildings, bird flock |
| 23 | 0.5.1 | Showcase polish — landmarks, exhaust smoke, minimap zoom, seasons, drifting clouds |

## Testing

`drive_test` (4 cases) verifies the headless physics harness on every commit:

- `harness_runs` — bare scenario boots
- `idle_settles` — chassis rests on terrain without drift
- `forward_moves_vehicle` — drive input produces forward motion
- `brake_stops_vehicle` — brake stops a moving vehicle

Each sprint runs all 4 in CI before the agent reports completion.

## License

Code: project license TBD. Vehicle silhouettes are original procedural primitive compositions (no GLTF / external 3D assets), so the asset pipeline is license-clean.
