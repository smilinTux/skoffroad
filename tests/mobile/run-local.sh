#!/usr/bin/env bash
# =============================================================================
# skoffroad — local mobile smoke test harness
# Sprint 74
#
# Usage:
#   ./tests/mobile/run-local.sh
#
# What it does:
#   1. Ensures trunk is installed (cargo install trunk --locked if missing)
#   2. Ensures the wasm32 target is added via rustup
#   3. Builds the WASM release bundle  (trunk build --release → dist/)
#   4. Serves dist/ on port 8080 with http-server
#   5. Installs Playwright Chromium browser if missing
#   6. Runs the full Playwright test suite
#   7. Tears down the static server on exit (trap)
#   8. Prints a clear PASS / FAIL summary line
#
# Idempotent: safe to re-run.  Steps that are already done (trunk installed,
# wasm target added, Playwright browser cached) are skipped automatically.
#
# Requirements:
#   - Rust + cargo (stable toolchain)
#   - Node 18+ and npm  (for npx)
#   - curl  (used to health-check the server)
# =============================================================================

set -euo pipefail

# ---------------------------------------------------------------------------
# Resolve repo root regardless of where the script is called from.
# ---------------------------------------------------------------------------
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

echo ""
echo "╔══════════════════════════════════════════════════════╗"
echo "║   skoffroad — mobile smoke test harness (local)     ║"
echo "╚══════════════════════════════════════════════════════╝"
echo ""
echo "Repo root : $REPO_ROOT"
echo "Test dir  : $SCRIPT_DIR"
echo ""

# ---------------------------------------------------------------------------
# Step 1 — Ensure trunk is installed
# ---------------------------------------------------------------------------
if ! command -v trunk &>/dev/null; then
  echo "▶ trunk not found — installing via cargo (this takes a few minutes)..."
  cargo install trunk --locked
else
  echo "✓ trunk already installed: $(trunk --version 2>&1 | head -1)"
fi

# ---------------------------------------------------------------------------
# Step 2 — Ensure wasm32 target is available
# ---------------------------------------------------------------------------
if ! rustup target list --installed 2>/dev/null | grep -q 'wasm32-unknown-unknown'; then
  echo "▶ Adding wasm32-unknown-unknown rustup target..."
  rustup target add wasm32-unknown-unknown
else
  echo "✓ wasm32-unknown-unknown target already installed"
fi

# ---------------------------------------------------------------------------
# Step 3 — Build the WASM release bundle
# ---------------------------------------------------------------------------
echo ""
echo "▶ Building WASM release bundle (trunk build --release)..."
echo "  This compiles ~34 MB of Rust + Bevy to WebAssembly — may take 5-15 min"
echo "  on a cold cache."
echo ""
cd "$REPO_ROOT"
trunk build --release
echo ""
echo "✓ WASM build complete — dist/ is ready"

# ---------------------------------------------------------------------------
# Step 4 — Serve dist/ on port 8080
# ---------------------------------------------------------------------------
# Kill any leftover server from a previous run on this port.
SERVER_PID=""

cleanup() {
  if [ -n "$SERVER_PID" ] && kill -0 "$SERVER_PID" 2>/dev/null; then
    echo ""
    echo "▶ Stopping http-server (pid $SERVER_PID)..."
    kill "$SERVER_PID" 2>/dev/null || true
  fi
}
trap cleanup EXIT INT TERM

# Start the server in background.
echo "▶ Starting http-server on http://localhost:8080 ..."
npx --yes http-server "$REPO_ROOT/dist" -p 8080 -s --cors &>/tmp/skoffroad-server.log &
SERVER_PID=$!

# Wait for the server to respond (up to 20 s).
echo "▶ Waiting for server to be ready..."
MAX_WAIT=20
for i in $(seq 1 $MAX_WAIT); do
  if curl -sf http://localhost:8080/ >/dev/null 2>&1; then
    echo "✓ Server is up (took ${i}s)"
    break
  fi
  if [ "$i" -eq "$MAX_WAIT" ]; then
    echo "✗ ERROR: http-server did not respond within ${MAX_WAIT}s"
    echo "  Server log:"
    cat /tmp/skoffroad-server.log || true
    exit 1
  fi
  sleep 1
done

# ---------------------------------------------------------------------------
# Step 5 — Install Playwright Chromium browser if missing
# ---------------------------------------------------------------------------
cd "$SCRIPT_DIR"
echo ""
echo "▶ Installing Playwright npm dependencies..."
npm install --silent

echo "▶ Ensuring Playwright Chromium browser is installed..."
# --with-deps installs OS-level dependencies needed for headless Chrome.
# If chromium is already installed, this is a fast no-op.
npx playwright install --with-deps chromium

# ---------------------------------------------------------------------------
# Step 6 — Run Playwright tests
# ---------------------------------------------------------------------------
echo ""
echo "▶ Running Playwright mobile smoke tests..."
echo "  (13 tests total: 12 HTML-overlay + 1 canvas-render)"
echo ""

PLAYWRIGHT_EXIT=0
npx playwright test || PLAYWRIGHT_EXIT=$?

# ---------------------------------------------------------------------------
# Step 7 — Summarise
# ---------------------------------------------------------------------------
echo ""
if [ "$PLAYWRIGHT_EXIT" -eq 0 ]; then
  echo "╔══════════════════════════════════════════╗"
  echo "║   PASS — all mobile smoke tests green   ║"
  echo "╚══════════════════════════════════════════╝"
else
  echo "╔═══════════════════════════════════════════════╗"
  echo "║   FAIL — one or more mobile smoke tests red  ║"
  echo "║   See tests/mobile/playwright-report/        ║"
  echo "╚═══════════════════════════════════════════════╝"
fi
echo ""

exit "$PLAYWRIGHT_EXIT"
