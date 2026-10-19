#!/usr/bin/env bash
# Capture PowerDash keyframes from characters-playground under lavapipe.
#
# Usage:
#   maybraid/items/tanz-user/tools/capture-power-dash-poses.sh
#   maybraid/items/tanz-user/tools/capture-power-dash-poses.sh /tmp/power-dash-poses
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
OUT_DIR="${1:-/tmp/maybraid-power-dash-poses}"
BIN="${TANZ_ICON_BIN:-$ROOT/target/debug/characters-playground}"

export WGPU_BACKEND="${WGPU_BACKEND:-vulkan}"
export WGPU_FORCE_FALLBACK_ADAPTER="${WGPU_FORCE_FALLBACK_ADAPTER:-1}"
export WGPU_ADAPTER_NAME="${WGPU_ADAPTER_NAME:-llvmpipe}"
export VK_ICD_FILENAMES="${VK_ICD_FILENAMES:-/usr/share/vulkan/icd.d/lvp_icd.json}"
export MAYBRAID_VSYNC="${MAYBRAID_VSYNC:-off}"
export CXX="${CXX:-g++-13}"

mkdir -p "$OUT_DIR"

if [[ ! -x "$BIN" ]]; then
  echo "building characters-playground…"
  (cd "$ROOT" && cargo build -p characters-playground)
fi

echo "capturing PowerDash poses into $OUT_DIR"
xvfb-run -a "$BIN" capture-power-dash-poses --out-dir "$OUT_DIR"
echo "poses written to $OUT_DIR"
