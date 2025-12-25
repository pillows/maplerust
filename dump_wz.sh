#!/bin/bash

# WZ Structure Dumper Script
# Usage: ./dump_wz.sh <input.img> [output.txt]

set -e

if [ $# -lt 1 ]; then
    echo "Usage: $0 <input.img> [output.txt]"
    echo "Example: $0 Login.img"
    echo "Example: $0 Login.img login_structure.txt"
    exit 1
fi

INPUT_FILE="$1"
OUTPUT_FILE="${2:-${INPUT_FILE%.img}_structure.txt}"

echo "🔨 Compiling WZ structure dumper..."
rustc --edition 2021 \
    -L dependency=target/debug/deps \
    -L dependency=target/release/deps \
    --extern wz_reader=wz_temp/target/debug/libwz_reader.rlib \
    dump_wz_structure.rs \
    -o dump_wz_structure 2>/dev/null || {
    
    # If compilation fails, try building wz_reader first
    echo "📦 Building wz_reader library..."
    cd wz_temp
    cargo build
    cd ..
    
    echo "🔨 Compiling WZ structure dumper..."
    rustc --edition 2021 \
        -L dependency=wz_temp/target/debug/deps \
        --extern wz_reader=wz_temp/target/debug/libwz_reader.rlib \
        dump_wz_structure.rs \
        -o dump_wz_structure
}

echo "📂 Dumping structure from: $INPUT_FILE"
echo "📝 Output file: $OUTPUT_FILE"
echo ""

./dump_wz_structure "$INPUT_FILE" "$OUTPUT_FILE"

echo ""
echo "✨ Done! You can view the structure with:"
echo "   cat $OUTPUT_FILE"
echo "   or"
echo "   less $OUTPUT_FILE"
