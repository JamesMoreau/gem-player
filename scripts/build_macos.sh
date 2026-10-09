#!/bin/bash

set -euo pipefail # Exit on any error

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

# Load build environment
if [[ ! -f "$SCRIPT_DIR/.env" ]]; then
    echo "❌ Error: $SCRIPT_DIR/.env not found."
    exit 1
fi

source "$SCRIPT_DIR/.env"

# Run all build commands from the project root
cd "$ROOT_DIR"

METADATA=$(cargo metadata --no-deps --format-version 1)
APP_NAME=$(jq -r '.packages[0].metadata.bundle.name' <<< "$METADATA")
APP_VERSION=$(jq -r '.packages[0].version' <<< "$METADATA")
EXECUTABLE_NAME="gem-player"

BUNDLE_DIR="target/universal/release/bundle/osx"
UNIVERSAL_APP="$BUNDLE_DIR/$APP_NAME.app"

DMG_FILENAME="gem_player_${APP_VERSION}_macos_universal_installer.dmg"
DMG_PATH="$BUNDLE_DIR/$DMG_FILENAME"

# ------------------------------------------------------------------------------

echo "🚀 Building universal macOS application..."
cargo bundle --release \
  --format osx \
  --target x86_64-apple-darwin \
  --target aarch64-apple-darwin

if [[ ! -d "$UNIVERSAL_APP" ]]; then
    echo "❌ Error: Expected application bundle not found at $UNIVERSAL_APP"
    exit 1
fi

echo "🔍 Verifying universal binary..."
lipo -info "$UNIVERSAL_APP/Contents/MacOS/$EXECUTABLE_NAME"

echo "🔏 Signing the universal app..."
codesign --force --options runtime --timestamp \
  --sign "$SIGNING_IDENTITY" \
  "$UNIVERSAL_APP"

dmgbuild \
  -s package/macos/dmg_build_settings.py \
  -D app="$UNIVERSAL_APP" \
  "$APP_NAME Installer" \
  "$DMG_PATH"

echo "📝 Notarizing the DMG..."
xcrun notarytool submit "$DMG_PATH" \
  --keychain-profile "$NOTARIZATION_KEYCHAIN_PROFILE" \
  --wait

echo "✅ Stapling the notarization..."
xcrun stapler staple "$UNIVERSAL_APP"
xcrun stapler staple "$DMG_PATH"

echo "🔍 Verifying notarization..."
spctl --assess --type execute --verbose "$UNIVERSAL_APP"

echo "🎉 Universal build and notarization complete!"
echo "📦 DMG saved at: $DMG_PATH"