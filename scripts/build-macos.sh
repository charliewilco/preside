#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/.."
cargo build --release --locked
swift build --package-path macos -c release
bin_dir="$(swift build --package-path macos -c release --show-bin-path)"
app="$(pwd)/dist/Preside.app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
cp "$bin_dir/Preside" "$app/Contents/MacOS/Preside"
cp target/release/pp "$app/Contents/Resources/pp"
cp macos/Info.plist "$app/Contents/Info.plist"
xattr -dr com.apple.FinderInfo "$app" 2>/dev/null || true
xattr -dr com.apple.ResourceFork "$app" 2>/dev/null || true
codesign --force --sign - "$app/Contents/Resources/pp"
codesign --force --sign - "$app"
codesign --verify --deep --strict "$app"
printf 'Built %s\n' "$app"
