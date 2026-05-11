// Ad SDK bridge — WASM-only externs to a JS shim (window.SkAdSdk).
//
// The JS side (web/sk_ad_sdk.js) routes to whatever ad provider is alive on
// this page: Playgama Bridge on Playgama-distributed portals, AppLixir on
// our self-host. On native, every call is a no-op.
//
// Game systems emit AdRequest events; the dispatcher forwards to JS; the
// drain system polls JS for completed results and re-emits as AdResult
// events. Listeners (e.g. fuel.rs for a refuel reward) react to AdResult.
//
// Public API:
//   AdSdkPlugin
//   AdRequest, AdResult  (Bevy events)
//   ad_sdk_available() -> bool

use bevy::prelude::*;
use serde::Deserialize;

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct AdSdkPlugin;

impl Plugin for AdSdkPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<AdRequest>()
            .add_message::<AdResult>()
            .add_systems(Update, (dispatch_requests, poll_results));
    }
}

// ---------------------------------------------------------------------------
// Messages (Bevy 0.18 renamed buffered Events → Messages)
// ---------------------------------------------------------------------------

#[derive(Message, Debug, Clone)]
pub enum AdRequest {
    /// Rewarded video — caller should grant the reward only on AdResult::Watched.
    Rewarded { slot: &'static str },
    /// Interstitial — fire-and-forget. No reward, just a between-runs break.
    Interstitial { placement: &'static str },
}

#[derive(Message, Debug, Clone)]
pub enum AdResult {
    Watched  { slot: String },
    Skipped  { slot: String },
    Shown    { placement: String },
    Failed   { slot: String, reason: String },
}

pub fn ad_sdk_available() -> bool {
    web::is_available()
}

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

fn dispatch_requests(mut requests: MessageReader<AdRequest>) {
    for req in requests.read() {
        match req {
            AdRequest::Rewarded { slot } => web::show_rewarded(slot),
            AdRequest::Interstitial { placement } => web::show_interstitial(placement),
        }
    }
}

fn poll_results(mut writer: MessageWriter<AdResult>) {
    let raw = web::drain_results();
    if raw == "[]" || raw.is_empty() { return; }
    let parsed: Vec<JsResult> = match serde_json::from_str(&raw) {
        Ok(v) => v,
        Err(e) => { warn!("ad_sdk: failed to parse results: {}", e); return; }
    };
    for r in parsed {
        match r.kind.as_str() {
            "watched"  => { writer.write(AdResult::Watched { slot: r.slot.unwrap_or_default() }); }
            "skipped"  => { writer.write(AdResult::Skipped { slot: r.slot.unwrap_or_default() }); }
            "shown"    => { writer.write(AdResult::Shown { placement: r.placement.unwrap_or_default() }); }
            "failed"   => { writer.write(AdResult::Failed {
                                slot: r.slot.unwrap_or_default(),
                                reason: r.reason.unwrap_or_default(),
                            }); }
            other => warn!("ad_sdk: unknown result kind \"{}\"", other),
        }
    }
}

#[derive(Deserialize)]
struct JsResult {
    kind: String,
    #[serde(default)] slot: Option<String>,
    #[serde(default)] placement: Option<String>,
    #[serde(default)] reason: Option<String>,
}

// ---------------------------------------------------------------------------
// WASM <-> JS interop (stubbed out on native)
// ---------------------------------------------------------------------------

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
