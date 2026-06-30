#!/bin/bash

set -euo pipefail # Exit on any error

# Load environment variables
source .env

# Go to root directory
ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT_DIR"

METADATA=$(cargo metadata --no-deps --format-version 1)
APP_NAME=$(jq -r '.packages[0].metadata.bundle.name' <<< "$METADATA")
APP_VERSION=$(jq -r '.packages[0].version' <<< "$METADATA")
EXECUTABLE_NAME="gem-player"

BUNDLE_DIR="target/release/bundle/osx"

INTEL_APP="target/x86_64-apple-darwin/release/bundle/osx/$APP_NAME.app"
ARM_APP="target/aarch64-apple-darwin/release/bundle/osx/$APP_NAME.app"
UNIVERSAL_APP="$BUNDLE_DIR/$APP_NAME.app"

DMG_FILENAME="gem_player_${APP_VERSION}_macos_universal_installer.dmg"
DMG_PATH="$BUNDLE_DIR/$DMG_FILENAME"

echo "🚀 Building macOS application (Intel)..."
cargo bundle --release --target x86_64-apple-darwin

echo "🚀 Building macOS application (Apple Silicon)..."
cargo bundle --release --target aarch64-apple-darwin

echo "🧬 Creating universal binary..."
rm -rf "$UNIVERSAL_APP"
mkdir -p "$(dirname "$UNIVERSAL_APP")"
ditto "$ARM_APP" "$UNIVERSAL_APP"

lipo -create \
  "$INTEL_APP/Contents/MacOS/$EXECUTABLE_NAME" \
  "$ARM_APP/Contents/MacOS/$EXECUTABLE_NAME" \
  -output "$UNIVERSAL_APP/Contents/MacOS/$EXECUTABLE_NAME"

echo "🔍 Verifying universal binary..."
lipo -info "$UNIVERSAL_APP/Contents/MacOS/$EXECUTABLE_NAME"

echo "🔏 Signing the universal app..."
codesign --force --options runtime --timestamp \
  --sign "$SIGNING_IDENTITY" \
  "$UNIVERSAL_APP"

dmgbuild \
  -s platform/macos/dmg_build_settings.py \
  -D app="$BUNDLE_DIR/$APP_NAME.app" \
  "$APP_NAME Installer" \
  "$DMG_PATH"

echo "📝 Notarizing the app..."
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
