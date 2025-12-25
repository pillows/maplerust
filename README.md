# WZ Structure Dumper

A standalone tool to dump the hierarchical structure of MapleStory WZ `.img` files to text files.

## Usage

### Quick Start

```bash
# Download a .img file first, then:
./dump_wz.sh path/to/file.img

# Or specify a custom output file:
./dump_wz.sh path/to/file.img output_structure.txt
```

### Example

```bash
# If you have Login.img in the current directory:
./dump_wz.sh Login.img

# This will create: Login_structure.txt
```

The script will:
1. Automatically compile the dumper tool (first run only)
2. Parse the WZ file structure
3. Save the complete node hierarchy to a text file

### Output Format

The output file contains all nodes in the WZ file with their types:

```
=== WZ Structure for Login.img ===

Login.img [Image]
Login.img/Common [Property]
Login.img/Common/BtStart [Property]
Login.img/Common/BtStart/normal [Property]
Login.img/Common/BtStart/normal/0 [PNG]
...
```

### Node Types

- **PNG**: Image data
- **Property**: Container node
- **Int/Short/Long**: Integer values
- **Float/Double**: Decimal values
- **String**: Text data
- **Vector**: 2D coordinates
- **Sound**: Audio data
- **UOL**: Link to another node

## Building the Game

To build the WASM game:

```bash
./build.sh
```

Then open `http://localhost:8000` in your browser (requires `python3 -m http.server` running).
