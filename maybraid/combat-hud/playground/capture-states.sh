#!/usr/bin/env bash
# Run the six-state Lubikan HUD capture under xvfb (import happens inside the binary).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
BIN="${ROOT}/target/release/combat-hud-playground"
ARTIFACTS="${ARTIFACTS_DIR:-/opt/cursor/artifacts}"

export CXX="${CXX:-g++-13}"
export CC="${CC:-gcc-13}"
export LIBRARY_PATH="${LIBRARY_PATH:-/usr/lib/gcc/x86_64-linux-gnu/13}"

mkdir -p "$ARTIFACTS"
rm -f "${ARTIFACTS}"/lubikan-hud-state-*.png

cd "$ROOT"
cargo build -p combat-hud-playground --release
xvfb-run -a env ARTIFACTS_DIR="$ARTIFACTS" "$BIN" --capture-states

echo "Captured:"
ls -la "${ARTIFACTS}"/lubikan-hud-state-*.png
md5sum "${ARTIFACTS}"/lubikan-hud-state-*.png | sort
