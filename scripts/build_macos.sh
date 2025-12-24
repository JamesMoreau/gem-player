#!/bin/bash

set -e  # Exit on any error

# Load environment variables
source .env

# Go to root directory
cd "$(dirname "$0")/.."

APP_NAME="Gem Player"
EXECUTABLE_NAME="gem-player"

INTEL_APP="target/x86_64-apple-darwin/release/bundle/osx/$APP_NAME.app"
ARM_APP="target/aarch64-apple-darwin/release/bundle/osx/$APP_NAME.app"
UNIVERSAL_APP="target/release/bundle/osx/$APP_NAME.app"
DMG_PATH="target/release/bundle/osx/$APP_NAME.dmg"

echo "🧹 Cleaning up previous builds..."
cargo clean
rm -rf target/release/bundle/osx

echo "🚀 Building macOS application (Intel)..."
cargo bundle --release --target x86_64-apple-darwin

echo "🚀 Building macOS application (Apple Silicon)..."
cargo bundle --release --target aarch64-apple-darwin

echo "🧬 Creating universal binary..."
rm -rf "$UNIVERSAL_APP"
mkdir -p "$(dirname "$UNIVERSAL_APP")"
cp -R "$ARM_APP" "$UNIVERSAL_APP"

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

echo "📦 Creating a DMG..."
create-dmg \
  --volname "$APP_NAME Installer" \
  --app-drop-link 0 0 \
  --codesign "$SIGNING_IDENTITY" \
  "$DMG_PATH" \
  "$UNIVERSAL_APP"

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
