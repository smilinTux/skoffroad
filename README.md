# skoffroad

Bevy 0.18 + Avian3D 0.6 + bevy_kira_audio + bevy_hanabi off-road sandbox.
A Jeep-ish chassis on procedural terrain with raycast suspension, day/night,
weather forces, mud, water, ramps, trampolines, scatter, livery, telemetry,
HUD, mini-map, stats, achievements, replay, photo mode, and a headless test
harness.

(The April 2025 prototype lives in `legacy/` — see `docs/PARKING_LOT.md` for
features worth pulling forward.)

## Play in browser

**[play.skoffroad.skworld.io](https://play.skoffroad.skworld.io)** — runs as
a WebAssembly bundle (Bevy + Avian compiled to wasm32, served via GitHub
Pages). No install. Best in a Chromium-based browser with WebGPU enabled.

## Run locally

```sh
cargo run --features dev          # fast iteration (dynamic linking, F3 inspector)
cargo run --release               # optimised build (defaults to High quality)
cargo run --release -- --quality=medium   # PBR + tonemap, no SSAO
cargo run --release -- --quality=low      # legacy look, lightest on GPU
cargo run --bin sim -- forward 5  # headless harness, JSON or human-readable
cargo test                        # drive_test physics regressions
```

The `--quality` flag persists in `~/.skoffroad/config.json`. You can
also cycle it live in-game from the pause overlay (Esc → `\`).

### WebAssembly build

```sh
cargo install trunk                # one-time
rustup target add wasm32-unknown-unknown
trunk serve --release              # http://localhost:8080
trunk build --release              # static dist/ ready for any HTTP server
```

## Driving

| Key | Action |
|-----|--------|
| W / Up | Throttle forward |
| S / Down | Reverse |
| A / Left | Steer left |
| D / Right | Steer right |
| Space | Brake |
| R | Reset chassis to spawn (refills fuel) |
| J | Auto-flip recovery (right the chassis in place) |
| N | Horn |
| 1–5 | Cycle paint livery |

## Camera

| Key | Action |
|-----|--------|
| V | Toggle chase / cockpit |
| Q / E | Orbit (chase only) |
| Right mouse drag | Mouse orbit (chase only) |

## UI overlays

| Key | Action |
|-----|--------|
| H | Toggle main HUD |
| M | Toggle mini-map |
| C | Toggle compass strip |
| L | Toggle time-trial panel |
| G | Toggle speedometer |
| Z | Toggle wind indicator |
| K | Toggle skid-mark spawning (`Shift+K` clears) |
| B | Toggle breadcrumbs (`Shift+B` clears) |
| Y | Toggle headlights (`Shift+Y` returns to auto) |
| F8 | Toggle perf overlay |
| F9 | Toggle fuel gauge |
| X | Toggle speed-line vignette |
| Tab (hold) | Stats screen |
| ? (Shift + /) | Keybind help overlay |
| E | Toggle event log |

## Time

| Key | Action |
|-----|--------|
| T | Pause day cycle |
| [ / ] | Scrub time of day |

## Save / load

| Key | Action |
|-----|--------|
| F5 / F6 / F7 | Save to slot 1 / 2 / 3 |
| F1 / F2 / F4 | Load slot 1 / 2 / 3 |

(Auto-save on exit; auto-load slot 1 on launch.)

## Replay & photo

| Key | Action |
|-----|--------|
| . (period) | Replay last 10 s as a translucent ghost |
| P | Photo mode (pauses physics, hides cursor, banner) |

## Multiplayer (Sprint 49 + Sprint 51)

P2P position sync and voice chat via WebRTC / matchbox. Two players in
different browsers (or native + browser) see each other's chassis as a
semi-transparent ghost in real time and can speak to each other.

| Key | Action |
|-----|--------|
| I | Toggle multiplayer panel |
| F (hold) | Push-to-talk voice (browser only) |
| Shift+F | Toggle always-on voice mode |

Connect: press **I**, check the room code matches on both clients, click
**Connect**. Press **F** to talk — the browser will prompt for microphone
permission on first use.

See `docs/MULTIPLAYER.md` for TURN, self-hosted signaling, NAT
troubleshooting, and voice chat architecture. Native voice is a TODO; see
`docs/PARKING_LOT.md` for the roadmap.

## Pause / settings

| Key | Action |
|-----|--------|
| Esc | Pause + settings overlay |
| -, = | Master volume −/+ (while paused) |
| , . | Mouse sensitivity −/+ (while paused) |
| ; ' | Day length −/+ (while paused) |
| \\  | Cycle graphics quality (Low / Medium / High) |

## Dev

| Key | Action |
|-----|--------|
| F3 | World inspector (only with `--features dev`) |

## Stack

| Crate | Version | Role |
|-------|---------|------|
| bevy | 0.18.1 | Engine |
| avian3d | 0.6.1 | Physics |
| bevy_hanabi | 0.18.0 | GPU particles (dust) |
| bevy_kira_audio | 0.25.0 | Procedural engine / horn / skid / wind / thud |
| noise | 0.9.0 | Heightmap generation |
| bevy-inspector-egui | 0.36.0 | F3 inspector (dev-only) |
| serde / serde_json | 1.x | Save files |

## Plugin tour (44 of them)

Vehicle physics: `vehicle`, `terrain`, `camera`, `recovery`.
World dressing: `sky`, `water`, `scatter`, `mud`, `ramps`, `trampolines`,
`speedtrap`, `repair`, `stars`.
HUD & UI: `hud`, `minimap`, `gauge`, `compass`, `events`, `stats_screen`,
`help`, `menu`, `perf`, `damage`, `speedlines`, `confetti`.
Game loops: `trial`, `xp`, `wheelie`, `airtime`, `breadcrumbs`, `fuel`,
`achievements`, `livery`.
Effects / FX: `particles` (dust), `shake`, `audio` (engine + skid + thud +
wind), `horn`.
System: `save`, `settings`, `photomode`, `replay`, `headlights`, `wind`,
`skidmarks`.

Plus the `headless` module and a `sim` binary that step Avian without a
window, used by `cargo test --test drive_test` and ad-hoc CLI scenarios.

## Headless harness

```sh
cargo run --bin sim -- forward 5             # human-readable
cargo run --bin sim -- forward 5 --json      # JSON for piping into jq
cargo run --bin sim -- brake-test 5
cargo run --bin sim -- right 3
cargo run --bin sim -- idle 3
cargo run --bin sim -- script:path.json      # custom timeline
```

Returns chassis position, velocity, distance, max speed, max tilt,
did_flip, time-above-terrain.

## License

GPL-3.0-or-later. See `LICENSE`.

## Status

Single playable window, ~46 plugins, ~13 000 lines of Rust across
~190 modules. Physics regressions gated by 4 `drive_test` integration
tests, all green on `main`.

**v0.8.0** ships the photoreal pass: triplanar terrain shader with
4-channel splat blend (dirt / grass / rock / mud) over CC0 PBR
materials, PBR rocks, wet-surface shader hook, AgX tonemap, color
grading and SSAO — all gated by the runtime `GraphicsQuality` tier so
the game scales from older laptops up to modern GPUs.
