# Building the WZ Viewer Application

This document provides detailed instructions for building the `wz-viewer` standalone desktop application for Windows and macOS.

## Overview

The `wz-viewer` is a native desktop application built with Rust, eframe, and egui that allows you to:
- Browse WZ and IMG file structures
- View PNG images extracted from WZ files
- Export PNGs and IMG files
- Bulk export WZ files to IMG files

## Prerequisites

- Rust and Cargo installed (https://rustup.rs/)
- For Windows cross-compilation from Linux: MinGW-w64
- For macOS builds: Must be run on macOS

## Quick Start

### Linux

Build the native Linux version:
```bash
cargo build --release --bin wz-viewer
```

The executable will be at: `target/release/wz-viewer`

### Windows

#### Option 1: Cross-compilation from Linux

1. Install MinGW-w64:
   ```bash
   # Ubuntu/Debian
   sudo apt-get install mingw-w64
   
   # Fedora
   sudo dnf install mingw64-gcc
   
   # Arch Linux
   sudo pacman -S mingw-w64-gcc
   ```

2. Run the build script:
   ```bash
   ./build-windows.sh
   ```

3. The executable will be at: `dist/windows/wz-viewer.exe`

#### Option 2: Native build on Windows

1. Install Rust from https://rustup.rs/
2. Run:
   ```cmd
   build-windows-native.bat
   ```

3. The executable will be at: `dist/windows/wz-viewer.exe`

**Note**: The MinGW version may require DLLs (`libgcc_s_seh-1.dll`, `libwinpthread-1.dll`) to be bundled. The MSVC version (from native Windows build) doesn't require these.

### macOS

1. Ensure you're on a Mac (Intel or Apple Silicon)
2. Run:
   ```bash
   ./build-mac.sh
   ```

3. The build will create:
   - `dist/macos/wz-viewer` - Standalone executable
   - `dist/macos/WZ Viewer.app` - macOS app bundle (double-clickable)

The script automatically detects your architecture (Intel x86_64 or Apple Silicon ARM64).

## Manual Build Commands

If you prefer to build manually:

### Windows (MinGW, from Linux)
```bash
rustup target add x86_64-pc-windows-gnu
cargo build --target x86_64-pc-windows-gnu --release --bin wz-viewer
```

### Windows (MSVC, on Windows)
```bash
rustup target add x86_64-pc-windows-msvc
cargo build --target x86_64-pc-windows-msvc --release --bin wz-viewer
```

### macOS (Intel)
```bash
rustup target add x86_64-apple-darwin
cargo build --target x86_64-apple-darwin --release --bin wz-viewer
```

### macOS (Apple Silicon)
```bash
rustup target add aarch64-apple-darwin
cargo build --target aarch64-apple-darwin --release --bin wz-viewer
```

## Troubleshooting

### Windows DLL Errors

If you get DLL errors when running the MinGW-built executable:
- Copy the required DLLs from your MinGW installation
- Or use the MSVC build (native Windows build) which doesn't require DLLs

### macOS Code Signing

The app bundle is not code-signed. If macOS complains:
- Right-click the app and select "Open"
- Or run: `xattr -cr "dist/macos/WZ Viewer.app"`

### Cross-compilation Issues

- **macOS from Linux**: Not easily possible. You must build on macOS.
- **Windows from macOS**: Requires additional setup. Use Linux or Windows for Windows builds.

## Distribution

### Windows

The standalone `.exe` file can be distributed as-is. For the MinGW version, you may need to bundle DLLs or use an installer.

### macOS

The `.app` bundle can be distributed. Users may need to right-click and select "Open" the first time due to code signing.

## Building All Platforms

Use the platform detection script:
```bash
./build-all-platforms.sh
```

This will build for all platforms available from your current system.
