#!/usr/bin/env bash
# Readies a signed build for Sign in with Apple: copies the Developer ID provisioning profile into
# src-tauri/embedded.provisionprofile and writes src-tauri/signin.entitlements to match it (both are
# git-ignored). Then build with the extra config:
#
#   scripts/prepare-signin.sh path/to/Friends.provisionprofile
#   npm run tauri build -- --config src-tauri/tauri.signin.conf.json
#
# The build must be signed (APPLE_SIGNING_IDENTITY) with a certificate the profile names: macOS
# refuses to open an app that claims Sign in with Apple without a matching profile.
set -euo pipefail

profile=${1:?usage: scripts/prepare-signin.sh <profile.provisionprofile>}
root=$(cd "$(dirname "$0")/.." && pwd)
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

security cms -D -i "$profile" > "$tmp/profile.plist"
get() { /usr/libexec/PlistBuddy -c "Print :$1" "$tmp/profile.plist" 2>/dev/null; }

app_id=$(get "Entitlements:com.apple.application-identifier") || {
  echo "error: $profile has no application identifier" >&2; exit 1; }
team=$(get "Entitlements:com.apple.developer.team-identifier") || {
  echo "error: $profile has no team identifier" >&2; exit 1; }
if ! get "Entitlements:com.apple.developer.applesignin" > /dev/null; then
  echo "error: $profile doesn't allow Sign in with Apple. Turn the capability on for the App ID," >&2
  echo "then edit the profile, save it again and download the new copy." >&2
  exit 1
fi
bundle_id=$(node -p "require('$root/src-tauri/tauri.conf.json').identifier")
if [ "$app_id" != "$team.$bundle_id" ]; then
  echo "error: $profile is for $app_id, not $team.$bundle_id" >&2
  exit 1
fi

cp "$profile" "$root/src-tauri/embedded.provisionprofile"
cat > "$root/src-tauri/signin.entitlements" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>com.apple.application-identifier</key>
  <string>$app_id</string>
  <key>com.apple.developer.team-identifier</key>
  <string>$team</string>
  <key>com.apple.developer.applesignin</key>
  <array>
    <string>Default</string>
  </array>
</dict>
</plist>
PLIST
echo "Ready for a Sign in with Apple build ($app_id)."
