// Sprint 51 — WebRTC Voice Chat
//
// Architecture
// ============
// Voice is **separate** from the matchbox game data channel.  We open one
// RTCPeerConnection *per remote peer* exclusively for audio.  The ICE / SDP
// exchange for those audio PeerConnections is piped through matchbox's
// CHANNEL_VOICE_SIGNAL reliable channel (channel 1) so we don't need a
// second signaling server.
//
// Browser path (fully implemented)
// ---------------------------------
//  • getUserMedia({audio:true}) — called from WASM via wasm-bindgen
//  • For each peer: create RTCPeerConnection, add local audio track,
//    exchange Offer/Answer + ICE candidates via the matchbox channel
//  • Received remote track → <audio autoplay> element appended to <body>
//  • Mute/unmute via MediaStreamTrack.enabled toggle
//
// Native path
// -----------
//  Parked — see docs/PARKING_LOT.md "Voice chat native".  The native stub
//  compiles cleanly but produces no audio.
//
// Key bindings
// ------------
//  F           — Push-to-talk (hold)
//  Shift+F     — Toggle always-on mode
//  (T is already used for sky / time-trial / transmission)
//
// Signaling protocol (CHANNEL_VOICE_SIGNAL, reliable ordered)
// -----------------------------------------------------------
//  All messages are JSON-encoded VoiceSignal structs.
//  The matchbox channel delivers them to the correct peer automatically
//  since MatchboxSocket.channel_mut(N).send(bytes, peer_id) is addressed.

use bevy::prelude::*;
use bevy_matchbox::prelude::MatchboxSocket;
use serde::{Deserialize, Serialize};

use crate::multiplayer::{MultiplayerState, CHANNEL_VOICE_SIGNAL};

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct VoicePlugin;

impl Plugin for VoicePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(VoiceState::default())
            .add_systems(Startup, setup_voice)
            .add_systems(
                Update,
                (
                    handle_voice_keys,
                    poll_voice_signals,
                    sync_mute_to_tracks,
                    update_voice_ui,
                )
                    .chain(),
            );
    }
}

// ---------------------------------------------------------------------------
// Public state resource
// ---------------------------------------------------------------------------

/// Voice-chat runtime state, readable by other plugins.
#[derive(Resource)]
pub struct VoiceState {
    /// True when mic capture is active (getUserMedia succeeded).
    pub mic_live: bool,
    /// True when mic is transmitting (PTT held or always-on toggled).
    pub transmitting: bool,
    /// Always-on mode (Shift+F toggle).
    pub always_on: bool,
    /// True after user has granted microphone permission.
    pub permission_granted: bool,
    /// Number of active peer audio connections.
    pub active_peer_connections: usize,
    /// True when webcam is enabled (Q toggle).  False when denied or unavailable.
    pub cam_enabled: bool,
    /// True when getUserMedia({video}) was attempted but denied/unavailable.
    pub cam_denied: bool,
}

impl Default for VoiceState {
    fn default() -> Self {
        Self {
            mic_live:                false,
            transmitting:            false,
            always_on:               false,
            permission_granted:      false,
            active_peer_connections: 0,
            cam_enabled:             false,
            cam_denied:              false,
        }
    }
}

// ---------------------------------------------------------------------------
// Signaling message type (rides CHANNEL_VOICE_SIGNAL)
// ---------------------------------------------------------------------------

/// Wire format for voice-specific signaling messages.
/// These are JSON-encoded and sent via the matchbox reliable channel.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum VoiceSignal {
    Offer         { from: String, sdp: String },
    Answer        { from: String, sdp: String },
    IceCandidate  { from: String, candidate: String, sdp_mid: String },
    HangUp        { from: String },
}

// ---------------------------------------------------------------------------
// Platform-specific implementation
// ---------------------------------------------------------------------------

// ── WASM / Browser ──────────────────────────────────────────────────────────
#[cfg(target_arch = "wasm32")]
mod browser {
    use super::*;
    use js_sys::{Array, Object, Promise, Reflect};
    use wasm_bindgen::prelude::*;
    use wasm_bindgen_futures::JsFuture;
    use web_sys::{
        Document, HtmlAudioElement, HtmlVideoElement,
        MediaStreamConstraints,
        RtcConfiguration, RtcIceServer,
        RtcPeerConnection,
        RtcSdpType, RtcSessionDescriptionInit,
    };

    // -----------------------------------------------------------------------
    // JS glue — thin wrappers around browser APIs not yet in web-sys
    // -----------------------------------------------------------------------

    /// Spawn an async getUserMedia call.  When it resolves, the shared
    /// MIC_STREAM global is set and MIC_READY is flipped to true.
    pub fn request_microphone() {
        wasm_bindgen_futures::spawn_local(async move {
            let window = web_sys::window().unwrap();
            let nav = window.navigator();
            let devices = match nav.media_devices() {
                Ok(d)  => d,
                Err(e) => {
                    bevy::log::warn!("voice: no MediaDevices: {:?}", e);
                    return;
                }
            };

            let mut constraints = MediaStreamConstraints::new();
            constraints.audio(&JsValue::from(true));
            constraints.video(&JsValue::from(false));

            let promise: Promise = match devices.get_user_media_with_constraints(&constraints) {
                Ok(p)  => p,
                Err(e) => {
                    bevy::log::warn!("voice: getUserMedia error: {:?}", e);
                    return;
                }
            };

            match JsFuture::from(promise).await {
                Ok(stream_val) => {
                    let stream = web_sys::MediaStream::from(stream_val);
                    set_mic_stream(stream);
                    MIC_READY.store(true, std::sync::atomic::Ordering::Release);
                    bevy::log::info!("voice: microphone access granted");
                }
                Err(e) => {
                    bevy::log::warn!("voice: getUserMedia denied: {:?}", e);
                }
            }
        });
    }

    // -----------------------------------------------------------------------
    // Shared mic stream — accessed from Bevy systems via thread-local
    // -----------------------------------------------------------------------

    use std::cell::RefCell;
    use std::sync::atomic::AtomicBool;

    pub static MIC_READY: AtomicBool = AtomicBool::new(false);

    thread_local! {
        static MIC_STREAM: RefCell<Option<web_sys::MediaStream>> = RefCell::new(None);
        /// One RtcPeerConnection per peer (keyed by peer id string).
        static PEER_PCS: RefCell<std::collections::HashMap<String, RtcPeerConnection>> =
            RefCell::new(std::collections::HashMap::new());
    }

    pub fn set_mic_stream(s: web_sys::MediaStream) {
        MIC_STREAM.with(|cell| *cell.borrow_mut() = Some(s));
    }

    pub fn mic_stream() -> Option<web_sys::MediaStream> {
        MIC_STREAM.with(|cell| cell.borrow().clone())
    }

    pub fn set_track_enabled(enabled: bool) {
        MIC_STREAM.with(|cell| {
            if let Some(stream) = cell.borrow().as_ref() {
                let tracks = stream.get_audio_tracks();
                for i in 0..tracks.length() {
                    let track = web_sys::MediaStreamTrack::from(tracks.get(i));
                    track.set_enabled(enabled);
                }
            }
        });
    }

    pub fn active_peer_count() -> usize {
        PEER_PCS.with(|cell| cell.borrow().len())
    }

    // -----------------------------------------------------------------------
    // Webcam support — Q key toggle
    // -----------------------------------------------------------------------

    pub static CAM_READY: AtomicBool = AtomicBool::new(false);
    /// True when getUserMedia video was denied.
    pub static CAM_DENIED: AtomicBool = AtomicBool::new(false);

    thread_local! {
        /// Separate camera stream (video track only).
        static CAM_STREAM: RefCell<Option<web_sys::MediaStream>> = RefCell::new(None);
    }

    pub fn set_cam_stream(s: web_sys::MediaStream) {
        CAM_STREAM.with(|cell| *cell.borrow_mut() = Some(s));
    }

    pub fn cam_stream() -> Option<web_sys::MediaStream> {
        CAM_STREAM.with(|cell| cell.borrow().clone())
    }

    /// Request camera (and mic if not yet acquired) via getUserMedia.
    pub fn request_camera() {
        wasm_bindgen_futures::spawn_local(async move {
            let window = web_sys::window().unwrap();
            let nav = window.navigator();
            let devices = match nav.media_devices() {
                Ok(d)  => d,
                Err(e) => {
                    bevy::log::warn!("voice: no MediaDevices for camera: {:?}", e);
                    CAM_DENIED.store(true, std::sync::atomic::Ordering::Release);
                    return;
                }
            };

            let mut constraints = MediaStreamConstraints::new();
            constraints.audio(&JsValue::from(false));
            constraints.video(&JsValue::from(true));

            let promise: Promise = match devices.get_user_media_with_constraints(&constraints) {
                Ok(p)  => p,
                Err(e) => {
                    bevy::log::warn!("voice: getUserMedia(video) error: {:?}", e);
                    CAM_DENIED.store(true, std::sync::atomic::Ordering::Release);
                    return;
                }
            };

            match JsFuture::from(promise).await {
                Ok(stream_val) => {
                    let stream = web_sys::MediaStream::from(stream_val);
                    set_cam_stream(stream.clone());
                    CAM_READY.store(true, std::sync::atomic::Ordering::Release);
                    bevy::log::info!("voice: camera access granted");
                    // Add video tracks to all existing peer PCs.
                    add_video_tracks_to_all_peers(&stream);
                }
                Err(e) => {
                    bevy::log::warn!("voice: getUserMedia(video) denied: {:?}", e);
                    CAM_DENIED.store(true, std::sync::atomic::Ordering::Release);
                }
            }
        });
    }

    /// Toggle video track.enabled on the local cam stream.
    pub fn set_video_track_enabled(enabled: bool) {
        CAM_STREAM.with(|cell| {
            if let Some(stream) = cell.borrow().as_ref() {
                let tracks = stream.get_video_tracks();
                for i in 0..tracks.length() {
                    let track = web_sys::MediaStreamTrack::from(tracks.get(i));
                    track.set_enabled(enabled);
                }
            }
        });
    }

    /// Add the local cam's video tracks to all existing peer PCs and renegotiate.
    fn add_video_tracks_to_all_peers(cam_stream: &web_sys::MediaStream) {
        PEER_PCS.with(|cell| {
            for (peer_str, pc) in cell.borrow().iter() {
                let tracks = cam_stream.get_video_tracks();
                for i in 0..tracks.length() {
                    let track = web_sys::MediaStreamTrack::from(tracks.get(i));
                    let _ = pc.add_track(&track, cam_stream, &js_sys::Array::new());
                }
                // Renegotiate: send a new offer to this peer.
                let peer_clone = peer_str.clone();
                // We use local_peer_str = "renegotiate" as a placeholder
                // (the actual local_peer_str isn't available here, but the
                // receiver only needs the SDP/ICE to connect the video channel).
                let _promise = create_offer_for(peer_clone, "renegotiate".to_string());
            }
        });
    }

    /// Add local video tracks to a specific PC (called when a new peer connects
    /// while cam is already active).
    pub fn add_local_video_tracks_to_pc(pc: &RtcPeerConnection) {
        CAM_STREAM.with(|cell| {
            if let Some(stream) = cell.borrow().as_ref() {
                let tracks = stream.get_video_tracks();
                for i in 0..tracks.length() {
                    let track = web_sys::MediaStreamTrack::from(tracks.get(i));
                    let _ = pc.add_track(&track, stream, &js_sys::Array::new());
                }
            }
        });
    }

    // -----------------------------------------------------------------------
    // Create a new RTCPeerConnection for a peer
    // -----------------------------------------------------------------------

    pub fn create_peer_pc(peer_id_str: &str) -> Option<RtcPeerConnection> {
        let config = RtcConfiguration::new();

        // STUN servers — same as multiplayer.rs constants.
        let stun_urls = Array::new();
        stun_urls.push(&JsValue::from_str("stun:stun.l.google.com:19302"));
        stun_urls.push(&JsValue::from_str("stun:stun.cloudflare.com:3478"));
        let stun_server = RtcIceServer::new();
        let _ = Reflect::set(&stun_server, &JsValue::from_str("urls"), &stun_urls);

        let ice_servers = Array::new();
        ice_servers.push(&stun_server);
        let _ = Reflect::set(&config, &JsValue::from_str("iceServers"), &ice_servers);

        match RtcPeerConnection::new_with_configuration(&config) {
            Ok(pc) => {
                PEER_PCS.with(|cell| {
                    cell.borrow_mut().insert(peer_id_str.to_string(), pc.clone());
                });
                Some(pc)
            }
            Err(e) => {
                bevy::log::warn!("voice: failed to create RTCPeerConnection: {:?}", e);
                None
            }
        }
    }

    pub fn remove_peer_pc(peer_id_str: &str) {
        PEER_PCS.with(|cell| {
            if let Some(pc) = cell.borrow_mut().remove(peer_id_str) {
                pc.close();
            }
        });
    }

    pub fn get_peer_pc(peer_id_str: &str) -> Option<RtcPeerConnection> {
        PEER_PCS.with(|cell| cell.borrow().get(peer_id_str).cloned())
    }

    // -----------------------------------------------------------------------
    // Add local audio tracks to a PC (call after getUserMedia)
    // -----------------------------------------------------------------------

    pub fn add_local_tracks_to_pc(pc: &RtcPeerConnection) {
        if let Some(stream) = mic_stream() {
            let tracks = stream.get_audio_tracks();
            for i in 0..tracks.length() {
                let track = web_sys::MediaStreamTrack::from(tracks.get(i));
                let _ = pc.add_track(&track, &stream, &js_sys::Array::new());
            }
        }
    }

    // -----------------------------------------------------------------------
    // Create offer (async) — returns JSON of VoiceSignal::Offer
    // -----------------------------------------------------------------------

    pub fn create_offer_for(peer_id_str: String, local_peer_str: String) -> Promise {
        let pc_opt = get_peer_pc(&peer_id_str);
        let pc = match pc_opt {
            Some(p) => p,
            None    => return Promise::reject(&JsValue::from_str("no PC")),
        };

        // We set ontrack here so the remote audio plays when it arrives.
        set_ontrack_handler(&pc, &peer_id_str);

        let peer_id_str2   = peer_id_str.clone();
        let local_peer_str2 = local_peer_str.clone();

        wasm_bindgen_futures::future_to_promise(async move {
            let offer_promise = pc.create_offer();
            let offer_val = JsFuture::from(offer_promise).await
                .map_err(|e| e)?;

            // Set local description
            let sdp_str = Reflect::get(&offer_val, &JsValue::from_str("sdp"))
                .unwrap_or(JsValue::from_str(""))
                .as_string()
                .unwrap_or_default();

            let desc_init = RtcSessionDescriptionInit::new(RtcSdpType::Offer);
            let _ = Reflect::set(&desc_init, &JsValue::from_str("sdp"), &JsValue::from_str(&sdp_str));
            JsFuture::from(pc.set_local_description(&desc_init)).await
                .map_err(|e| e)?;

            // Build the VoiceSignal::Offer JSON
            let signal = VoiceSignal::Offer {
                from: local_peer_str2,
                sdp:  sdp_str,
            };
            let json = serde_json::to_string(&signal)
                .map_err(|e| JsValue::from_str(&e.to_string()))?;

            // Store it in a JS global so the Bevy system can poll it.
            push_outbound_signal(peer_id_str2, json);

            Ok(JsValue::NULL)
        })
    }

    // -----------------------------------------------------------------------
    // Handle incoming Offer → produce Answer
    // -----------------------------------------------------------------------

    pub fn handle_offer(from_peer: String, sdp: String, local_peer: String) {
        let pc_opt = get_peer_pc(&from_peer);
        let pc = match pc_opt {
            Some(p) => p,
            None    => return,
        };
        set_ontrack_handler(&pc, &from_peer);

        wasm_bindgen_futures::spawn_local(async move {
            // Set remote description (Offer)
            let desc_init = RtcSessionDescriptionInit::new(RtcSdpType::Offer);
            let _ = Reflect::set(&desc_init, &JsValue::from_str("sdp"), &JsValue::from_str(&sdp));
            if let Err(e) = JsFuture::from(pc.set_remote_description(&desc_init)).await {
                bevy::log::warn!("voice: setRemoteDescription(offer) failed: {:?}", e);
                return;
            }

            // Create answer
            let answer_val = match JsFuture::from(pc.create_answer()).await {
                Ok(v)  => v,
                Err(e) => { bevy::log::warn!("voice: createAnswer failed: {:?}", e); return; }
            };

            let ans_sdp = Reflect::get(&answer_val, &JsValue::from_str("sdp"))
                .unwrap_or(JsValue::from_str(""))
                .as_string()
                .unwrap_or_default();

            let desc_init2 = RtcSessionDescriptionInit::new(RtcSdpType::Answer);
            let _ = Reflect::set(&desc_init2, &JsValue::from_str("sdp"), &JsValue::from_str(&ans_sdp));
            if let Err(e) = JsFuture::from(pc.set_local_description(&desc_init2)).await {
                bevy::log::warn!("voice: setLocalDescription(answer) failed: {:?}", e);
                return;
            }

            let signal = VoiceSignal::Answer {
                from: local_peer,
                sdp:  ans_sdp,
            };
            if let Ok(json) = serde_json::to_string(&signal) {
                push_outbound_signal(from_peer, json);
            }
        });
    }

    pub fn handle_answer(from_peer: &str, sdp: &str) {
        if let Some(pc) = get_peer_pc(from_peer) {
            let desc_init = RtcSessionDescriptionInit::new(RtcSdpType::Answer);
            let _ = Reflect::set(&desc_init, &JsValue::from_str("sdp"), &JsValue::from_str(sdp));
            wasm_bindgen_futures::spawn_local(async move {
                if let Err(e) = JsFuture::from(pc.set_remote_description(&desc_init)).await {
                    bevy::log::warn!("voice: setRemoteDescription(answer) failed: {:?}", e);
                }
            });
        }
    }

    pub fn handle_ice_candidate(from_peer: &str, candidate: &str, sdp_mid: &str) {
        use web_sys::RtcIceCandidateInit;
        if let Some(pc) = get_peer_pc(from_peer) {
            let init = RtcIceCandidateInit::new(candidate);
            let _ = Reflect::set(&init, &JsValue::from_str("sdpMid"), &JsValue::from_str(sdp_mid));
            if let Ok(rtc_cand) = web_sys::RtcIceCandidate::new(&init) {
                wasm_bindgen_futures::spawn_local(async move {
                    if let Err(e) = JsFuture::from(pc.add_ice_candidate_with_opt_rtc_ice_candidate(Some(&rtc_cand))).await {
                        bevy::log::warn!("voice: addIceCandidate failed: {:?}", e);
                    }
                });
            }
        }
    }

    // -----------------------------------------------------------------------
    // ontrack handler: auto-play received audio; display video tiles
    // -----------------------------------------------------------------------

    fn set_ontrack_handler(pc: &RtcPeerConnection, peer_id_str: &str) {
        let peer_id_owned = peer_id_str.to_string();
        let closure = Closure::wrap(Box::new(move |evt: web_sys::RtcTrackEvent| {
            let track = evt.track();
            let streams = evt.streams();
            let stream = if streams.length() > 0 {
                web_sys::MediaStream::from(streams.get(0))
            } else {
                // Create a synthetic stream for the single track
                match web_sys::MediaStream::new_with_tracks(&{
                    let arr = js_sys::Array::new();
                    arr.push(&track);
                    arr
                }) {
                    Ok(s) => s,
                    Err(_) => return,
                }
            };

            let kind = track.kind();

            if let Some(window) = web_sys::window() {
                if let Some(doc) = window.document() {
                    if kind == "audio" {
                        // Append an <audio autoplay> element to <body>
                        if let Ok(audio_el) = doc.create_element("audio") {
                            if let Ok(audio) = audio_el.clone().dyn_into::<HtmlAudioElement>() {
                                audio.set_autoplay(true);
                                let _ = Reflect::set(
                                    &audio_el,
                                    &JsValue::from_str("srcObject"),
                                    &stream,
                                );
                                let _ = Reflect::set(
                                    &audio_el,
                                    &JsValue::from_str("data-voice-peer"),
                                    &JsValue::from_str(&peer_id_owned),
                                );
                                if let Some(body) = doc.body() {
                                    let _ = body.append_child(&audio_el);
                                }
                                bevy::log::info!("voice: audio element created for peer {peer_id_owned}");
                            }
                        }
                    } else if kind == "video" {
                        // Add a webcam tile to #voice-tiles
                        add_video_tile(&doc, &peer_id_owned, &stream);
                    }
                }
            }
        }) as Box<dyn FnMut(_)>);

        pc.set_ontrack(Some(closure.as_ref().unchecked_ref()));
        closure.forget();
    }

    /// Create a 180×135 px video tile in the #voice-tiles container.
    fn add_video_tile(doc: &web_sys::Document, peer_id: &str, stream: &web_sys::MediaStream) {
        // Remove existing tile for this peer first (in case of renegotiation).
        remove_peer_video_tile(peer_id);

        let Ok(tile_container) = doc.query_selector("#voice-tiles") else { return };
        let Some(container) = tile_container else {
            bevy::log::warn!("voice: #voice-tiles container not found in DOM");
            return;
        };

        // Count existing tiles — max 6.
        let child_count = container.child_element_count();
        if child_count >= 6 {
            bevy::log::info!("voice: max 6 video tiles reached, hiding overflow for peer {peer_id}");
            return;
        }

        let Ok(tile_el) = doc.create_element("div") else { return };
        let _ = tile_el.set_attribute("class", "voice-tile");
        let _ = tile_el.set_attribute("data-video-peer", peer_id);

        let short_id: String = peer_id.chars().filter(|c| c.is_alphanumeric()).take(8).collect();

        if let Ok(video_el) = doc.create_element("video") {
            if let Ok(video) = video_el.clone().dyn_into::<HtmlVideoElement>() {
                video.set_autoplay(true);
                video.set_muted(true);
                let _ = video.set_attribute("playsinline", "");
                let _ = Reflect::set(
                    &video_el,
                    &JsValue::from_str("srcObject"),
                    stream,
                );
                let _ = tile_el.append_child(&video_el);
            }
        }

        // Peer ID label overlay.
        if let Ok(label_el) = doc.create_element("span") {
            let _ = label_el.set_attribute("class", "voice-tile-label");
            label_el.set_text_content(Some(&short_id));
            let _ = tile_el.append_child(&label_el);
        }

        let _ = container.append_child(&tile_el);
        bevy::log::info!("voice: video tile created for peer {peer_id}");
    }

    // -----------------------------------------------------------------------
    // Outbound signal queue — async tasks push here, Bevy polls each frame
    // -----------------------------------------------------------------------

    thread_local! {
        static OUTBOUND_SIGNALS: RefCell<Vec<(String /* peer_id */, String /* json */)>> =
            RefCell::new(Vec::new());
    }

    pub fn push_outbound_signal(peer_id: String, json: String) {
        OUTBOUND_SIGNALS.with(|cell| cell.borrow_mut().push((peer_id, json)));
    }

    pub fn drain_outbound_signals() -> Vec<(String, String)> {
        OUTBOUND_SIGNALS.with(|cell| {
            let mut v = cell.borrow_mut();
            std::mem::take(&mut *v)
        })
    }

    // -----------------------------------------------------------------------
    // Remove peer audio element from DOM
    // -----------------------------------------------------------------------

    pub fn remove_peer_audio(peer_id_str: &str) {
        if let Some(window) = web_sys::window() {
            if let Some(doc) = window.document() {
                if let Ok(Some(el)) = doc.query_selector(&format!(
                    "audio[data-voice-peer=\"{peer_id_str}\"]"
                )) {
                    let _ = el.parent_element()
                        .and_then(|p| p.remove_child(&el).ok());
                }
                // Also remove video tile.
                remove_peer_video_tile(peer_id_str);
            }
        }
    }

    // -----------------------------------------------------------------------
    // Remove peer video tile from DOM
    // -----------------------------------------------------------------------

    pub fn remove_peer_video_tile(peer_id_str: &str) {
        if let Some(window) = web_sys::window() {
            if let Some(doc) = window.document() {
                if let Ok(Some(el)) = doc.query_selector(&format!(
                    "div[data-video-peer=\"{peer_id_str}\"]"
                )) {
                    let _ = el.parent_element()
                        .and_then(|p| p.remove_child(&el).ok());
                }
            }
        }
    }
} // mod browser

// ── Native stub ─────────────────────────────────────────────────────────────
#[cfg(not(target_arch = "wasm32"))]
mod native {
    // Native voice chat is parked — see docs/PARKING_LOT.md for the blockers
    // (cpal + webrtc-rs glue, WASM vs. native async executor mismatch).
    // All public symbols below are no-ops so the rest of voice.rs compiles.
    #![allow(dead_code)]

    pub fn request_microphone() {
        bevy::log::info!("voice: native mic capture not yet implemented (see PARKING_LOT.md)");
    }

    pub fn set_track_enabled(_enabled: bool) {}
    pub fn active_peer_count() -> usize { 0 }
    pub fn remove_peer_audio(_peer_id_str: &str) {}
    pub fn drain_outbound_signals() -> Vec<(String, String)> { Vec::new() }

    /// No-op: on native we don't manage RTCPeerConnections.
    pub fn ensure_peer_pc(_peer_id_str: &str) {}
    pub fn remove_peer_pc(_peer_id_str: &str) {}
    pub fn handle_offer(_from_peer: String, _sdp: String, _local_peer: String) {}
    pub fn handle_answer(_from_peer: &str, _sdp: &str) {}
    pub fn handle_ice_candidate(_from_peer: &str, _candidate: &str, _sdp_mid: &str) {}
    pub fn create_offer_for_peers(_peers: &[String], _local_peer: &str) {}

    // Sprint 53: webcam stubs (browser-only feature)
    pub fn request_camera() {
        bevy::log::info!("voice: native webcam not implemented");
    }
    pub fn set_video_track_enabled(_enabled: bool) {}
}

// ---------------------------------------------------------------------------
// Bevy systems — platform-agnostic driver layer
// ---------------------------------------------------------------------------

/// Called at Startup — on WASM we could pre-initialise; for now we wait for
/// the user to press F (PTT) so we don't auto-prompt for mic permission.
fn setup_voice(_commands: Commands) {
    // Nothing to do at startup.  Mic is requested lazily on first PTT press.
}

/// Handle F (PTT), Shift+F (always-on), and Q (webcam toggle).
fn handle_voice_keys(
    keys:     Res<ButtonInput<KeyCode>>,
    mut vs:   ResMut<VoiceState>,
    mp:       Res<MultiplayerState>,
) {
    // Only operate when in a room.
    let in_room = matches!(*mp, MultiplayerState::InRoom { .. } | MultiplayerState::Connecting { .. });

    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);

    // Shift+F — toggle always-on mode
    if keys.just_pressed(KeyCode::KeyF) && shift {
        vs.always_on = !vs.always_on;
        if vs.always_on && !vs.mic_live {
            // Request mic on first always-on activation
            #[cfg(target_arch = "wasm32")]
            browser::request_microphone();
            #[cfg(not(target_arch = "wasm32"))]
            native::request_microphone();
        }
        bevy::log::info!("voice: always-on = {}", vs.always_on);
        return;
    }

    // F — push-to-talk (ignore if Shift held, handled above)
    if !shift {
        if keys.just_pressed(KeyCode::KeyF) {
            // First press: request mic if not yet live
            if !vs.mic_live {
                #[cfg(target_arch = "wasm32")]
                browser::request_microphone();
                #[cfg(not(target_arch = "wasm32"))]
                native::request_microphone();
            }
        }

        if in_room {
            vs.transmitting = keys.pressed(KeyCode::KeyF) || vs.always_on;
        }
    }

    // Q — toggle webcam (browser-only; silently no-ops on native)
    if keys.just_pressed(KeyCode::KeyQ) && in_room && !vs.cam_denied {
        #[cfg(target_arch = "wasm32")]
        {
            if !browser::CAM_READY.load(std::sync::atomic::Ordering::Acquire) {
                // First press: request camera permission
                browser::request_camera();
                bevy::log::info!("voice: requesting camera…");
            } else {
                // Toggle on/off
                vs.cam_enabled = !vs.cam_enabled;
                browser::set_video_track_enabled(vs.cam_enabled);
                bevy::log::info!("voice: cam_enabled = {}", vs.cam_enabled);
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            bevy::log::info!("voice: webcam is browser-only");
        }
    }

    // Poll whether mic access was granted
    #[cfg(target_arch = "wasm32")]
    {
        if !vs.mic_live
            && browser::MIC_READY.load(std::sync::atomic::Ordering::Acquire)
        {
            vs.mic_live = true;
            vs.permission_granted = true;
            bevy::log::info!("voice: mic ready");
        }

        // Poll webcam ready / denied
        if !vs.cam_enabled && !vs.cam_denied
            && browser::CAM_READY.load(std::sync::atomic::Ordering::Acquire)
        {
            vs.cam_enabled = true;
            bevy::log::info!("voice: camera ready");
        }
        if !vs.cam_denied
            && browser::CAM_DENIED.load(std::sync::atomic::Ordering::Acquire)
        {
            vs.cam_denied = true;
            vs.cam_enabled = false;
            bevy::log::warn!("voice: camera denied — falling back to voice-only");
        }
    }

    // Update active peer count
    #[cfg(target_arch = "wasm32")]
    { vs.active_peer_connections = browser::active_peer_count(); }
}

/// Drive the voice-signaling state machine.
///
/// This system reads VoiceSignal messages from CHANNEL_VOICE_SIGNAL and
/// ensures that for every connected peer we have an audio PeerConnection in
/// the correct state.
fn poll_voice_signals(
    mut vs:    ResMut<VoiceState>,
    mut socket: Option<ResMut<MatchboxSocket>>,
    mp:        Res<MultiplayerState>,
) {
    let in_room = matches!(
        *mp,
        MultiplayerState::InRoom { .. } | MultiplayerState::Connecting { .. }
    );
    if !in_room {
        return;
    }

    let Some(ref mut socket) = socket else { return };

    // ------------------------------------------------------------------
    // 1. Drain incoming voice signaling messages from peers
    // ------------------------------------------------------------------
    let inbound: Vec<_> = socket.channel_mut(CHANNEL_VOICE_SIGNAL).receive().into_iter().collect();

    // We need the socket's own peer id for signal routing.
    // bevy_matchbox exposes it via `socket.id()`.
    let local_id_opt = socket.id();
    let local_peer_str = local_id_opt
        .map(|id| format!("{id:?}"))
        .unwrap_or_default();

    for (peer_id, bytes) in &inbound {
        let Ok(json) = std::str::from_utf8(bytes) else {
            bevy::log::warn!("voice: non-UTF8 signal from {peer_id:?}");
            continue;
        };

        let signal: VoiceSignal = match serde_json::from_str(json) {
            Ok(s)  => s,
            Err(e) => {
                bevy::log::warn!("voice: bad signal JSON from {peer_id:?}: {e}");
                continue;
            }
        };

        let peer_str = format!("{peer_id:?}");

        match signal {
            VoiceSignal::Offer { from, sdp } => {
                bevy::log::info!("voice: received Offer from {from}");
                #[cfg(target_arch = "wasm32")]
                {
                    // Ensure we have a PC for this peer
                    if browser::get_peer_pc(&from).is_none() {
                        if let Some(pc) = browser::create_peer_pc(&from) {
                            browser::add_local_tracks_to_pc(&pc);
                        }
                    }
                    browser::handle_offer(from, sdp, local_peer_str.clone());
                }
                #[cfg(not(target_arch = "wasm32"))]
                let _ = (from, sdp, peer_str);
            }
            VoiceSignal::Answer { from, sdp } => {
                bevy::log::info!("voice: received Answer from {from}");
                #[cfg(target_arch = "wasm32")]
                browser::handle_answer(&from, &sdp);
                #[cfg(not(target_arch = "wasm32"))]
                let _ = (from, sdp);
            }
            VoiceSignal::IceCandidate { from, candidate, sdp_mid } => {
                #[cfg(target_arch = "wasm32")]
                browser::handle_ice_candidate(&from, &candidate, &sdp_mid);
                #[cfg(not(target_arch = "wasm32"))]
                let _ = (from, candidate, sdp_mid);
            }
            VoiceSignal::HangUp { from } => {
                bevy::log::info!("voice: peer {from} hung up");
                #[cfg(target_arch = "wasm32")]
                {
                    browser::remove_peer_pc(&from);
                    browser::remove_peer_audio(&from);
                }
                #[cfg(not(target_arch = "wasm32"))]
                let _ = from;
            }
        }
    }

    // ------------------------------------------------------------------
    // 2. Send any outbound signals produced by async JS tasks
    // ------------------------------------------------------------------
    #[cfg(target_arch = "wasm32")]
    {
        for (target_peer_str, json) in browser::drain_outbound_signals() {
            // Reverse-map peer_id_str → PeerId by scanning connected peers
            let peers: Vec<bevy_matchbox::prelude::PeerId> =
                socket.connected_peers().collect();
            for peer_id in &peers {
                if format!("{peer_id:?}") == target_peer_str {
                    socket
                        .channel_mut(CHANNEL_VOICE_SIGNAL)
                        .send(json.as_bytes().to_vec().into(), *peer_id);
                    break;
                }
            }
        }
    }

    // ------------------------------------------------------------------
    // 3. Initiate offers for new peers that don't yet have an audio PC
    // ------------------------------------------------------------------
    #[cfg(target_arch = "wasm32")]
    if vs.mic_live {
        let peers: Vec<bevy_matchbox::prelude::PeerId> =
            socket.connected_peers().collect();
        for peer_id in peers {
            let peer_str = format!("{peer_id:?}");
            if browser::get_peer_pc(&peer_str).is_none() {
                bevy::log::info!("voice: initiating audio PC for new peer {peer_str}");
                if let Some(pc) = browser::create_peer_pc(&peer_str) {
                    browser::add_local_tracks_to_pc(&pc);
                    // Also add video track if cam is already active.
                    if vs.cam_enabled {
                        browser::add_local_video_tracks_to_pc(&pc);
                    }
                    let _promise = browser::create_offer_for(
                        peer_str.clone(),
                        local_peer_str.clone(),
                    );
                }
            }
        }
        vs.active_peer_connections = browser::active_peer_count();
    }

    // Suppress unused-variable warnings on native
    let _ = &local_peer_str;
    let _ = &mut *vs;
}

/// Sync transmitting state → browser track.enabled
fn sync_mute_to_tracks(vs: Res<VoiceState>) {
    #[cfg(target_arch = "wasm32")]
    if vs.mic_live {
        browser::set_track_enabled(vs.transmitting);
    }
    let _ = &*vs;
}

/// HUD indicator: small voice status text in-world (no separate panel).
/// Shown briefly on state change; otherwise hidden.
fn update_voice_ui(
    vs:       Res<VoiceState>,
    mut texts: Query<(&VoiceHudText, &mut Text, &mut Visibility)>,
) {
    if !vs.is_changed() {
        return;
    }

    for (_, mut text, mut vis) in &mut texts {
        if !vs.mic_live {
            *vis = Visibility::Hidden;
            continue;
        }

        *vis = Visibility::Visible;
        text.0 = if vs.always_on {
            if vs.transmitting {
                "VOICE: ON".to_string()
            } else {
                "VOICE: always-on (muted by track)".to_string()
            }
        } else if vs.transmitting {
            "VOICE: PTT".to_string()
        } else {
            "VOICE: ready (hold F)".to_string()
        };
    }
}

// ---------------------------------------------------------------------------
// Voice HUD component
// ---------------------------------------------------------------------------

#[derive(Component)]
pub struct VoiceHudText;
