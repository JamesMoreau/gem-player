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

BUNDLE_DIR="target/release/bundle/osx"

INTEL_APP="target/x86_64-apple-darwin/release/bundle/osx/$APP_NAME.app"
ARM_APP="target/aarch64-apple-darwin/release/bundle/osx/$APP_NAME.app"
UNIVERSAL_APP="$BUNDLE_DIR/$APP_NAME.app"

PKG_FILENAME="gem_player_${APP_VERSION}_macos_app_store.pkg"
PKG_PATH="$BUNDLE_DIR/$PKG_FILENAME"

ENTITLEMENTS="platform/macos/macos.entitlements"
PROVISIONING_PROFILE="platform/macos/Gem_Player_App_Store.provisionprofile"

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

echo "📜 Embedding provisioning profile..."
cp "$PROVISIONING_PROFILE" \
  "$UNIVERSAL_APP/Contents/embedded.provisionprofile"

echo "🔏 Signing the universal app..."
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

security cms -D -i "$PROVISIONING_PROFILE" > /tmp/gem-profile.plist

/usr/libexec/PlistBuddy \
  -c "Print :Entitlements:application-identifier" \
  /tmp/gem-profile.plist

exit

productbuild \
  --sign "$INSTALLER_SIGNING_IDENTITY" \
  --component "$UNIVERSAL_APP" \
  /Applications \
  "$PKG_PATH"

pkgutil --check-signature "$PKG_PATH"

echo "🎉 App Store app successfully built and signed! the .pkg may now be uploaded via Transporter."
echo "📦 App: $UNIVERSAL_APP"

