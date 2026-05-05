# SandK Offroad — Keyboard Reference

## Driving

| Key | Action |
|---|---|
| `W` | Forward (rebindable) |
| `S` | Reverse (rebindable) |
| `A` | Steer left (rebindable) |
| `D` | Steer right (rebindable) |
| `Space` | Brake (rebindable) |
| `Shift` | Handbrake (rebindable) |
| `B` | Boost (rebindable) |

## Game modes

| Key | Action |
|---|---|
| `R` | Start / restart race (Lobby → Countdown → Active → Finished) |
| `T` | Start / cancel time trial |
| `P` | Toggle pursuit (cop chases you for 60s) |
| `X` | Toggle demolition (30 smashable crates) |
| `C` | Start random 30s mini-challenge |

## World

| Key | Action |
|---|---|
| `Tab` | Map select (VALLEY / DUNES / CANYON) |
| `8` | Toggle ambient traffic (5 NPC trucks) |
| `9` | Toggle storm (rain + lightning) |
| `+` / `-` | Zoom minimap in / out |

## Camera & post-FX

| Key | Action |
|---|---|
| `J` | Toggle bloom + tonemapping |

## HUD panels

| Key | Action |
|---|---|
| `H` | Help overlay |
| `L` | Unlocks panel |
| `M` | Medal cabinet |
| `K` | Credits roll |
| `;` | Changelog |
| `0` | Accessibility settings |
| `/` | Configurable keybindings |

## System

| Key | Action |
|---|---|
| `Esc` | Pause menu |
| `F3` | World inspector (dev builds only) |
| `F12` | Start 30s benchmark (writes to `~/.sandk-offroad/benchmark-{ts}.txt`) |

## Other

| Key | Action |
|---|---|
| `U` | Toggle music |
| `Shift+W` | Wash vehicle (reset dirt) |
| `Any key during loading splash` | Skip splash |
| `Any key during intro cinematic` | Skip 5s orbit |
| `Any key during demo mode` | Exit auto-drive |

## Idle behavior

If no key has been pressed for 30 seconds, the game enters **demo mode** and auto-drives the player vehicle along the race path until any key is pressed.

## Persistence

- `~/.sandk-offroad/config.json` — master volume, mouse sensitivity, day length
- `~/.sandk-offroad/keybindings.json` — drive bindings
- `~/.sandk-offroad/benchmark-{ts}.txt` — F12 benchmark reports
