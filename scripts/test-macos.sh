#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/.."
scratch="$(mktemp -d "${TMPDIR:-/tmp}/preside-swift-tests.XXXXXX")"
trap 'rm -rf "$scratch"' EXIT
swift test --package-path macos --scratch-path "$scratch"
