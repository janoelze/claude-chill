#!/bin/bash
set -e

echo "Building claude-chill (release)..."
cargo build --release

echo "Installing to ~/.cargo/bin/claude-chill..."
cp target/release/claude-chill ~/.cargo/bin/claude-chill

echo "Done! Installed version:"
claude-chill --version
