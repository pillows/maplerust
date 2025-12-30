#!/bin/bash

# Build script that attempts to build for all platforms
# This script detects the current platform and builds accordingly

set -e  # Exit on error

echo "🔨 Building wz-viewer for all available platforms..."
echo ""

# Detect current platform
if [[ "$OSTYPE" == "linux-gnu"* ]]; then
    echo "🐧 Detected Linux"
    echo "   Building for Linux..."
    cargo build --release --bin wz-viewer
    mkdir -p dist/linux
    cp target/release/wz-viewer dist/linux/
    echo "✅ Linux build complete: dist/linux/wz-viewer"
    echo ""
    
    echo "   Attempting Windows cross-compilation..."
    if command -v x86_64-w64-mingw32-gcc &> /dev/null; then
        ./build-windows.sh
    else
        echo "⚠️  MinGW-w64 not found. Skipping Windows build."
        echo "   Install it to enable Windows cross-compilation:"
        echo "   - Ubuntu/Debian: sudo apt-get install mingw-w64"
    fi
    echo ""
    
    echo "⚠️  macOS builds require a Mac. Skipping macOS build."
    echo "   Run build-mac.sh on a Mac to build for macOS."
    
elif [[ "$OSTYPE" == "darwin"* ]]; then
    echo "🍎 Detected macOS"
    ./build-mac.sh
    echo ""
    
    echo "⚠️  Windows builds from macOS require additional setup."
    echo "   Use build-windows-native.bat on Windows, or build-windows.sh on Linux."
    
elif [[ "$OSTYPE" == "msys" ]] || [[ "$OSTYPE" == "win32" ]]; then
    echo "🪟 Detected Windows"
    echo "   Please run: build-windows-native.bat"
    
else
    echo "⚠️  Unknown platform: $OSTYPE"
    echo "   Attempting generic build..."
    cargo build --release --bin wz-viewer
    mkdir -p dist
    cp target/release/wz-viewer dist/
fi

echo ""
echo "✅ Build process complete!"
echo "📁 Check the dist/ directory for built executables"
