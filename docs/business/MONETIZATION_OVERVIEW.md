# skoffroad Monetization — Overview

The master plan. Three parallel revenue tracks, sequenced so early-stage revenue funds later-stage build-out.

## Three tracks

### Track A — Brand / B2B (highest near-term revenue)

Sell custom-branded builds, sponsor billboards, and white-label experiences to overlanding / OEM / outdoor brands. Doesn't need a player audience — it needs a portfolio piece and a sales motion.

**Deliverables:**
- [`web/partners/index.html`](../../web/partners/index.html) — landing page (deploys at play.skoffroad.skworld.io/partners/ via release.yml).
- [`docs/business/DEPLOY_PARTNERS.md`](./DEPLOY_PARTNERS.md) — deployment + Formspree setup steps.
- [`docs/business/ONEPAGER.md`](./ONEPAGER.md) — brand-facing one-pager.
- [`docs/business/OUTREACH.md`](./OUTREACH.md) — cold/warm/agency email templates + 30-brand target list.
- [`docs/business/P0_APPLICATIONS.md`](./P0_APPLICATIONS.md) — pre-filled copy for the 4 P0 sponsorship programs (ARB, Smittybilt, YETI, BRCC).
- [`docs/business/BILLBOARD_SCATTER_SPEC.md`](./BILLBOARD_SCATTER_SPEC.md) — engineering spec for sponsor billboard placement.
- `src/sponsor_scatter.rs` — implementation (in progress).
- `src/brand_pack.rs` — JSON-driven brand pack loader (in progress).
- `assets/brand_packs/_house.json` — house-ads default pack.

**Revenue model:** $2.5k–$25k+ per deal; ~3-month sales cycle for cash deals, ~2 weeks for trade-for-promotion.

### Track B — Player monetization (highest long-term revenue)

Free-to-play with cosmetic IAP, three-currency economy, Trail Pass (Battle Pass), and rewarded video. Fortnite-style mechanics scaled to indie reality.

**Deliverables:**
- [`docs/business/TRAIL_PASS_DESIGN.md`](./TRAIL_PASS_DESIGN.md) — Battle Pass design doc.
- `src/wallet.rs` — wallet + inventory stub (in progress).
- `src/trail_pass.rs` — XP, tiers, claim flow (future).
- `src/account.rs` — local-first account record (future).

**Revenue model:** ~5% conversion to premium pass × $4.99 × MAU + cosmetic top-up purchases. Compounds with audience size.

### Track C — Ad networks + portals (passive)

Integrate Playgama Bridge for rewarded video + interstitials, submit to CrazyGames + Poki for sponsorship offers and rev share.

**Deliverables:**
- [`docs/business/AD_SDK_INTEGRATION.md`](./AD_SDK_INTEGRATION.md) — Playgama Bridge + AppLixir wiring.
- [`docs/business/PORTAL_SUBMISSIONS.md`](./PORTAL_SUBMISSIONS.md) — CrazyGames / Poki / Playgama submission package.
- `src/ad_sdk_bridge.rs` — wasm-bindgen JS bridge (in progress).
- `web/sk_ad_sdk.js` — JS shim.

**Revenue model:** $0.50–$25 eCPM × ad views × DAU. Higher with US/UK premium traffic.

## Sequencing — what to do, in what order

### Week 0–2 (now)

- Ship `web/partners/index.html` to skoffroad.skworld.io/partners.
- Send 10 cold outreach emails using `OUTREACH.md` templates.
- Apply to Smittybilt + ARB + YETI + Black Rifle Coffee partner programs (free, takes 1 hour total).
- Land `sponsor_scatter.rs` + `brand_pack.rs` so we can mock up any brand in 1 hour.
- Build one mockup brand pack as portfolio piece (pick one P0 brand from outreach list).

### Week 2–6

- Wire `ad_sdk_bridge.rs` + JS shim + Playgama Bridge.
- Submit to CrazyGames + Poki.
- Add wallet + inventory stub (`src/wallet.rs`).
- Continue outreach: 10 emails/week. Target 1 closed Trailhead-tier deal by week 6.

### Week 6–12

- Ship Trail Pass v1 with default season + premium upsell via Mud Coins (no Stripe yet).
- Iterate first-30s onboarding based on portal analytics.
- Land first paid brand deal; use revenue to fund Stripe + account system.

### Week 12+

- Add Stripe IAP for Premium Gems / Trail Pass.
- Build account system (local-first → cloud sync).
- Negotiate first portal sponsorship buyout (likely CrazyGames Featured slot).
- Pitch agencies for white-label builds (Template D in `OUTREACH.md`).

## Dependency graph

```
brand_pack.rs ──┬──► sponsor_scatter.rs ───► billboard impressions
                │
                └──► livery brand-pack overrides ───► Trail Pass cosmetic rewards

wallet.rs ──────► trail_pass.rs ───► premium upsell ─┬─► Mud Coins (v1)
                                                     │
                                                     └─► Stripe IAP (v2)

ad_sdk_bridge.rs ──► refuel / respawn / livery slots
                │
                └──► CrazyGames + Poki + Playgama submissions
```

## Single source of truth

This file is the canonical roadmap. Other docs in `docs/business/` are detail sub-plans. If anything contradicts, this file wins.

## Open questions

- **Stripe vs. Paddle vs. Lemon Squeezy** for IAP? (Future decision; Stripe default unless EU VAT handling becomes painful.)
- **Where does account state live** — `~/.skoffroad/account.json`, our own auth, or piggyback skworld identity? (Open. Probably skworld identity once it exists.)
- **Should we open-source `_house.json` brand pack format** to let community ship reskins? (Pros: viral. Cons: complicates moderation.)
- **Real OEM (Jeep/Ford/Toyota) trademark risk** — defer to legal before naming vehicles. House models stay generically named.
