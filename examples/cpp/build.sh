#!/usr/bin/env bash
set -euo pipefail

CRATE_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")"/../.. && pwd)
TARGET_DIR="$CRATE_DIR/target/release"
OUTPUT_DIR="$CRATE_DIR/examples/cpp/build"
SOURCE_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)

cargo build --manifest-path "$CRATE_DIR/Cargo.toml" --release

mkdir -p "$OUTPUT_DIR"

c++ "$SOURCE_DIR/main.cpp" \
    -I"$CRATE_DIR" \
    -L"$TARGET_DIR" \
    -Wl,-rpath,"$TARGET_DIR" \
    -lvlfd_ffi \
    -lpthread \
    -o "$OUTPUT_DIR/hotplug_demo"

echo "Example built at $OUTPUT_DIR/hotplug_demo"
