#!/bin/bash

# Build script for macOS
# This script builds the wz-viewer application for macOS (both Intel and Apple Silicon)

set -e  # Exit on error

echo "🔨 Building wz-viewer for macOS..."

# Check if we're on macOS
if [[ "$OSTYPE" != "darwin"* ]]; then
    echo "❌ Error: This script must be run on macOS!"
    echo "   Cross-compilation to macOS from Linux is not easily possible."
    echo "   Please run this script on a Mac, or use a Mac build service."
    exit 1
fi

# Detect architecture
ARCH=$(uname -m)
if [[ "$ARCH" == "arm64" ]]; then
    TARGET="aarch64-apple-darwin"
    echo "🍎 Detected Apple Silicon (ARM64)"
elif [[ "$ARCH" == "x86_64" ]]; then
    TARGET="x86_64-apple-darwin"
    echo "🍎 Detected Intel Mac (x86_64)"
else
    echo "⚠️  Unknown architecture: $ARCH"
    TARGET="aarch64-apple-darwin"  # Default to Apple Silicon
fi

# Install target if not already installed
echo "📦 Installing macOS target..."
rustup target add "$TARGET"

# Build for macOS
echo "🔨 Compiling for macOS ($TARGET)..."
cargo build --target "$TARGET" --release --bin wz-viewer

# Create output directory
mkdir -p "dist/macos"
OUTPUT_DIR="dist/macos"

# Copy the executable
echo "📦 Copying executable..."
cp "target/$TARGET/release/wz-viewer" "$OUTPUT_DIR/"

# Create a macOS app bundle (optional but recommended)
echo "📦 Creating macOS app bundle..."
APP_NAME="WZ Viewer"
APP_DIR="$OUTPUT_DIR/$APP_NAME.app"
CONTENTS_DIR="$APP_DIR/Contents"
MACOS_DIR="$CONTENTS_DIR/MacOS"
RESOURCES_DIR="$CONTENTS_DIR/Resources"

mkdir -p "$MACOS_DIR"
mkdir -p "$RESOURCES_DIR"

# Copy executable to app bundle
cp "target/$TARGET/release/wz-viewer" "$MACOS_DIR/$APP_NAME"

# Create Info.plist
cat > "$CONTENTS_DIR/Info.plist" << EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key>
    <string>$APP_NAME</string>
    <key>CFBundleIdentifier</key>
    <string>com.maplerust.wzviewer</string>
    <key>CFBundleName</key>
    <string>$APP_NAME</string>
    <key>CFBundleVersion</key>
    <string>1.0</string>
    <key>CFBundleShortVersionString</key>
    <string>1.0</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>LSMinimumSystemVersion</key>
    <string>10.13</string>
    <key>NSHighResolutionCapable</key>
    <true/>
</dict>
</plist>
EOF

# Make executable
chmod +x "$MACOS_DIR/$APP_NAME"

echo "✅ Build complete!"
echo "📁 macOS executable: $OUTPUT_DIR/wz-viewer"
echo "📁 macOS app bundle: $APP_DIR"
echo ""
echo "💡 You can run the app by double-clicking $APP_DIR"
echo "   Or run directly: $OUTPUT_DIR/wz-viewer"
