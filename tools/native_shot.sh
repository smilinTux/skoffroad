#!/usr/bin/env bash
# native_shot.sh — launch the native skoffroad binary on the real X11 display,
# optionally drive it with keystrokes, capture a screenshot of its window, and
# clean up. Lets us validate actual GPU rendering (garage, biomes, truck, FX)
# without a human watching.
#
# Usage:
#   tools/native_shot.sh OUT.png "BOOT_WAIT" "key1:delay1 key2:delay2 ..."
#
#   OUT.png      where to save the screenshot
#   BOOT_WAIT    seconds to wait for the window to render before driving (default 8)
#   key spec     space-separated xdotool keys, each "KEY:POSTDELAY_SECONDS"
#                e.g. "space:2 shift+g:2"  (start game, then open garage)
#
# Keys are sent to the specific skoffroad window id (no global focus theft).
set -uo pipefail
cd "$(dirname "$0")/.."

OUT="${1:-/tmp/skoffroad_shot.png}"
BOOT_WAIT="${2:-8}"
KEYSPEC="${3:-}"
BIN="target/debug/skoffroad"

[ -x "$BIN" ] || { echo "FAIL: $BIN not built (run: cargo build --bin skoffroad)"; exit 1; }

export DISPLAY="${DISPLAY:-:0}"

# Run in LOW graphics mode, and allow a software-rendering fallback so this
# works on machines without a usable GPU (the game uses wgpu; WGPU_BACKENDS=gl
# + LIBGL_ALWAYS_SOFTWARE=1 forces Mesa llvmpipe software GL — slow but always
# renders). Override SK_SOFTWARE=0 to use the real GPU if present.
if [ "${SK_SOFTWARE:-1}" = "1" ]; then
  export WGPU_BACKENDS="${WGPU_BACKENDS:-gl}"
  export LIBGL_ALWAYS_SOFTWARE="${LIBGL_ALWAYS_SOFTWARE:-1}"
fi

# Launch detached; log to a temp file. --quality=low keeps it light.
LOG="$(mktemp /tmp/skoffroad_run.XXXX.log)"
"$BIN" --quality=low >"$LOG" 2>&1 &
GAME_PID=$!
cleanup() { kill "$GAME_PID" 2>/dev/null; wait "$GAME_PID" 2>/dev/null; }
trap cleanup EXIT

# Wait for the window to appear (up to 40s of compile-free boot).
WID=""
for _ in $(seq 1 40); do
  WID="$(xdotool search --name '^skoffroad$' 2>/dev/null | head -1)"
  [ -n "$WID" ] && break
  # bail early if the process died (panic)
  kill -0 "$GAME_PID" 2>/dev/null || { echo "FAIL: game exited early"; tail -20 "$LOG"; exit 2; }
  sleep 1
done
[ -n "$WID" ] || { echo "FAIL: no skoffroad window after 40s"; tail -20 "$LOG"; exit 3; }
echo "window id: $WID"

# Let it render a few frames.
sleep "$BOOT_WAIT"

# Drive keys (each sent to the specific window, then wait POSTDELAY).
if [ -n "$KEYSPEC" ]; then
  for spec in $KEYSPEC; do
    key="${spec%%:*}"; delay="${spec##*:}"
    echo "key: $key (then ${delay}s)"
    xdotool key --window "$WID" "$key" 2>/dev/null
    sleep "$delay"
  done
fi

# Capture the window.
import -window "$WID" "$OUT" 2>/dev/null && echo "OK: saved $OUT" || { echo "FAIL: capture"; exit 4; }
echo "--- last game log lines ---"; tail -6 "$LOG"
