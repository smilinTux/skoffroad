# skoffroad — MVP Scope (v2, May 2026)

## TL;DR

skoffroad is a single-player, physics-driven off-road vehicle simulation built in Rust
with Bevy and Avian3D. The v2 restart targets a single playable vertical slice: one vehicle,
one procedurally generated terrain, 60+ FPS, no networking, no economy, no voice chat.
Everything else is deferred until the core loop is playable. Canonical code now lives in the
`next/` subdirectory; the legacy `src/` tree is reference-only.

---

## Why this rewrite exists

The original codebase accumulated roughly 60,000 lines of Bevy 0.12 code across a feature
list that included multiplayer, a token economy, a vehicle marketplace, CB radio with voice
chat, an emergency-services dispatch system, hardware ray tracing, modding support, and
in-game radio stations. None of these features shipped. The build broke in May 2025 and was
never fixed; at last count it produced 422 compile errors. Every one of the 47 tasks in
`tasks/tasks.json` remains `"status": "pending"`. The CHANGELOG shows two version tags
(v0.1.0 and v0.2.0) whose "added" sections describe basic directory structure and
documentation scaffolding, not playable features.

The scope was approximately ten times what one developer can reasonably deliver. This is not
unusual — LLM-assisted planning tends to produce exhaustive feature lists without regard for
sequential dependencies or delivery capacity. The Bevy 0.12 API is also two major versions
stale as of May 2026, meaning the legacy code would require a non-trivial migration even
before the compile errors are addressed.

The decision is to start fresh in `next/` on Bevy 0.18 + Avian3D rather than rehabilitate
the legacy tree. The goal is a tight, shippable vertical slice first, with the legacy code
available for reference but carrying no obligation.

---

## Canonical codebase

- `next/` is the project. It targets Bevy 0.18.1, Avian3D 0.6.1, bevy_hanabi 0.18.0, and
  noise 0.9.0. All new work goes here.
- `src/` is reference-only. It will be deleted once `next/` reaches feature parity with
  what `src/` actually implemented (vehicle physics shell, terrain mesh, basic audio). No
  new code should be added to `src/`.

---

## Vertical slice goal (MVP)

A player can spawn into a procedurally generated off-road terrain, drive a Jeep-class
vehicle over rocks and slopes, watch the suspension respond to terrain, and get to 60+ FPS
on a mid-tier discrete GPU — all in a single-player session with no network dependency.

### Acceptance criteria

- Vehicle spawns on terrain and can be driven with keyboard/gamepad input (accelerate,
  brake, steer, reverse).
- All four wheels maintain independent contact with uneven terrain; suspension visibly
  compresses and extends as wheels cross obstacles.
- Engine torque model produces realistic low-speed crawling behavior distinct from
  high-speed behavior (RPM-based torque curve, not a flat force).
- Vehicle does not tunnel through terrain geometry at any reachable speed.
- Procedural terrain generates deterministically from a seed; re-running with the same seed
  produces identical output.
- Terrain covers at least 200 m x 200 m with varied elevation, rocks/boulders as static
  collision obstacles, and at least two surface-friction zones (e.g., dirt and mud).
- Particle effects fire on wheel contact with dirt/mud surfaces (bevy_hanabi).
- Dynamic directional sun light casts real-time shadows on vehicle and terrain.
- Session runs at 60+ FPS on a mid-tier discrete GPU (e.g., RTX 3060 or equivalent AMD) at
  1080p with shadows enabled.
- The application builds and runs from `next/` with `cargo run` and no external setup
  beyond Rust stable + Vulkan drivers.
- An inspector overlay (bevy-inspector-egui, `dev` feature flag only) allows live tuning
  of suspension stiffness, damping, and engine torque without recompiling.

---

## Phases / Milestones

### Phase 0 — Scaffold (complete)

The `next/` tree is scaffolded and compiles clean on Bevy 0.18.1 + Avian3D 0.6.1 +
bevy_hanabi 0.18.0 + noise 0.9.0.

- `Cargo.toml` with correct dependency versions and `dev` feature for dynamic linking.
- `main.rs`: window, physics plugin, ambient + directional lighting, plugin registration.
- `terrain.rs` (`TerrainPlugin`): 128x128 FBM heightmap, triangle mesh, smooth normals,
  UV coords, PBR material, `ColliderConstructor::TrimeshFromMesh` for static collision.
- Module stubs declared for `camera` and `vehicle` (files not yet written; mod declarations
  present in `main.rs`).
- `bevy_hanabi` wired as a dependency; dust particle effect stubbed.

### Phase 1 — Drivable vertical slice

Target: a player can drive the vehicle over the existing terrain. Estimated solo effort:
2–4 weeks.

- `vehicle.rs` (`VehiclePlugin`): rigid-body chassis with Avian3D, four-wheel suspension
  via raycast or joint constraints, RPM-based engine torque, keyboard + gamepad input.
- `camera.rs` (`CameraPlugin`): smooth chase camera that follows the vehicle body with
  configurable lag and pitch offset.
- Dust particle effect from Phase 0 connected to wheel ground-contact events.
- `bevy-inspector-egui` panel (gated behind `dev` feature) for suspension and engine
  parameters.
- Acceptance criteria from the vertical slice goal above are fully met.

### Phase 2 — Terrain depth and weather

Target: the terrain is visually and mechanically richer. Solo estimate: 2–4 weeks after
Phase 1.

- Seed-selectable terrain at startup (command-line arg or config file).
- At least two distinct friction zones rendered with texture splatting or vertex-color
  blending (dirt, mud, rock).
- Static obstacle placement (boulders) procedurally scattered via noise mask with
  collision geometry.
- Dynamic time-of-day: sun angle animates over a configurable cycle; ambient light tracks.
- Rain particle system via bevy_hanabi; rain affects mud-zone friction coefficient.
- Volumetric fog as a post-process or Bevy fog resource (not full volumetric raymarching —
  simple exponential fog is acceptable).

Acceptance criteria: terrain variety is perceptible within 60 seconds of driving; weather
transitions do not drop below 60 FPS; rain visibly affects wheel traction on mud surfaces.

### Phase 3 — Vehicle systems (winch, damage, tire deformation)

Target: the vehicle feels like an off-road tool, not a physics box. Solo estimate: 3–5
weeks after Phase 2.

Features selected from legacy `src/` as worth porting (reference only; re-implement
cleanly in `next/`):
- Winch: attach point on vehicle, attach point on static geometry, cable tension applies
  force toward anchor. Input to spool/unspool.
- Damage: chassis health reduces from collision impulse above a threshold; visual deform
  via shader parameter or mesh morph target (simple is fine).
- Tire deformation: squash/stretch visual on contact patch proportional to load (shader or
  morph-target approximation — physical mesh deformation is Phase 5+).
- Tire temperature: thermal model accumulates heat under load and dissipates over time; grip
  coefficient is multiplied by a temperature factor so cold tires offer less traction than
  tires at operating temperature.
- 4WD/2WD selector: high/low range affects torque delivery and top speed.

Acceptance criteria: winch pulls vehicle up a slope too steep to drive unassisted; repeated
hard impacts visibly mark the vehicle; tire squash is visible at low speed over large rocks;
tire temperature affects grip — a measurable handling delta exists between cold tires at
session start and hot tires under sustained high-load driving.

### Phase 4 — UX polish (HUD, audio, camera)

Target: the session is presentable to an outside player. Solo estimate: 2–3 weeks after
Phase 3.

- HUD: speedometer, gear indicator, RPM bar, winch status, compass heading.
- Engine audio: RPM-driven pitch and volume via `bevy_kira_audio`; spatial collision
  sounds. (`bevy_kira_audio` is the selected audio crate — more featureful than Bevy's
  built-in audio, has mature spatial-audio support, and matches the approach in the legacy
  `src/` plan.)
- Ambient environment sounds (wind, birds) with day/night variation.
- Additional camera modes: cockpit view (first-person from driver seat), free orbit.
- Main menu: seed input, graphics quality preset, start/quit.
- Pause menu with resume and quit.

Acceptance criteria: a player unfamiliar with the project can start a session, understand
vehicle state from the HUD, and exit cleanly without touching the source code.

### Phase 5+ — Post-MVP (deferred)

These features are explicitly deferred until Phases 1–4 are complete and the game loop is
validated with real players:

- Mission system and structured objectives.
- Multiple distinct vehicle configurations (not just parameter tuning of the same model).
- Replay and spectator systems.
- Save / load session state.
- Modding API.
- Multiplayer.
- Physical mesh tire deformation (real geometry, not shader morph — significant scope
  expansion relative to the visual approximation in Phase 3).

---

## Explicitly cut from scope (and why)

- **Multiplayer / netcode.** Synchronizing physics-driven vehicle state across a network
  requires deterministic simulation or state reconciliation that is fundamentally
  incompatible with Avian3D's current feature set, and adds months of infrastructure before
  a single additional gameplay feature can be built. There is no game to be multiplayer yet.

- **Token economy / vehicle marketplace.** Requires backend services, wallet integration,
  and regulatory consideration. None of those exist. Building an economy before the
  underlying game is playable inverts the dependency order entirely.

- **CB radio with voice chat.** Voice chat requires a relay server, codec integration, and
  audio effects pipeline. The in-game CB radio effect layer on top of that is cosmetic.
  Neither is justified without a multiplayer foundation.

- **Emergency-services dispatch system.** This feature appeared in `src/` as a partial
  implementation with no connection to playable mechanics. It is an entirely separate product
  concept grafted onto an off-road simulator. It is removed permanently from this scope.

- **Real-time global illumination / hardware ray tracing.** Bevy's render pipeline does not
  expose hardware RT at a stable API level as of May 2026. The legacy README listed this as
  complete; it was not. Dropping it removes a GPU requirement that would exclude the majority
  of potential players and has zero impact on gameplay.

- **Modding support.** A modding API requires stable internal interfaces. Stable interfaces
  require a finished game. Modding is a post-1.0 concern.

- **In-game radio stations with custom playlists.** Ambient audio is in Phase 4. Curated
  radio with licensing, playlist management, and UI is a product feature for a shipped game.

- **Mission system.** Structured objectives and progression require a designed level, a
  scoring system, and a save system. None of those are built. Gameplay value must be proven
  with free-roam driving first.

- **Advanced post-processing pipeline (TAA, motion blur, depth of field, color grading,
  ray-traced reflections).** The legacy README listed all of these as complete; the build
  was broken before any of them could be verified. Standard Bevy post-processing (bloom,
  FXAA) is acceptable for MVP. Custom pipeline work deferred to post-Phase 4.

- **Comprehensive benchmarking suite / hot-reload / asset streaming.** Infrastructure
  polish deferred until the feature set is stable enough to benchmark meaningfully.

---

## Tech stack rationale

| Dependency            | Version  | Rationale |
|-----------------------|----------|-----------|
| Bevy                  | 0.18.1   | Current stable release; ECS architecture maps cleanly to game object model; large ecosystem; active development. Chosen over Fyrox because Fyrox's scene-graph approach adds overhead for a physics-heavy simulation and its plugin ecosystem is smaller. |
| Avian3D               | 0.6.1    | Native Bevy integration, built for ECS scheduling, active maintenance. Chosen over bevy_rapier3d because Avian3D is developed alongside Bevy's schedule model and avoids the callback-translation layer that bevy_rapier3d requires; suspension raycasts integrate cleanly with Avian3D's query API. |
| bevy_hanabi           | 0.18.0   | GPU-driven particle system with Bevy version parity. Chosen over a custom particle implementation because dust/rain effects require thousands of particles at 60 FPS; CPU-driven alternatives cannot sustain that on the render thread. |
| noise                 | 0.9.0    | Proven FBM/Perlin/Simplex implementations; no unsafe dependencies; deterministic output. Sufficient for Phase 0–2 terrain; more exotic generation can be added later. |
| bevy_kira_audio       | (Phase 4) | Selected audio crate. More featureful than Bevy's built-in audio; mature spatial-audio support; consistent with the approach used in the legacy `src/` plan. Added in Phase 4 when engine audio and ambient sounds are implemented. |
| bevy-inspector-egui   | 0.36.0   | In-session parameter tuning without recompile. gated behind `dev` feature; zero runtime cost in release builds. Essential for rapid iteration on vehicle physics constants. |

---

## Definition of done for this scope document

This document governs the current restart. It is reviewed and updated when Phase 1 ships
(vertical slice playable). At that point, acceptance criteria for Phases 2–4 may be revised
based on what was learned during Phase 1. Phases 5+ are not scheduled and will not be
scheduled until Phase 4 is complete.
