#!/bin/bash

set -e  # Exit on any error

# Load environment variables
source .env

# Go to root directory
cd "$(dirname "$0")/.."

APP_NAME="Gem Player"
BUNDLE_PATH="target/release/bundle/osx/$APP_NAME.app"
DMG_PATH="target/release/bundle/osx/$APP_NAME.dmg"

echo "🧹 Cleaning up previous builds..."
cargo clean

echo "🚀 Building macOS application..."
cargo bundle --release

echo "🔏 Signing the app..."
codesign --force --deep --options runtime --sign "Developer ID Application: $APPLE_NAME ($APPLE_TEAM_ID)" "$BUNDLE_PATH"

echo "📦 Creating a DMG..."
create-dmg \
  --volname "$APP_NAME Installer" \
  --app-drop-link 0 0 \
  --codesign "Developer ID Application: $APPLE_NAME ($APPLE_TEAM_ID)" \
  "$DMG_PATH" \
  "$BUNDLE_PATH"

echo "📝 Notarizing the app..."
xcrun notarytool submit "$DMG_PATH" --keychain-profile "$NOTARIZATION_KEYCHAIN_PROFILE" --wait

echo "✅ Stapling the notarization..."
xcrun stapler staple "$BUNDLE_PATH"
xcrun stapler staple "$DMG_PATH"

echo "🔍 Verifying notarization..."
spctl --assess --type execute --verbose "$BUNDLE_PATH"

echo "🎉 Build and notarization complete! DMG saved at: $DMG_PATH"
