#!/bin/bash

# Build script for rust-maple WASM game

set -e  # Exit on error

echo "🔨 Building rust-maple for WASM..."

# Build the WASM binary
cargo build --target wasm32-unknown-unknown --release

# Copy the WASM file to the project root
echo "📦 Copying WASM file..."
cp target/wasm32-unknown-unknown/release/rust-maple.wasm .

echo "✅ Build complete!"
echo "🌐 You can now serve the game with: python3 -m http.server"
echo "   Then open http://localhost:8000 in your browser"
