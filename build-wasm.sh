#!/bin/bash
# Build script for WASM target
# Builds only the rust-maple binary (excludes native-only binaries like wz-viewer)

echo "Building rust-maple for WASM..."
cargo build --target wasm32-unknown-unknown --release --bin rust-maple

if [ $? -eq 0 ]; then
    echo "Copying WASM binary to project root..."
    cp target/wasm32-unknown-unknown/release/rust-maple.wasm .
    echo "✓ Build successful!"
    ls -lh rust-maple.wasm
else
    echo "✗ Build failed!"
    exit 1
fi
