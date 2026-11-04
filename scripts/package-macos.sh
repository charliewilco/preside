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
  : "${PRESIDE_NOTARY_PROFILE:?Set the notarytool keychain profile}"
fi
./scripts/build-macos.sh "$mode"
version="$(/usr/libexec/PlistBuddy -c 'Print :CFBundleShortVersionString' dist/Preside.app/Contents/Info.plist)"
architecture="$(uname -m)"
name="Preside-${version}-macos-${architecture}"
if [[ "$mode" == --development ]]; then name="${name}-development"; fi
scratch="$(mktemp -d "${TMPDIR:-/tmp}/preside-package.XXXXXX")"
trap 'rm -rf "$scratch"' EXIT
mkdir -p "$scratch/volume"
ditto --norsrc dist/Preside.app "$scratch/volume/Preside.app"
app="$scratch/volume/Preside.app"
notary=(--keychain-profile "${PRESIDE_NOTARY_PROFILE:-}")
if [[ -n "${PRESIDE_SIGNING_KEYCHAIN:-}" ]]; then
  notary+=(--keychain "$PRESIDE_SIGNING_KEYCHAIN")
fi
if [[ "$mode" == --distribution ]]; then
  ditto -c -k --keepParent --norsrc "$app" "$scratch/notarization.zip"
  xcrun notarytool submit "$scratch/notarization.zip" "${notary[@]}" --wait --output-format json > "$scratch/app-notarization.json"
  /usr/bin/python3 -c 'import json,sys; r=json.load(open(sys.argv[1])); sys.exit(0 if r.get("status")=="Accepted" else "App notarization was not accepted")' "$scratch/app-notarization.json"
  xcrun stapler staple "$app"
  xcrun stapler validate "$app"
  spctl --assess --type execute --verbose=2 "$app"
fi
codesign --verify --deep --strict "$app"
ditto -c -k --keepParent --norsrc "$app" "dist/${name}.zip"
ln -s /Applications "$scratch/volume/Applications"
hdiutil create -volname Preside -srcfolder "$scratch/volume" -ov -format UDZO "dist/${name}.dmg"
if [[ "$mode" == --distribution ]]; then
  signing=(--force --timestamp --sign "$PRESIDE_SIGNING_IDENTITY")
  if [[ -n "${PRESIDE_SIGNING_KEYCHAIN:-}" ]]; then
    signing+=(--keychain "$PRESIDE_SIGNING_KEYCHAIN")
  fi
  codesign "${signing[@]}" "dist/${name}.dmg"
  xcrun notarytool submit "dist/${name}.dmg" "${notary[@]}" --wait --output-format json > "$scratch/dmg-notarization.json"
  /usr/bin/python3 -c 'import json,sys; r=json.load(open(sys.argv[1])); sys.exit(0 if r.get("status")=="Accepted" else "DMG notarization was not accepted")' "$scratch/dmg-notarization.json"
  xcrun stapler staple "dist/${name}.dmg"
  xcrun stapler validate "dist/${name}.dmg"
  spctl --assess --type open --context context:primary-signature --verbose=2 "dist/${name}.dmg"
fi
(cd dist && shasum -a 256 "${name}.zip" "${name}.dmg" > "${name}.sha256")
printf 'Packaged dist/%s.{zip,dmg,sha256}\n' "$name"
