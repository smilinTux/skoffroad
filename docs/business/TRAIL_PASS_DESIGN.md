# Trail Pass — Battle Pass design doc

## TL;DR

A 60-day seasonal progression with a free track and a $4.99 premium track. Players earn **Trail XP** by playing (time trials, distance, achievements). Tiers unlock cosmetics (liveries, decals, wheels, light bars, horn sounds). Drives daily/weekly engagement and monthly revenue without touching gameplay balance — everything is cosmetic.

This is the Fortnite-style "Battle Pass" reskinned for skoffroad. Out of all Fortnite mechanics this is the highest-ROI to port: it gives you predictable MRR, drives DAU, and creates a content cadence brands can sponsor as title sponsors.

## Why this works at small scale

A Battle Pass doesn't need a huge audience to print money. At 5,000 MAU × 5% conversion × $4.99 = **$1,250/season**. At 50,000 MAU × 5% × $4.99 = **$12,500/season**. The math compounds with audience; the systems are the same.

Fortnite's Battle Pass is also their *most stable* revenue line — concerts and skins spike, but the Pass is metronome MRR. Same dynamic here.

## Season structure

- **Length:** 60 days (matches Fortnite cadence, gives breathing room for content drops).
- **Tiers:** 50 levels per pass.
- **Total XP needed:** 100,000 (so a level = 2,000 XP, achievable in ~15 min of solid play).
- **Free track:** ~12 unlocks across 50 tiers (every ~4 tiers).
- **Premium track:** all 50 unlocks + 800 Mud Coins refund baked in (Fortnite trick — premium "pays itself back" if you grind hard, which it doesn't, but the option matters).

## Currencies

| Currency | Source | Spend |
| --- | --- | --- |
| **Trail XP** | Earned passively from play | Levels up the Trail Pass only |
| **Mud Coins** | Earned in small amounts from achievements; purchased in bulk for $ | Buy Trail Pass, buy cosmetics from shop |
| **Premium Gems** | Watching rewarded videos; bonus drops; premium Pass tier rewards | Time-skip unlocks, premium-only cosmetics |

Three currencies is one more than Fortnite — but Premium Gems are specifically the ad-driven currency, which keeps the IAP economy separate from the rewarded-video economy. Don't merge them.

## XP sources

| Source | XP | Cap |
| --- | --- | --- |
| Time-trial completion | 200 | — |
| New personal best on a trial | 500 | — |
| First 10 km of the day | 1× = 10 XP/km | 100 XP/day from this source |
| Achievement unlock | 250–1000 | One-time per achievement |
| Daily login | 100 | 1×/day |
| Daily challenge completion | 500 each | 3 challenges/day |
| Weekly challenge | 2,000 each | 5 challenges/week |

Targets: a casual player who logs in daily and does dailies hits **~2,500 XP/day** = full pass in 40 days (comfortably under the 60-day window). A heavy player finishes in 2–3 weeks.

## Reward content by tier (example pass)

| Tier | Free track | Premium track |
| --- | --- | --- |
| 1 | "Rookie" decal | Premium splash + 100 Premium Gems |
| 5 | 50 Mud Coins | Branded livery #1 |
| 10 | "Dust Devil" horn sound | "Canyon Crawler" wheel set + 100 Gems |
| 15 | Decal pack | Light bar variant |
| 20 | Mini-map skin | Tier-2 livery + 200 Gems |
| 25 | 100 Mud Coins | Custom HUD theme |
| 30 | Trail badge | Exhaust note variant |
| 35 | "Storm Chaser" decal | Tier-3 livery + 200 Gems |
| 40 | Compass skin | Animated underglow |
| 45 | 100 Mud Coins | Wheel set #2 |
| 50 | "Season Veteran" badge | **Exclusive livery** + 400 Gems |

For sponsored seasons, replace 1–2 tier-rewards with the brand's livery / decals.

## Sponsor integration

Each season can have a **title sponsor**. This becomes the "Campaign" tier offering in [PARTNERS.md](./ONEPAGER.md) but applied to the *player-facing* pass instead of just billboards.

- Title sponsor's livery → tier 50 premium reward (most coveted slot).
- Title sponsor's logo on the Trail Pass UI banner.
- One tier reward is "watch a sponsor's 30s video, get 200 Gems" (rewarded-video integration).
- Season name reflects sponsor (e.g. "ARB Summer Series", "BFGoodrich Mud Season").

Title sponsorship priced at **$15k–$30k/season** depending on audience size. At 50k MAU with ~5% premium-pass conversion the brand effectively co-brands ~2,500 paying sessions' worth of content.

## UI sketch

```
+---------------------------------- TRAIL PASS · Season 1: "Open Range" ----+
|                                                                            |
|  XP: 23,400 / 100,000      Tier 12 / 50      37 days left                  |
|  [█████████████░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░]      |
|                                                                            |
|   FREE      [11]  [12·U]  [13]   [14]   [15·U]  [16]   ...                |
|   PREMIUM   [11·🔒]  [12·🔒]  [13·🔒]  [14·🔒]  [15·🔒]  [16·🔒]  ...    |
|                                                                            |
|  [ UNLOCK PREMIUM — $4.99 / 800 Mud Coins ]   [ SHOP ]   [ DAILIES ]      |
+----------------------------------------------------------------------------+
```

## State / persistence

Extend `config.rs::ConfigData` with:

```rust
struct TrailPassState {
    season_id: String,
    xp: u32,
    premium_owned: bool,
    claimed_tiers: u64,  // bitmask, 50 bits
    last_daily_claim_at: i64,
    daily_challenge_progress: [u32; 3],
    weekly_challenge_progress: [u32; 5],
}
```

Add `TrailPassState` to the persisted save layer. WASM persists via existing `platform_storage` localStorage abstraction. Cloud sync is a future concern (account system not yet built).

## XP system implementation

New plugin `src/trail_pass.rs`:

- Subscribe to existing `GameEvent` ring buffer for `DistanceMilestone`, `SpeedMilestone`, time-trial completion events (need to verify which events exist).
- Maintain a session XP delta, flush to persistent `TrailPassState` on quit / period.
- UI: new HUD overlay accessed via a new key (`P` for Pass?), or a button in the pause menu.

## Tier unlock flow

When XP crosses a tier threshold:

1. Set the corresponding bit in `claimed_tiers` only when the player explicitly claims it (not auto — Fortnite's data shows manual claim drives retention).
2. Push a `TierUnlockEvent` to event log; HUD ticker shows "Tier 13 ready to claim!".
3. On claim, apply reward to inventory / wallet / livery system.
4. For premium-track tiers without premium-owned, show the upsell modal.

## Premium-pass purchase flow

1. Player clicks "Unlock Premium".
2. Modal offers $4.99 USD OR 800 Mud Coins.
3. USD path → Stripe Checkout (or equivalent). For initial launch, **no IAP** — drive premium via Mud Coin spend, sell Mud Coins later. Lower regulatory friction (no in-game purchase laws), lower Stripe-setup cost.
4. Mud Coin path → debit wallet, set `premium_owned = true`, retroactively claim premium rewards for tiers already passed.

## Acceptance criteria for v1

- [ ] `TrailPassState` persists across sessions on native + WASM.
- [ ] XP increments correctly from at least 3 sources (distance, time-trial, daily login).
- [ ] HUD shows current tier and XP progress.
- [ ] Player can claim unlocked free-track tiers.
- [ ] Premium upsell modal appears for locked premium-track tiers.
- [ ] One full default season ("Open Range") is content-complete with placeholder cosmetics.

## Out of scope for v1

- Daily / weekly challenges (start with a single XP fountain: play = XP).
- Stripe / actual payments.
- Cloud sync (local-only persistence is fine until accounts exist).
- Sponsored season UI banner (wait until first sponsor signs).

## Sequencing with other monetization tracks

This depends on:

- Wallet + inventory stub (track #7) → must land first.
- Brand-pack loader (track #6) → premium liveries reuse the same JSON pack format.
- Account system → required for cloud sync and IAP fraud prevention; **not** required for v1.

Build order: wallet stub → brand-pack loader → Trail Pass core (XP + UI) → premium upsell → IAP integration.
