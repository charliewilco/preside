#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/.."
mode="${1:---development}"
if [[ "$mode" != --development && "$mode" != --distribution ]]; then
  printf 'Usage: %s [--development|--distribution]\n' "$0" >&2
  exit 2
fi
if [[ "$mode" == --distribution ]]; then
  : "${PRESIDE_SIGNING_IDENTITY:?Set a Developer ID Application signing identity}"
  if [[ "$(uname -m)" != arm64 ]]; then
    printf 'Distribution builds currently support Apple Silicon only.\n' >&2
    exit 1
  fi
fi
export MACOSX_DEPLOYMENT_TARGET=14.0
cargo build --release --locked
swift build --package-path macos -c release
bin_dir="$(swift build --package-path macos -c release --show-bin-path)"
version="$(cargo metadata --locked --no-deps --format-version 1 | /usr/bin/python3 -c 'import json,sys; print(json.load(sys.stdin)["packages"][0]["version"])')"
app="$(pwd)/dist/Preside.app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
cp "$bin_dir/Preside" "$app/Contents/MacOS/Preside"
cp target/release/pp "$app/Contents/Resources/pp"
cp macos/Info.plist "$app/Contents/Info.plist"
cp macos/Resources/Preside.icns "$app/Contents/Resources/Preside.icns"
/usr/libexec/PlistBuddy -c "Set :CFBundleShortVersionString $version" "$app/Contents/Info.plist"
/usr/libexec/PlistBuddy -c "Set :CFBundleVersion ${PRESIDE_BUILD_NUMBER:-1}" "$app/Contents/Info.plist"
xattr -dr com.apple.FinderInfo "$app" 2>/dev/null || true
xattr -dr com.apple.ResourceFork "$app" 2>/dev/null || true
if [[ "$mode" == --distribution ]]; then
  signing=(--force --options runtime --timestamp --sign "$PRESIDE_SIGNING_IDENTITY")
  if [[ -n "${PRESIDE_SIGNING_KEYCHAIN:-}" ]]; then
    signing+=(--keychain "$PRESIDE_SIGNING_KEYCHAIN")
  fi
  codesign "${signing[@]}" "$app/Contents/Resources/pp"
  codesign "${signing[@]}" "$app"
  for target in "$app/Contents/Resources/pp" "$app"; do
    if ! codesign --display --verbose=4 "$target" 2>&1 | /usr/bin/grep -q '^Authority=Developer ID Application:'; then
      printf 'Distribution requires a Developer ID Application signature: %s\n' "$target" >&2
      exit 1
    fi
  done
else
  codesign --force --sign - "$app/Contents/Resources/pp"
  codesign --force --sign - "$app"
fi
codesign --verify --deep --strict "$app"
printf 'Built %s (%s, %s)\n' "$app" "$version" "$mode"
