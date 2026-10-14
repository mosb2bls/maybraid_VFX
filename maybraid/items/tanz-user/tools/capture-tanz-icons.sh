#!/usr/bin/env bash
# Capture starter Tanz poses from characters-playground and convert them to
# 128px white silhouettes under maybraid/assets/iconography/tanz/.
#
# Usage:
#   maybraid/items/tanz-user/tools/capture-tanz-icons.sh
#   maybraid/items/tanz-user/tools/capture-tanz-icons.sh /tmp/tanz-raw /path/to/out
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
RAW_DIR="${1:-/tmp/maybraid-tanz-icon-raw}"
OUT_DIR="${2:-$ROOT/maybraid/assets/iconography/tanz}"
BIN="${TANZ_ICON_BIN:-$ROOT/target/debug/characters-playground}"

export WGPU_BACKEND="${WGPU_BACKEND:-vulkan}"
export WGPU_FORCE_FALLBACK_ADAPTER="${WGPU_FORCE_FALLBACK_ADAPTER:-1}"
export WGPU_ADAPTER_NAME="${WGPU_ADAPTER_NAME:-llvmpipe}"
export VK_ICD_FILENAMES="${VK_ICD_FILENAMES:-/usr/share/vulkan/icd.d/lvp_icd.json}"
export MAYBRAID_VSYNC="${MAYBRAID_VSYNC:-off}"
export CXX="${CXX:-g++-13}"

mkdir -p "$RAW_DIR" "$OUT_DIR"

if [[ ! -x "$BIN" ]]; then
  echo "building characters-playground…"
  (cd "$ROOT" && cargo build -p characters-playground)
fi

echo "capturing raw poses into $RAW_DIR"
xvfb-run -a "$BIN" capture-tanz-icons --out-dir "$RAW_DIR"

python3 "$ROOT/maybraid/items/tanz-user/tools/silhouette_from_capture.py" "$RAW_DIR" "$OUT_DIR"
echo "silhouettes written to $OUT_DIR"
