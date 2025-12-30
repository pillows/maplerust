#!/bin/bash

# Build script for Windows (cross-compilation from Linux)
# This script builds the wz-viewer application for Windows

set -e  # Exit on error

echo "🔨 Building wz-viewer for Windows..."

# Check if we're on Linux (required for cross-compilation)
if [[ "$OSTYPE" != "linux-gnu"* ]]; then
    echo "⚠️  Warning: This script is designed for Linux. For Windows builds on Windows, use build-windows-native.bat"
fi

# Install Windows target if not already installed
echo "📦 Installing Windows target..."
rustup target add x86_64-pc-windows-gnu

# Check if MinGW-w64 is installed (required for cross-compilation)
if ! command -v x86_64-w64-mingw32-gcc &> /dev/null; then
    echo "❌ MinGW-w64 is not installed!"
    echo "   Please install it:"
    echo "   - Ubuntu/Debian: sudo apt-get install mingw-w64"
    echo "   - Fedora: sudo dnf install mingw64-gcc"
    echo "   - Arch: sudo pacman -S mingw-w64-gcc"
    exit 1
fi

# Configure cargo for Windows cross-compilation
mkdir -p .cargo
cat > .cargo/config.toml << 'EOF'
[build]
rustflags = ["-A non_snake_case"]

[target.x86_64-pc-windows-gnu]
linker = "x86_64-w64-mingw32-gcc"
EOF

# Build for Windows
echo "🔨 Compiling for Windows (x86_64-pc-windows-gnu)..."
cargo build --target x86_64-pc-windows-gnu --release --bin wz-viewer

# Create output directory
mkdir -p dist/windows
OUTPUT_DIR="dist/windows"

# Copy the executable
echo "📦 Copying executable..."
cp target/x86_64-pc-windows-gnu/release/wz-viewer.exe "$OUTPUT_DIR/"

echo "✅ Build complete!"
echo "📁 Windows executable: $OUTPUT_DIR/wz-viewer.exe"
echo ""
echo "💡 Note: The executable may require MinGW runtime DLLs."
echo "   If you encounter DLL errors, you may need to bundle:"
echo "   - libgcc_s_seh-1.dll"
echo "   - libwinpthread-1.dll"
echo "   These can be found in your MinGW installation."
