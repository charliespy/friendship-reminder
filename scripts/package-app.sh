#!/usr/bin/env bash
# scripts/package-app.sh (the old Swift build; removed with the Swift code)
# Builds the Friends executable in release mode and wraps it into a
# minimal, ad-hoc-signed Friends.app bundle at the repo root.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

echo "Building release binary..."
swift build -c release

APP_DIR="$REPO_ROOT/Friends.app"
CONTENTS_DIR="$APP_DIR/Contents"
MACOS_DIR="$CONTENTS_DIR/MacOS"
RESOURCES_DIR="$CONTENTS_DIR/Resources"

echo "Assembling app bundle at $APP_DIR..."
rm -rf "$APP_DIR"
mkdir -p "$MACOS_DIR" "$RESOURCES_DIR"

BINARY_PATH="$(swift build -c release --show-bin-path)/Friends"
cp "$BINARY_PATH" "$MACOS_DIR/Friends"
cp "$REPO_ROOT/Sources/Friends/Resources/Info.plist" "$CONTENTS_DIR/Info.plist"

echo "Ad-hoc signing (no Apple Developer account required)..."
codesign --force --deep --sign - "$APP_DIR"

echo "Done. Launch with: open '$APP_DIR'"
