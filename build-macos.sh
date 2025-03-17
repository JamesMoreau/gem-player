#!/bin/bash

set -e  # Exit on any error

APP_NAME="Gem Player"
BUNDLE_DIR="target/release/bundle/osx/$APP_NAME.app"
DMG_PATH="target/release/bundle/osx/$APP_NAME.dmg"
TEAM_ID="NJXX6CLLB6"

echo "🚀 Building macOS application..."
cargo bundle --release

echo "📦 Creating a DMG..."
create-dmg \
  --volname "$APP_NAME" \
  --codesign "Developer ID Application: James Moreau ($TEAM_ID)" \
  "$DMG_PATH" \
  "$BUNDLE_DIR"

echo "✅ Verifying app..."
spctl --assess --type execute --verbose "$BUNDLE_DIR"

echo "🎉 Build complete! DMG saved at: $DMG_PATH"
