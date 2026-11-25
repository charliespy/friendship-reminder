#!/usr/bin/env bash
# Builds the iPhone app for the App Store and uploads it to App Store Connect, where it shows up in
# TestFlight once Apple has processed it (5 to 30 minutes). docs/RELEASING.md has the whole procedure.
#
#   npm run ios:testflight
#
# Reads these from the environment, or else from .env.local at the repo root (which git ignores):
#   FRIENDS_SUPABASE_URL, FRIENDS_SUPABASE_PUBLISHABLE_KEY   the account server, as for the Mac app
#   APPLE_DEVELOPMENT_TEAM                                   the Apple Developer team ID
#   APPLE_API_KEY, APPLE_API_ISSUER                          an App Store Connect API key (Admin) ...
#   APPLE_API_KEY_PATH                                       ... and its AuthKey_<key id>.p8 file
# The key lets Xcode make the App Store certificate and profile itself, and lets altool upload.
#
# Each upload needs a build number Apple hasn't seen for this version: by default the time in UTC
# (YYYYMMDDHHMMSS), or set FRIENDS_BUILD_NUMBER.
#
# Testers get exactly the code in this folder, so it has to be committed: the script stops if there
# are uncommitted changes, and says which branch and commit it uploads.
set -euo pipefail
cd "$(dirname "$0")/.."

names=(FRIENDS_SUPABASE_URL FRIENDS_SUPABASE_PUBLISHABLE_KEY APPLE_DEVELOPMENT_TEAM
  APPLE_API_KEY APPLE_API_ISSUER APPLE_API_KEY_PATH FRIENDS_BUILD_NUMBER)
if [ -f .env.local ]; then
  # What's set in the environment wins over .env.local.
  given=$(for name in "${names[@]}"; do
    if [ -n "${!name+x}" ]; then printf 'export %s=%q\n' "$name" "${!name}"; fi
  done)
  set -a
  . ./.env.local
  set +a
  eval "$given"
fi
for name in "${names[@]:0:6}"; do
  if [ -z "${!name:-}" ]; then
    echo "ios-testflight: $name isn't set (see docs/RELEASING.md)" >&2
    exit 1
  fi
done
if [ "$(basename "$APPLE_API_KEY_PATH")" != "AuthKey_$APPLE_API_KEY.p8" ]; then
  echo "ios-testflight: APPLE_API_KEY_PATH must be the file AuthKey_$APPLE_API_KEY.p8" >&2
  exit 1
fi

if [ -n "$(git status --porcelain --untracked-files=no)" ]; then
  echo "ios-testflight: commit or set aside your changes first: testers get exactly what's committed" >&2
  exit 1
fi
echo "ios-testflight: uploading $(git rev-parse --abbrev-ref HEAD) at $(git rev-parse --short HEAD)"

build="${FRIENDS_BUILD_NUMBER:-$(date -u +%Y%m%d%H%M%S)}"
if ! [[ "$build" =~ ^[0-9]+$ ]]; then
  echo "ios-testflight: FRIENDS_BUILD_NUMBER must be a whole number" >&2
  exit 1
fi

# The build writes the team, build number and export settings into the Xcode project: put those
# files back after, so none of it lands in a commit.
project=(src-tauri/gen/apple/friends.xcodeproj/project.pbxproj src-tauri/gen/apple/friends_iOS/Info.plist
  src-tauri/gen/apple/ExportOptions.plist)
saved=$(mktemp -d)
for f in "${project[@]}"; do cp "$f" "$saved/$(basename "$f")"; done
restore() {
  for f in "${project[@]}"; do cp "$saved/$(basename "$f")" "$f" || echo "ios-testflight: couldn't put back $f" >&2; done
  rm -rf "$saved"
}
trap restore EXIT

out=src-tauri/gen/apple/build/arm64
rm -f "$out"/*.ipa
npx tauri ios build --export-method app-store-connect --ci \
  --config "{\"bundle\":{\"iOS\":{\"bundleVersion\":\"$build\"}}}"

ipas=("$out"/*.ipa)
if [ "${#ipas[@]}" -ne 1 ] || [ ! -f "${ipas[0]}" ]; then
  echo "ios-testflight: expected one .ipa in $out" >&2
  exit 1
fi

# A build without the account server can't sign in: don't send that to testers.
# (grep reads to the end: with -q it would stop early and unzip would fail on the closed pipe. And
# LC_ALL=C, or macOS's grep skips bytes that aren't valid UTF-8 and misses it.)
if ! unzip -p "${ipas[0]}" | LC_ALL=C grep -aF "$FRIENDS_SUPABASE_URL" >/dev/null; then
  echo "ios-testflight: the build doesn't have the account server built in; not uploading" >&2
  exit 1
fi

# altool finds the key by its id in API_PRIVATE_KEYS_DIR.
API_PRIVATE_KEYS_DIR="$(dirname "$APPLE_API_KEY_PATH")" xcrun altool --upload-app --type ios \
  --file "${ipas[0]}" --apiKey "$APPLE_API_KEY" --apiIssuer "$APPLE_API_ISSUER"
echo "ios-testflight: uploaded build $build. It shows up in App Store Connect → TestFlight once Apple has processed it."
