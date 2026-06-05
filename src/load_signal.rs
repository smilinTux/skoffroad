//! LoadSignalPlugin — fires `window.skoffroadReady()` once the world is
//! actually rendering (VehicleRoot exists + several frames have elapsed).
//!
//! On WASM this calls a JS function defined in index.html that removes the
//! splash screen.  On native it is a complete no-op.
//!
//! Gate all web-sys / wasm-bindgen calls with `#[cfg(target_arch = "wasm32")]`
//! so native builds and drive_test compile cleanly.

use bevy::prelude::*;

// ── Resource that tracks how many frames we've waited after VehicleRoot ──────

#[derive(Resource, Default)]
struct ReadyFrames(u32);

/// Minimum number of Update ticks after VehicleRoot appears before we signal.
/// A few frames ensures the first rendered image is real geometry, not a
/// WebGPU clear-black frame.
const READY_AFTER_FRAMES: u32 = 8;

// ── Plugin ────────────────────────────────────────────────────────────────────

pub struct LoadSignalPlugin;

impl Plugin for LoadSignalPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ReadyFrames>()
           .init_resource::<SignalSent>()
           .add_systems(Update, maybe_signal_ready);
    }
}

/// Flag so we call the JS function exactly once.
#[derive(Resource, Default)]
struct SignalSent(bool);

fn maybe_signal_ready(
    vehicle: Option<Res<crate::vehicle::VehicleRoot>>,
    mut frames: ResMut<ReadyFrames>,
    mut sent: ResMut<SignalSent>,
) {
    if sent.0 {
        return;
    }
    if vehicle.is_none() {
        // World not ready yet — reset counter so it only starts once the
        // vehicle actually exists (handles respawns on the first load).
        frames.0 = 0;
        return;
    }

    frames.0 += 1;

    if frames.0 >= READY_AFTER_FRAMES {
        sent.0 = true;
        call_js_ready();
    }
}

// ── WASM-only JS bridge ───────────────────────────────────────────────────────

#[cfg(target_arch = "wasm32")]
fn call_js_ready() {
    use wasm_bindgen::prelude::*;
    use wasm_bindgen::JsCast;

    let window = match web_sys::window() {
        Some(w) => w,
        None => return,
    };

    // Call window.skoffroadReady() if it exists.
    // js_sys::Reflect lets us look up an arbitrary property without needing
    // a typed binding.  The function is defined in index.html.
    // Convert window to JsValue so Reflect::get accepts it as the object.
    let window_val: JsValue = window.into();
    let key = JsValue::from_str("skoffroadReady");
    if let Ok(func_val) = js_sys::Reflect::get(&window_val, &key) {
        if let Some(func) = func_val.dyn_ref::<js_sys::Function>() {
            let _ = func.call0(&window_val);
        }
    }
}

/// Native no-op.
#[cfg(not(target_arch = "wasm32"))]
fn call_js_ready() {}
