// skoffroad ad SDK JS shim.
//
// Bevy/WASM side (src/ad_sdk_bridge.rs) calls window.SkAdSdk.* via wasm-bindgen.
// This shim routes to whichever ad provider is alive on the current page:
//   - Playgama Bridge   (on Playgama-distributed portals: CrazyGames, Poki, etc.)
//   - AppLixir          (on self-hosted skworld.io domain)
//   - no-op             (otherwise)
//
// Results are queued so the WASM side can drain them on its own update tick.

(function () {
  const queue = [];

  function provider() {
    if (typeof window.PlaygamaBridge !== "undefined") return "playgama";
    if (typeof window.applixir !== "undefined")       return "applixir";
    return "none";
  }

  function showRewardedPlaygama(slot) {
    return window.PlaygamaBridge.advertisement.showRewarded()
      .then(() => queue.push({ kind: "watched", slot }))
      .catch((e) => queue.push({ kind: "failed", slot, reason: String(e) }));
  }

  function showRewardedApplixir(slot) {
    return new Promise((resolve) => {
      window.applixir.renderAd({
        zoneId: window.SK_APPLIXIR_ZONE,
        callbacks: {
          adFinished: () => { queue.push({ kind: "watched", slot }); resolve(); },
          adCanceled: () => { queue.push({ kind: "skipped", slot }); resolve(); },
          adError:    (e) => { queue.push({ kind: "failed", slot, reason: String(e) }); resolve(); },
        },
      });
    });
  }

  function showInterstitialPlaygama(placement) {
    return window.PlaygamaBridge.advertisement.showInterstitial()
      .then(() => queue.push({ kind: "shown", placement }))
      .catch((e) => queue.push({ kind: "failed", placement, reason: String(e) }));
  }

  window.SkAdSdk = {
    show_rewarded(slot) {
      switch (provider()) {
        case "playgama": return showRewardedPlaygama(slot);
        case "applixir": return showRewardedApplixir(slot);
        default:
          queue.push({ kind: "failed", slot, reason: "no_provider" });
          return Promise.resolve();
      }
    },

    show_interstitial(placement) {
      switch (provider()) {
        case "playgama": return showInterstitialPlaygama(placement);
        default:
          queue.push({ kind: "skipped", placement });
          return Promise.resolve();
      }
    },

    drain_results() {
      if (queue.length === 0) return "[]";
      const out = JSON.stringify(queue);
      queue.length = 0;
      return out;
    },

    is_available() {
      return provider() !== "none";
    },
  };
})();
