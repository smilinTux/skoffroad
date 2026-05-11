# Ad SDK Integration Plan (Playgama Bridge + AppLixir fallback)

## Why Playgama Bridge

- One SDK, one build → publishes to **CrazyGames, Poki, Y8, GameMonetize, Facebook Instant Games, Yandex Games**.
- Standardized rewarded / interstitial / banner API.
- ~80% revenue share (most portals are ~60%).
- Free, no upfront cost.

Docs: https://playgama.com/bridge/

## Fallback: AppLixir for self-hosted

When the game runs on our own domain (`play.skoffroad.skworld.io`), Playgama Bridge fails over to a no-op. For self-hosted ad revenue use **AppLixir** (rewarded video specialist) or **Google AdSense for Games**.

## Architecture

```
┌────────────────────┐
│  Bevy / Rust       │   emits AdRequest events via Bevy EventWriter
│  src/ad_sdk_bridge │
└─────────┬──────────┘
          │ wasm-bindgen extern "C"
          ▼
┌────────────────────┐
│  index.html JS     │   thin shim: routes to Playgama OR AppLixir
│  window.SkAdSdk    │
└─────────┬──────────┘
          │
          ├──► Playgama Bridge SDK (if window.PlaygamaBridge exists)
          └──► AppLixir SDK         (fallback)
```

The Bevy side stays agnostic. The JS shim decides which provider is alive based on `window.PlaygamaBridge` presence.

## Bevy-side API

```rust
// src/ad_sdk_bridge.rs

#[derive(Event)]
pub enum AdRequest {
    Rewarded { slot: &'static str },
    Interstitial { placement: &'static str },
}

#[derive(Event)]
pub enum AdResult {
    Watched { slot: &'static str },
    Skipped { slot: &'static str },
    Failed  { slot: &'static str, reason: String },
}

pub struct AdSdkPlugin;

impl Plugin for AdSdkPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<AdRequest>()
           .add_event::<AdResult>()
           .add_systems(Update, dispatch_ad_request)
           .add_systems(Update, poll_ad_results);
    }
}
```

## JS shim (`web/sk_ad_sdk.js`)

```javascript
(function(){
  const queue = []; // results pending pickup by WASM

  window.SkAdSdk = {
    show_rewarded(slot) {
      if (window.PlaygamaBridge) {
        return window.PlaygamaBridge.advertisement.showRewarded()
          .then(()  => queue.push({ kind: 'watched', slot }))
          .catch(e => queue.push({ kind: 'failed',  slot, reason: e.message }));
      }
      if (window.applixir) {
        return new Promise(resolve => {
          window.applixir.renderAd({
            zoneId: window.SK_APPLIXIR_ZONE,
            callbacks: {
              adFinished: () => { queue.push({ kind: 'watched', slot }); resolve(); },
              adCanceled: () => { queue.push({ kind: 'skipped', slot }); resolve(); },
              adError:    e  => { queue.push({ kind: 'failed', slot, reason: e }); resolve(); },
            },
          });
        });
      }
      queue.push({ kind: 'failed', slot, reason: 'no_provider' });
      return Promise.resolve();
    },

    show_interstitial(placement) {
      if (window.PlaygamaBridge) {
        return window.PlaygamaBridge.advertisement.showInterstitial()
          .then(()  => queue.push({ kind: 'shown', placement }))
          .catch(e => queue.push({ kind: 'failed', placement, reason: e.message }));
      }
      queue.push({ kind: 'skipped', placement });
      return Promise.resolve();
    },

    drain_results() {
      const r = queue.slice();
      queue.length = 0;
      return JSON.stringify(r);
    },

    is_available() { return !!(window.PlaygamaBridge || window.applixir); },
  };
})();
```

## index.html additions

```html
<!-- Playgama Bridge (loaded conditionally by Playgama portals — safe everywhere) -->
<script src="https://cdn.playgama.com/bridge/v1.js"></script>

<!-- AppLixir fallback (only on self-host) -->
<script>
  if (location.hostname.endsWith('skworld.io')) {
    window.SK_APPLIXIR_ZONE = '<YOUR_ZONE_ID>';
    var s = document.createElement('script');
    s.src = 'https://applixir.com/applixir.js';
    document.head.appendChild(s);
  }
</script>

<script src="./sk_ad_sdk.js"></script>
```

## Bevy ↔ JS bindings

```rust
#[cfg(target_arch = "wasm32")]
mod web {
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(js_namespace = SkAdSdk, js_name = show_rewarded)]
        pub fn show_rewarded(slot: &str);

        #[wasm_bindgen(js_namespace = SkAdSdk, js_name = show_interstitial)]
        pub fn show_interstitial(placement: &str);

        #[wasm_bindgen(js_namespace = SkAdSdk, js_name = drain_results)]
        pub fn drain_results() -> String;

        #[wasm_bindgen(js_namespace = SkAdSdk, js_name = is_available)]
        pub fn is_available() -> bool;
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod web {
    pub fn show_rewarded(_slot: &str) {}
    pub fn show_interstitial(_placement: &str) {}
    pub fn drain_results() -> String { "[]".to_string() }
    pub fn is_available() -> bool { false }
}
```

## Ad placement plan

| Slot ID | Trigger | Provider call | Reward |
| --- | --- | --- | --- |
| `refuel` | Fuel hits 0 | `show_rewarded` | Refill to 50% |
| `respawn_3` | 3rd respawn in a session | `show_rewarded` | Skip 3s respawn timer |
| `livery_preview` | Player views a brand-locked livery | `show_rewarded` | 24-hour preview of livery |
| `daily_bonus` | First login of the day | `show_rewarded` | 50 Premium Gems |
| `time_trial_retry` | After failing a time trial | `show_interstitial` | (none — between-runs ad) |
| `season_skip` | Player wants to skip a Pass tier | `show_rewarded` | +500 XP toward next tier |

**Rate limits** (avoid Playgama / Poki rejection):
- No interstitial in the first 60 seconds of a session.
- No interstitial within 60 seconds of another interstitial.
- Rewarded video is always opt-in (must show "Watch ad for X" UI).

## Refuel example wiring

```rust
// In src/fuel.rs, modify consume_fuel:
fn consume_fuel(
    /* existing params */
    mut ad_writer: EventWriter<AdRequest>,
    mut just_emptied: Local<bool>,
) {
    /* existing logic */
    if fuel.current_l == 0.0 && !*just_emptied {
        ad_writer.send(AdRequest::Rewarded { slot: "refuel" });
        *just_emptied = true;
    } else if fuel.current_l > 0.0 {
        *just_emptied = false;
    }
}

// In src/ad_sdk_bridge.rs, listen for the result:
fn apply_refuel_reward(
    mut events: EventReader<AdResult>,
    mut fuel: ResMut<Fuel>,
) {
    for ev in events.read() {
        if let AdResult::Watched { slot: "refuel" } = ev {
            fuel.current_l = (fuel.current_l + 30.0).min(60.0);
        }
    }
}
```

## Portal eligibility checklist

Before submitting to Playgama / Poki / CrazyGames:

- [ ] Game loads in <10 seconds on a mid-range mobile (3G simulation).
- [ ] Mobile touch controls work (already shipped per `assets/touch-controls.js`).
- [ ] No copyrighted assets (audio, fonts, vehicles, brand names — except house-pack).
- [ ] Pause menu works (Esc / equivalent).
- [ ] No interstitials in first 60s.
- [ ] No external links open without user gesture (click-to-open only).
- [ ] All audio respects a master mute toggle.
- [ ] HTTPS-only assets.
- [ ] Build size <50 MB (current ~12 MB ✓).

## Revenue model — rough envelope

Industry numbers, conservative side:

- Rewarded video eCPM: $8–$25 (varies by region, premium for US/UK).
- Interstitial eCPM: $4–$10.
- Banner eCPM: $0.50–$2 (don't bother).
- Programmatic in-game ambient (Bidstack/Anzu): $2–$6 eCPM passive.

At 10k DAU, 60% session ad opt-in, 2 rewarded videos avg/session, $12 eCPM:
**10,000 × 0.6 × 2 × $0.012 = $144/day = ~$4,300/month.**

That number is fragile — heavily depends on geography mix, ad fill rate, and seasonality — but it's the realistic order of magnitude. Doubles or triples with strong premium-region traffic.

## Sequencing

1. Build the JS shim (`web/sk_ad_sdk.js`) and load conditionally in `index.html`.
2. Add the Rust bridge crate (`src/ad_sdk_bridge.rs`).
3. Wire the `refuel` slot first (lowest risk, highest user-value).
4. Validate on a Playgama dev account before submitting.
5. Add remaining slots as analytics warrant.
6. Submit to CrazyGames, then Poki, then Playgama portal network.
