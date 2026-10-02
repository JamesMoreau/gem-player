#!/usr/bin/env bash
set -euo pipefail

# make_icns.sh
# Usage: ./make_icns.sh <input-image> <output-dir>
# Example: ./make_icns.sh SysMonitor_1024.png ./assets

if [[ $# -ne 2 ]]; then
  echo "Usage: $0 <input-image> <output-dir>"
  exit 1
fi

INPUT="$1"
OUTDIR="$2"

# ---- Preflight checks -------------------------------------------------------
if [[ ! -f "$INPUT" ]]; then
  echo "Error: input file not found: $INPUT" >&2
  exit 1
fi

if ! command -v sips >/dev/null 2>&1; then
  echo "Error: 'sips' not found (macOS only). Install Xcode command line tools." >&2
  exit 1
fi

if ! command -v iconutil >/dev/null 2>&1; then
  echo "Error: 'iconutil' not found. Install Xcode or its command line tools." >&2
  exit 1
fi

mkdir -p "$OUTDIR"

# Derive names
BASENAME="$(basename "$INPUT")"
STEM="${BASENAME%.*}"
ICONSET_DIR="$OUTDIR/${STEM}.iconset"
ICNS_PATH="$OUTDIR/$STEM.icns"

# ---- Validate input dimensions ---------------------------------------------
# We want at least 1024x1024 for best results.
WIDTH=$(sips -g pixelWidth "$INPUT" 2>/dev/null | awk '/pixelWidth:/ {print $2}')
HEIGHT=$(sips -g pixelHeight "$INPUT" 2>/dev/null | awk '/pixelHeight:/ {print $2}')

if [[ -z "${WIDTH:-}" || -z "${HEIGHT:-}" ]]; then
  echo "Error: couldn't read image dimensions from: $INPUT" >&2
  exit 1
fi

if [[ "$WIDTH" -ne "$HEIGHT" ]]; then
  echo "Error: input image must be square. Got ${WIDTH}x${HEIGHT}." >&2
  exit 1
fi

if [[ "$WIDTH" -lt 1024 ]]; then
  echo "Error: input image must be at least 1024x1024. Got ${WIDTH}x${HEIGHT}." >&2
  exit 1
fi

# Create a 1024x1024 working copy (downscale if larger, keep quality)
TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT
WORK_1024="$TMPDIR/icon_1024.png"
sips -s format png -z 1024 1024 "$INPUT" --out "$WORK_1024" >/dev/null

# ---- Build iconset ----------------------------------------------------------
rm -rf "$ICONSET_DIR"
mkdir -p "$ICONSET_DIR"

sips -z 16 16     "$WORK_1024" --out "$ICONSET_DIR/icon_16x16.png"      >/dev/null
sips -z 32 32     "$WORK_1024" --out "$ICONSET_DIR/icon_16x16@2x.png"   >/dev/null
sips -z 32 32     "$WORK_1024" --out "$ICONSET_DIR/icon_32x32.png"      >/dev/null
sips -z 64 64     "$WORK_1024" --out "$ICONSET_DIR/icon_32x32@2x.png"   >/dev/null
sips -z 128 128   "$WORK_1024" --out "$ICONSET_DIR/icon_128x128.png"    >/dev/null
sips -z 256 256   "$WORK_1024" --out "$ICONSET_DIR/icon_128x128@2x.png" >/dev/null
sips -z 256 256   "$WORK_1024" --out "$ICONSET_DIR/icon_256x256.png"    >/dev/null
sips -z 512 512   "$WORK_1024" --out "$ICONSET_DIR/icon_256x256@2x.png" >/dev/null
sips -z 512 512   "$WORK_1024" --out "$ICONSET_DIR/icon_512x512.png"    >/dev/null
cp "$WORK_1024"         "$ICONSET_DIR/icon_512x512@2x.png"  # 1024x1024

# ---- Create .icns -----------------------------------------------------------
iconutil -c icns "$ICONSET_DIR" -o "$ICNS_PATH"

rm -rf "$ICONSET_DIR"

echo "ICNS:    $ICNS_PATH"
echo