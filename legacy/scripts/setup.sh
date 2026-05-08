#!/bin/bash

# SandK Offroad Development Environment Setup Script

echo "Setting up SandK Offroad development environment..."

# Check for Rust installation
if ! command -v rustc &> /dev/null; then
    echo "Installing Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
    source $HOME/.cargo/env
else
    echo "Rust is already installed. Updating..."
    rustup update
fi

# Install required cargo tools
echo "Installing cargo tools..."
cargo install cargo-make
cargo install cargo-watch
cargo install cargo-flamegraph

# Check for Vulkan SDK
if [ -z "$VULKAN_SDK" ]; then
    echo "WARNING: Vulkan SDK not found. Please install it from your package manager or https://vulkan.lunarg.com/"
fi

# Check for CMake
if ! command -v cmake &> /dev/null; then
    echo "WARNING: CMake not found. Please install it from your package manager."
fi

# Setup git hooks
echo "Setting up git hooks..."
if [ -d .git/hooks ]; then
    cp scripts/hooks/* .git/hooks/
    chmod +x .git/hooks/*
fi

# Build the project
echo "Building project..."
cargo build

echo "Setup complete! You can now run the game with 'cargo run'" 