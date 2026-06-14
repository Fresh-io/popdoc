#!/usr/bin/env bash
# Brand + notarize + staple the Popdoc DMG.
#
# Tauri builds a DMG whose *mounted volume* carries the Popdoc icon, but it
# leaves the .dmg *file* with the generic disk-image icon and does NOT notarize
# the DMG itself (only the .app inside). This script finishes the job:
#   1. applies the app icon to the .dmg file (resource fork + custom-icon flag),
#   2. notarizes the DMG with Apple,
#   3. staples the ticket so it opens offline / when downloaded.
#
# Requires the same notarization env vars as `tauri build`:
#   APPLE_ID, APPLE_PASSWORD (app-specific password), APPLE_TEAM_ID
#
# Usage:
#   scripts/brand-dmg.sh [path/to/Popdoc_x.y.z_aarch64.dmg]
# If no path is given, the newest DMG under the release bundle dir is used.

set -euo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ICNS="$REPO/src-tauri/icons/icon.icns"
BUNDLE_DIR="$REPO/src-tauri/target/release/bundle/dmg"

DMG="${1:-}"
if [[ -z "$DMG" ]]; then
  DMG="$(ls -t "$BUNDLE_DIR"/*.dmg 2>/dev/null | head -1 || true)"
fi
if [[ -z "$DMG" || ! -f "$DMG" ]]; then
  echo "error: no DMG found (looked in $BUNDLE_DIR). Build one first: npm run tauri build -- --bundles dmg" >&2
  exit 1
fi
for v in APPLE_ID APPLE_PASSWORD APPLE_TEAM_ID; do
  if [[ -z "${!v:-}" ]]; then echo "error: \$$v is not set" >&2; exit 1; fi
done

echo "==> Branding DMG file icon: $(basename "$DMG")"
TMP_ICNS="$(mktemp -t popdoc-icon).icns"
TMP_RSRC="$(mktemp -t popdoc-icon).rsrc"
trap 'rm -f "$TMP_ICNS" "$TMP_RSRC"' EXIT
cp "$ICNS" "$TMP_ICNS"
sips -i "$TMP_ICNS" >/dev/null              # make the icns carry itself as a custom icon
DeRez -only icns "$TMP_ICNS" > "$TMP_RSRC"  # extract the icns resource
xattr -d com.apple.ResourceFork "$DMG" 2>/dev/null || true  # idempotent: drop any prior icon
Rez -append "$TMP_RSRC" -o "$DMG"           # graft it onto the .dmg file
SetFile -a C "$DMG"                         # flip the "has custom icon" Finder bit

echo "==> Notarizing DMG"
xcrun notarytool submit "$DMG" \
  --apple-id "$APPLE_ID" --password "$APPLE_PASSWORD" --team-id "$APPLE_TEAM_ID" \
  --wait

echo "==> Stapling ticket"
xcrun stapler staple "$DMG"

echo "==> Verifying"
xcrun stapler validate "$DMG"
spctl -a -t open --context context:primary-signature -v "$DMG"

echo "==> Done: $DMG"
