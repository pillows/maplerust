#!/bin/bash

# Build script for rust-maple WASM game

set -e  # Exit on error

echo "🔨 Building rust-maple for WASM..."

# Build the WASM binary (only the main binary, not wz-viewer which is native-only)
cargo build --target wasm32-unknown-unknown --release --bin rust-maple

# Copy the WASM file to the project root
echo "📦 Copying WASM file..."
cp target/wasm32-unknown-unknown/release/rust-maple.wasm .

echo "✅ Build complete!"
echo "🌐 You can now serve the game with: python3 -m http.server"
echo "   Then open http://localhost:8000 in your browser"
