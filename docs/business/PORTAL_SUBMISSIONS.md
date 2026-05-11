# Portal Submission Package — CrazyGames, Poki, Playgama, Y8

## Why submit to portals

- **Free traffic.** Portal directs millions of MAU at indie games. CrazyGames alone has ~30M MAU.
- **Sponsorship buyouts.** $5k–$25k+ for exclusivity / featured placement on quality builds.
- **Revenue share on portal traffic.** Typically 50%–60% of ad revenue, no upfront work after integration.
- **SEO halo.** Portal listings rank for niche queries ("off-road browser game", "free 4x4 game online").

## Submission targets, ranked

| Portal | MAU | Genre fit | Pay model | Submit at |
| --- | --- | --- | --- | --- |
| **CrazyGames** | ~30M | Excellent (top racing portal) | Rev share + sponsorship buyouts | developer.crazygames.com |
| **Poki** | ~50M | Strong | Rev share + featured slots | poki.com/developers |
| **Playgama Bridge** | (network of portals) | Strong | 80% rev share, distributes broadly | playgama.com |
| **GameMonetize** | (B2B network) | Medium | Rev share + non-exclusive license | gamemonetize.com |
| **Y8** | ~10M | OK | Rev share | y8.com/work-with-us |
| **Newgrounds** | ~5M | Niche; cult quality audience | Cuts on tips + tournaments | newgrounds.com |

Start with **CrazyGames + Poki**. Submit to Playgama once those are live and reviewed.

## Pre-submission checklist

- [ ] Game runs at 60fps on a mid-range laptop with integrated GPU.
- [ ] Loads in <10 seconds on a 10 Mbps connection.
- [ ] Mobile (touch) playable.
- [ ] No copyrighted brand names, music, vehicle names (Cherokee, Wrangler, etc.) in default build.
- [ ] Pause menu / Esc works.
- [ ] Mute button accessible.
- [ ] No outbound links without click consent.
- [ ] No social-media share buttons that auto-open.
- [ ] HTTPS-only assets.
- [ ] Bundle size < 50 MB (current ~12 MB ✓).
- [ ] Playgama Bridge SDK integrated (see [AD_SDK_INTEGRATION.md](./AD_SDK_INTEGRATION.md)).

## Asset deliverables

CrazyGames + Poki both require roughly:

1. **Build:** WASM bundle, runnable via single `index.html`.
2. **Icon:** 512×512 PNG, branded, no UI text.
3. **Cover image:** 1280×720 PNG, action shot of vehicle on terrain.
4. **Screenshots:** 5–10× 1920×1080, varied scenes (cockpit, chase cam, night, mud splash, jump).
5. **Gameplay video:** 30–60 seconds, MP4, 1080p, no narration, just play.
6. **Description (short):** 80–150 chars.
7. **Description (long):** 500–1000 chars.
8. **Genre tags:** racing, driving, simulation, 3D, off-road, jeep, sandbox.
9. **Controls list:** Keyboard + mouse + gamepad mappings.
10. **Privacy / age rating:** Everyone (E).

## Draft descriptions

### Short (140 chars)

> Drive a tough off-road truck across procedurally generated trails. Real physics, real suspension, mud, water, weather. Free in your browser.

### Long (~800 chars)

> skoffroad is a physics-driven off-road driving sandbox. Take a Jeep-class truck across procedurally generated trails — mud, water, rocks, ramps, weather, day/night cycles. Real raycast suspension, real torque curves, real consequences if you flip it.
>
> Tune the chassis. Pick a livery. Set personal-best times. Chase the horizon.
>
> Built in Rust with WebGPU, so it looks console-quality and runs in any modern browser without an install.
>
> **Controls:** WASD or arrows to drive, Space to brake, R to reset, J to right the chassis after a roll. 1–6 to switch paint. V to toggle cockpit camera. M for the mini-map. H to toggle HUD.
>
> No ads in gameplay, no pay-to-win. Optional cosmetics. Just driving.

### Tags

`racing` `driving` `simulation` `3d` `off-road` `physics` `sandbox` `jeep` `truck` `webgpu` `single-player` `procedural` `relaxing` `realistic`

## Build packaging steps

```bash
# Production WASM build
trunk build --release

# Verify bundle size
du -sh dist/

# Bundle for upload
cd dist && zip -r ../skoffroad-portal-v1.zip ./* && cd ..

# Test locally one more time
python -m http.server 8080 -d dist
```

## Submission notes per portal

### CrazyGames (https://developer.crazygames.com)

- Sign up → verify email → "Submit Game".
- Upload `skoffroad-portal-v1.zip` (their CDN handles serving).
- Required: their **CrazyGames SDK** for game lifecycle events (ad triggers, happytime, sad time, gameplay start/stop). Lightweight; one extra script tag.
- Review time: 5–10 business days.
- Revenue: rev share starts immediately; sponsorship offers come after audience builds.
- After acceptance, can apply for "Featured" / "Editor's Choice" — these are negotiated separately.

### Poki (https://poki.com/developers)

- Sign up → submit game URL (not a zip — hosted somewhere they can crawl).
- Required: **Poki SDK** integration (interstitials, rewarded ads, gameplay lifecycle).
- Review time: 2–4 weeks (slower than CrazyGames).
- Will request changes (typically: faster initial load, mobile touch QA, reduce intro friction).
- Revenue model: rev share, no upfront buyout typically for indie new submissions.

### Playgama Bridge (https://playgama.com)

- Integrate Bridge SDK (already in [AD_SDK_INTEGRATION.md](./AD_SDK_INTEGRATION.md)).
- Submit single build → Playgama distributes to their network (Yandex, vk.com, etc.) including auto-localized landing pages.
- 80% rev share.
- Review time: ~1 week.

### GameMonetize (https://gamemonetize.com)

- Lower bar. Non-exclusive license deals.
- Sometimes offers small upfront cash ($200–$2,000) for exclusive non-exclusive listing.
- Decent if you want to maximize footprint, but lower-quality audience.

## What to expect post-launch

| Milestone | Realistic timeline | Revenue order of magnitude |
| --- | --- | --- |
| Listed on CrazyGames | 1 month after submit | $50–$500/mo |
| Listed on Poki | 2–3 months after submit | $200–$2k/mo |
| Featured / Editor's Pick | 3–6 months, depending on retention metrics | $2k–$20k/mo |
| First sponsorship buyout offer | After ~6 months + good metrics | $5k–$25k one-time |
| Multi-portal distribution via Playgama | Within a quarter | $500–$5k/mo |

Numbers are rough industry envelopes. Retention is the variable that moves them by 10×.

## Metrics they care about

Portals score games on:

- **D1 retention** (target: >25%)
- **Average session length** (target: >4 minutes)
- **Pageviews per session** (target: >1.5)
- **Ad fill rate × completion** (target: >70% completion on rewarded videos)
- **Bounce rate** (target: <50%)

Optimize the **first 30 seconds** — that's where 60% of players churn. Trim any loading screen, autostart the engine sound, drop the player on a hill with momentum.

## Submission TODO

1. [ ] Bundle production build (`trunk build --release`).
2. [ ] Cut a 45-second gameplay video.
3. [ ] Capture 8 screenshots (use F2 photo mode).
4. [ ] Make 512×512 icon + 1280×720 cover.
5. [ ] Write final descriptions (use drafts above).
6. [ ] Integrate Playgama Bridge SDK.
7. [ ] Test on iPhone 13 + mid-range Android (touch controls).
8. [ ] Submit CrazyGames.
9. [ ] Submit Poki.
10. [ ] Submit Playgama.
11. [ ] After 30 days: review analytics, iterate first-30s flow.
