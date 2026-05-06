#!/bin/bash
# Smoke-test the dev binary: launch with 8s timeout, scan output for panics.
# Used to validate the game boots past startup and stays alive during the
# first physics ticks.
set -e
cd "$(dirname "$0")"
cargo build --bin sandk-offroad-next --features dev 2>&1 | tail -3
LD_LIBRARY_PATH="$PWD/target/debug/deps" timeout 8 ./target/debug/sandk-offroad-next > /tmp/sandk_smoke.log 2>&1 || true
if grep -q "panicked at" /tmp/sandk_smoke.log; then
    echo "=== PANIC DETECTED ==="
    grep -B1 -A3 "panicked at" /tmp/sandk_smoke.log | head -8
    exit 1
else
    echo "=== boot OK (no panics in 8s) ==="
    grep "INFO sandk_offroad" /tmp/sandk_smoke.log | tail -8
fi
