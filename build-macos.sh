#!/bin/bash

set -e  # Exit on any error

APP_NAME="Gem Player"
BUNDLE_DIR="target/release/bundle/osx/$APP_NAME.app"
DMG_PATH="target/release/bundle/osx/$APP_NAME.dmg"
ZIP_PATH="target/release/bundle/osx/$APP_NAME.zip"
TEAM_ID="NJXX6CLLB6"
APPLE_ID="jamespmoreau@protonmail.ch"
NOTARIZATION_PROFILE="your-keychain-profile"  # Run `xcrun notarytool store-credentials`

echo "🚀 Building macOS application..."
cargo bundle --release

echo "🔏 Signing the app..."
codesign --force --deep --sign "Developer ID Application: Your Name ($TEAM_ID)" "$BUNDLE_DIR"

echo "📦 Creating a DMG..."
create-dmg \
  --volname "Gem Player" \
  "target/release/bundle/osx/Gem Player.dmg" \
  "target/release/bundle/osx/Gem Player.app"

echo "✅ Verifying app..."
spctl --assess --type execute --verbose "$BUNDLE_DIR"

echo "🎉 Build complete! DMG saved at: $DMG_PATH"
