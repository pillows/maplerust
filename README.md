# MapleStory WZ Viewer

A WASM-based MapleStory WZ file viewer with animated sprite support, built with Rust and Macroquad.

## Features

- 🎮 **WASM Game Engine**: Runs in the browser using Macroquad
- 🖼️ **WZ File Parsing**: Extracts and displays PNG images from MapleStory `.img` files
- 🎬 **Animated Sprites**: Automatically discovers and plays all animation frames
- 💾 **Smart Caching**: Uses IndexedDB to cache downloaded WZ files
- 🔍 **Structure Dumper**: Standalone CLI tool to inspect WZ file contents

## Quick Start

### Clone the Repository

```bash
git clone --recursive git@github.com:pillows/maplerust.git
cd maplerust
```

**Note**: The `--recursive` flag is important to clone the `wz-reader-rs` submodule.

If you already cloned without `--recursive`, run:
```bash
git submodule update --init --recursive
```

### Build and Run

1. **Build the WASM game:**
   ```bash
   ./build.sh
   ```

2. **Start a local server:**
   ```bash
   python3 -m http.server
   ```

3. **Open in browser:**
   Navigate to `http://localhost:8000`

## Standalone WZ Viewer Application

The project includes a standalone desktop application (`wz-viewer`) for viewing WZ and IMG files with a GUI.

### Building for Windows

**On Linux (cross-compilation):**
```bash
./build-windows.sh
```
This requires MinGW-w64 to be installed:
- Ubuntu/Debian: `sudo apt-get install mingw-w64`
- Fedora: `sudo dnf install mingw64-gcc`
- Arch: `sudo pacman -S mingw-w64-gcc`

**On Windows (native):**
```cmd
build-windows-native.bat
```

The Windows executable will be in `dist/windows/wz-viewer.exe`.

### Building for macOS

**On macOS:**
```bash
./build-mac.sh
```

This will create:
- `dist/macos/wz-viewer` - Standalone executable
- `dist/macos/WZ Viewer.app` - macOS app bundle (double-clickable)

The script automatically detects whether you're on Intel or Apple Silicon and builds accordingly.

### Building for All Platforms

Run the appropriate script for your platform:
```bash
./build-all-platforms.sh  # Detects platform and builds accordingly
```

## WZ Structure Dumper

Inspect the contents of any WZ `.img` file:

```bash
./dump_wz.sh path/to/file.img [output.txt]
```

Example:
```bash
./dump_wz.sh Logo.img
# Creates: Logo_structure.txt
```

## Project Structure

```
maplerust/
├── src/
│   ├── main.rs          # Main game loop and animation logic
│   └── assets.rs        # WZ file loading and PNG extraction
├── wz_temp/             # wz-reader-rs submodule (WASM-compatible fork)
├── build.sh             # WASM build script
├── dump_wz.sh           # WZ structure dumper script
├── dump_wz_structure.rs # Structure dumper source
├── index.html           # Game HTML entry point
└── mq_js_bundle.js      # Macroquad JavaScript bundle with custom FFI

```

## How It Works

1. **WZ File Loading**: Downloads `.img` files from a URL and caches them in IndexedDB
2. **Frame Discovery**: Parses the WZ structure to find all animation frames
3. **PNG Extraction**: Converts WZ PNG data to RGBA8 textures
4. **Animation**: Cycles through frames at configurable FPS

## Technologies

- **Rust** - Core logic and WZ parsing
- **Macroquad** - WASM game framework
- **wz-reader-rs** - MapleStory WZ file parser (modified for WASM)
- **IndexedDB** - Browser-based asset caching

## Configuration

Edit `src/main.rs` to change:
- Animation source: `base_url`, `cache_name`, `base_path`
- Animation speed: `frame_duration` (default: 0.05s = 20 FPS)
- Display position: `draw_texture(tex, x, y, WHITE)`

## License

This project uses the `wz-reader-rs` library as a submodule. See the submodule's repository for its license.

## Credits

- [wz-reader-rs](https://github.com/spd789562/wz-reader-rs) - WZ file parsing library
- [Macroquad](https://github.com/not-fl3/macroquad) - WASM game framework
