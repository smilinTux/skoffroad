#!/bin/bash
# Smoke-test the dev binary: launch with 8s timeout, scan output for panics.
# Used to validate the game boots past startup and stays alive during the
# first physics ticks.
set -e
cd "$(dirname "$0")"
cargo build --bin skoffroad --features dev 2>&1 | tail -3
LD_LIBRARY_PATH="$PWD/target/debug/deps" timeout 8 ./target/debug/skoffroad > /tmp/skoffroad_smoke.log 2>&1 || true
if grep -q "panicked at" /tmp/skoffroad_smoke.log; then
    echo "=== PANIC DETECTED ==="
    grep -B1 -A3 "panicked at" /tmp/skoffroad_smoke.log | head -8
    exit 1
else
    echo "=== boot OK (no panics in 8s) ==="
    grep "INFO skoffroad" /tmp/skoffroad_smoke.log | tail -8
fi
