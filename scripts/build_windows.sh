#!/bin/bash

set -euo pipefail

# Go to project root
ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT_DIR"

echo "Getting package metadata..."

METADATA=$(cargo metadata --no-deps --format-version 1)
APP_NAME=$(jq -r '.packages[0].metadata.bundle.name' <<< "$METADATA")
APP_VERSION=$(jq -r '.packages[0].version' <<< "$METADATA")

echo "Building $APP_NAME v$APP_VERSION..."

cargo build --release

INSTALLER_DIR="target/release/installer"
mkdir -p "$INSTALLER_DIR"

EXE_PATH="$(realpath target/release/gem-player.exe)"

echo "Building installer..."

iscc \
  "platform/windows/installer_script.iss" \
  "/DAppVersion=$APP_VERSION" \
  "/DExePath=$EXE_PATH" \
  "/O$INSTALLER_DIR" \
  "/Fgem_player_${APP_VERSION}_windows_x64_installer"

echo
echo "Done!"
echo "Installer:"
echo "$INSTALLER_DIR/gem_player_${APP_VERSION}_windows_x64_installer.exe"