#!/bin/bash

function show_help() {
    echo "SandK Offroad Development Script"
    echo "Usage: ./dev.sh [command]"
    echo ""
    echo "Commands:"
    echo "  run         - Run the game in development mode"
    echo "  run-release - Run the game in release mode"
    echo "  watch      - Run the game with auto-reload on code changes"
    echo "  test       - Run all tests"
    echo "  test-watch - Run tests with auto-reload on code changes"
    echo "  lint       - Run clippy lints"
    echo "  format     - Format all code"
    echo "  clean      - Clean build artifacts"
    echo "  profile    - Run with CPU profiling enabled"
    echo "  help       - Show this help message"
}

case "$1" in
    "run")
        cargo run
        ;;
    "run-release")
        cargo run --release
        ;;
    "watch")
        cargo watch -x run
        ;;
    "test")
        cargo test
        ;;
    "test-watch")
        cargo watch -x test
        ;;
    "lint")
        cargo clippy -- -D warnings
        ;;
    "format")
        cargo fmt
        ;;
    "clean")
        cargo clean
        ;;
    "profile")
        cargo flamegraph
        ;;
    *)
        show_help
        ;;
esac 