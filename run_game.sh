#!/bin/bash

# Run the rust-maple game as a standalone app
# Stdout and stderr will be visible in the terminal

set -e  # Exit on error

echo "🎮 Running rust-maple game..."
echo ""

# Build in release mode if needed, or just run
# Using cargo run ensures it's up-to-date
cargo run --bin rust-maple --release

