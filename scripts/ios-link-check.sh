#!/usr/bin/env bash
# Builds the whole iPhone app for the simulator, Swift and Xcode's final link step included, which
# `npm run check:ios` (Rust only) never reaches: a framework missing from the Xcode project (as
# Contacts once was) fails here, not in a TestFlight build. No signing needed.
#
#   npm run check:ios-link
set -euo pipefail
cd "$(dirname "$0")/.."

# The build writes the version into the Xcode project's files: put them back after.
project=(src-tauri/gen/apple/friends.xcodeproj/project.pbxproj src-tauri/gen/apple/friends_iOS/Info.plist)
saved=$(mktemp -d)
for f in "${project[@]}"; do cp "$f" "$saved/$(basename "$f")"; done
restore() {
  for f in "${project[@]}"; do cp "$saved/$(basename "$f")" "$f"; done
  rm -rf "$saved"
}
trap restore EXIT

npx tauri ios build --debug --target aarch64-sim --ci
