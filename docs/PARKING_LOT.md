# Parking Lot — Features Worth Considering

## Active deferrals (Sprint 46 polish / v0.10.4)

- **WebGL DOF render-graph spam.** The browser console message
  `INFO bevy_post_process::dof: Disabling depth of field on this platform`
  is emitted by Bevy's `DepthOfFieldPlugin` (part of `DefaultPlugins` via
  `PostProcessPlugin`) when depth-texture sampling is unsupported. While Bevy
  correctly skips the DOF component extraction, the plugin itself and its
  render-graph node remain registered, and wgpu's WebGL2 backend emits
  `GL_INVALID_FRAMEBUFFER_OPERATION: glCopyTexSubImage2D` during the depth
  prepass that SSAO (not DOF) triggers. We silenced the framebuffer spam by
  dropping `ScreenSpaceAmbientOcclusion` on wasm32 (see v0.10.4 CHANGELOG).
  The DOF `INFO` line itself is a single one-shot log entry from Bevy internals
  and cannot be suppressed without patching Bevy or disabling the entire
  `bevy_post_process` feature. If the log line bothers future maintainers,
  disable `default-features` on the `bevy` dep and re-enable all features
  except `bevy_post_process` — but that risks missing other default features
  accidentally. Defer until Bevy 0.19 or the log line is promoted to `debug`.

## Active deferrals (Sprint 41–42)

- **Triplanar terrain shader bind-group layout bug.** The custom
  `ExtendedMaterial<StandardMaterial, TriplanarTerrainExt>` in
  `src/terrain_pbr.rs` + `assets/shaders/triplanar_terrain.wgsl` compiles and
  the AsBindGroup macro accepts the layout, but at runtime wgpu reports
  `Shader global ResourceBinding { group: 2, binding: 100 } is not available
  in the pipeline layout` against `pbr_opaque_mesh_pipeline`. Forcing
  `OpaqueRendererMethod::Forward` did not help. For Sprint 41 ship we fall
  back to a regular textured StandardMaterial on the terrain in `terrain.rs`
  (one layer, no triplanar / splat / wet), and `TerrainPbrPlugin` is
  unregistered. Re-investigation should:
  1. Check whether the AsBindGroup-emitted layout actually includes 100–116;
     compare against `cargo expand -p skoffroad terrain_pbr` output.
  2. Try mirroring Bevy's `examples/shader/extended_material.rs` exactly with
     a single uniform field and incrementally add textures.
  3. Verify that nothing else in the project loads
     `shaders/triplanar_terrain.wgsl` for the wrong material type.
  All textures, attribution, GraphicsQuality plumbing and the WGSL itself stay
  in tree — only the `add_plugins(TerrainPbrPlugin)` line is parked.

---

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
