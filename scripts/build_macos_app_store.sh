#!/bin/bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

# Run all build commands from the project root
cd "$ROOT_DIR"

# Load build environment
if [[ ! -f "$SCRIPT_DIR/.env" ]]; then
    echo "❌ Error: $SCRIPT_DIR/.env not found."
    exit 1
fi

source "$SCRIPT_DIR/.env"

METADATA=$(cargo metadata --no-deps --format-version 1)
APP_NAME=$(jq -r '.packages[0].metadata.bundle.name' <<< "$METADATA")
APP_VERSION=$(jq -r '.packages[0].version' <<< "$METADATA")
EXECUTABLE_NAME="gem-player"

BUNDLE_DIR="target/release/bundle/osx"
INTEL_APP="target/x86_64-apple-darwin/release/bundle/osx/$APP_NAME.app"
ARM_APP="target/aarch64-apple-darwin/release/bundle/osx/$APP_NAME.app"
UNIVERSAL_APP="$BUNDLE_DIR/$APP_NAME.app"
PKG_PATH="$BUNDLE_DIR/gem_player_${APP_VERSION}_macos_app_store.pkg"

ENTITLEMENTS="package/macos/entitlements.plist"
PROVISIONING_PROFILE="package/macos/private/appstore.provisionprofile"

# ------------------------------------------------------------------------------

echo "🚀 Building macOS application (Intel)..."
cargo bundle --release --target x86_64-apple-darwin

echo "🚀 Building macOS application (Apple Silicon)..."
cargo bundle --release --target aarch64-apple-darwin

echo "🧬 Creating universal binary..."
rm -rf "$UNIVERSAL_APP"
mkdir -p "$BUNDLE_DIR"

ditto "$ARM_APP" "$UNIVERSAL_APP"

lipo -create \
  "$INTEL_APP/Contents/MacOS/$EXECUTABLE_NAME" \
  "$ARM_APP/Contents/MacOS/$EXECUTABLE_NAME" \
  -output "$UNIVERSAL_APP/Contents/MacOS/$EXECUTABLE_NAME"

echo "🔍 Verifying universal binary..."
lipo -info "$UNIVERSAL_APP/Contents/MacOS/$EXECUTABLE_NAME"

echo "📜 Embedding provisioning profile..."
cp "$PROVISIONING_PROFILE" \
  "$UNIVERSAL_APP/Contents/embedded.provisionprofile"

echo "🔏 Signing the universal app with entitlements..."
codesign --force --options runtime --timestamp \
  --entitlements "$ENTITLEMENTS" \
  --sign "$APP_STORE_SIGNING_IDENTITY" \
  "$UNIVERSAL_APP"

echo "🔍 Verifying app signature..."
codesign --verify --deep --strict --verbose=2 \
  "$UNIVERSAL_APP"

echo "🔍 Checking app entitlements..."
codesign --display --entitlements - \
  "$UNIVERSAL_APP"

echo "📦 Creating installer package..."
productbuild \
  --sign "$INSTALLER_SIGNING_IDENTITY" \
  --component "$UNIVERSAL_APP" \
  /Applications \
  "$PKG_PATH"

pkgutil --check-signature "$PKG_PATH"

echo "🔍 Verifying minimum macOS version..."
/usr/libexec/PlistBuddy \
  -c "Print :LSMinimumSystemVersion" \
  "$UNIVERSAL_APP/Contents/Info.plist"

echo "🎉 App Store package successfully built and signed!"
echo "📦 App:     $UNIVERSAL_APP"
echo "📦 Package: $PKG_PATH"

