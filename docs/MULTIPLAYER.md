# skoffroad Multiplayer (Sprint 49 + Sprint 51 voice)

P2P over WebRTC via `bevy_matchbox` / `matchbox_socket`.
No dedicated server — peers broadcast chassis position at 20 Hz.

---

## Quick start

Both players open the game (native or browser).  
Press **I** to open the multiplayer panel.  
Both enter the same **room code** and click **Connect**.

Once connected you will see the other player's vehicle as a semi-transparent
ghost with a short peer-ID label.

---

## Signaling server

The signaling server exchanges WebRTC ICE candidates between peers. It does
**not** relay game traffic — traffic goes directly peer-to-peer after
handshake.

Default URL: `wss://signaling.skoffroad.skworld.io/skoffroad-1`

Override (in priority order):

| Method | Value |
|--------|-------|
| Environment variable | `SKOFFROAD_SIGNALING_URL=wss://your-server/room` |
| Platform storage (`signaling.json`) | `{"url":"wss://your-server/room"}` |
| Built-in default | `wss://signaling.skoffroad.skworld.io/skoffroad-1` |

To run your own signaling server:

```sh
cargo install matchbox_server
matchbox_server --host 0.0.0.0 --port 3536
SKOFFROAD_SIGNALING_URL=ws://localhost:3536/skoffroad-1 cargo run --release
```

---

## ICE / STUN servers

Three Google / Cloudflare / Twilio STUN servers are hardcoded as defaults.
These are sufficient for most home networks and cloud VMs.

Constants in `src/multiplayer.rs` (easy to swap):
```
STUN_URLS = [
    "stun:stun.l.google.com:19302",
    "stun:stun.cloudflare.com:3478",
    "stun:global.stun.twilio.com:3478",
]
```

---

## TURN server (for symmetric NAT)

STUN alone fails when both peers are behind symmetric NAT (e.g. many
mobile carriers). A TURN server relays traffic in that case.

Override (in priority order):

| Method | Value |
|--------|-------|
| Environment variables | `SKOFFROAD_TURN_URL`, `SKOFFROAD_TURN_USERNAME`, `SKOFFROAD_TURN_PASSWORD` |
| Platform storage (`turn.json`) | `{"url":"turn:...","username":"...","password":"..."}` |
| Default | none — STUN-only |

Example using a Coturn server:

```sh
export SKOFFROAD_TURN_URL="turn:turn.example.com:3478"
export SKOFFROAD_TURN_USERNAME="skoffroad"
export SKOFFROAD_TURN_PASSWORD="secret"
cargo run --release
```

Recommended free TURN providers: **Open Relay** (`openrelay.metered.ca`) or
**Twillio Network Traversal Service** (paid but reliable).

---

## NAT troubleshooting

| Symptom | Likely cause | Fix |
|---------|--------------|-----|
| Panel shows "Connecting…" forever | Signaling server unreachable | Check URL / firewall |
| Peers see each other in panel but no ghost car | STUN working, packet decode issue | Check `RUST_LOG=warn` output |
| Ghost car present but stuck | Symmetric NAT | Add TURN server (see above) |
| Ghost car lags behind 200–500 ms | High latency peer | Expected; lerp hides jitter |

---

## Packet format

```rust
struct ChassisPacket {
    translation:  [f32; 3],   // 12 bytes
    rotation:     [f32; 4],   // 16 bytes  (quaternion xyzw)
    linear_vel:   [f32; 3],   // 12 bytes
    angular_vel:  [f32; 3],   // 12 bytes
    paint_index:  u8,         //  1 byte
    variant_disc: u8,         //  1 byte
}                             // = 54 bytes × 20 Hz = ~8.6 Kbps per peer pair
```

Serialized with `bincode 2.x` (`config::standard()`), sent over an
**unreliable** WebRTC data channel (packet loss is fine — next frame
arrives in 50 ms anyway).

---

## Ghost car rendering

- Same Cuboid silhouette as the local chassis.
- `AlphaMode::Blend`, alpha = 0.55 (semi-transparent).
- Short 8-char peer-ID label rendered above the roof with `Text2d`.
- Transform lerped from current → received target over 50 ms (1 tick),
  hiding jitter between 20 Hz updates.
- Ghost despawned automatically when peer disconnects.

---

---

## Voice Chat (Sprint 51)

When two or more players are in the same room, they can speak to each other
via WebRTC audio.

### Browser (implemented, works now)

| Key | Action |
|-----|--------|
| F (hold) | Push-to-talk — transmit while held |
| Shift+F  | Toggle always-on mode (mic stays live) |

**First use**: pressing F triggers a browser mic-permission prompt.
After granting, the local audio track is captured once and reused for all
peer connections.

**How it works (browser):**
1. `getUserMedia({audio:true})` captures the local microphone.
2. For each remote matchbox peer, a separate `RTCPeerConnection` is created
   exclusively for audio (separate from matchbox's game data channel).
3. SDP Offer/Answer and ICE candidates are exchanged through matchbox's
   **reliable** channel 1 (`CHANNEL_VOICE_SIGNAL`), so no second signaling
   server is needed.
4. On receiving a remote audio track, an `<audio autoplay>` DOM element is
   appended to `<body>` — audio plays immediately in the browser.
5. Mute/unmute is `MediaStreamTrack.enabled = true/false` — no
   renegotiation needed.

**Key conflict note:** `T` is already bound to sky time-of-day and
transmission controls. Voice uses `F` instead.

### Native (TODO — parked)

See `docs/PARKING_LOT.md` → "Voice chat native" for the blocker details.
Short version: `cpal` + `webrtc-rs` glue is non-trivial under Bevy's
executor, and the browser path is the primary deliverable. Native voice will
be tackled in a future sprint.

### Packet channels

| Channel | Index | Type | Used for |
|---------|-------|------|----------|
| `CHANNEL_STATE` | 0 | Unreliable | Chassis state (20 Hz) |
| `CHANNEL_VOICE_SIGNAL` | 1 | Reliable ordered | SDP Offer/Answer + ICE (voice only) |

The game-state channel is unchanged. Voice signaling is additive.

---

## Out of scope (later sprints)

- Native voice (cpal + webrtc-rs) — see PARKING_LOT.md.
- Matchmaking, player names, server-authoritative physics.
