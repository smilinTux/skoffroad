# skoffroad — PRD v3
## Rock Crawl & Trail Focus

**Date:** 2026-05-07
**Author:** Claude + chefboyrdave2.1
**Replaces:** MVP_SCOPE.md (v2, May 2026) for the next phase only — v2 is still the canonical scope doc for Phases 0–4.

---

## TL;DR

The codebase has gone **wide but not deep**. 147 modules, 30 sprints, every cosmetic feature you can imagine — and the vehicle is still the same constant-force suspension box from week 1. v3 stops adding cosmetics and goes deep on the off-road simulation: wheel-cam, articulation, rock garden, hillclimb, tire/terrain interaction.

---

## Honest state of the project (2026-05-07)

| Layer | Module count | Status |
|---|---|---|
| World decoration (sky, weather, biomes, mountains, clouds, godrays, snow, storm, ufo, birds, fish, wildlife, campfires) | ~25 | Excellent breadth, shallow depth — fine |
| HUD widgets (compass, minimap, speedometer, gauge, radar, fuel, low-fuel, course, race, rival, jump, drift, stunt, combo, nitro, damage, season, accessibility, paint, garage…) | ~30 | Saturated — diminishing returns |
| Game modes (race, time-trial, pursuit, demolition, explore, challenges, daily, career) | 8 | Saturated |
| Audio (music, engine_pro, surfaces, world_audio, mixer, crash, foghorn, thunder) | 8 | Excellent |
| UI menus (menu, help, map_select, fast_travel, session_summary, changelog, credits, intro_video, loading_screen) | ~10 | Saturated |
| Persistence (config, paint, keybindings, save, spawn_points) | 5 | Done |
| **Vehicle simulation** | **1 (vehicle.rs)** | **Single constant-force model from week 1** |
| **Camera** | **1 (camera.rs)** | **Single chase cam** |
| **Tire / surface interaction** | **0 functional** | surfaces.rs only changes AUDIO; mud.rs has drag but no real tire model |
| **Off-road challenge content** | **0 dedicated** | obstacles.rs scatters 69 generic obstacles; no rock garden, no hillclimb track |

**The architectural gap:** Phase 3 of MVP_SCOPE.md (winch, 4WD/2WD, low-range, tire deformation, tire temperature) was **never implemented**. The 30 sprints went into Phase 4 polish + Phase 5+ cosmetic content instead. v3 corrects course.

---

## v3 Goal (one sentence)

A player can switch to a wheel-mounted camera, drive into a dedicated rock-garden zone or up a hillclimb course, and *watch* their tires squash, articulate, and throw mud — with vehicle systems (low-range, airdown, winch) that meaningfully change how they navigate it.

---

## Must-haves (v3)

These are the user-flagged priorities and the simulation-depth gaps that block "best off-road simulator."

### M1. Wheel camera (front-left tire close-up)
**The user request:** "camera right behind the front left tire, so you can see it turn and articulate and get squished by going over rocks and see the mud/smoke etc coming out — think rock crawling and up close."
- Press a key (suggest `V` cycles modes) to switch camera among: chase / wheel-FL / wheel-FR / first-person interior / free-orbit
- Wheel-cam is mounted to the chassis offset to sit just outside the FL tire, looking inward & forward, so the tire fills ~30% of the frame
- FOV slightly wider so terrain detail in front of tire is visible
- Camera follows tire articulation but smooths jitter

### M2. Visible suspension articulation
**Why:** The current suspension RAYCASTS work but the wheel mesh doesn't visibly travel — wheels appear glued to the chassis. Without visible articulation, M1 (wheel-cam) shows nothing interesting.
- Each wheel mesh's local Y translates by the per-wheel compression value each frame
- Add visible control-arm cuboids that pivot to match the wheel travel angle
- Chassis already leans correctly (Avian rigid body) — verify

### M3. Rock garden zone
**The user request:** "maybe a rock garden — current and new plan together."
- A 60m × 60m dedicated zone (placed at e.g. (120, 0, 0)) with ~40 large procedurally-placed irregular boulders 0.5–3m radius, varied heights creating crawl lines
- Larger boulders use compound colliders (3–5 small spheres roughly approximating shape) so wheels actually contour over the curvature instead of flatlining on cuboid faces
- Visible from minimap with a "ROCK GARDEN" label
- Listed as a fast-travel destination

### M4. Hillclimb track + mode
**Confirmed missing — the user explicitly asked.**
- A dedicated hillclimb course: steep procedurally-generated 30°–55° slope, ~150m long, narrow channel with rock walls forcing the line
- Press `K` (or rebind) to enter hillclimb mode: spawn at base, timer starts at first 1m elevation gain, finishes at marked summit gate
- HUD: elevation gained, % grade, time, best time
- Best time persisted to `~/.skoffroad/hillclimb.json`
- Failure conditions: chassis flips (>120° tilt) or slides backward >5m

### M5. Tire deformation visual
**Currently:** wheels are perfect cylinders. Real off-road tires squash visibly on rocks.
- On contact, scale wheel mesh's local Y by `(1 - load_pressure * deform_factor)` so it visibly squashes at high load
- A subtle effect (max ~15% squash) — physical accuracy isn't the goal, the *feeling* is
- Bonus: scale tire X slightly outward (1 + 0.5 * squash) to show the bulge

### M6. Mud puddles + rut deformation
**Currently:** mud.rs adds drag but the ground looks identical. Player can't see where mud is.
- Place 8–12 visible dark puddles at low elevation points
- Driving through a puddle leaves a transient track mark (decals.rs already has the framework)
- Splash particle on puddle entry

---

## Should-haves (v3 if time permits)

These are real simulator features. Each is a single-sprint module.

### S1. Low-range gearing toggle (`L` key)
- Cuts max speed to 1/3, multiplies wheel torque by 3×, no actual engine RPM change required for v3
- Bottom-right HUD shows "LOW" / "HIGH"
- Persisted

### S2. Tire airdown (`T` key cycles 5/15/35 psi)
- Lower psi = more grip on rocks (×1.4 lateral grip at 5 psi) but reduced top speed (×0.8 at 5 psi) and faster suspension bump-stop hits
- HUD shows current psi
- Visual: tires look slightly squashed at low psi

### S3. Winch (`Y` key spool/unspool while looking at attach point)
- E key when within 10m of a hardpoint (boulders, posts) attaches winch line
- Spool pulls vehicle toward anchor at 1.5 m/s when active
- Visible cable: thin white cylinder between chassis-front and anchor
- Releases on Esc or if anchor distance > 30m

### S4. RPM-based torque curve
- Replace constant `DRIVE_FORCE_PER_WHEEL = 2600` with a real torque curve: peak torque at 2500 RPM, falls off above 4500
- Engine RPM = abs(wheel_avg_angular_speed) * gear_ratio * final_drive
- Engine sound (engine_pro.rs) already adapts; just feed it real RPM

### S5. 4WD / 2WD selector (`4` key)
- 2WD: rear-wheel drive only (front wheels free-spin)
- 4WD: all 4 driven (current behavior — default)
- Slight 2WD performance boost on hard surfaces, big disadvantage in mud / rocks

---

## Parked (deliberately)

These will probably never be needed for the "best off-road simulator" goal. Listed so we don't accidentally do them again.

| Parked item | Why parked |
|---|---|
| More cosmetic content (more biomes, more wildlife species, more buildings, more billboards, more landmarks) | Saturated. Adding more is busywork that doesn't change game feel. |
| More game modes (rally, drift competition, smash mode, capture-flag) | Saturated. 8 modes is plenty. |
| More HUD widgets | Saturated. Sprint 30 already had to invent the "STUNT" panel just to use the bottom-right corner. |
| Multiplayer / netcode | MVP_SCOPE.md cut this for good reason. |
| Token economy / NFTs / marketplace | MVP_SCOPE.md cut. |
| Voice CB radio | MVP_SCOPE.md cut. |
| Real FEM tire deformation (mesh-level, not visual scale) | Wildly out of scope; visual squash (M5) gives 95% of the feeling. |
| Modding API | Requires stable internal interfaces. We have 147 modules with shifting public APIs. Not now. |
| Mobile / Switch / Android port | Bevy's mobile story isn't there yet. |

---

## Comparison: original MVP_SCOPE.md (v2) vs reality

| MVP v2 Phase | Status (v0.5.8) |
|---|---|
| Phase 0 — Scaffold | ✅ Done |
| Phase 1 — Drivable vertical slice | ✅ Done — vehicle drives, suspension responds, dust particles, inspector |
| Phase 2 — Terrain depth + weather | ✅✅ Way overshot — 3 maps, biomes, storm/snow/foghorn, time-of-day, fog, godrays |
| **Phase 3 — Vehicle systems (winch, damage, tire deform, 4WD)** | **❌ Skipped entirely.** Damage exists as visual only; no winch, no tire deform, no low-range, no 4WD |
| Phase 4 — UX polish | ✅✅ Way overshot — 30 HUD widgets, full audio system, multiple cameras (well, one camera but…), menus |
| Phase 5+ — Mission system, save/load, replay, multi-vehicle | ✅ Partially done — race + replay + save + 5 vehicle variants exist; multiplayer / modding / FEM not done |

**v3's job: go back to Phase 3.**

---

## Sprint plan (v0.6.0 series)

### Sprint 31 — Wheel-cam + articulation (M1, M2)
5 modules, all single-file:
1. `camera_modes.rs` — cycle through 5 camera modes via `V` key. Modes: Chase (current default), WheelFL, WheelFR, FirstPerson, FreeOrbit
2. `wheel_articulation.rs` — translate wheel mesh local-Y by per-wheel compression each frame. Read existing `Wheel.current_compression`
3. `suspension_arms.rs` — spawn 4 control-arm cuboids per chassis that pivot in real-time to track wheel travel
4. `wheel_steer_visual.rs` — front wheels actually rotate around their Y-axis by steer_angle (verify if this exists; the orig conversation summary hinted it was added). Re-do if missing
5. `camera_smoothing.rs` — low-pass filter for wheel-cam jitter so on-rocks footage isn't seasick-inducing

### Sprint 32 — Rock garden + tire-on-rock + mud puddles (M3, M5, M6)
1. `rock_garden.rs` — 60×60m zone at (120, 0, 0); 40 boulders with compound sphere colliders for curvature
2. `tire_squash.rs` — visual scale of wheel mesh based on suspension load
3. `mud_puddles.rs` — 10 visible dark puddles at low elevations; transient track decals on entry
4. `splash_particles.rs` — water splash on puddle entry; mud kick on mud-zone wheel contact (separate from existing dust)
5. `ground_ruts.rs` — Persistent rut decals where wheels leave deep impression in mud (extension of decals.rs)

### Sprint 33 — Hillclimb mode + low-range + airdown (M4, S1, S2)
1. `hillclimb_track.rs` — procedural 150m slope course; spawn rock walls; finish gate
2. `hillclimb.rs` — mode state machine (Lobby → Active → Finished), timer, elevation tracking, persistence
3. `hillclimb_hud.rs` — elevation, grade, time, best time HUD
4. `low_range.rs` — `L` key toggle; multiplies torque ×3, divides max speed ÷3
5. `tire_pressure.rs` — `T` key cycles 5/15/35 psi; affects lateral grip + top speed; visual squash at low psi

### Sprint 34 (stretch) — Winch + RPM curve + 4WD selector (S3, S4, S5)
Three modules — only schedule if 31–33 land cleanly.

---

## Acceptance criteria for v0.6.0

A neutral player can:
1. Press `V` and see the camera attach behind the front-left tire, with the tire taking ~30% of the frame
2. Drive over a 1m rock and *visibly see the tire compress and deform*
3. See the suspension control arm pivot up while the wheel rises
4. Drive to the rock garden via fast-travel; crawl over varied boulders without flatlining on flat-faced cuboids
5. Press `K` to enter hillclimb; see the slope; race up; see best time persisted across sessions
6. Toggle `L` for low-range; observe vehicle climbs steeper grades (up the hillclimb) at 1/3 max speed
7. Toggle `T` to airdown; observe more grip, smaller top speed
8. Drive through a mud puddle; see splash particles and persistent rut tracks behind

---

## Non-goals (v3)

- We are not adding more biomes
- We are not adding more game modes
- We are not adding more vehicle variants beyond the 5 we have (Sprint 14's variants.rs)
- We are not adding more procedural ambient creatures (deer, fish, birds are enough)
- We are not adding more HUD widgets unless directly required by a M-tier feature
- We are not adding multiplayer, networking, or any backend service

---

## Risk + mitigations

| Risk | Mitigation |
|---|---|
| Wheel-cam motion sickness | Aggressive smoothing low-pass on the camera Transform. Add an `0` accessibility toggle to use a static "wheel TV monitor" overlay instead of camera-mounted |
| Compound rock colliders kill physics FPS | Cap to ~150 colliders total in rock garden; profile with F12 benchmark |
| Tire squash visual conflicts with vehicle_dirt / paint_shop / damage_visual material lerps | Squash is *scale*, not material — orthogonal, no conflict |
| Hillclimb track collides with existing terrain mesh | Use a Mesh-replacement strategy: spawn the slope as a separate static mesh + collider; existing terrain stays |

---

## Definition of done

This document is reviewed when **Sprint 33** lands (hillclimb playable). At that point we evaluate whether Sprint 34 (winch / RPM / 4WD) is necessary or whether the game already feels like the user's vision of a deep off-road simulator. The latter is acceptable.

---

## Appendix A: Why no more cosmetic sprints

Counting current cosmetic-only modules: birds_flock, fish, boats, wildlife, ufo, mountain_range, clouds, snow, storm, godrays, fireworks, campfires, billboards, gas_stations, traffic, buildings, landmarks, intro_video, ascii_logo, season, paint_shop, license_plate, exhaust_smoke, tire_smoke = **24 modules**. Adding a 25th will not change how the game *feels to drive*. Returning to Phase 3 (vehicle systems) will.
