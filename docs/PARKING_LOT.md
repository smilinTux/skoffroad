# Parking Lot — Features Worth Considering

Curated list of features from the **archived legacy codebase** (`legacy/` after
the v0.7.0 refactor) that are missing from the current `skoffroad` crate and
might be worth porting. Sourced from a one-time scan of the April 2025 attempt.

## Tier 1 — Multiplayer / Co-op pillar

- **Voice chat with VAD + jitter buffers** (`legacy/src/game/audio/voice_chat.rs`, ~1400 lines)
  — Full multiplayer voice system with voice-activity detection, jitter
  buffering, network synchronization, and latency handling via `cpal` +
  `ringbuf`. The hardest piece of co-op infra; reusing this would shortcut a
  multi-week build.

- **CB radio with terrain-aware signal model** (`legacy/src/game/audio/cb_radio.rs`, ~1200 lines)
  — Terrain-aware radio simulation: signal degradation by line-of-sight,
  interference from power lines/towers, frequency-dependent range, multi-channel.
  Real gamification hook for multiplayer roleplay; pairs naturally with voice
  chat.

## Tier 2 — Dynamic events

- **Emergency dispatch system** (`legacy/src/game/audio/emergency_services.rs`, ~1100 lines)
  — Procedural ambulance/fire/police dispatch with AI responders, event
  generation, scenario templates, response statistics. Could drive dynamic
  career events (recovery missions, escort runs).

## Tier 3 — Environmental depth

- **Tire temperature model** (`legacy/src/physics/tire_temperature.rs`, ~150 lines)
  — Dynamic grip curves on temp hysteresis, wear from overheating, optimal
  operating window. Adds tuning depth without much code.

- **Advanced snow physics** (`legacy/src/physics/snow_handling.rs`, ~900 lines)
  — Seasonal snow layers, density/hardness transitions, temperature gradients,
  melting/refreezing, powder vs. ice. Substantial winter-event content.

- **Persistent terrain deformation** (`legacy/src/terrain/deformation.rs`, ~440 lines)
  — Vehicle tracks that persist, snow compression, dynamic mesh updates,
  weather-driven healing. Immersion win.

- **Procedural terrain features** (`legacy/src/terrain/terrain_features.rs`, ~560 lines)
  — Procedural snowfields, drifts, rock-crawling zones, water crossings,
  mudpits, ice formations. Biome-specific obstacle library.

## Already in skoffroad — don't re-port

achievements, career, challenges, daily, audio (engine + environmental), tire
pressure, diff lock, transmission, terrain splatmap/LOD, combo, confetti,
compass, breadcrumbs, boost.

## Stubs only — discard

`legacy/src/.../vehicle_customization_options.rs`, `legacy/src/.../suspension.rs`
(2-line stubs).

---

**Recommended order if we ever pull from this list:**
voice + CB radio → emergency dispatch → tire/snow polish.
