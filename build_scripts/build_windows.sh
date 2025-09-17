#!/bin/bash

set -e # Exit on any error

# Load environmentx
source .env

# Go to root directory
cd "$(dirname "$0")/.."

echo "🧹 Cleaning up previous builds..."
cargo clean

APP_NAME="Gem Player"
WIN_TARGET="x86_64-pc-windows-gnu"
RUST_EXE="target/$WIN_TARGET/release/gem-player.exe"   # Rust output
WIN_EXE="target/$WIN_TARGET/release/$APP_NAME.exe"     # Friendly name
WIN_ZIP="target/$WIN_TARGET/release/$APP_NAME.zip"

echo "🚀 Building Windows exe..."
rustup target add $WIN_TARGET >/dev/null 2>&1 || true
cargo build --release --target $WIN_TARGET

# Rename the exe to friendly name
cp "$RUST_EXE" "$WIN_EXE"

# Package into ZIP
rm -f "$WIN_ZIP"
zip -j "$WIN_ZIP" "$WIN_EXE"

echo "🪟 Windows build complete: $WIN_ZIP"
