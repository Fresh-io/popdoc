#!/usr/bin/env bash
# Build, sign (minisign), notarize, and publish a Popdoc release to GitHub.
#
# This is the one command that ships an auto-updatable release. It:
#   1. signs the updater archive with the minisign private key (separate from
#      Apple code signing — the updater verifies via its own pubkey),
#   2. builds the notarized .app + .dmg (createUpdaterArtifacts emits the
#      Popdoc.app.tar.gz + .sig the updater downloads),
#   3. brands + notarizes the .dmg (scripts/brand-dmg.sh),
#   4. writes latest.json pointing at this tag's updater archive,
#   5. creates/updates the GitHub release with the .dmg, .tar.gz and latest.json.
#
# The updater endpoint (tauri.conf.json) is:
#   https://github.com/Fresh-io/popdoc/releases/latest/download/latest.json
#
# Requirements:
#   - Minisign key at ~/Developer/Certificats/popdoc_updater.key
#     (override with TAURI_SIGNING_PRIVATE_KEY_PATH). Generated with no
#     password; set TAURI_SIGNING_PRIVATE_KEY_PASSWORD if yours has one.
#   - Apple notarization env: APPLE_ID, APPLE_PASSWORD (app-specific), APPLE_TEAM_ID
#   - gh (authenticated) and jq.
#
# Usage:
#   scripts/release.sh ["release notes"]
# Bump package.json + tauri.conf.json + Cargo.toml versions BEFORE running.

set -euo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO"

command -v jq >/dev/null || { echo "error: jq is required" >&2; exit 1; }
command -v gh >/dev/null || { echo "error: gh is required" >&2; exit 1; }

# --- Minisign signing key for the updater ---
KEY="${TAURI_SIGNING_PRIVATE_KEY_PATH:-$HOME/Developer/Certificats/popdoc_updater.key}"
[[ -f "$KEY" ]] || { echo "error: minisign key not found: $KEY" >&2; exit 1; }
export TAURI_SIGNING_PRIVATE_KEY="$(cat "$KEY")"
export TAURI_SIGNING_PRIVATE_KEY_PASSWORD="${TAURI_SIGNING_PRIVATE_KEY_PASSWORD:-}"

# --- Apple notarization env (same as a normal build) ---
for v in APPLE_ID APPLE_PASSWORD APPLE_TEAM_ID; do
  [[ -n "${!v:-}" ]] || { echo "error: \$$v is not set" >&2; exit 1; }
done

VERSION="$(node -p "require('./package.json').version")"
TAG="v$VERSION"
NOTES="${1:-Popdoc $VERSION}"

if gh release view "$TAG" --repo Fresh-io/popdoc >/dev/null 2>&1; then
  echo "==> Release $TAG already exists — assets will be re-uploaded (--clobber)."
fi

echo "==> Building + signing Popdoc $VERSION (universal, notarized)"
npm run tauri build -- --target universal-apple-darwin --bundles app dmg

MACOS_DIR="$REPO/src-tauri/target/universal-apple-darwin/release/bundle/macos"
DMG_DIR="$REPO/src-tauri/target/universal-apple-darwin/release/bundle/dmg"
TARGZ="$MACOS_DIR/Popdoc.app.tar.gz"
SIG="$TARGZ.sig"
[[ -f "$TARGZ" && -f "$SIG" ]] || {
  echo "error: updater artifact missing ($TARGZ[.sig]). Is bundle.createUpdaterArtifacts true and the minisign key set?" >&2
  exit 1
}
DMG="$(ls -t "$DMG_DIR"/*.dmg | head -1)"

echo "==> Branding + notarizing DMG"
scripts/brand-dmg.sh "$DMG"

# The universal archive serves both arches, so both platform keys point at it
# with the same signature.
URL="https://github.com/Fresh-io/popdoc/releases/download/$TAG/Popdoc.app.tar.gz"
PUB_DATE="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
LATEST_JSON="$MACOS_DIR/latest.json"

jq -n \
  --arg v "$VERSION" \
  --arg n "$NOTES" \
  --arg d "$PUB_DATE" \
  --arg sig "$(cat "$SIG")" \
  --arg url "$URL" \
  '{version:$v, notes:$n, pub_date:$d, platforms:{
     "darwin-aarch64":{signature:$sig, url:$url},
     "darwin-x86_64":{signature:$sig, url:$url}
   }}' \
  > "$LATEST_JSON"

echo "==> Publishing GitHub release $TAG"
if gh release view "$TAG" --repo Fresh-io/popdoc >/dev/null 2>&1; then
  gh release upload "$TAG" "$DMG" "$TARGZ" "$LATEST_JSON" --repo Fresh-io/popdoc --clobber
else
  gh release create "$TAG" "$DMG" "$TARGZ" "$LATEST_JSON" \
    --repo Fresh-io/popdoc --title "Popdoc $VERSION" --notes "$NOTES"
fi

echo "==> Done. Endpoint: https://github.com/Fresh-io/popdoc/releases/latest/download/latest.json"
